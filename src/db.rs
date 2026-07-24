use tokio_postgres::{Client, NoTls};
use crate::config::ConnectionConfig;

#[derive(Debug, Clone)]
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
            table_name::text, 
            table_schema::text, 
            0::bigint as row_count
        FROM information_schema.tables
        ORDER BY 
            CASE WHEN table_schema NOT IN ('pg_catalog', 'information_schema') THEN 0 ELSE 1 END,
            table_schema, 
            table_name;
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
            c.column_name::text, 
            c.data_type::text, 
            c.is_nullable::text,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY'
                AND tc.table_schema = $1
                AND tc.table_name = $2
        ) pk ON c.column_name = pk.column_name
        WHERE c.table_schema = $1 AND c.table_name = $2
        ORDER BY c.ordinal_position;
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
            kcu.column_name::text,
            ccu.table_schema::text AS foreign_table_schema,
            ccu.table_name::text AS foreign_table_name,
            ccu.column_name::text AS foreign_column_name
        FROM information_schema.table_constraints AS tc
        JOIN information_schema.key_column_usage AS kcu
            ON tc.constraint_name = kcu.constraint_name
            AND tc.table_schema = kcu.table_schema
        JOIN information_schema.constraint_column_usage AS ccu
            ON ccu.constraint_name = tc.constraint_name
            AND ccu.table_schema = tc.table_schema
        WHERE tc.constraint_type = 'FOREIGN KEY'
            AND tc.table_schema = $1
            AND tc.table_name = $2;
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
