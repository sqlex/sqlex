use sqlex_analyzer::Analyzer;
use sqlex_common::types::DataType;
use sqlex_database_analyzer::mysql::MySqlDatabaseAnalyzer;

#[tokio::test]
async fn test_mysql_analyzer_basic() -> anyhow::Result<()> {
    let mut analyzer = MySqlDatabaseAnalyzer::new().await?;

    // Create a simple table
    analyzer
        .execute(
            r#"
        CREATE TABLE users (
            id INT AUTO_INCREMENT PRIMARY KEY,
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
    // Note: MySQL column order might not be guaranteed unless we check ordering,
    // but the implementation sorts by ordinal position.

    let columns = &user_table.columns;
    assert_eq!(columns.len(), 5);

    let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    assert_eq!(id_col.data_type, DataType::Int(false));

    let username_col = columns.iter().find(|c| c.name == "username").unwrap();
    assert_eq!(username_col.data_type, DataType::Varchar);
    assert!(!username_col.nullability);

    let email_col = columns.iter().find(|c| c.name == "email").unwrap();
    assert_eq!(email_col.data_type, DataType::Varchar);
    assert!(email_col.nullability);

    // MySQL BOOLEAN is usually TINYINT(1)
    let is_active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
    assert_eq!(is_active_col.data_type, DataType::Bool);

    Ok(())
}

#[tokio::test]
async fn test_mysql_analyzer_all_types() -> anyhow::Result<()> {
    let mut analyzer = MySqlDatabaseAnalyzer::new().await?;

    // Create a table with all supported types
    analyzer
        .execute(
            r#"
        CREATE TABLE all_types (
            id INT AUTO_INCREMENT PRIMARY KEY,
            col_tinyint TINYINT,
            col_smallint SMALLINT,
            col_int INT,
            col_bigint BIGINT,
            col_decimal DECIMAL(10, 2),
            col_float FLOAT,
            col_double DOUBLE,
            col_char CHAR(10),
            col_varchar VARCHAR(255),
            col_text TEXT,
            col_date DATE,
            col_datetime DATETIME,
            col_timestamp TIMESTAMP,
            col_json JSON,
            col_blob BLOB
        )
    "#,
        )
        .await?;

    let tables = analyzer.get_all_tables().await?;
    let table = tables.iter().find(|t| t.name == "all_types").unwrap();

    let find_col = |name: &str| table.columns.iter().find(|c| c.name == name).unwrap();

    assert_eq!(find_col("col_tinyint").data_type, DataType::Bool); // map_string_type "tinyint" -> Bool
    assert_eq!(
        find_col("col_smallint").data_type,
        DataType::SmallInt(false)
    );
    assert_eq!(find_col("col_int").data_type, DataType::Int(false));
    assert_eq!(find_col("col_bigint").data_type, DataType::BigInt(false));
    assert_eq!(find_col("col_decimal").data_type, DataType::Decimal);
    assert_eq!(find_col("col_float").data_type, DataType::Float);
    assert_eq!(find_col("col_double").data_type, DataType::Double);
    assert_eq!(find_col("col_char").data_type, DataType::Char);
    assert_eq!(find_col("col_varchar").data_type, DataType::Varchar);
    assert_eq!(find_col("col_text").data_type, DataType::Text);
    assert_eq!(find_col("col_date").data_type, DataType::Date);
    assert_eq!(find_col("col_datetime").data_type, DataType::DateTime);
    assert_eq!(find_col("col_timestamp").data_type, DataType::Timestamp);
    assert_eq!(find_col("col_json").data_type, DataType::Json);
    assert_eq!(find_col("col_blob").data_type, DataType::Binary);

    Ok(())
}

#[tokio::test]
async fn test_mysql_analyzer_query_analysis() -> anyhow::Result<()> {
    let mut analyzer = MySqlDatabaseAnalyzer::new().await?;

    analyzer
        .execute(
            r#"
        CREATE TABLE products (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100),
            price DECIMAL(10, 2),
            in_stock BOOLEAN
        )
    "#,
        )
        .await?;

    // Analyze SELECT
    let result = analyzer
        .analyze("SELECT id, name, price, in_stock FROM products")
        .await?;
    assert_eq!(result.columns.len(), 4);
    assert_eq!(result.columns[0].name, "id");
    assert_eq!(result.columns[0].data_type, DataType::Int(false));
    assert_eq!(result.columns[1].name, "name");
    assert_eq!(result.columns[1].data_type, DataType::Varchar);
    assert_eq!(result.columns[2].name, "price");
    assert_eq!(result.columns[2].data_type, DataType::Decimal);
    assert_eq!(result.columns[3].name, "in_stock");
    assert_eq!(result.columns[3].data_type, DataType::Bool); // tinyint(1) -> bool

    // Analyze simplified INSERT
    // MySQL INSERT doesn't typically return values unless using specialized syntax or just nothing
    // So we just check a simple select again or another query type.
    // Or we test a complex query with joins if needed, but basic select covers most logic.

    Ok(())
}
