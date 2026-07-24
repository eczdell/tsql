use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::app::{ActiveTab, AppState, FocusedPanel};

pub enum AppAction {
    None,
    Quit,
    Connect,
    RefreshTables,
    ExecuteQuery,
    FetchTableData(String, String),
    FetchTableDataPage(String, String, usize),
    SwitchDatabase(String),
    SaveConnection(crate::config::ConnectionConfig),
}

pub fn handle_key(key: KeyEvent, app: &mut AppState) -> AppAction {
    if app.is_filtering {
        let mut changed = false;
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.is_filtering = false;
            }
            KeyCode::Backspace => {
                app.filter_text.pop();
                changed = true;
            }
            KeyCode::Char(c) => {
                app.filter_text.push(c);
                changed = true;
            }
            _ => {}
        }
        if changed {
            app.selected_table_idx = 0;
            if let Some(tbl) = app.filtered_tables().get(0) {
                return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
            }
        }
        return AppAction::None;
    }

    if app.is_adding_conn {
        match key.code {
            KeyCode::Esc => {
                app.is_adding_conn = false;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.conn_form_step = (app.conn_form_step + 1) % 6;
            }
            KeyCode::Up => {
                if app.conn_form_step > 0 {
                    app.conn_form_step -= 1;
                } else {
                    app.conn_form_step = 5;
                }
            }
            KeyCode::Backspace => {
                let target = match app.conn_form_step {
                    0 => &mut app.conn_input_name,
                    1 => &mut app.conn_input_host,
                    2 => &mut app.conn_input_port,
                    3 => &mut app.conn_input_user,
                    4 => &mut app.conn_input_pass,
                    _ => &mut app.conn_input_dbname,
                };
                target.pop();
            }
            KeyCode::Enter => {
                if app.conn_form_step < 5 {
                    app.conn_form_step += 1;
                } else {
                    // Form complete - create connection profile
                    let port_val = app.conn_input_port.parse::<u16>().unwrap_or(5432);
                    let new_cfg = crate::config::ConnectionConfig {
                        name: if app.conn_input_name.is_empty() { "new_connection".to_string() } else { app.conn_input_name.clone() },
                        host: if app.conn_input_host.is_empty() { "127.0.0.1".to_string() } else { app.conn_input_host.clone() },
                        port: port_val,
                        user: if app.conn_input_user.is_empty() { "postgres".to_string() } else { app.conn_input_user.clone() },
                        password: if app.conn_input_pass.is_empty() { None } else { Some(app.conn_input_pass.clone()) },
                        dbname: if app.conn_input_dbname.is_empty() { "postgres".to_string() } else { app.conn_input_dbname.clone() },
                        sslmode: None,
                    };
                    app.is_adding_conn = false;
                    return AppAction::SaveConnection(new_cfg);
                }
            }
            KeyCode::Char(c) => {
                let target = match app.conn_form_step {
                    0 => &mut app.conn_input_name,
                    1 => &mut app.conn_input_host,
                    2 => &mut app.conn_input_port,
                    3 => &mut app.conn_input_user,
                    4 => &mut app.conn_input_pass,
                    _ => &mut app.conn_input_dbname,
                };
                target.push(c);
            }
            _ => {}
        }
        return AppAction::None;
    }

    if app.focused_panel == FocusedPanel::QueryEditor && app.active_tab == ActiveTab::QueryRunner {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('e') {
            return AppAction::ExecuteQuery;
        }
        match key.code {
            KeyCode::Esc => {
                app.focused_panel = FocusedPanel::Tables;
            }
            KeyCode::Char(c) => {
                app.sql_input.push(c);
            }
            KeyCode::Backspace => {
                app.sql_input.pop();
            }
            KeyCode::Enter => {
                app.sql_input.push('\n');
            }
            KeyCode::Tab => {
                app.focused_panel = FocusedPanel::Results;
            }
            _ => {}
        }
        return AppAction::None;
    }

    // Global Keybindings
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return AppAction::Quit;
    }

    match key.code {
        KeyCode::Char('q') => return AppAction::Quit,
        KeyCode::Esc => {
            if app.is_fullscreen_data {
                app.is_fullscreen_data = false;
                app.focused_panel = FocusedPanel::Tables;
            } else {
                return AppAction::Quit;
            }
        }
        KeyCode::Char('1') => app.active_tab = ActiveTab::Browser,
        KeyCode::Char('2') => app.active_tab = ActiveTab::Databases,
        KeyCode::Char('3') => app.active_tab = ActiveTab::Users,
        KeyCode::Char('4') => app.active_tab = ActiveTab::QueryRunner,
        KeyCode::Char('5') => app.active_tab = ActiveTab::Connections,
        KeyCode::Char('?') => app.active_tab = ActiveTab::Help,
        
        KeyCode::Char('s') => {
            app.show_system_tables = !app.show_system_tables;
            app.selected_table_idx = 0;
            if let Some(tbl) = app.filtered_tables().get(0) {
                return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
            }
        }
        KeyCode::Char('/') => {
            if app.active_tab == ActiveTab::Browser {
                app.is_filtering = !app.is_filtering;
            }
        }
        KeyCode::Char('a') => {
            if app.active_tab == ActiveTab::Connections {
                app.is_adding_conn = true;
                app.conn_form_step = 0;
                app.conn_input_name.clear();
                app.conn_input_host = "127.0.0.1".to_string();
                app.conn_input_port = "5432".to_string();
                app.conn_input_user = "postgres".to_string();
                app.conn_input_pass.clear();
                app.conn_input_dbname = "postgres".to_string();
            }
        }
        KeyCode::Delete | KeyCode::Char('d') => {
            if app.active_tab == ActiveTab::Connections {
                if !app.config.connections.is_empty() && app.selected_conn_idx < app.config.connections.len() {
                    app.config.connections.remove(app.selected_conn_idx);
                    let _ = crate::config::save_config(&app.config);
                    if app.selected_conn_idx > 0 && app.selected_conn_idx >= app.config.connections.len() {
                        app.selected_conn_idx -= 1;
                    }
                    app.status_message = "Connection profile deleted & saved.".to_string();
                }
            }
        }
        KeyCode::Char('c') => return AppAction::Connect,
        KeyCode::Char('r') => return AppAction::RefreshTables,
        
        KeyCode::Tab => {
            if app.active_tab == ActiveTab::Browser {
                app.focused_panel = match app.focused_panel {
                    FocusedPanel::Tables => FocusedPanel::DataPreview,
                    _ => FocusedPanel::Tables,
                };
            } else if app.active_tab == ActiveTab::QueryRunner {
                app.focused_panel = match app.focused_panel {
                    FocusedPanel::Tables | FocusedPanel::DataPreview => FocusedPanel::QueryEditor,
                    FocusedPanel::QueryEditor => FocusedPanel::Results,
                    FocusedPanel::Results => FocusedPanel::QueryEditor,
                };
            }
        }

        KeyCode::Char('+') | KeyCode::Char('=') => {
            if app.active_tab == ActiveTab::Browser {
                app.cell_width = (app.cell_width + 4).min(80);
            }
        }

        KeyCode::Char('-') | KeyCode::Char('_') => {
            if app.active_tab == ActiveTab::Browser {
                app.cell_width = app.cell_width.saturating_sub(4).max(8);
            }
        }

        KeyCode::Char('m') | KeyCode::Char('z') => {
            if app.active_tab == ActiveTab::Browser {
                app.is_fullscreen_data = !app.is_fullscreen_data;
                if app.is_fullscreen_data {
                    app.focused_panel = FocusedPanel::DataPreview;
                }
            }
        }

        KeyCode::Left | KeyCode::Char('h') => {
            if app.active_tab == ActiveTab::Browser {
                if app.focused_panel == FocusedPanel::DataPreview {
                    if app.selected_data_col > 0 {
                        app.selected_data_col -= 1;
                        if app.selected_data_col < app.data_col_offset {
                            app.data_col_offset = app.selected_data_col;
                        }
                    }
                } else if app.data_col_offset > 0 {
                    app.data_col_offset -= 1;
                }
            }
        }

        KeyCode::Right | KeyCode::Char('l') => {
            if app.active_tab == ActiveTab::Browser {
                if app.focused_panel == FocusedPanel::DataPreview {
                    if let Some(ref res) = app.table_data_result {
                        if !res.columns.is_empty() {
                            app.selected_data_col = (app.selected_data_col + 1).min(res.columns.len() - 1);
                            if app.selected_data_col >= app.data_col_offset + 5 {
                                app.data_col_offset = app.selected_data_col.saturating_sub(4);
                            }
                        }
                    }
                } else {
                    app.data_col_offset = app.data_col_offset.saturating_add(1);
                }
            }
        }

        KeyCode::Char('n') => {
            if app.active_tab == ActiveTab::Browser {
                app.data_page = app.data_page.saturating_add(1);
                if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                    return AppAction::FetchTableDataPage(tbl.schema.clone(), tbl.name.clone(), app.data_page);
                }
            }
        }

        KeyCode::Char('p') => {
            if app.active_tab == ActiveTab::Browser && app.data_page > 0 {
                app.data_page -= 1;
                if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                    return AppAction::FetchTableDataPage(tbl.schema.clone(), tbl.name.clone(), app.data_page);
                }
            }
        }

        KeyCode::Down | KeyCode::Char('j') => match app.active_tab {
            ActiveTab::Browser => {
                if app.focused_panel == FocusedPanel::DataPreview {
                    if let Some(ref res) = app.table_data_result {
                        if !res.rows.is_empty() {
                            app.selected_data_row = (app.selected_data_row + 1).min(res.rows.len() - 1);
                            if app.selected_data_row >= app.data_scroll_offset + 10 {
                                app.data_scroll_offset += 1;
                            }
                        }
                    }
                } else {
                    let count = app.filtered_tables().len();
                    if count > 0 {
                        app.selected_table_idx = (app.selected_table_idx + 1).min(count - 1);
                        app.data_page = 0;
                        app.data_scroll_offset = 0;
                        app.selected_data_row = 0;
                        app.selected_data_col = 0;
                        if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                            return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
                        }
                    }
                }
            }
            ActiveTab::Databases => {
                if let Some(ref res) = app.databases_result {
                    if !res.rows.is_empty() {
                        app.selected_db_idx = (app.selected_db_idx + 1).min(res.rows.len() - 1);
                    }
                }
            }
            ActiveTab::Connections => {
                if !app.config.connections.is_empty() {
                    app.selected_conn_idx = (app.selected_conn_idx + 1).min(app.config.connections.len() - 1);
                }
            }
            ActiveTab::QueryRunner => {
                if app.focused_panel == FocusedPanel::Results {
                    app.result_scroll = app.result_scroll.saturating_add(1);
                }
            }
            _ => {}
        },

        KeyCode::Up | KeyCode::Char('k') => match app.active_tab {
            ActiveTab::Browser => {
                if app.focused_panel == FocusedPanel::DataPreview {
                    if app.selected_data_row > 0 {
                        app.selected_data_row -= 1;
                        if app.selected_data_row < app.data_scroll_offset {
                            app.data_scroll_offset = app.selected_data_row;
                        }
                    }
                } else if app.selected_table_idx > 0 {
                    app.selected_table_idx -= 1;
                    app.data_page = 0;
                    app.data_scroll_offset = 0;
                    app.selected_data_row = 0;
                    app.selected_data_col = 0;
                    if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                        return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
                    }
                }
            }
            ActiveTab::Databases => {
                if app.selected_db_idx > 0 {
                    app.selected_db_idx -= 1;
                }
            }
            ActiveTab::Connections => {
                if app.selected_conn_idx > 0 {
                    app.selected_conn_idx -= 1;
                }
            }
            ActiveTab::QueryRunner => {
                if app.focused_panel == FocusedPanel::Results {
                    app.result_scroll = app.result_scroll.saturating_sub(1);
                }
            }
            _ => {}
        },

        KeyCode::Char('b') => {
            if app.active_tab == ActiveTab::Browser && app.active_breadcrumb_idx > 0 {
                app.active_breadcrumb_idx -= 1;
                if let Some(target_tag) = app.breadcrumbs.get(app.active_breadcrumb_idx).cloned() {
                    let parts: Vec<&str> = target_tag.split('.').collect();
                    if parts.len() == 2 {
                        let target_schema = parts[0];
                        let target_name = parts[1];
                        if let Some(pos) = app.tables.iter().position(|t| t.schema == target_schema && t.name == target_name) {
                            app.selected_table_idx = pos;
                            app.data_page = 0;
                            app.data_scroll_offset = 0;
                            app.selected_data_row = 0;
                            app.selected_data_col = 0;
                            return AppAction::FetchTableData(target_schema.to_string(), target_name.to_string());
                        }
                    }
                }
            }
        }

        KeyCode::Char('f') => {
            if app.active_tab == ActiveTab::Browser {
                if !app.breadcrumbs.is_empty() && app.active_breadcrumb_idx + 1 < app.breadcrumbs.len() {
                    app.active_breadcrumb_idx += 1;
                    if let Some(target_tag) = app.breadcrumbs.get(app.active_breadcrumb_idx).cloned() {
                        let parts: Vec<&str> = target_tag.split('.').collect();
                        if parts.len() == 2 {
                            let target_schema = parts[0];
                            let target_name = parts[1];
                            if let Some(pos) = app.tables.iter().position(|t| t.schema == target_schema && t.name == target_name) {
                                app.selected_table_idx = pos;
                                app.data_page = 0;
                                app.data_scroll_offset = 0;
                                app.selected_data_row = 0;
                                app.selected_data_col = 0;
                                return AppAction::FetchTableData(target_schema.to_string(), target_name.to_string());
                            }
                        }
                    }
                } else {
                    app.is_filtering = true;
                }
            }
        }

        KeyCode::Enter => match app.active_tab {
            ActiveTab::Browser => {
                if app.focused_panel == FocusedPanel::DataPreview {
                    if let Some(ref res) = app.table_data_result {
                        if let Some(col_name) = res.columns.get(app.selected_data_col) {
                            let mut target_schema_name: Option<(String, String)> = None;

                            // 1. Check explicit Foreign Key constraints
                            if let Some(fk) = app.foreign_keys.iter().find(|k| k.column_name == *col_name) {
                                target_schema_name = Some((fk.foreign_table_schema.clone(), fk.foreign_table_name.clone()));
                            } else {
                                // 2. Polymorphic entity_id + entity_type heuristic (e.g. audit_logs)
                                if col_name == "entity_id" {
                                    if let Some(type_col_idx) = res.columns.iter().position(|c| c == "entity_type" || c == "entity") {
                                        if let Some(row) = res.rows.get(app.selected_data_row) {
                                            if let Some(entity_val) = row.get(type_col_idx) {
                                                let clean_entity = entity_val.trim().to_lowercase();
                                                let plural_entity = if clean_entity.ends_with('s') {
                                                    clean_entity.clone()
                                                } else {
                                                    format!("{}s", clean_entity)
                                                };
                                                if let Some(t) = app.tables.iter().find(|t| t.name == clean_entity || t.name == plural_entity) {
                                                    target_schema_name = Some((t.schema.clone(), t.name.clone()));
                                                }
                                            }
                                        }
                                    }
                                }

                                // 3. Smart _id suffix matching (e.g. case_id -> cases, reporter_id -> users/roles)
                                if target_schema_name.is_none() && col_name.ends_with("_id") {
                                    let base_name = col_name.trim_end_matches("_id").to_lowercase();
                                    let plural_name = format!("{}s", base_name);
                                    if let Some(t) = app.tables.iter().find(|t| t.name == base_name || t.name == plural_name) {
                                        target_schema_name = Some((t.schema.clone(), t.name.clone()));
                                    }
                                }
                            }

                            if let Some((target_schema, target_name)) = target_schema_name {
                                // Initialize or append to breadcrumbs chain
                                if let Some(cur_tbl) = app.filtered_tables().get(app.selected_table_idx) {
                                    let current_tag = format!("{}.{}", cur_tbl.schema, cur_tbl.name);
                                    if app.breadcrumbs.is_empty() {
                                        app.breadcrumbs.push(current_tag);
                                        app.active_breadcrumb_idx = 0;
                                    } else if app.active_breadcrumb_idx + 1 < app.breadcrumbs.len() {
                                        app.breadcrumbs.truncate(app.active_breadcrumb_idx + 1);
                                    }
                                }

                                let target_tag = format!("{}.{}", target_schema, target_name);
                                app.breadcrumbs.push(target_tag);
                                app.active_breadcrumb_idx = app.breadcrumbs.len() - 1;

                                if let Some(pos) = app.tables.iter().position(|t| t.schema == target_schema && t.name == target_name) {
                                    app.selected_table_idx = pos;
                                    app.focused_panel = FocusedPanel::DataPreview;
                                    app.data_page = 0;
                                    app.data_scroll_offset = 0;
                                    app.selected_data_row = 0;
                                    app.selected_data_col = 0;
                                    return AppAction::FetchTableData(target_schema, target_name);
                                }
                            }
                        }
                    }
                } else if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                    return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
                }
            }
            ActiveTab::Databases => {
                if let Some(ref res) = app.databases_result {
                    if let Some(row) = res.rows.get(app.selected_db_idx) {
                        if let Some(db_name) = row.get(0) {
                            return AppAction::SwitchDatabase(db_name.clone());
                        }
                    }
                }
            }
            ActiveTab::Connections => {
                return AppAction::Connect;
            }
            _ => {}
        },

        _ => {}
    }

    AppAction::None
}
