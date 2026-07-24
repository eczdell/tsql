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
    SaveConnection(crate::config::ConnectionConfig, Option<usize>),
}

pub fn handle_key(key: KeyEvent, app: &mut AppState) -> AppAction {
    if app.is_filtering_data {
        let mut changed = false;
        match key.code {
            KeyCode::Esc => {
                app.is_filtering_data = false;
            }
            KeyCode::Enter => {
                app.is_filtering_data = false;
            }
            KeyCode::Backspace => {
                app.filter_data_text.pop();
                changed = true;
            }
            KeyCode::Char(c) => {
                app.filter_data_text.push(c);
                changed = true;
            }
            _ => {}
        }
        if changed {
            app.data_scroll_offset = 0;
            app.selected_data_row = 0;
        }
        return AppAction::None;
    }

    if app.is_filtering {
        let mut changed = false;
        match key.code {
            KeyCode::Esc => {
                app.is_filtering = false;
            }
            KeyCode::Enter => {
                app.is_filtering = false;
                if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                    return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
                }
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
            app.update_filtered_tables();
        }
        return AppAction::None;
    }

    if app.is_adding_conn {
        match key.code {
            KeyCode::Esc => {
                app.is_adding_conn = false;
                app.editing_conn_idx = None;
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
                    let edit_idx = app.editing_conn_idx;
                    app.is_adding_conn = false;
                    app.editing_conn_idx = None;
                    return AppAction::SaveConnection(new_cfg, edit_idx);
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
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') => return AppAction::Quit,
            KeyCode::Char('d') => {
                if app.active_tab == ActiveTab::Relationships && app.focused_panel == FocusedPanel::DataPreview {
                    app.diagram_scroll_offset_y += 10;
                    return AppAction::None;
                }
            }
            KeyCode::Char('u') => {
                if app.active_tab == ActiveTab::Relationships && app.focused_panel == FocusedPanel::DataPreview {
                    app.diagram_scroll_offset_y -= 10;
                    return AppAction::None;
                }
            }
            _ => {}
        }
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
        KeyCode::Char('6') => app.active_tab = ActiveTab::Relationships,
        KeyCode::Char('?') => app.active_tab = ActiveTab::Help,
        
        KeyCode::Char('s') => {
            app.show_system_tables = !app.show_system_tables;
            app.selected_table_idx = 0;
            app.update_filtered_tables();
            if let Some(tbl) = app.filtered_tables().get(0) {
                return AppAction::FetchTableData(tbl.schema.clone(), tbl.name.clone());
            }
        }
        KeyCode::Char('/') => {
            if app.active_tab == ActiveTab::Browser || app.active_tab == ActiveTab::Relationships {
                if app.is_fullscreen_data {
                    app.is_filtering_data = !app.is_filtering_data;
                } else {
                    app.is_filtering = !app.is_filtering;
                }
            }
        }
        KeyCode::Char('a') => {
            if app.active_tab == ActiveTab::Connections {
                app.is_adding_conn = true;
                app.editing_conn_idx = None;
                app.conn_form_step = 0;
                app.conn_input_name.clear();
                app.conn_input_host = "127.0.0.1".to_string();
                app.conn_input_port = "5432".to_string();
                app.conn_input_user = "postgres".to_string();
                app.conn_input_pass.clear();
                app.conn_input_dbname = "postgres".to_string();
            } else if app.active_tab == ActiveTab::Relationships {
                app.show_all_relationships = !app.show_all_relationships;
            }
        }
        KeyCode::Char('e') | KeyCode::Char('u') => {
            if app.active_tab == ActiveTab::Connections {
                if !app.config.connections.is_empty() && app.selected_conn_idx < app.config.connections.len() {
                    let conn = &app.config.connections[app.selected_conn_idx];
                    app.conn_input_name = conn.name.clone();
                    app.conn_input_host = conn.host.clone();
                    app.conn_input_port = conn.port.to_string();
                    app.conn_input_user = conn.user.clone();
                    app.conn_input_pass = conn.password.clone().unwrap_or_default();
                    app.conn_input_dbname = conn.dbname.clone();
                    app.conn_form_step = 0;
                    app.editing_conn_idx = Some(app.selected_conn_idx);
                    app.is_adding_conn = true;
                }
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
        KeyCode::Char('r') => {
            if app.active_tab == ActiveTab::Relationships {
                app.layout_seed += 1;
                app.diagram_scroll_offset_y = 0;
                app.diagram_scroll_offset_x = 0;
            } else {
                return AppAction::RefreshTables;
            }
        }
        
        KeyCode::Tab => {
            if app.active_tab == ActiveTab::Browser || app.active_tab == ActiveTab::Relationships {
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
                app.cell_width = (app.cell_width + 4).min(500);
            }
        }

        KeyCode::Char('-') | KeyCode::Char('_') => {
            if app.active_tab == ActiveTab::Browser {
                app.cell_width = app.cell_width.saturating_sub(4).max(8);
            }
        }

        KeyCode::Char('i') => {
            if app.active_tab == ActiveTab::Relationships {
                app.relationship_zoom = (app.relationship_zoom + 1).min(4);
            }
        }

        KeyCode::Char('o') => {
            if app.active_tab == ActiveTab::Relationships {
                app.relationship_zoom = app.relationship_zoom.saturating_sub(1).max(1);
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
            } else if app.active_tab == ActiveTab::Relationships {
                if app.focused_panel == FocusedPanel::DataPreview {
                    navigate_relationships_2d(app, 'h');
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
            } else if app.active_tab == ActiveTab::Relationships {
                if app.focused_panel == FocusedPanel::DataPreview {
                    navigate_relationships_2d(app, 'l');
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
                    }
                }
            }
            ActiveTab::Relationships => {
                if app.focused_panel == FocusedPanel::DataPreview {
                    navigate_relationships_2d(app, 'j');
                } else {
                    let count = app.filtered_tables().len();
                    if count > 0 {
                        app.selected_table_idx = (app.selected_table_idx + 1).min(count - 1);
                        app.selected_data_row = 0;
                        app.diagram_scroll_offset_y = 0;
                        app.diagram_scroll_offset_x = 0;
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
                }
            }
            ActiveTab::Relationships => {
                if app.focused_panel == FocusedPanel::DataPreview {
                    navigate_relationships_2d(app, 'k');
                } else if app.selected_table_idx > 0 {
                    app.selected_table_idx -= 1;
                    app.selected_data_row = 0;
                    app.diagram_scroll_offset_y = 0;
                    app.diagram_scroll_offset_x = 0;
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
            ActiveTab::Relationships => {
                if let Some(tbl) = app.filtered_tables().get(app.selected_table_idx) {
                    let schema = tbl.schema.clone();
                    let name = tbl.name.clone();
                    app.active_tab = ActiveTab::Browser;
                    app.focused_panel = FocusedPanel::DataPreview;
                    app.data_page = 0;
                    app.data_scroll_offset = 0;
                    app.selected_data_row = 0;
                    app.selected_data_col = 0;
                    return AppAction::FetchTableData(schema, name);
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

fn navigate_relationships_2d(app: &mut AppState, direction: char) {
    // Reset manual scroll offsets when hopping
    app.diagram_scroll_offset_y = 0;
    app.diagram_scroll_offset_x = 0;

    let tables = app.filtered_tables();
    let current_tbl = match tables.get(app.selected_table_idx) {
        Some(t) => t,
        None => return,
    };

    // Calculate layout coordinates
    let (layout, _) = crate::app::get_global_table_layout(&app.tables, &app.all_foreign_keys, app.relationship_zoom, app.layout_seed);

    // Find current table coordinates
    let current_pos = layout.iter().find(|(key, _, _, _)| key.0 == current_tbl.schema && key.1 == current_tbl.name);
    let (curr_x, curr_y) = match current_pos {
        Some(&(_, x, y, _)) => (x as isize, y as isize),
        None => return,
    };

    // Find the closest table in the given direction
    let mut closest_tbl_idx = None;
    let mut min_distance = isize::MAX;

    for (idx, tbl) in tables.iter().enumerate() {
        if idx == app.selected_table_idx {
            continue;
        }

        let pos = layout.iter().find(|(key, _, _, _)| key.0 == tbl.schema && key.1 == tbl.name);
        if let Some(&(_, x, y, _)) = pos {
            let tx = x as isize;
            let ty = y as isize;

            let is_in_direction = match direction {
                'k' => ty < curr_y,  // Up
                'j' => ty > curr_y,  // Down
                'h' => tx < curr_x,  // Left
                'l' => tx > curr_x,  // Right
                _ => false,
            };

            if is_in_direction {
                let dist = (tx - curr_x).pow(2) + (ty - curr_y).pow(2);
                if dist < min_distance {
                    min_distance = dist;
                    closest_tbl_idx = Some(idx);
                }
            }
        }
    }

    if let Some(idx) = closest_tbl_idx {
        app.selected_table_idx = idx;
    }
}
