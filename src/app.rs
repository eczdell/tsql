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
    
    // Fast In-Memory Caching
    pub column_cache: std::collections::HashMap<String, Vec<ColumnInfo>>,
    pub table_data_cache: std::collections::HashMap<String, QueryResult>,
    pub show_system_tables: bool,
    pub filter_text: String,
    pub is_filtering: bool,

    // Custom SQL Query Runner
    pub sql_input: String,
    pub query_result: Option<QueryResult>,
    pub query_error: Option<String>,
    pub result_scroll: usize,

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
    pub conn_form_step: usize,
    pub conn_input_name: String,
    pub conn_input_host: String,
    pub conn_input_port: String,
    pub conn_input_user: String,
    pub conn_input_pass: String,
    pub conn_input_dbname: String,
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
            column_cache: std::collections::HashMap::new(),
            table_data_cache: std::collections::HashMap::new(),
            show_system_tables: false,
            filter_text: String::new(),
            is_filtering: false,
            sql_input: "SELECT * FROM information_schema.tables LIMIT 10;".to_string(),
            query_result: None,
            query_error: None,
            result_scroll: 0,
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
            conn_form_step: 0,
            conn_input_name: String::new(),
            conn_input_host: "127.0.0.1".to_string(),
            conn_input_port: "5432".to_string(),
            conn_input_user: "postgres".to_string(),
            conn_input_pass: String::new(),
            conn_input_dbname: "postgres".to_string(),
        }
    }

    pub fn filtered_tables(&self) -> Vec<&TableInfo> {
        let fl = self.filter_text.trim();
        if fl.is_empty() {
            return self.tables
                .iter()
                .filter(|t| self.show_system_tables || (t.schema != "information_schema" && t.schema != "pg_catalog" && !t.name.starts_with("_pg_")))
                .collect();
        }

        let mut matcher = nucleo_matcher::Matcher::default();
        let pattern = nucleo_matcher::pattern::Pattern::parse(fl, nucleo_matcher::pattern::CaseMatching::Ignore, nucleo_matcher::pattern::Normalization::Smart);
        let mut buf = Vec::new();

        let mut matched_tables: Vec<(&TableInfo, u32)> = self.tables
            .iter()
            .filter(|t| self.show_system_tables || (t.schema != "information_schema" && t.schema != "pg_catalog" && !t.name.starts_with("_pg_")))
            .filter_map(|t| {
                let full_name = format!("{}.{}", t.schema, t.name);
                let haystack = nucleo_matcher::Utf32Str::new(&full_name, &mut buf);
                pattern.score(haystack, &mut matcher).map(|score| (t, score))
            })
            .collect();

        matched_tables.sort_by(|a, b| b.1.cmp(&a.1));
        matched_tables.into_iter().map(|(t, _)| t).collect()
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
