use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Table as RatatuiTable, Row, Tabs, Wrap},
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
        ActiveTab::Relationships => render_relationships(f, app, main_chunks[1]),
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
        " [6] Relationships ",
        " [?] Help ",
    ];
    let selected_idx = match app.active_tab {
        ActiveTab::Browser => 0,
        ActiveTab::Databases => 1,
        ActiveTab::Users => 2,
        ActiveTab::QueryRunner => 3,
        ActiveTab::Connections => 4,
        ActiveTab::Relationships => 5,
        ActiveTab::Help => 6,
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

    let mut header_spans = vec![conn_status];
    if app.is_loading {
        header_spans.push(Span::styled(" ⌛ [LOADING...] ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    }

    // Overlay connection indicator on the right of header
    let status_rect = Rect {
        x: area.width.saturating_sub(45),
        y: area.y,
        width: 44,
        height: 1,
    };
    f.render_widget(Paragraph::new(Line::from(header_spans)).alignment(ratatui::layout::Alignment::Right), status_rect);
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
            ]))
        })
        .collect();

    let filter_display = if app.is_filtering {
        format!(" [{}█]", app.filter_text)
    } else if !app.filter_text.is_empty() {
        format!(" [{}]", app.filter_text)
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
        .filter(|col| {
            app.field_search_text.is_empty()
                || col.name.to_lowercase().contains(&app.field_search_text.to_lowercase())
        })
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

            let is_match = !app.field_search_text.is_empty();
            let name_style = if is_match {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };

            ListItem::new(Line::from(vec![
                badge_span,
                Span::styled(format!("{:<20}", col.name), name_style),
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

    let field_search_display = if app.is_field_searching {
        format!(" [Field Search: {}█]", app.field_search_text)
    } else if !app.field_search_text.is_empty() {
        format!(" [Field Search: {}]", app.field_search_text)
    } else {
        String::new()
    };

    if app.is_loading {
        let loading_widget = Paragraph::new("\n  ⠋ Loading columns...")
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Columns: {} {} ", selected_table_name, field_search_display))
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        f.render_widget(loading_widget, right_chunks[0]);
    } else {
        let col_list = List::new(col_items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Columns: {} {} ", selected_table_name, field_search_display))
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(col_list, right_chunks[0]);
    }

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
        let mut header_cells: Vec<ratatui::widgets::Cell> = visible_cols.iter().enumerate().map(|(rel_idx, h)| {
            let actual_col_idx = app.data_col_offset + rel_idx;
            let is_col_selected = data_focused && actual_col_idx == app.selected_data_col;
            let is_pk = app.columns.iter().any(|c| c.name == *h && c.is_primary_key) || *h == "id";
            let is_fk = app.foreign_keys.iter().any(|fk| fk.column_name == *h)
                || *h == "entity_id"
                || h.ends_with("_id");
            let is_match = !app.field_search_text.is_empty() && h.to_lowercase().contains(&app.field_search_text.to_lowercase());

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
            } else if is_match {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
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
        }).collect();
        header_cells.push(ratatui::widgets::Cell::from("")); // Spacer column header
        let header = Row::new(header_cells).height(1);

        let col_widths: Vec<usize> = visible_cols.iter().enumerate().map(|(rel_idx, col_name)| {
            let actual_col_idx = app.data_col_offset + rel_idx;
            let mut max_len = col_name.len() + 10;
            for row in &res.rows {
                if let Some(val) = row.get(actual_col_idx) {
                    max_len = max_len.max(val.len());
                }
            }
            let zoom_factor = app.cell_width as f32 / 22.0;
            let final_width = ((max_len as f32) * zoom_factor) as usize;
            final_width.max(10)
        }).collect();

        let rows = res.rows.iter().enumerate().skip(app.data_scroll_offset).map(|(r_idx, row)| {
            let visible_cells = &row[app.data_col_offset.min(row.len())..];
            let mut cells: Vec<ratatui::widgets::Cell> = visible_cells.iter().enumerate().map(|(c_idx, c)| {
                let actual_col_idx = app.data_col_offset + c_idx;
                let is_cell_selected = data_focused && r_idx == app.selected_data_row && actual_col_idx == app.selected_data_col;
                let is_match = !app.field_search_text.is_empty() && res.columns.get(actual_col_idx).map_or(false, |col| col.to_lowercase().contains(&app.field_search_text.to_lowercase()));

                let col_w = col_widths.get(c_idx).copied().unwrap_or(22);
                let w = col_w.saturating_sub(2);
                if is_cell_selected {
                    let text = if c.len() > w {
                        format!("▶[{}..]", &c[..w.saturating_sub(4)])
                    } else {
                        format!("▶[ {:<width$} ]◀", c, width = w.saturating_sub(6))
                    };
                    let style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
                    ratatui::widgets::Cell::from(Span::styled(text, style))
                } else if is_match {
                    let style = Style::default().fg(Color::Black).bg(Color::Green);
                    let text = if c.len() > w {
                        format!("{}..", &c[..w.saturating_sub(2)])
                    } else {
                        format!(" {:<width$} ", c, width = w.saturating_sub(2))
                    };
                    ratatui::widgets::Cell::from(Span::styled(text, style))
                } else {
                    let style = if c == "NULL" {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let text = if c.len() > w {
                        format!(" {}.. ", &c[..w.saturating_sub(4)])
                    } else {
                        format!(" {:<width$} ", c, width = w.saturating_sub(2))
                    };
                    ratatui::widgets::Cell::from(Span::styled(text, style))
                }
            }).collect();
            cells.push(ratatui::widgets::Cell::from("")); // Spacer column cell
            Row::new(cells)
        });

        let mut widths: Vec<Constraint> = col_widths.iter().map(|&w| Constraint::Length(w as u16)).collect();
        widths.push(Constraint::Min(0)); // Spacer column constraint

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

        let field_search_display = if app.is_field_searching {
            format!(" [Field Search: {}█]", app.field_search_text)
        } else if !app.field_search_text.is_empty() {
            format!(" [Field Search: {}]", app.field_search_text)
        } else {
            "".to_string()
        };

        let title_text = format!(
            " Fullscreen Data View: {}{}{}{} — Page {} (R{} C{}) [Zoom: {}px]{} [Enter=Jump, b=Back, f=Forward, m/Esc=Exit, /=Field Search] ",
            selected_table_name,
            filter_display,
            field_search_display,
            breadcrumb_str,
            app.data_page + 1,
            app.selected_data_row + 1,
            app.selected_data_col + 1,
            app.cell_width,
            fk_hint
        );

        let table_widget = RatatuiTable::new(rows, widths)
            .header(header)
            .block(data_block.title(title_text));
        f.render_widget(table_widget, right_chunks[1]);
    } else if app.is_loading {
        let loading_msg = Paragraph::new("\n  ⠋ Loading table records...")
            .style(Style::default().fg(Color::Yellow))
            .block(data_block.title(" Data View "));
        f.render_widget(loading_msg, right_chunks[1]);
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
        let filtered_rows: Vec<_> = if app.filter_data_text.is_empty() {
            res.rows.iter().collect()
        } else {
            let query = app.filter_data_text.to_lowercase();
            res.rows.iter().filter(|row| {
                row.iter().any(|c| c.to_lowercase().contains(&query))
            }).collect()
        };

        let visible_cols = &res.columns[app.data_col_offset.min(res.columns.len())..];
        let mut header_cells: Vec<ratatui::widgets::Cell> = visible_cols.iter().enumerate().map(|(rel_idx, h)| {
            let actual_col_idx = app.data_col_offset + rel_idx;
            let is_col_selected = actual_col_idx == app.selected_data_col;
            let is_pk = app.columns.iter().any(|c| c.name == *h && c.is_primary_key) || *h == "id";
            let is_fk = app.foreign_keys.iter().any(|fk| fk.column_name == *h)
                || *h == "entity_id"
                || h.ends_with("_id");
            let is_match = !app.field_search_text.is_empty() && h.to_lowercase().contains(&app.field_search_text.to_lowercase());

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
            } else if is_match {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
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
        }).collect();
        header_cells.push(ratatui::widgets::Cell::from("")); // Spacer column header
        let header = Row::new(header_cells).height(1);

        let col_widths: Vec<usize> = visible_cols.iter().enumerate().map(|(rel_idx, col_name)| {
            let actual_col_idx = app.data_col_offset + rel_idx;
            let mut max_len = col_name.len() + 10;
            for row in &filtered_rows {
                if let Some(val) = row.get(actual_col_idx) {
                    max_len = max_len.max(val.len());
                }
            }
            let zoom_factor = app.cell_width as f32 / 22.0;
            let final_width = ((max_len as f32) * zoom_factor) as usize;
            final_width.max(10)
        }).collect();

        let rows = filtered_rows.iter().enumerate().skip(app.data_scroll_offset).map(|(r_idx, row)| {
            let visible_cells = &row[app.data_col_offset.min(row.len())..];
            let mut cells: Vec<ratatui::widgets::Cell> = visible_cells.iter().enumerate().map(|(c_idx, c)| {
                let actual_col_idx = app.data_col_offset + c_idx;
                let is_cell_selected = r_idx == app.selected_data_row && actual_col_idx == app.selected_data_col;
                let is_match = !app.field_search_text.is_empty() && res.columns.get(actual_col_idx).map_or(false, |col| col.to_lowercase().contains(&app.field_search_text.to_lowercase()));

                let col_w = col_widths.get(c_idx).copied().unwrap_or(22);
                let w = col_w.saturating_sub(2);
                if is_cell_selected {
                    let text = if c.len() > w {
                        format!("▶[{}..]", &c[..w.saturating_sub(4)])
                    } else {
                        format!("▶[ {:<width$} ]◀", c, width = w.saturating_sub(6))
                    };
                    let style = Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD);
                    ratatui::widgets::Cell::from(Span::styled(text, style))
                } else if is_match {
                    let style = Style::default().fg(Color::Black).bg(Color::Green);
                    let text = if c.len() > w {
                        format!("{}..", &c[..w.saturating_sub(2)])
                    } else {
                        format!(" {:<width$} ", c, width = w.saturating_sub(2))
                    };
                    ratatui::widgets::Cell::from(Span::styled(text, style))
                } else {
                    let style = if c == "NULL" {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let text = if c.len() > w {
                        format!(" {}.. ", &c[..w.saturating_sub(4)])
                    } else {
                        format!(" {:<width$} ", c, width = w.saturating_sub(2))
                    };
                    ratatui::widgets::Cell::from(Span::styled(text, style))
                }
            }).collect();
            cells.push(ratatui::widgets::Cell::from("")); // Spacer column cell
            Row::new(cells)
        });

        let mut widths: Vec<Constraint> = col_widths.iter().map(|&w| Constraint::Length(w as u16)).collect();
        widths.push(Constraint::Min(0)); // Spacer column constraint

        // Check if currently selected cell is a Foreign Key or Relational ID
        let cur_col_name = res.columns.get(app.selected_data_col).cloned().unwrap_or_default();
        let fk_info = app.foreign_keys.iter().find(|fk| fk.column_name == cur_col_name);
        
        let fk_hint = if let Some(fk) = fk_info {
            format!(" [🔗 FK -> {}.{} | Press Enter to Jump]", fk.foreign_table_schema, fk.foreign_table_name)
        } else if cur_col_name == "entity_id" {
            let target_name = if let Some(type_idx) = res.columns.iter().position(|c| c == "entity_type" || c == "entity") {
                filtered_rows.get(app.selected_data_row).and_then(|r| r.get(type_idx)).cloned().unwrap_or_default()
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

        let filter_display = if app.is_filtering_data {
            format!(" [Filter: {}█]", app.filter_data_text)
        } else if !app.filter_data_text.is_empty() {
            format!(" [Filter: {}]", app.filter_data_text)
        } else {
            "".to_string()
        };

        let field_search_display = if app.is_field_searching {
            format!(" [Field Search: {}█]", app.field_search_text)
        } else if !app.field_search_text.is_empty() {
            format!(" [Field Search: {}]", app.field_search_text)
        } else {
            "".to_string()
        };

        let title_text = format!(
            " Fullscreen Data View: {}{}{}{} — Page {} (R{} C{}) [Zoom: {}px]{} [Enter=Jump, b=Back, f=Forward, m/Esc=Exit, /=Field Search] ",
            selected_table_name,
            filter_display,
            field_search_display,
            breadcrumb_str,
            app.data_page + 1,
            app.selected_data_row + 1,
            app.selected_data_col + 1,
            app.cell_width,
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

        let current_db_name = app.current_connection().map(|conn| conn.dbname.clone()).unwrap_or_default();

        let rows = res.rows.iter().enumerate().map(|(idx, row)| {
            let is_selected = idx == app.selected_db_idx;
            let cells = row.iter().enumerate().map(|(c_idx, c)| {
                let is_active_db = c_idx == 0 && current_db_name == *c;
                
                if is_selected {
                    let text = if c_idx == 0 && is_active_db {
                        format!("❯ ● {}", c)
                    } else if c_idx == 0 {
                        format!("❯   {}", c)
                    } else {
                        c.clone()
                    };
                    ratatui::widgets::Cell::from(Span::styled(text, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                } else if is_active_db {
                    let text = format!("  ● {}", c);
                    ratatui::widgets::Cell::from(Span::styled(text, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)))
                } else if c_idx == 0 {
                    ratatui::widgets::Cell::from(format!("    {}", c))
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
            .title(" Connections List (j/k=Nav, 'a'=Add, 'e'=Edit, 'd'=Del, Enter='c') ")
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
            Line::from(Span::styled("   • Press 'e' or 'u' to edit/update this connection profile", Style::default().fg(Color::Yellow))),
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
    let title_text = if app.editing_conn_idx.is_some() {
        " Edit Connection Profile (Press Tab/Enter to next step, Esc to Cancel) "
    } else {
        " Add New Connection Profile (Press Tab/Enter to next step, Esc to Cancel) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title_text)
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
    let submit_hint = if app.editing_conn_idx.is_some() {
        "Press Enter on step 6 to Save Updates & Connect!"
    } else {
        "Press Enter on step 6 to Save & Connect!"
    };
    lines.push(Line::from(Span::styled(submit_hint, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));

    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, area);
}

fn render_help(f: &mut Frame, _app: &AppState, area: Rect) {
    let help_text = vec![
        Line::from(Span::styled("tsql Keyboard Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("1 - 6         ", Style::default().fg(Color::Yellow)), Span::raw("Switch tabs (Tables, Databases, Users, Query, Connections, Relationships)")]),
        Line::from(vec![Span::styled("Tab           ", Style::default().fg(Color::Yellow)), Span::raw("Toggle focus between Tables panel and Data View panel")]),
        Line::from(vec![Span::styled("j, k, Up, Down", Style::default().fg(Color::Yellow)), Span::raw("Navigate table list / scroll table rows in Data View")]),
        Line::from(vec![Span::styled("n / p         ", Style::default().fg(Color::Yellow)), Span::raw("Next page / Previous page in Data View (50 rows/page)")]),
        Line::from(vec![Span::styled("/             ", Style::default().fg(Color::Yellow)), Span::raw("Fuzzy search/filter table list (Tables panel)")]),
        Line::from(vec![Span::styled("                ", Style::default().fg(Color::Yellow)), Span::raw("or search field names (Data View when focused)")]),
        Line::from(vec![Span::styled("                ", Style::default().fg(Color::Yellow)), Span::raw("or filter data rows (Fullscreen Data View)")]),
        Line::from(vec![Span::styled("                ", Style::default().fg(Color::Yellow)), Span::raw("or filter tables/columns (Relationships ER Diagram)")]),
        Line::from(vec![Span::styled("Enter         ", Style::default().fg(Color::Yellow)), Span::raw("Jump to Relational Foreign Key table")]),
        Line::from(vec![Span::styled("b / f         ", Style::default().fg(Color::Yellow)), Span::raw("Step Back / Step Forward in Breadcrumb history")]),
        Line::from(vec![Span::styled("+ / -         ", Style::default().fg(Color::Yellow)), Span::raw("Zoom In / Zoom Out (Widen or Narrow table grid columns)")]),
        Line::from(vec![Span::styled("m / z         ", Style::default().fg(Color::Yellow)), Span::raw("Toggle Fullscreen Data View mode")]),
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

struct Canvas {
    width: usize,
    height: usize,
    cells: Vec<Vec<char>>,
    styles: Vec<Vec<Style>>,
    protected: Vec<Vec<bool>>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![' '; width]; height],
            styles: vec![vec![Style::default(); width]; height],
            protected: vec![vec![false; width]; height],
        }
    }

    fn set_cell(&mut self, x: usize, y: usize, c: char, style: Style) {
        if x >= self.width || y >= self.height {
            return;
        }
        if self.protected[y][x] {
            return; // Protect boxes/text from being overwritten by connection lines!
        }
        let existing = self.cells[y][x];
        let new_char = match (existing, c) {
            ('━', '│') | ('│', '━') => '┼',
            ('─', '│') | ('│', '─') => '┼',
            _ => c,
        };
        self.cells[y][x] = new_char;
        self.styles[y][x] = style;
    }

    fn set_cell_protected(&mut self, x: usize, y: usize, c: char, style: Style) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.cells[y][x] = c;
        self.styles[y][x] = style;
        self.protected[y][x] = true;
    }

    fn draw_string(&mut self, x: usize, y: usize, s: &str, style: Style) {
        let mut curr_x = x;
        for c in s.chars() {
            if curr_x >= self.width {
                break;
            }
            self.set_cell_protected(curr_x, y, c, style);
            curr_x += 1;
        }
    }

    fn draw_box(&mut self, x: usize, y: usize, w: usize, h: usize, title: &str, border_style: Style, title_style: Style) {
        for curr_x in x..(x + w) {
            self.set_cell_protected(curr_x, y, '─', border_style);
            self.set_cell_protected(curr_x, y + h - 1, '─', border_style);
        }
        for curr_y in y..(y + h) {
            self.set_cell_protected(x, curr_y, '│', border_style);
            self.set_cell_protected(x + w - 1, curr_y, '│', border_style);
        }
        self.set_cell_protected(x, y, '┌', border_style);
        self.set_cell_protected(x + w - 1, y, '┐', border_style);
        self.set_cell_protected(x, y + h - 1, '└', border_style);
        self.set_cell_protected(x + w - 1, y + h - 1, '┘', border_style);

        if h > 3 {
            self.set_cell_protected(x, y + 2, '├', border_style);
            for curr_x in (x + 1)..(x + w - 1) {
                self.set_cell_protected(curr_x, y + 2, '─', border_style);
            }
            self.set_cell_protected(x + w - 1, y + 2, '┤', border_style);
        }

        if !title.is_empty() {
            let title_y = if h > 3 { y + 1 } else { y };
            let truncated_title = if title.chars().count() > w - 4 {
                let take_chars: String = title.chars().take(w - 4).collect();
                take_chars
            } else {
                title.to_string()
            };
            self.draw_string(x + 2, title_y, &truncated_title, title_style);
        }
    }
}

fn render_relationships(f: &mut Frame, app: &AppState, area: Rect) {
    let is_fullscreen = app.focused_panel == FocusedPanel::DataPreview;

    let (chunks, diagram_area) = if is_fullscreen {
        (None, area)
    } else {
        let split_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);
        (Some(split_chunks.clone()), split_chunks[1])
    };

    // Field search: determine which tables have matching columns
    let field_search_match: std::collections::HashSet<(String, String)> = if !app.field_search_text.is_empty() {
        let query = app.field_search_text.to_lowercase();
        let mut matches = std::collections::HashSet::new();
        for tbl in &app.tables {
            let has_match = app.all_foreign_keys.iter().any(|fk| {
                (fk.table_schema == tbl.schema && fk.table_name == tbl.name && fk.column_name.to_lowercase().contains(&query))
                    || (fk.foreign_table_schema == tbl.schema && fk.foreign_table_name == tbl.name && fk.foreign_column_name.to_lowercase().contains(&query))
            });
            if has_match {
                matches.insert((tbl.schema.clone(), tbl.name.clone()));
            }
        }
        matches
    } else {
        std::collections::HashSet::new()
    };

    if let Some(ref split_chunks) = chunks {
        let tables = app.filtered_tables();
        let items: Vec<ListItem> = tables
            .iter()
            .enumerate()
            .filter(|(_, tbl)| {
                app.field_search_text.is_empty()
                    || field_search_match.contains(&(tbl.schema.clone(), tbl.name.clone()))
            })
            .map(|(orig_idx, tbl)| {
                let is_sel = orig_idx == app.selected_table_idx;
                let prefix = if is_sel { "❯ " } else { "  " };
                let style = if is_sel {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                    Span::styled(&tbl.name, style),
                ]))
            })
            .collect();

        let filter_display = if app.is_field_searching {
            format!(" [Field Search: {}█]", app.field_search_text)
        } else if !app.field_search_text.is_empty() {
            format!(" [Field Search: {}]", app.field_search_text)
        } else if app.is_filtering {
            format!(" [{}█]", app.filter_text)
        } else if !app.filter_text.is_empty() {
            format!(" [{}]", app.filter_text)
        } else {
            " [Press / to Search]".to_string()
        };

        let table_list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Tables ({}){}", tables.len(), filter_display))
                .border_style(if app.is_field_searching || app.is_filtering || app.focused_panel == FocusedPanel::Tables {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        );
        f.render_widget(table_list, split_chunks[0]);
    }

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(diagram_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Relationships ER Diagram ")
        .border_style(if app.focused_panel == FocusedPanel::DataPreview {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        });

    if app.all_foreign_keys.is_empty() {
        let p = Paragraph::new("\n  No foreign key relationships found in database.")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(p, right_chunks[0]);
        return;
    }

    let selected_table = app.filtered_tables().get(app.selected_table_idx).cloned();

    // Setup Canvas with dynamic viewport sizing & Zoom scale
    let viewport_width = diagram_area.width as usize;
    let viewport_height = diagram_area.height as usize;

    let zoom = app.relationship_zoom;
    let box_width = match zoom {
        1 => 18,
        2 => 24,
        3 => 30,
        _ => 36,
    };
    let spacing = match zoom {
        1 => 12,
        2 => 16,
        3 => 20,
        _ => 24,
    };

    let left_x = 4;
    let center_x = left_x + box_width + spacing;
    let right_x = center_x + box_width + spacing;

    // Calculate layout coordinates
    let (layout, col_coords) = crate::app::get_global_table_layout(&app.tables, &app.all_foreign_keys, zoom, app.layout_seed);

    // Field search: filter layout to only show matching tables
    let layout_tables: Vec<_> = if app.field_search_text.is_empty() {
        layout.iter().collect()
    } else {
        layout.iter().filter(|(key, _, _, _)| {
            field_search_match.contains(&(key.0.clone(), key.1.clone()))
        }).collect()
    };

    // Compute bounding canvas size
    let mut canvas_width = viewport_width;
    let mut canvas_height = viewport_height;

    for (_, x, y, h) in &layout_tables {
        canvas_width = canvas_width.max(x + box_width + 4);
        canvas_height = canvas_height.max(y + h + 15); // Extra padding for bypass lines
    }

    let mut canvas = Canvas::new(canvas_width, canvas_height);

    // 1. Draw all table boxes (filtered by field search)
    for ((schema, name), x, y, h) in &layout_tables {

        let is_selected_table = selected_table.as_ref()
            .map(|st| st.schema == *schema && st.name == *name)
            .unwrap_or(false);

        let border_color = if is_selected_table { Color::Yellow } else { Color::DarkGray };
        let title_color = if is_selected_table { Color::Yellow } else { Color::Cyan };

        let border_style = Style::default().fg(border_color);
        let title_style = Style::default().fg(title_color).add_modifier(Modifier::BOLD);

        let title = format!("{}.{}", schema, name);
        canvas.draw_box(*x, *y, box_width, *h, &title, border_style, title_style);

        // Draw key column fields with field search highlighting
        let mut key_cols = std::collections::BTreeSet::new();
        for fk in &app.all_foreign_keys {
            if fk.table_schema == *schema && fk.table_name == *name {
                key_cols.insert(fk.column_name.clone());
            }
            if fk.foreign_table_schema == *schema && fk.foreign_table_name == *name {
                key_cols.insert(fk.foreign_column_name.clone());
            }
        }
        let mut col_y = y + 3;
        for col in key_cols {
            let is_fk = app.all_foreign_keys.iter().any(|fk| fk.table_schema == *schema && fk.table_name == *name && fk.column_name == col);
            let is_pk = app.all_foreign_keys.iter().any(|fk| fk.foreign_table_schema == *schema && fk.foreign_table_name == *name && fk.foreign_column_name == col);

            let badge = if is_fk && is_pk {
                "[fk/pk]"
            } else if is_fk {
                "[fk]"
            } else if is_pk {
                "[pk]"
            } else {
                ""
            };

            let badge_style = if is_fk && is_pk {
                Style::default().fg(Color::LightMagenta)
            } else if is_fk {
                Style::default().fg(Color::LightCyan)
            } else if is_pk {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let is_match = !app.field_search_text.is_empty() && col.to_lowercase().contains(&app.field_search_text.to_lowercase());
            let name_style = if is_match {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
            } else if is_selected_table {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let badge_len = if badge.is_empty() { 0 } else { badge.len() + 1 };
            let max_name_len = (box_width.saturating_sub(4 + badge_len)).max(1);
            let col_name_display = if col.chars().count() > max_name_len {
                col.chars().take(max_name_len).collect::<String>()
            } else {
                col.clone()
            };

            canvas.draw_string(x + 2, col_y, &col_name_display, name_style);

            if !badge.is_empty() {
                let badge_x = (x + box_width - 1).saturating_sub(badge.len() + 1);
                if badge_x >= x + 2 + col_name_display.chars().count() {
                    canvas.draw_string(badge_x, col_y, badge, badge_style);
                }
            }

            col_y += 1;
        }
    }

    // Find vertical boundaries of all layout boxes
    let mut global_min_y = 10;
    let mut global_max_y = 10;
    if !layout_tables.is_empty() {
        global_min_y = layout_tables.iter().map(|(_, _, y, _)| *y).min().unwrap_or(10);
        global_max_y = layout_tables.iter().map(|(_, _, y, h)| y + h).max().unwrap_or(10);
    }

    // Filter relevant connection lines (field search + focused table)
    let relevant_fks: Vec<(usize, &crate::db::AllForeignKeyInfo)> = app.all_foreign_keys.iter().enumerate().filter(|(_, fk)| {
        if app.show_all_relationships {
            // In field search mode, only show lines between matching tables
            if !app.field_search_text.is_empty() {
                let src_match = field_search_match.contains(&(fk.table_schema.clone(), fk.table_name.clone()));
                let dst_match = field_search_match.contains(&(fk.foreign_table_schema.clone(), fk.foreign_table_name.clone()));
                return src_match && dst_match;
            }
            true
        } else if let Some(ref st) = selected_table {
            (fk.table_schema == st.schema && fk.table_name == st.name)
                || (fk.foreign_table_schema == st.schema && fk.foreign_table_name == st.name)
        } else {
            false
        }
    }).collect();

    // 2. Draw Connection Lines
    for (active_idx, (_, fk)) in relevant_fks.into_iter().enumerate() {
        let src_key = (fk.table_schema.clone(), fk.table_name.clone(), fk.column_name.clone());
        let dst_key = (fk.foreign_table_schema.clone(), fk.foreign_table_name.clone(), fk.foreign_column_name.clone());

        let src_tbl_x = layout.iter().find(|(key, _, _, _)| key.0 == fk.table_schema && key.1 == fk.table_name).map(|pos| pos.1);
        let dst_tbl_x = layout.iter().find(|(key, _, _, _)| key.0 == fk.foreign_table_schema && key.1 == fk.foreign_table_name).map(|pos| pos.1);

        if let (Some(sx_tbl), Some(dx_tbl), Some(&(_, src_y)), Some(&(_, dst_y))) = (src_tbl_x, dst_tbl_x, col_coords.get(&src_key), col_coords.get(&dst_key)) {
            let src_x = if sx_tbl == left_x {
                left_x + box_width - 1
            } else if sx_tbl == right_x {
                right_x
            } else {
                if dx_tbl == left_x {
                    center_x
                } else {
                    center_x + box_width - 1
                }
            };

            let dst_x = if dx_tbl == left_x {
                left_x + box_width - 1
            } else if dx_tbl == right_x {
                right_x
            } else {
                if sx_tbl == left_x {
                    center_x
                } else {
                    center_x + box_width - 1
                }
            };

            let is_selected_conn = selected_table.as_ref()
                .map(|st| (st.schema == fk.table_schema && st.name == fk.table_name) || (st.schema == fk.foreign_table_schema && st.name == fk.foreign_table_name))
                .unwrap_or(false);

            let is_field_match = !app.field_search_text.is_empty()
                && field_search_match.contains(&(fk.table_schema.clone(), fk.table_name.clone()))
                && field_search_match.contains(&(fk.foreign_table_schema.clone(), fk.foreign_table_name.clone()));

            let line_color = if is_selected_conn {
                Color::Yellow
            } else if is_field_match {
                Color::Green
            } else {
                Color::DarkGray
            };
            let line_style = Style::default().fg(line_color);

            let max_tracks = ((spacing.saturating_sub(4)) / 2).max(1);
            let is_bypass = (sx_tbl == left_x && dx_tbl == right_x) || (sx_tbl == right_x && dx_tbl == left_x);

            if is_bypass {
                // 5-segment bypass routing
                let track_x1 = (left_x + box_width - 1) + 2 + (active_idx % max_tracks) * 2;
                let track_x2 = right_x - 2 - (active_idx % max_tracks) * 2;

                let bypass_y = if active_idx % 2 == 0 {
                    global_min_y.saturating_sub(3 + (active_idx / 2) * 2)
                } else {
                    global_max_y + 3 + (active_idx / 2) * 2
                };

                let is_left_to_right = sx_tbl == left_x;

                let (src_track_x, dst_track_x) = if is_left_to_right {
                    (track_x1, track_x2)
                } else {
                    (track_x2, track_x1)
                };

                // 1. Horizontal from source to its gap track
                let h1_start = src_x.min(src_track_x);
                let h1_end = src_x.max(src_track_x);
                for x in h1_start..=h1_end {
                    canvas.set_cell(x, src_y, '━', line_style);
                }
                canvas.set_cell(src_x, src_y, '◉', line_style);

                // 2. Vertical along source gap track to bypass_y
                let v1_start = src_y.min(bypass_y);
                let v1_end = src_y.max(bypass_y);
                for y in v1_start..=v1_end {
                    canvas.set_cell(src_track_x, y, '│', line_style);
                }

                // Corner at source track and src_y / bypass_y
                if src_y > bypass_y {
                    if is_left_to_right {
                        canvas.set_cell(src_track_x, src_y, '┐', line_style);
                        canvas.set_cell(src_track_x, bypass_y, '└', line_style);
                    } else {
                        canvas.set_cell(src_track_x, src_y, '┌', line_style);
                        canvas.set_cell(src_track_x, bypass_y, '┘', line_style);
                    }
                } else {
                    if is_left_to_right {
                        canvas.set_cell(src_track_x, src_y, '┘', line_style);
                        canvas.set_cell(src_track_x, bypass_y, '┌', line_style);
                    } else {
                        canvas.set_cell(src_track_x, src_y, '└', line_style);
                        canvas.set_cell(src_track_x, bypass_y, '┐', line_style);
                    }
                }

                // 3. Horizontal bypass line from src_track_x to dst_track_x
                let h2_start = src_track_x.min(dst_track_x);
                let h2_end = src_track_x.max(dst_track_x);
                for x in h2_start..=h2_end {
                    canvas.set_cell(x, bypass_y, '━', line_style);
                }

                // 4. Vertical along destination gap track from bypass_y to dst_y
                let v2_start = dst_y.min(bypass_y);
                let v2_end = dst_y.max(bypass_y);
                for y in v2_start..=v2_end {
                    canvas.set_cell(dst_track_x, y, '│', line_style);
                }

                // Corner at dst_track_x and bypass_y / dst_y
                if bypass_y > dst_y {
                    if is_left_to_right {
                        canvas.set_cell(dst_track_x, bypass_y, '┘', line_style);
                        canvas.set_cell(dst_track_x, dst_y, '┌', line_style);
                    } else {
                        canvas.set_cell(dst_track_x, bypass_y, '└', line_style);
                        canvas.set_cell(dst_track_x, dst_y, '┐', line_style);
                    }
                } else {
                    if is_left_to_right {
                        canvas.set_cell(dst_track_x, bypass_y, '┐', line_style);
                        canvas.set_cell(dst_track_x, dst_y, '└', line_style);
                    } else {
                        canvas.set_cell(dst_track_x, bypass_y, '┌', line_style);
                        canvas.set_cell(dst_track_x, dst_y, '┘', line_style);
                    }
                }

                // 5. Horizontal from destination track to dst_x
                let h3_start = dst_x.min(dst_track_x);
                let h3_end = dst_x.max(dst_track_x);
                for x in h3_start..=h3_end {
                    canvas.set_cell(x, dst_y, '━', line_style);
                }
                let arrow_char = if is_left_to_right { '▶' } else { '◄' };
                canvas.set_cell(dst_x, dst_y, arrow_char, line_style);
            } else {
                // Standard 3-segment orthogonal routing (stays strictly within adjacent column gap)
                let is_left_to_right = src_x < dst_x;

                let track_x = if is_left_to_right {
                    src_x + 2 + (active_idx % max_tracks) * 2
                } else {
                    src_x - 2 - (active_idx % max_tracks) * 2
                };

                // Draw horizontal from source to track_x
                let h_start = src_x.min(track_x);
                let h_end = src_x.max(track_x);
                for x in h_start..=h_end {
                    canvas.set_cell(x, src_y, '━', line_style);
                }
                canvas.set_cell(src_x, src_y, '◉', line_style);

                // Draw vertical along track
                let y_start = src_y.min(dst_y);
                let y_end = src_y.max(dst_y);
                for y in y_start..=y_end {
                    canvas.set_cell(track_x, y, '│', line_style);
                }

                // Draw corners
                if is_left_to_right {
                    if src_y > dst_y {
                        canvas.set_cell(track_x, src_y, '┘', line_style);
                        canvas.set_cell(track_x, dst_y, '┌', line_style);
                    } else if src_y < dst_y {
                        canvas.set_cell(track_x, src_y, '┐', line_style);
                        canvas.set_cell(track_x, dst_y, '└', line_style);
                    }
                } else {
                    if src_y > dst_y {
                        canvas.set_cell(track_x, src_y, '└', line_style);
                        canvas.set_cell(track_x, dst_y, '┐', line_style);
                    } else if src_y < dst_y {
                        canvas.set_cell(track_x, src_y, '┌', line_style);
                        canvas.set_cell(track_x, dst_y, '┘', line_style);
                    }
                }

                // Draw horizontal from track back to target
                let h2_start = dst_x.min(track_x);
                let h2_end = dst_x.max(track_x);
                for x in h2_start..=h2_end {
                    canvas.set_cell(x, dst_y, '━', line_style);
                }
                let arrow_char = if is_left_to_right { '▶' } else { '◄' };
                canvas.set_cell(dst_x, dst_y, arrow_char, line_style);
            }
        }
    }

    // 3. Center the Viewport on the Selected Table
    let mut start_y = 0;
    let mut start_x = 0;

    if let Some(ref st) = selected_table {
        if let Some(pos) = layout.iter().find(|(key, _, _, _)| key.0 == st.schema && key.1 == st.name) {
            let (tbl_x, tbl_y, tbl_h) = (pos.1, pos.2, pos.3);
            let center_y = (tbl_y + tbl_h / 2).saturating_sub(viewport_height / 2);
            let center_x = (tbl_x + box_width / 2).saturating_sub(viewport_width / 2);
            
            start_y = (center_y as isize + app.diagram_scroll_offset_y).max(0) as usize;
            start_x = (center_x as isize + app.diagram_scroll_offset_x).max(0) as usize;
        }
    }

    // Clip scroll ranges
    start_y = start_y.min(canvas_height.saturating_sub(viewport_height));
    start_x = start_x.min(canvas_width.saturating_sub(viewport_width));

    // Convert Canvas to Paragraph
    let mut lines = Vec::new();
    for y in start_y..(start_y + viewport_height).min(canvas_height) {
        let mut line_spans = Vec::new();
        for x in start_x..(start_x + viewport_width).min(canvas_width) {
            let c = canvas.cells[y][x];
            let style = canvas.styles[y][x];
            line_spans.push(Span::styled(c.to_string(), style));
        }
        lines.push(Line::from(line_spans));
    }

    let current_table_name = selected_table.map(|st| format!("{}.{}", st.schema, st.name)).unwrap_or_else(|| "None".to_string());
    let view_mode = if app.show_all_relationships { "All Lines" } else { "Focused View" };
    let field_search_str = if app.is_field_searching {
        format!(" [Field Search: {}█]", app.field_search_text)
    } else if !app.field_search_text.is_empty() {
        format!(" [Field Search: {}]", app.field_search_text)
    } else {
        "".to_string()
    };
    let p = Paragraph::new(lines).block(block.title(format!(
        " ER Diagram: {}{} [{}] ",
        current_table_name,
        field_search_str,
        view_mode
    )));
    f.render_widget(p, right_chunks[0]);

    // Keyboard hints helper line
    let keyboard_hints = Line::from(vec![
        Span::styled("Keyboard: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("hjkl / Arrows ", Style::default().fg(Color::Yellow)),
        Span::raw("Navigate  "),
        Span::styled("Ctrl+d/u ", Style::default().fg(Color::Yellow)),
        Span::raw("Scroll  "),
        Span::styled("i/o ", Style::default().fg(Color::Yellow)),
        Span::raw("Zoom  "),
        Span::styled("a ", Style::default().fg(Color::Yellow)),
        Span::raw("Toggle All Lines  "),
        Span::styled("r ", Style::default().fg(Color::Yellow)),
        Span::raw("Reposition  "),
        Span::styled("Enter ", Style::default().fg(Color::Yellow)),
        Span::raw("View Data  "),
        Span::styled("Tab ", Style::default().fg(Color::Yellow)),
        Span::raw("Sidebar  "),
        Span::styled("Esc ", Style::default().fg(Color::Yellow)),
        Span::raw("Exit"),
    ]);
    f.render_widget(Paragraph::new(vec![Line::from(""), keyboard_hints]), right_chunks[1]);
}
