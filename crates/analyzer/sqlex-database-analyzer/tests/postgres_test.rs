use sqlex_analyzer::{Analyzer, DataType};
use sqlex_database_analyzer::PostgresDatabaseAnalyzer;

#[tokio::test]
async fn test_postgres_analyzer_basic() -> anyhow::Result<()> {
    let mut analyzer = PostgresDatabaseAnalyzer::new().await?;

    // Create a simple table
    analyzer
        .execute(
            r#"
        CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(50) NOT NULL,
            email VARCHAR(100),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            is_active BOOLEAN DEFAULT TRUE
        )
    "#,
        )
        .await?;

    // Insert some data
    analyzer
        .execute(
            r#"
        INSERT INTO users (username, email, is_active)
        VALUES ('alice', 'alice@example.com', true)
    "#,
        )
        .await?;

    // Get all tables
    let tables = analyzer.get_all_tables().await?;
    assert_eq!(tables.len(), 1);

    let user_table = &tables[0];
    assert_eq!(user_table.name, "users");

    // Verify columns
    let columns = &user_table.columns;
    assert_eq!(columns.len(), 5);

    // Check specific columns
    let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    assert_eq!(id_col.data_type, DataType::Int); // SERIAL is integer

    let username_col = columns.iter().find(|c| c.name == "username").unwrap();
    assert_eq!(username_col.data_type, DataType::Text);
    assert!(!username_col.nullability);

    let email_col = columns.iter().find(|c| c.name == "email").unwrap();
    assert_eq!(email_col.data_type, DataType::Text);
    assert!(email_col.nullability);

    let is_active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
    assert_eq!(is_active_col.data_type, DataType::Bool);

    Ok(())
}

#[tokio::test]
async fn test_postgres_analyzer_all_types() -> anyhow::Result<()> {
    let mut analyzer = PostgresDatabaseAnalyzer::new().await?;

    // Create a table with all supported types
    analyzer
        .execute(
            r#"
        CREATE TABLE all_types (
            id SERIAL PRIMARY KEY,
            col_smallint SMALLINT,
            col_integer INTEGER,
            col_bigint BIGINT,
            col_decimal DECIMAL(10, 2),
            col_real REAL,
            col_double DOUBLE PRECISION,
            col_boolean BOOLEAN,
            col_varchar VARCHAR(255),
            col_text TEXT,
            col_date DATE,
            col_time TIME,
            col_timestamp TIMESTAMP,
            col_timestamptz TIMESTAMPTZ,
            col_json JSON,
            col_jsonb JSONB,
            col_bytea BYTEA,
            col_uuid UUID
        )
    "#,
        )
        .await?;

    let tables = analyzer.get_all_tables().await?;
    let table = tables.iter().find(|t| t.name == "all_types").unwrap();

    let find_col = |name: &str| table.columns.iter().find(|c| c.name == name).unwrap();

    assert_eq!(find_col("col_smallint").data_type, DataType::SmallInt);
    assert_eq!(find_col("col_integer").data_type, DataType::Int);
    assert_eq!(find_col("col_bigint").data_type, DataType::BigInt);
    assert_eq!(find_col("col_decimal").data_type, DataType::Decimal);
    assert_eq!(find_col("col_real").data_type, DataType::Float);
    assert_eq!(find_col("col_double").data_type, DataType::Double);
    assert_eq!(find_col("col_boolean").data_type, DataType::Bool);
    assert_eq!(find_col("col_varchar").data_type, DataType::Text);
    assert_eq!(find_col("col_text").data_type, DataType::Text);
    assert_eq!(find_col("col_date").data_type, DataType::Date);
    assert_eq!(find_col("col_time").data_type, DataType::Time);
    assert_eq!(find_col("col_timestamp").data_type, DataType::DateTime);
    assert_eq!(find_col("col_timestamptz").data_type, DataType::Timestamp);
    assert_eq!(find_col("col_json").data_type, DataType::Json);
    assert_eq!(find_col("col_jsonb").data_type, DataType::Json);
    assert_eq!(find_col("col_bytea").data_type, DataType::Binary);
    assert_eq!(find_col("col_uuid").data_type, DataType::Uuid);

    Ok(())
}

#[tokio::test]
async fn test_postgres_analyzer_query_analysis() -> anyhow::Result<()> {
    let mut analyzer = PostgresDatabaseAnalyzer::new().await?;

    analyzer
        .execute(
            r#"
        CREATE TABLE products (
            id SERIAL PRIMARY KEY,
            name VARCHAR(100),
            price DECIMAL(10, 2),
            in_stock BOOLEAN
        )
    "#,
        )
        .await?;

    // Analyze SELECT
    let result = analyzer
        .analyze("SELECT id, name, price FROM products")
        .await?;
    assert_eq!(result.columns.len(), 3);
    assert_eq!(result.columns[0].name, "id");
    assert_eq!(result.columns[0].data_type, DataType::Int); // SERIAL is int4 which maps to Int
    assert_eq!(result.columns[1].name, "name");
    assert_eq!(result.columns[1].data_type, DataType::Text);
    assert_eq!(result.columns[2].name, "price");
    assert_eq!(result.columns[2].data_type, DataType::Decimal);

    // Analyze INSERT returning
    let result = analyzer
        .analyze("INSERT INTO products (name, price) VALUES ('foo', 10.0) RETURNING id, in_stock")
        .await?;
    assert_eq!(result.columns.len(), 2);
    assert_eq!(result.columns[0].name, "id");
    assert_eq!(result.columns[1].name, "in_stock");
    assert_eq!(result.columns[1].data_type, DataType::Bool);

    Ok(())
}
