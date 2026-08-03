use crate::config::{Config, ConnectionConfig};
use crate::db::{ColumnInfo, QueryResult, TableInfo};
use tokio_postgres::Client;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FocusedPanel {
    Tables,
    DataPreview,
    QueryEditor,
    Results,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ActiveTab {
    Browser,
    Databases,
    Users,
    QueryRunner,
    Connections,
    Relationships,
    Help,
}

pub struct AppState {
    pub config: Config,
    pub client: Option<Client>,
    pub connected: bool,
    pub status_message: String,
    
    pub active_tab: ActiveTab,
    pub focused_panel: FocusedPanel,
    
    // Database schema data
    pub tables: Vec<TableInfo>,
    pub databases_result: Option<QueryResult>,
    pub selected_db_idx: usize,
    pub users_result: Option<QueryResult>,
    pub selected_table_idx: usize,
    pub columns: Vec<ColumnInfo>,
    pub foreign_keys: Vec<crate::db::ForeignKeyInfo>,
    pub all_foreign_keys: Vec<crate::db::AllForeignKeyInfo>,
    
    // Fast In-Memory Caching
    pub column_cache: std::collections::HashMap<String, Vec<ColumnInfo>>,
    pub table_data_cache: std::collections::HashMap<String, QueryResult>,
    pub show_system_tables: bool,
    pub filter_text: String,
    pub is_filtering: bool,
    pub filter_data_text: String,
    pub is_filtering_data: bool,
    pub field_search_text: String,
    pub is_field_searching: bool,
    pub filtered_table_indices: Vec<usize>,
    pub matcher: nucleo_matcher::Matcher,

    // Custom SQL Query Runner
    pub sql_input: String,
    pub query_result: Option<QueryResult>,
    pub query_error: Option<String>,
    pub result_scroll: usize,
    pub selected_query_row: usize,
    pub selected_query_col: usize,
    pub query_col_offset: usize,

    // Table Data View & Grid Navigation
    pub table_data_result: Option<QueryResult>,
    pub data_page: usize,
    pub data_scroll_offset: usize,
    pub data_col_offset: usize,
    pub selected_data_row: usize,
    pub selected_data_col: usize,
    pub is_fullscreen_data: bool,
    pub breadcrumbs: Vec<String>,
    pub active_breadcrumb_idx: usize,
    pub cell_width: u16,

    // Connection Switcher & Manager
    pub selected_conn_idx: usize,
    pub is_adding_conn: bool,
    pub editing_conn_idx: Option<usize>,
    pub conn_form_step: usize,
    pub conn_input_name: String,
    pub conn_input_host: String,
    pub conn_input_port: String,
    pub conn_input_user: String,
    pub conn_input_pass: String,
    pub conn_input_dbname: String,
    pub is_loading: bool,
    pub relationship_zoom: usize,
    pub diagram_scroll_offset_y: isize,
    pub diagram_scroll_offset_x: isize,
    pub layout_seed: usize,
    pub show_all_relationships: bool,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        AppState {
            config,
            client: None,
            connected: false,
            status_message: "Disconnected. Press 'c' to connect.".to_string(),
            active_tab: ActiveTab::Browser,
            focused_panel: FocusedPanel::Tables,
            tables: Vec::new(),
            databases_result: None,
            selected_db_idx: 0,
            users_result: None,
            selected_table_idx: 0,
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            all_foreign_keys: Vec::new(),
            column_cache: std::collections::HashMap::new(),
            table_data_cache: std::collections::HashMap::new(),
            show_system_tables: false,
            filter_text: String::new(),
            is_filtering: false,
            filter_data_text: String::new(),
            is_filtering_data: false,
            field_search_text: String::new(),
            is_field_searching: false,
            sql_input: "SELECT * FROM information_schema.tables LIMIT 10;".to_string(),
            query_result: None,
            query_error: None,
            result_scroll: 0,
            selected_query_row: 0,
            selected_query_col: 0,
            query_col_offset: 0,
            table_data_result: None,
            data_page: 0,
            data_scroll_offset: 0,
            data_col_offset: 0,
            selected_data_row: 0,
            selected_data_col: 0,
            is_fullscreen_data: false,
            breadcrumbs: Vec::new(),
            active_breadcrumb_idx: 0,
            cell_width: 22,
            selected_conn_idx: 0,
            is_adding_conn: false,
            editing_conn_idx: None,
            conn_form_step: 0,
            conn_input_name: String::new(),
            conn_input_host: "127.0.0.1".to_string(),
            conn_input_port: "5432".to_string(),
            conn_input_user: "postgres".to_string(),
            conn_input_pass: String::new(),
            conn_input_dbname: "postgres".to_string(),
            filtered_table_indices: Vec::new(),
            is_loading: false,
            matcher: nucleo_matcher::Matcher::default(),
            relationship_zoom: 2,
            diagram_scroll_offset_y: 0,
            diagram_scroll_offset_x: 0,
            layout_seed: 0,
            show_all_relationships: false,
        }
    }

    pub fn filtered_tables(&self) -> Vec<&TableInfo> {
        self.filtered_table_indices.iter().map(|&idx| &self.tables[idx]).collect()
    }

    pub fn update_filtered_tables(&mut self) {
        let fl = self.filter_text.trim();
        if fl.is_empty() {
            self.filtered_table_indices = self.tables
                .iter()
                .enumerate()
                .filter(|(_, t)| self.show_system_tables || (t.schema != "information_schema" && t.schema != "pg_catalog" && !t.name.starts_with("_pg_")))
                .map(|(i, _)| i)
                .collect();
            return;
        }

        let pattern = nucleo_matcher::pattern::Pattern::parse(fl, nucleo_matcher::pattern::CaseMatching::Ignore, nucleo_matcher::pattern::Normalization::Smart);
        let mut buf = Vec::new();
        let matcher = &mut self.matcher;

        let mut matched_tables: Vec<(usize, u32)> = self.tables
            .iter()
            .enumerate()
            .filter(|(_, t)| self.show_system_tables || (t.schema != "information_schema" && t.schema != "pg_catalog" && !t.name.starts_with("_pg_")))
            .filter_map(|(i, t)| {
                let haystack = nucleo_matcher::Utf32Str::new(&t.name, &mut buf);
                pattern.score(haystack, matcher).map(|score| (i, score))
            })
            .collect();

        matched_tables.sort_by(|a, b| b.1.cmp(&a.1));
        self.filtered_table_indices = matched_tables.into_iter().map(|(i, _)| i).collect();
    }

    pub fn current_connection(&self) -> Option<&ConnectionConfig> {
        if self.config.connections.is_empty() {
            None
        } else {
            let idx = self.selected_conn_idx.min(self.config.connections.len() - 1);
            Some(&self.config.connections[idx])
        }
    }
}

pub fn get_global_table_layout(
    tables: &[crate::db::TableInfo],
    all_foreign_keys: &[crate::db::AllForeignKeyInfo],
    zoom: usize,
    layout_seed: usize,
) -> (
    Vec<((String, String), usize, usize, usize)>, // ((schema, name), x, y, h)
    std::collections::HashMap<(String, String, String), (usize, usize)>, // ((schema, table, col) -> (x, y))
) {
    let mut layout = Vec::new();
    let mut col_coords = std::collections::HashMap::new();

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
    let vertical_gap = match zoom {
        1 => 3,
        2 => 4,
        3 => 5,
        _ => 6,
    };

    // Calculate connection count for each table to determine center vs left vs right
    let mut table_conn_counts = std::collections::HashMap::new();
    for fk in all_foreign_keys {
        *table_conn_counts.entry((fk.table_schema.clone(), fk.table_name.clone())).or_insert(0) += 1;
        *table_conn_counts.entry((fk.foreign_table_schema.clone(), fk.foreign_table_name.clone())).or_insert(0) += 1;
    }

    // Sort tables by connection count
    let mut center_tables = Vec::new();
    let mut left_tables = Vec::new();
    let mut right_tables = Vec::new();

    let mut tbls_with_relations = Vec::new();
    for tbl in tables {
        let count = *table_conn_counts.get(&(tbl.schema.clone(), tbl.name.clone())).unwrap_or(&0);
        if count > 0 {
            tbls_with_relations.push((tbl, count));
        }
    }
    tbls_with_relations.sort_by(|a, b| b.1.cmp(&a.1));

    for (idx, (tbl, _)) in tbls_with_relations.iter().enumerate() {
        let column = (idx + layout_seed) % 3;
        match column {
            0 => left_tables.push(*tbl),
            1 => center_tables.push(*tbl),
            _ => right_tables.push(*tbl),
        }
    }

    let left_x = 4;
    let center_x = left_x + box_width + spacing;
    let right_x = center_x + box_width + spacing;

    // Stack Left
    let mut y_left = 10;
    for tbl in left_tables {
        let mut key_cols = std::collections::BTreeSet::new();
        for fk in all_foreign_keys {
            if fk.table_schema == tbl.schema && fk.table_name == tbl.name {
                key_cols.insert(fk.column_name.clone());
            }
            if fk.foreign_table_schema == tbl.schema && fk.foreign_table_name == tbl.name {
                key_cols.insert(fk.foreign_column_name.clone());
            }
        }
        let box_height = 4 + key_cols.len();
        layout.push(((tbl.schema.clone(), tbl.name.clone()), left_x, y_left, box_height));

        let mut col_y = y_left + 3;
        for col in key_cols {
            col_coords.insert((tbl.schema.clone(), tbl.name.clone(), col), (left_x + box_width - 1, col_y));
            col_y += 1;
        }
        y_left += box_height + vertical_gap;
    }

    // Stack Right
    let mut y_right = 10;
    for tbl in right_tables {
        let mut key_cols = std::collections::BTreeSet::new();
        for fk in all_foreign_keys {
            if fk.table_schema == tbl.schema && fk.table_name == tbl.name {
                key_cols.insert(fk.column_name.clone());
            }
            if fk.foreign_table_schema == tbl.schema && fk.foreign_table_name == tbl.name {
                key_cols.insert(fk.foreign_column_name.clone());
            }
        }
        let box_height = 4 + key_cols.len();
        layout.push(((tbl.schema.clone(), tbl.name.clone()), right_x, y_right, box_height));

        let mut col_y = y_right + 3;
        for col in key_cols {
            col_coords.insert((tbl.schema.clone(), tbl.name.clone(), col), (right_x, col_y));
            col_y += 1;
        }
        y_right += box_height + vertical_gap;
    }

    // Stack Center
    let mut y_center = 10;
    for tbl in center_tables {
        let mut key_cols = std::collections::BTreeSet::new();
        for fk in all_foreign_keys {
            if fk.table_schema == tbl.schema && fk.table_name == tbl.name {
                key_cols.insert(fk.column_name.clone());
            }
            if fk.foreign_table_schema == tbl.schema && fk.foreign_table_name == tbl.name {
                key_cols.insert(fk.foreign_column_name.clone());
            }
        }
        let box_height = 4 + key_cols.len();
        layout.push(((tbl.schema.clone(), tbl.name.clone()), center_x, y_center, box_height));

        let mut col_y = y_center + 3;
        for col in key_cols {
            col_coords.insert((tbl.schema.clone(), tbl.name.clone(), col), (center_x + box_width - 1, col_y));
            col_y += 1;
        }
        y_center += box_height + vertical_gap;
    }

    (layout, col_coords)
}


