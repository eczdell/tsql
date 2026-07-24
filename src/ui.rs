use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Table as RatatuiTable, Row, Tabs, Wrap},
    Frame,
};

use crate::app::{ActiveTab, AppState, FocusedPanel};

pub fn render_ui(f: &mut Frame, app: &AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(0),    // Content Area
            Constraint::Length(1), // Footer / Status bar
        ])
        .split(f.area());

    render_header(f, app, main_chunks[0]);
    
    match app.active_tab {
        ActiveTab::Browser => render_browser(f, app, main_chunks[1]),
        ActiveTab::Databases => render_databases(f, app, main_chunks[1]),
        ActiveTab::Users => render_users(f, app, main_chunks[1]),
        ActiveTab::QueryRunner => render_query_runner(f, app, main_chunks[1]),
        ActiveTab::Connections => render_connections(f, app, main_chunks[1]),
        ActiveTab::Help => render_help(f, app, main_chunks[1]),
    }

    render_status_bar(f, app, main_chunks[2]);
}

fn render_header(f: &mut Frame, app: &AppState, area: Rect) {
    let titles = vec![
        " [1] Tables ",
        " [2] Databases ",
        " [3] Users/Roles ",
        " [4] Query Runner ",
        " [5] Connections ",
        " [?] Help ",
    ];
    let selected_idx = match app.active_tab {
        ActiveTab::Browser => 0,
        ActiveTab::Databases => 1,
        ActiveTab::Users => 2,
        ActiveTab::QueryRunner => 3,
        ActiveTab::Connections => 4,
        ActiveTab::Help => 5,
    };

    let conn_name = app.current_connection().map(|c| c.name.as_str()).unwrap_or("None");
    let conn_status = if app.connected {
        Span::styled(format!(" ● Connected ({}) ", conn_name), Style::default().fg(Color::Green))
    } else {
        Span::styled(" ○ Disconnected ", Style::default().fg(Color::Red))
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(vec![
                    Span::styled(" tsql ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("— PostgreSQL Terminal Visualizer "),
                ])
                .title_alignment(ratatui::layout::Alignment::Left),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .select(selected_idx);

    f.render_widget(tabs, area);

    // Overlay connection indicator on the right of header
    let status_rect = Rect {
        x: area.width.saturating_sub(25),
        y: area.y,
        width: 24,
        height: 1,
    };
    f.render_widget(Paragraph::new(conn_status), status_rect);
}

fn render_browser(f: &mut Frame, app: &AppState, area: Rect) {
    if app.is_fullscreen_data {
        render_fullscreen_data(f, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // Left Panel: Table List
    let tables = app.filtered_tables();
    let items: Vec<ListItem> = tables
        .iter()
        .enumerate()
        .map(|(idx, tbl)| {
            let is_sel = idx == app.selected_table_idx;
            let prefix = if is_sel { "❯ " } else { "  " };
            let style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Cyan)),
                Span::styled(&tbl.name, style),
                Span::styled(format!(" ({})", tbl.schema), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let filter_display = if app.is_filtering {
        format!(" [Fuzzy Filter: {}█]", app.filter_text)
    } else if !app.filter_text.is_empty() {
        format!(" [Fuzzy Filter: {}]", app.filter_text)
    } else {
        " [Press / to Search]".to_string()
    };

    let table_list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Tables ({}){}", tables.len(), filter_display))
            .border_style(if app.is_filtering || app.focused_panel == FocusedPanel::Tables {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            }),
    );
    f.render_widget(table_list, chunks[0]);

    // Right Panel: Schema + Data Preview
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[1]);

    // Columns schema
    let col_items: Vec<ListItem> = app
        .columns
        .iter()
        .map(|col| {
            let is_fk = app.foreign_keys.iter().any(|fk| fk.column_name == col.name)
                || col.name == "entity_id"
                || col.name.ends_with("_id");

            let badge_span = if col.is_primary_key && is_fk {
                Span::styled("[PK/FK] ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD))
            } else if col.is_primary_key {
                Span::styled("[PK]    ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else if is_fk {
                Span::styled("[FK]    ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("        ", Style::default())
            };

            ListItem::new(Line::from(vec![
                badge_span,
                Span::styled(format!("{:<20}", col.name), Style::default().fg(Color::Green)),
                Span::styled(format!("{:<16}", col.data_type), Style::default().fg(Color::Yellow)),
                Span::styled(
                    if col.is_nullable == "YES" { "NULL" } else { "NOT NULL" },
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let selected_table_name = tables
        .get(app.selected_table_idx)
        .map(|t| t.name.as_str())
        .unwrap_or("None");

    let col_list = List::new(col_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Columns: {} ", selected_table_name))
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(col_list, right_chunks[0]);

    // Data Preview Table
    let data_focused = app.focused_panel == FocusedPanel::DataPreview;
    let data_block = Block::default()
        .borders(Borders::ALL)
        .border_style(if data_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if let Some(ref res) = app.table_data_result {
        let visible_cols = &res.columns[app.data_col_offset.min(res.columns.len())..];
        let header_cells = visible_cols.iter().enumerate().map(|(rel_idx, h)| {
            let actual_col_idx = app.data_col_offset + rel_idx;
            let is_col_selected = data_focused && actual_col_idx == app.selected_data_col;
            let is_pk = app.columns.iter().any(|c| c.name == *h && c.is_primary_key) || *h == "id";
            let is_fk = app.foreign_keys.iter().any(|fk| fk.column_name == *h)
                || *h == "entity_id"
                || h.ends_with("_id");

            let col_name_fmt = if is_pk && is_fk {
                format!(" 🔑🔑 PK/FK: {} ", h)
            } else if is_pk {
                format!(" 🔑 PK: {} ", h)
            } else if is_fk {
                format!(" 🔗 FK: {} ", h)
            } else {
                format!(" {} ", h)
            };
            
            let style = if is_col_selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else if is_pk && is_fk {
                Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
            } else if is_pk {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if is_fk {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };
            ratatui::widgets::Cell::from(Span::styled(col_name_fmt, style))
        });
        let header = Row::new(header_cells).height(1);

        let rows = res.rows.iter().enumerate().skip(app.data_scroll_offset).map(|(r_idx, row)| {
            let visible_cells = &row[app.data_col_offset.min(row.len())..];
            let cells = visible_cells.iter().enumerate().map(|(c_idx, c)| {
                let actual_col_idx = app.data_col_offset + c_idx;
                let is_cell_selected = data_focused && r_idx == app.selected_data_row && actual_col_idx == app.selected_data_col;
                
                if is_cell_selected {
                    let cell_content = format!("▶[ {} ]◀", c);
                    let style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
                    ratatui::widgets::Cell::from(Span::styled(cell_content, style))
                } else {
                    let style = if c == "NULL" {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ratatui::widgets::Cell::from(Span::styled(format!(" {} ", c), style))
                }
            });
            Row::new(cells)
        });

        let widths: Vec<Constraint> = visible_cols.iter().map(|_| Constraint::Length(app.cell_width)).collect();

        // Check if currently selected cell is a Foreign Key or Relational ID
        let cur_col_name = res.columns.get(app.selected_data_col).cloned().unwrap_or_default();
        let fk_info = app.foreign_keys.iter().find(|fk| fk.column_name == cur_col_name);
        
        let fk_hint = if let Some(fk) = fk_info {
            format!(" [🔗 FK -> {}.{} | Press Enter to Jump]", fk.foreign_table_schema, fk.foreign_table_name)
        } else if cur_col_name == "entity_id" {
            let target_name = if let Some(type_idx) = res.columns.iter().position(|c| c == "entity_type" || c == "entity") {
                res.rows.get(app.selected_data_row).and_then(|r| r.get(type_idx)).cloned().unwrap_or_default()
            } else {
                "entity".to_string()
            };
            format!(" [🔗 Jump -> {} | Press Enter]", target_name)
        } else if cur_col_name.ends_with("_id") {
            let base = cur_col_name.trim_end_matches("_id");
            format!(" [🔗 Jump -> {}s | Press Enter]", base)
        } else {
            "".to_string()
        };

        let breadcrumb_str = if app.breadcrumbs.is_empty() {
            "".to_string()
        } else {
            let items: Vec<String> = app.breadcrumbs.iter().enumerate().map(|(idx, b)| {
                if idx == app.active_breadcrumb_idx {
                    format!("•{}•", b)
                } else {
                    b.clone()
                }
            }).collect();
            format!(" [{}] ", items.join(" ➔ "))
        };

        let title_text = format!(
            " Data View{} — Page {} (R{} C{}){} [Enter=Jump, b/f=Nav, +/-=Zoom, m=Fullscreen] ",
            breadcrumb_str,
            app.data_page + 1,
            app.selected_data_row + 1,
            app.selected_data_col + 1,
            fk_hint
        );

        let table_widget = RatatuiTable::new(rows, widths)
            .header(header)
            .block(data_block.title(title_text));
        f.render_widget(table_widget, right_chunks[1]);
    } else {
        let empty_msg = Paragraph::new(" No table selected or empty data ")
            .block(data_block.title(" Data View "));
        f.render_widget(empty_msg, right_chunks[1]);
    }
}

fn render_fullscreen_data(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let selected_table_name = app.filtered_tables()
        .get(app.selected_table_idx)
        .map(|t| format!("{}.{}", t.schema, t.name))
        .unwrap_or_else(|| "Table".to_string());

    if let Some(ref res) = app.table_data_result {
        let visible_cols = &res.columns[app.data_col_offset.min(res.columns.len())..];
        let header_cells = visible_cols.iter().enumerate().map(|(rel_idx, h)| {
            let actual_col_idx = app.data_col_offset + rel_idx;
            let is_col_selected = actual_col_idx == app.selected_data_col;
            let is_pk = app.columns.iter().any(|c| c.name == *h && c.is_primary_key) || *h == "id";
            let is_fk = app.foreign_keys.iter().any(|fk| fk.column_name == *h)
                || *h == "entity_id"
                || h.ends_with("_id");

            let col_name_fmt = if is_pk && is_fk {
                format!(" 🔑🔑 PK/FK: {} ", h)
            } else if is_pk {
                format!(" 🔑 PK: {} ", h)
            } else if is_fk {
                format!(" 🔗 FK: {} ", h)
            } else {
                format!(" {} ", h)
            };
            
            let style = if is_col_selected {
                Style::default().fg(Color::Yellow).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else if is_pk && is_fk {
                Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
            } else if is_pk {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if is_fk {
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            };
            ratatui::widgets::Cell::from(Span::styled(col_name_fmt, style))
        });
        let header = Row::new(header_cells).height(1);

        let rows = res.rows.iter().enumerate().skip(app.data_scroll_offset).map(|(r_idx, row)| {
            let visible_cells = &row[app.data_col_offset.min(row.len())..];
            let cells = visible_cells.iter().enumerate().map(|(c_idx, c)| {
                let actual_col_idx = app.data_col_offset + c_idx;
                let is_cell_selected = r_idx == app.selected_data_row && actual_col_idx == app.selected_data_col;
                
                if is_cell_selected {
                    let cell_content = format!("▶[ {} ]◀", c);
                    let style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
                    ratatui::widgets::Cell::from(Span::styled(cell_content, style))
                } else {
                    let style = if c == "NULL" {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    ratatui::widgets::Cell::from(Span::styled(format!(" {} ", c), style))
                }
            });
            Row::new(cells)
        });

        let widths: Vec<Constraint> = visible_cols.iter().map(|_| Constraint::Length(app.cell_width)).collect();

        // Check if currently selected cell is a Foreign Key or Relational ID
        let cur_col_name = res.columns.get(app.selected_data_col).cloned().unwrap_or_default();
        let fk_info = app.foreign_keys.iter().find(|fk| fk.column_name == cur_col_name);
        
        let fk_hint = if let Some(fk) = fk_info {
            format!(" [🔗 FK -> {}.{} | Press Enter to Jump]", fk.foreign_table_schema, fk.foreign_table_name)
        } else if cur_col_name == "entity_id" {
            let target_name = if let Some(type_idx) = res.columns.iter().position(|c| c == "entity_type" || c == "entity") {
                res.rows.get(app.selected_data_row).and_then(|r| r.get(type_idx)).cloned().unwrap_or_default()
            } else {
                "entity".to_string()
            };
            format!(" [🔗 Jump -> {} | Press Enter]", target_name)
        } else if cur_col_name.ends_with("_id") {
            let base = cur_col_name.trim_end_matches("_id");
            format!(" [🔗 Jump -> {}s | Press Enter]", base)
        } else {
            "".to_string()
        };

        let breadcrumb_str = if app.breadcrumbs.is_empty() {
            "".to_string()
        } else {
            let items: Vec<String> = app.breadcrumbs.iter().enumerate().map(|(idx, b)| {
                if idx == app.active_breadcrumb_idx {
                    format!("•{}•", b)
                } else {
                    b.clone()
                }
            }).collect();
            format!(" [{}] ", items.join(" ➔ "))
        };

        let title_text = format!(
            " Fullscreen Data View: {}{} — Page {} (R{} C{}){} [Enter=Jump, b=Back, f=Forward, m/Esc=Exit] ",
            selected_table_name,
            breadcrumb_str,
            app.data_page + 1,
            app.selected_data_row + 1,
            app.selected_data_col + 1,
            fk_hint
        );

        let table_widget = RatatuiTable::new(rows, widths)
            .header(header)
            .block(block.title(title_text));
        f.render_widget(table_widget, area);
    } else {
        let p = Paragraph::new(" No data available ").block(block.title(" Fullscreen Data View "));
        f.render_widget(p, area);
    }
}

fn render_databases(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Databases in Server (Press Enter to switch active database) ");

    if let Some(ref res) = app.databases_result {
        let header_cells = res.columns.iter().map(|h| {
            ratatui::widgets::Cell::from(Span::styled(
                h.as_str(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ))
        });
        let header = Row::new(header_cells).height(1);

        let rows = res.rows.iter().enumerate().map(|(idx, row)| {
            let is_selected = idx == app.selected_db_idx;
            let cells = row.iter().enumerate().map(|(c_idx, c)| {
                if is_selected {
                    ratatui::widgets::Cell::from(Span::styled(c.as_str(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                } else if c_idx == 0 && app.current_connection().map(|conn| conn.dbname.as_str()) == Some(c.as_str()) {
                    ratatui::widgets::Cell::from(Span::styled(format!("● {}", c), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
                } else {
                    ratatui::widgets::Cell::from(c.as_str())
                }
            });
            Row::new(cells)
        });

        let widths: Vec<Constraint> = res.columns.iter().map(|_| Constraint::Min(25)).collect();

        let table_widget = RatatuiTable::new(rows, widths).header(header).block(block);
        f.render_widget(table_widget, area);
    } else {
        let p = Paragraph::new(" Loading databases... ").block(block);
        f.render_widget(p, area);
    }
}

fn render_users(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Database Roles & Users ");

    if let Some(ref res) = app.users_result {
        let header_cells = res.columns.iter().map(|h| {
            ratatui::widgets::Cell::from(Span::styled(
                h.as_str(),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ))
        });
        let header = Row::new(header_cells).height(1);

        let rows = res.rows.iter().map(|row| {
            let cells = row.iter().map(|c| ratatui::widgets::Cell::from(c.as_str()));
            Row::new(cells)
        });

        let widths: Vec<Constraint> = res.columns.iter().map(|_| Constraint::Min(20)).collect();

        let table_widget = RatatuiTable::new(rows, widths).header(header).block(block);
        f.render_widget(table_widget, area);
    } else {
        let p = Paragraph::new(" Loading roles and users... ").block(block);
        f.render_widget(p, area);
    }
}

fn render_query_runner(f: &mut Frame, app: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    // Query Editor
    let editor_block = Block::default()
        .borders(Borders::ALL)
        .title(" SQL Query (Press Ctrl+E to Execute) ")
        .border_style(if app.focused_panel == FocusedPanel::QueryEditor {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    let editor = Paragraph::new(app.sql_input.as_str())
        .block(editor_block)
        .wrap(Wrap { trim: false });
    f.render_widget(editor, chunks[0]);

    // Results or Error
    let res_block = Block::default()
        .borders(Borders::ALL)
        .title(" Execution Results ")
        .border_style(if app.focused_panel == FocusedPanel::Results {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if let Some(ref err) = app.query_error {
        let err_paragraph = Paragraph::new(Span::styled(err, Style::default().fg(Color::Red)))
            .block(res_block)
            .wrap(Wrap { trim: false });
        f.render_widget(err_paragraph, chunks[1]);
    } else if let Some(ref res) = app.query_result {
        let header_cells = res.columns.iter().map(|h| {
            ratatui::widgets::Cell::from(Span::styled(
                h.as_str(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
        });
        let header = Row::new(header_cells).height(1);

        let visible_rows = res.rows.iter().skip(app.result_scroll).map(|row| {
            let cells = row.iter().map(|c| ratatui::widgets::Cell::from(c.as_str()));
            Row::new(cells)
        });

        let widths: Vec<Constraint> = res.columns.iter().map(|_| Constraint::Min(15)).collect();

        let res_table = RatatuiTable::new(visible_rows, widths)
            .header(header)
            .block(res_block.title(format!(
                " Results: {} rows returned in {} ms (Scrolled: {}) ",
                res.rows.len(),
                res.execution_time_ms,
                app.result_scroll
            )));
        f.render_widget(res_table, chunks[1]);
    } else {
        let placeholder = Paragraph::new(" Press Ctrl+E to run query... ").block(res_block);
        f.render_widget(placeholder, chunks[1]);
    }
}

fn render_connections(f: &mut Frame, app: &AppState, area: Rect) {
    if app.is_adding_conn {
        render_add_connection_modal(f, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items: Vec<ListItem> = app
        .config
        .connections
        .iter()
        .enumerate()
        .map(|(idx, conn)| {
            let is_sel = idx == app.selected_conn_idx;
            let prefix = if is_sel { "❯ " } else { "  " };
            let style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Cyan)),
                Span::styled(&conn.name, style),
                Span::styled(
                    format!(" [{}:{}]", conn.host, conn.port),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Connections List (j/k=Nav, 'a'=Add, 'd'=Delete, Enter='c') ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    f.render_widget(list, chunks[0]);

    // Right detail card
    if let Some(conn) = app.config.connections.get(app.selected_conn_idx) {
        let pass_masked = if let Some(ref p) = conn.password {
            if p.is_empty() { "None".to_string() } else { "•".repeat(p.len().min(16)) }
        } else {
            "None".to_string()
        };

        let detail_text = vec![
            Line::from(Span::styled(" Connection Details", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![Span::styled("  Profile Name:  ", Style::default().fg(Color::DarkGray)), Span::styled(&conn.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("  Host Address:  ", Style::default().fg(Color::DarkGray)), Span::styled(&conn.host, Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled("  Port Number:   ", Style::default().fg(Color::DarkGray)), Span::styled(conn.port.to_string(), Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled("  User Name:     ", Style::default().fg(Color::DarkGray)), Span::styled(&conn.user, Style::default().fg(Color::Cyan))]),
            Line::from(vec![Span::styled("  Password:      ", Style::default().fg(Color::DarkGray)), Span::styled(pass_masked, Style::default().fg(Color::Magenta))]),
            Line::from(vec![Span::styled("  Database:      ", Style::default().fg(Color::DarkGray)), Span::styled(&conn.dbname, Style::default().fg(Color::Yellow))]),
            Line::from(""),
            Line::from(Span::styled("  Actions:", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("   • Press Enter or 'c' to connect to this server", Style::default().fg(Color::White))),
            Line::from(Span::styled("   • Press 'a' to add a new connection profile", Style::default().fg(Color::White))),
            Line::from(Span::styled("   • Press 'd' or 'Delete' to remove this profile", Style::default().fg(Color::Red))),
        ];

        let p = Paragraph::new(detail_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Selected Connection Profile "),
        );
        f.render_widget(p, chunks[1]);
    } else {
        let p = Paragraph::new(" No connection profile selected. Press 'a' to add one. ")
            .block(Block::default().borders(Borders::ALL).title(" Details "));
        f.render_widget(p, chunks[1]);
    }
}

fn render_add_connection_modal(f: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add New Connection Profile (Press Tab/Enter to next step, Esc to Cancel) ")
        .border_style(Style::default().fg(Color::Green));

    let fields = [
        ("1. Profile Name", &app.conn_input_name),
        ("2. Host IP / Domain", &app.conn_input_host),
        ("3. Port", &app.conn_input_port),
        ("4. Username", &app.conn_input_user),
        ("5. Password", &app.conn_input_pass),
        ("6. Database Name", &app.conn_input_dbname),
    ];

    let mut lines = vec![
        Line::from(Span::styled("Fill in Connection Details:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
    ];

    for (step_idx, (label, val)) in fields.iter().enumerate() {
        let is_active = step_idx == app.conn_form_step;
        let prefix = if is_active { " ❯ " } else { "   " };
        let style = if is_active {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let val_disp = if *label == "5. Password" {
            "•".repeat(val.len())
        } else {
            val.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Green)),
            Span::styled(format!("{:<20}: ", label), style),
            Span::styled(format!("{}█", val_disp), if is_active { Style::default().fg(Color::Green).bg(Color::DarkGray) } else { Style::default().fg(Color::White) }),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Press Enter on step 6 to Save & Connect!", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));

    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, area);
}

fn render_help(f: &mut Frame, _app: &AppState, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("tsql Keyboard Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("1 - 5         ", Style::default().fg(Color::Yellow)), Span::raw("Switch tabs (Tables, Databases, Users, Query, Connections)")]),
        Line::from(vec![Span::styled("Tab           ", Style::default().fg(Color::Yellow)), Span::raw("Toggle focus between Tables panel and Data View panel")]),
        Line::from(vec![Span::styled("j, k, Up, Down", Style::default().fg(Color::Yellow)), Span::raw("Navigate table list / scroll table rows in Data View")]),
        Line::from(vec![Span::styled("n / p         ", Style::default().fg(Color::Yellow)), Span::raw("Next page / Previous page in Data View (50 rows/page)")]),
        Line::from(vec![Span::styled("/ or f        ", Style::default().fg(Color::Yellow)), Span::raw("Fuzzy search/filter table list")]),
        Line::from(vec![Span::styled("s             ", Style::default().fg(Color::Yellow)), Span::raw("Toggle system tables (information_schema, pg_catalog)")]),
        Line::from(vec![Span::styled("c             ", Style::default().fg(Color::Yellow)), Span::raw("Connect/reconnect to selected connection")]),
        Line::from(vec![Span::styled("r             ", Style::default().fg(Color::Yellow)), Span::raw("Refresh database schemas & table list")]),
        Line::from(vec![Span::styled("Ctrl+E        ", Style::default().fg(Color::Yellow)), Span::raw("Execute SQL query in Query Runner")]),
        Line::from(vec![Span::styled("q, Esc, Ctrl+C", Style::default().fg(Color::Yellow)), Span::raw("Quit & close application")]),
        Line::from(""),
        Line::from(Span::styled("Config file location:", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(format!("  {}", crate::config::get_config_path().display()), Style::default().fg(Color::Green))),
    ];

    let p = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title(" Help & Keybindings "));
    f.render_widget(p, area);
}

fn render_status_bar(f: &mut Frame, app: &AppState, area: Rect) {
    let status_line = Line::from(vec![
        Span::styled(" STATUS: ", Style::default().fg(Color::Black).bg(Color::Cyan)),
        Span::styled(format!(" {} ", app.status_message), Style::default().fg(Color::White)),
        Span::styled(" | Press '?' for help ", Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(status_line), area);
}
