use tokio_postgres::{Client, NoTls};
use crate::config::ConnectionConfig;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: String,
    pub row_count_est: i64,
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_nullable: String,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone)]
pub struct ForeignKeyInfo {
    pub column_name: String,
    pub foreign_table_schema: String,
    pub foreign_table_name: String,
    #[allow(dead_code)]
    pub foreign_column_name: String,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub execution_time_ms: u128,
}

pub async fn connect(cfg: &ConnectionConfig) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn_str = format!(
        "host={} port={} user={} dbname={}",
        cfg.host, cfg.port, cfg.user, cfg.dbname
    );
    if let Some(ref pass) = cfg.password {
        conn_str.push_str(&format!(" password={}", pass));
    }

    let (client, connection) = tokio_postgres::connect(&conn_str, NoTls).await?;
    
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn fetch_tables(client: &Client) -> Result<Vec<TableInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let query = "
        SELECT 
            c.relname::text AS table_name,
            n.nspname::text AS table_schema,
            c.reltuples::bigint AS row_count
        FROM pg_catalog.pg_class c
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        WHERE c.relkind = 'r'
        ORDER BY 
            CASE WHEN n.nspname NOT IN ('pg_catalog', 'information_schema') THEN 0 ELSE 1 END,
            n.nspname, 
            c.relname;
    ";
    let rows = client.query(query, &[]).await?;
    let mut tables = Vec::new();
    for r in rows {
        tables.push(TableInfo {
            name: r.get(0),
            schema: r.get(1),
            row_count_est: r.get(2),
        });
    }
    Ok(tables)
}

pub async fn fetch_columns(client: &Client, schema: &str, table: &str) -> Result<Vec<ColumnInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let query = "
        SELECT 
            a.attname::text AS column_name,
            pg_catalog.format_type(a.atttypid, a.atttypmod)::text AS data_type,
            CASE WHEN a.attnotnull THEN 'NO' ELSE 'YES' END::text AS is_nullable,
            CASE WHEN pk.attnum IS NOT NULL THEN true ELSE false END AS is_primary_key
        FROM pg_catalog.pg_attribute a
        JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN (
            SELECT unnest(conkey) as attnum, conrelid
            FROM pg_catalog.pg_constraint
            WHERE contype = 'p'
        ) pk ON pk.conrelid = c.oid AND pk.attnum = a.attnum
        WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped
        ORDER BY a.attnum;
    ";
    let rows = client.query(query, &[&schema, &table]).await?;
    let mut cols = Vec::new();
    for r in rows {
        cols.push(ColumnInfo {
            name: r.get(0),
            data_type: r.get(1),
            is_nullable: r.get(2),
            is_primary_key: r.get(3),
        });
    }
    Ok(cols)
}

pub async fn fetch_foreign_keys(client: &Client, schema: &str, table: &str) -> Result<Vec<ForeignKeyInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let query = "
        SELECT
            a.attname::text AS column_name,
            confns.nspname::text AS foreign_table_schema,
            confcl.relname::text AS foreign_table_name,
            fa.attname::text AS foreign_column_name
        FROM pg_catalog.pg_constraint con
        JOIN pg_catalog.pg_class c ON c.oid = con.conrelid
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(con.conkey)
        JOIN pg_catalog.pg_class confcl ON confcl.oid = con.confrelid
        JOIN pg_catalog.pg_namespace confns ON confns.oid = confcl.relnamespace
        JOIN pg_catalog.pg_attribute fa ON fa.attrelid = confcl.oid AND fa.attnum = ANY(con.confkey)
        WHERE con.contype = 'f'
          AND n.nspname = $1
          AND c.relname = $2;
    ";
    let rows = client.query(query, &[&schema, &table]).await?;
    let mut fks = Vec::new();
    for r in rows {
        fks.push(ForeignKeyInfo {
            column_name: r.get(0),
            foreign_table_schema: r.get(1),
            foreign_table_name: r.get(2),
            foreign_column_name: r.get(3),
        });
    }
    Ok(fks)
}

#[derive(Debug, Clone)]
pub struct AllForeignKeyInfo {
    pub table_schema: String,
    pub table_name: String,
    pub column_name: String,
    pub foreign_table_schema: String,
    pub foreign_table_name: String,
    pub foreign_column_name: String,
}

pub async fn fetch_all_foreign_keys(client: &Client) -> Result<Vec<AllForeignKeyInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let query = "
        SELECT
            n.nspname::text AS table_schema,
            c.relname::text AS table_name,
            a.attname::text AS column_name,
            confns.nspname::text AS foreign_table_schema,
            confcl.relname::text AS foreign_table_name,
            fa.attname::text AS foreign_column_name
        FROM pg_catalog.pg_constraint con
        JOIN pg_catalog.pg_class c ON c.oid = con.conrelid
        JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
        JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid AND a.attnum = ANY(con.conkey)
        JOIN pg_catalog.pg_class confcl ON confcl.oid = con.confrelid
        JOIN pg_catalog.pg_namespace confns ON confns.oid = confcl.relnamespace
        JOIN pg_catalog.pg_attribute fa ON fa.attrelid = confcl.oid AND fa.attnum = ANY(con.confkey)
        WHERE con.contype = 'f'
        ORDER BY n.nspname, c.relname, a.attname;
    ";
    let rows = client.query(query, &[]).await?;
    let mut fks = Vec::new();
    for r in rows {
        fks.push(AllForeignKeyInfo {
            table_schema: r.get(0),
            table_name: r.get(1),
            column_name: r.get(2),
            foreign_table_schema: r.get(3),
            foreign_table_name: r.get(4),
            foreign_column_name: r.get(5),
        });
    }
    Ok(fks)
}

pub async fn execute_sql(client: &Client, sql: &str) -> Result<QueryResult, Box<dyn std::error::Error + Send + Sync>> {
    let start = std::time::Instant::now();
    let rows = client.query(sql, &[]).await?;
    let duration = start.elapsed().as_millis();

    if rows.is_empty() {
        return Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            execution_time_ms: duration,
        });
    }

    let columns: Vec<String> = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
    let mut data_rows = Vec::new();

    for row in &rows {
        let mut row_str = Vec::new();
        for i in 0..row.columns().len() {
            let val: String = if let Ok(v) = row.try_get::<_, String>(i) {
                v
            } else if let Ok(v) = row.try_get::<_, i64>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, i32>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, i16>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, f64>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, f32>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, bool>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, u32>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, chrono::NaiveDateTime>(i) {
                v.to_string()
            } else if let Ok(v) = row.try_get::<_, Vec<String>>(i) {
                format!("{:?}", v)
            } else if let Ok(v) = row.try_get::<_, Vec<i32>>(i) {
                format!("{:?}", v)
            } else if let Ok(v) = row.try_get::<_, Vec<u32>>(i) {
                format!("{:?}", v)
            } else {
                "NULL".to_string()
            };
            row_str.push(val);
        }
        data_rows.push(row_str);
    }

    Ok(QueryResult {
        columns,
        rows: data_rows,
        execution_time_ms: duration,
    })
}

#[derive(Serialize, Deserialize)]
struct TableCache {
    tables: Vec<TableInfo>,
}

pub fn get_table_cache_path(conn_name: &str) -> std::path::PathBuf {
    if let Some(mut path) = dirs::cache_dir() {
        path.push("tsql");
        path.push(format!("{}_tables.toml", conn_name));
        path
    } else {
        std::path::PathBuf::from(format!("{}_tables.toml", conn_name))
    }
}

pub fn load_table_cache(conn_name: &str) -> Option<Vec<TableInfo>> {
    let path = get_table_cache_path(conn_name);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(cache) = toml::from_str::<TableCache>(&content) {
                return Some(cache.tables);
            }
        }
    }
    None
}

pub fn save_table_cache(conn_name: &str, tables: &[TableInfo]) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_table_cache_path(conn_name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cache = TableCache { tables: tables.to_vec() };
    let content = toml::to_string_pretty(&cache)?;
    std::fs::write(path, content)?;
    Ok(())
}

