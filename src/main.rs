mod ansi;
mod app;
mod config;
mod db;
mod input;
mod ui;

use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use crate::app::AppState;
use crate::config::{load_config, ConnectionConfig};
use crate::input::{handle_key, AppAction};

#[derive(Parser, Debug)]
#[command(name = "tsql", author, version, about = "Terminal GUI visualizer for PostgreSQL", long_about = None)]
struct Args {
    /// Host for connection
    #[arg(short = 'H', long)]
    host: Option<String>,

    /// Port for connection
    #[arg(short = 'P', long)]
    port: Option<u16>,

    /// Database user
    #[arg(short = 'U', long)]
    user: Option<String>,

    /// Database name
    #[arg(short = 'd', long)]
    dbname: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut config = load_config();

    // Override with CLI flags if provided
    if args.host.is_some() || args.port.is_some() || args.user.is_some() || args.dbname.is_some() {
        let cli_conn = ConnectionConfig {
            name: "cli-override".to_string(),
            host: args.host.unwrap_or_else(|| "localhost".to_string()),
            port: args.port.unwrap_or(5432),
            user: args.user.unwrap_or_else(|| "postgres".to_string()),
            password: None,
            dbname: args.dbname.unwrap_or_else(|| "postgres".to_string()),
            sslmode: Some("disable".to_string()),
        };
        config.connections.insert(0, cli_conn);
    }

    let mut app = AppState::new(config);

    // Initial connection attempt
    attempt_connect(&mut app).await;

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut app).await;

    // Explicitly release all cached memory structures and close database connection
    app.client = None;
    app.tables.clear();
    app.columns.clear();
    app.query_result = None;
    app.table_data_result = None;
    app.databases_result = None;
    app.users_result = None;
    drop(app);

    // Terminal restore
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Application Error: {:?}", err);
    }

    Ok(())
}

async fn attempt_connect(app: &mut AppState) {
    if let Some(conn_cfg) = app.current_connection().cloned() {
        app.status_message = format!("Connecting to {} ({}:{})...", conn_cfg.name, conn_cfg.host, conn_cfg.port);
        // Load table cache immediately so UI doesn't look empty while connecting
        if let Some(cached_tables) = db::load_table_cache(&conn_cfg.name) {
            app.tables = cached_tables;
            app.selected_table_idx = 0;
            app.update_filtered_tables();
        }
        match db::connect(&conn_cfg).await {
            Ok(client) => {
                app.client = Some(client);
                app.connected = true;
                app.status_message = format!("Connected to PostgreSQL ({}:{})", conn_cfg.host, conn_cfg.port);
                refresh_tables(app).await;
            }
            Err(err) => {
                app.client = None;
                app.connected = false;
                app.status_message = format!("Connection failed: {}", err);
            }
        }
    }
}

async fn refresh_tables(app: &mut AppState) {
    app.column_cache.clear();
    app.table_data_cache.clear();
    let mut schema_table_to_fetch = None;

    if let Some(ref client) = app.client {
        if let Ok(all_fks) = db::fetch_all_foreign_keys(client).await {
            app.all_foreign_keys = all_fks;
        } else {
            app.all_foreign_keys.clear();
        }
        match db::fetch_tables(client).await {
            Ok(tables) => {
                // Save to persistent file cache
                if let Some(conn_cfg) = app.current_connection() {
                    let _ = db::save_table_cache(&conn_cfg.name, &tables);
                }
                app.tables = tables;
                app.selected_table_idx = 0;
                app.update_filtered_tables();
                let filtered = app.filtered_tables();
                if let Some(first) = filtered.first() {
                    schema_table_to_fetch = Some((first.schema.clone(), first.name.clone()));
                }
            }
            Err(err) => {
                app.status_message = format!("Error fetching tables: {}", err);
            }
        }
    }

    if let Some((schema, name)) = schema_table_to_fetch {
        fetch_table_schema_and_data(app, &schema, &name).await;
    }
}

async fn fetch_table_schema_and_data(app: &mut AppState, schema: &str, table_name: &str) {
    app.field_search_text.clear();
    app.is_field_searching = false;
    let cache_key = format!("{}.{}", schema, table_name);

    // Fast Cache Check
    let mut hit_cols = false;
    let mut hit_data = false;

    if let Some(cached_cols) = app.column_cache.get(&cache_key) {
        app.columns = cached_cols.clone();
        hit_cols = true;
    }

    if app.is_filtering {
        app.table_data_result = None;
        hit_data = true;
    } else if let Some(cached_data) = app.table_data_cache.get(&cache_key) {
        app.table_data_result = Some(cached_data.clone());
        hit_data = true;
    }

    app.data_col_offset = 0;
    app.selected_data_col = 0;
    app.selected_data_row = 0;
    app.data_scroll_offset = 0;

    if hit_cols && hit_data {
        return; // Ultra-fast return on cache hit, zero network overhead
    }

    if let Some(ref client) = app.client {
        if let Ok(fks) = db::fetch_foreign_keys(client, schema, table_name).await {
            app.foreign_keys = fks;
        } else {
            app.foreign_keys.clear();
        }
        if !hit_cols {
            match db::fetch_columns(client, schema, table_name).await {
                Ok(cols) => {
                    app.column_cache.insert(cache_key.clone(), cols.clone());
                    app.columns = cols;
                }
                Err(_) => app.columns.clear(),
            }
        }

        if !hit_data {
            let sql = format!("SELECT * FROM \"{}\".\"{}\" LIMIT 50 OFFSET {};", schema, table_name, app.data_page * 50);
            match db::execute_sql(client, &sql).await {
                Ok(res) => {
                    app.table_data_cache.insert(cache_key, res.clone());
                    app.table_data_result = Some(res);
                }
                Err(e) => {
                    app.table_data_result = None;
                    app.status_message = format!("Cannot preview table {}.{}: {}", schema, table_name, e);
                }
            }
        }
    }
}

async fn fetch_table_data_page(app: &mut AppState, schema: &str, table_name: &str, page: usize) {
    if let Some(ref client) = app.client {
        let sql = format!("SELECT * FROM \"{}\".\"{}\" LIMIT 50 OFFSET {};", schema, table_name, page * 50);
        match db::execute_sql(client, &sql).await {
            Ok(res) => {
                app.table_data_result = Some(res);
                app.data_scroll_offset = 0;
            }
            Err(e) => {
                app.status_message = format!("Error loading page {}: {}", page + 1, e);
            }
        }
    }
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut AppState,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::render_ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let action = handle_key(key, app);
                match action {
                    AppAction::Quit => break,
                    AppAction::SaveConnection(cfg, edit_idx_opt) => {
                        if let Some(idx) = edit_idx_opt {
                            if idx < app.config.connections.len() {
                                app.config.connections[idx] = cfg;
                                app.selected_conn_idx = idx;
                                app.status_message = "Updated connection profile! Connecting...".to_string();
                            }
                        } else {
                            app.config.connections.push(cfg);
                            app.selected_conn_idx = app.config.connections.len() - 1;
                            app.status_message = "Saved new connection profile! Connecting...".to_string();
                        }
                        let _ = crate::config::save_config(&app.config);
                        attempt_connect(app).await;
                    }
                    AppAction::Connect => {
                        app.is_loading = true;
                        let _ = terminal.draw(|f| ui::render_ui(f, app));
                        attempt_connect(app).await;
                        app.is_loading = false;
                    }
                    AppAction::RefreshTables => {
                        app.is_loading = true;
                        let _ = terminal.draw(|f| ui::render_ui(f, app));
                        refresh_tables(app).await;
                        app.is_loading = false;
                    }
                    AppAction::FetchTableData(schema, tbl) => {
                        app.is_loading = true;
                        let _ = terminal.draw(|f| ui::render_ui(f, app));
                        fetch_table_schema_and_data(app, &schema, &tbl).await;
                        app.is_loading = false;
                    }
                    AppAction::FetchTableDataPage(schema, tbl, page) => {
                        app.is_loading = true;
                        let _ = terminal.draw(|f| ui::render_ui(f, app));
                        fetch_table_data_page(app, &schema, &tbl, page).await;
                        app.is_loading = false;
                    }
                    AppAction::SwitchDatabase(db_name) => {
                        if let Some(conn) = app.config.connections.get_mut(app.selected_conn_idx) {
                            conn.dbname = db_name.clone();
                            app.status_message = format!("Switched active database to '{}'. Reconnecting...", db_name);
                        }
                        app.is_loading = true;
                        let _ = terminal.draw(|f| ui::render_ui(f, app));
                        attempt_connect(app).await;
                        app.is_loading = false;
                        app.active_tab = crate::app::ActiveTab::Browser;
                    }
                    AppAction::ExecuteQuery => {
                        if let Some(ref client) = app.client {
                            let sql = app.sql_input.clone();
                            match db::execute_sql(client, &sql).await {
                                Ok(res) => {
                                    app.selected_query_row = 0;
                                    app.selected_query_col = 0;
                                    app.result_scroll = 0;
                                    app.query_col_offset = 0;
                                    if !res.rows.is_empty() {
                                        app.focused_panel = crate::app::FocusedPanel::Results;
                                        app.status_message = format!("Query returned {} rows! Navigating results (hjkl / Arrows)...", res.rows.len());
                                    } else {
                                        app.status_message = "Query executed successfully (0 rows returned).".to_string();
                                    }
                                    app.query_result = Some(res);
                                    app.query_error = None;
                                }
                                Err(err) => {
                                    app.query_result = None;
                                    app.query_error = Some(err.to_string());
                                    app.status_message = "Query execution failed.".to_string();
                                }
                            }
                        } else {
                            app.status_message = "Not connected to PostgreSQL!".to_string();
                        }
                    }
                    AppAction::None => {}
                }
            }
        }

        // Lazy load active tab data & selected table on demand
        match app.active_tab {
            crate::app::ActiveTab::Browser => {
                if app.is_filtering {
                    app.columns.clear();
                    app.table_data_result = None;
                } else if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                    let cache_key = format!("{}.{}", tbl.schema, tbl.name);
                    if app.table_data_result.is_none() || !app.table_data_cache.contains_key(&cache_key) {
                        let schema = tbl.schema.clone();
                        let name = tbl.name.clone();
                        app.is_loading = true;
                        let _ = terminal.draw(|f| ui::render_ui(f, app));
                        fetch_table_schema_and_data(app, &schema, &name).await;
                        app.is_loading = false;
                    }
                }
            }
            crate::app::ActiveTab::Databases if app.databases_result.is_none() => {
                if let Some(ref client) = app.client {
                    if let Ok(dbs) = db::execute_sql(client, "SELECT datname as database_name, pg_size_pretty(pg_database_size(datname)) as size, datcollate as collation FROM pg_database WHERE datistemplate = false;").await {
                        app.databases_result = Some(dbs);
                    }
                }
            }
            crate::app::ActiveTab::Users if app.users_result.is_none() => {
                if let Some(ref client) = app.client {
                    if let Ok(users) = db::execute_sql(client, "SELECT rolname as username, rolsuper as is_superuser, rolcreaterole as can_create_role, rolcreatedb as can_create_db, rolcanlogin as can_login FROM pg_roles ORDER BY rolname;").await {
                        app.users_result = Some(users);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}
