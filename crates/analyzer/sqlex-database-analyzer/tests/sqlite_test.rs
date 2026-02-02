use sqlex_analyzer::{Analyzer, DataType};
use sqlex_database_analyzer::SqliteDatabaseAnalyzer;

#[tokio::test]
async fn test_sqlite_analyzer_basic() -> anyhow::Result<()> {
    // Sqlite analyzer runs in memory, no need for docker
    let mut analyzer = SqliteDatabaseAnalyzer::new().await?;

    // Create a simple table
    analyzer
        .execute(
            r#"
        CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            email VARCHAR(100),
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP,
            is_active BOOLEAN
        )
    "#,
        )
        .await?;

    // Insert some data
    analyzer
        .execute(
            r#"
        INSERT INTO users (username, email, updated_at, is_active)
        VALUES ('alice', 'alice@example.com', CURRENT_TIMESTAMP, 1)
    "#,
        )
        .await?;

    // Get all tables
    let tables = analyzer.get_all_tables().await?;
    assert_eq!(tables.len(), 1);

    let user_table = &tables[0];
    assert_eq!(user_table.name, "users");

    let columns = &user_table.columns;
    assert_eq!(columns.len(), 6);

    let id_col = columns.iter().find(|c| c.name == "id").unwrap();
    assert_eq!(id_col.data_type, DataType::BigInt); // INTEGER is mapped to BigInt

    let username_col = columns.iter().find(|c| c.name == "username").unwrap();
    assert_eq!(username_col.data_type, DataType::Text);
    assert_eq!(username_col.nullability, false);

    let email_col = columns.iter().find(|c| c.name == "email").unwrap();
    assert_eq!(email_col.data_type, DataType::Text); // varchar contains char

    let created_at_col = columns.iter().find(|c| c.name == "created_at").unwrap();
    // DATETIME is mapped to Custom because it's not explicitly handled in map_string_type logic for datetime keyword?
    // Wait, map_string_type:
    // if contains int -> BigInt
    // else if contains char, clob, text -> Text
    // else if contains blob -> Binary
    // else if contains real, floa, doub -> Double
    // else -> Custom
    // "DATETIME" does not contain any of those. So usage depends on exact string.
    // Actually the mapping logic is a bit barebones for SQLite. Let's see what happens.
    // If it returns custom("datetime"), that's acceptable for now as per current implementation.
    // Or maybe I should update the implementation to handle datetime?
    // The prompt asks for comprehensive tests, maybe I should just assert what it currently does.
    // But "DATETIME" is common.
    // Let's assume it returns Custom("datetime") for now based on reading the code.
    // Update: wait datetime contains "time" which is not in the list.
    assert_eq!(created_at_col.data_type, DataType::DateTime);

    let is_active_col = columns.iter().find(|c| c.name == "is_active").unwrap();
    // BOOLEAN contains none of the keywords above?
    assert_eq!(is_active_col.data_type, DataType::Bool);

    Ok(())
}

#[tokio::test]
async fn test_sqlite_analyzer_all_types() -> anyhow::Result<()> {
    let mut analyzer = SqliteDatabaseAnalyzer::new().await?;

    // Create a table with robust types
    // SQLite types are affinities really.
    analyzer
        .execute(
            r#"
        CREATE TABLE all_types (
            id INTEGER PRIMARY KEY,
            col_int INT,
            col_integer INTEGER,
            col_tinyint TINYINT,
            col_smallint SMALLINT,
            col_mediumint MEDIUMINT,
            col_bigint BIGINT,
            col_unsigned_big_int UNSIGNED BIG INT,
            col_int2 INT2,
            col_int8 INT8,
            col_character CHAR(20),
            col_varchar VARCHAR(255),
            col_varying_character VARYING CHARACTER(255),
            col_nchar NCHAR(55),
            col_native_character NATIVE CHARACTER(70),
            col_nvarchar NVARCHAR(100),
            col_text TEXT,
            col_clob CLOB,
            col_blob BLOB,
            col_real REAL,
            col_double DOUBLE,
            col_double_precision DOUBLE PRECISION,
            col_float FLOAT
        )
    "#,
        )
        .await?;

    let tables = analyzer.get_all_tables().await?;
    let table = tables.iter().find(|t| t.name == "all_types").unwrap();

    let find_col = |name: &str| table.columns.iter().find(|c| c.name == name).unwrap();
    // Helper to check type
    let check_type = |name: &str, expected: DataType| {
        let col = find_col(name);
        // dbg!(&col.name, &col.data_type);
        assert_eq!(col.data_type, expected, "Column: {}", name);
    };

    // All int-like types should map to BigInt because they contain "int"
    check_type("col_int", DataType::BigInt);
    check_type("col_integer", DataType::BigInt);
    check_type("col_tinyint", DataType::BigInt);
    check_type("col_smallint", DataType::BigInt);
    check_type("col_mediumint", DataType::BigInt);
    check_type("col_bigint", DataType::BigInt);
    check_type("col_unsigned_big_int", DataType::BigInt);
    check_type("col_int2", DataType::BigInt);
    check_type("col_int8", DataType::BigInt);

    // Text-like types
    check_type("col_character", DataType::Text);
    check_type("col_varchar", DataType::Text);
    check_type("col_varying_character", DataType::Text);
    check_type("col_nchar", DataType::Text);
    check_type("col_native_character", DataType::Text);
    check_type("col_nvarchar", DataType::Text);
    check_type("col_text", DataType::Text);
    check_type("col_clob", DataType::Text);

    // Blob
    check_type("col_blob", DataType::Binary);

    // Floating point
    check_type("col_real", DataType::Double);
    check_type("col_double", DataType::Double);
    check_type("col_double_precision", DataType::Double);
    check_type("col_float", DataType::Double); // "float" contains "floa"

    Ok(())
}

#[tokio::test]
async fn test_sqlite_analyzer_query_analysis() -> anyhow::Result<()> {
    let mut analyzer = SqliteDatabaseAnalyzer::new().await?;

    analyzer
        .execute(
            r#"
        CREATE TABLE products (
            id INTEGER PRIMARY KEY,
            name TEXT,
            price REAL,
            in_stock INTEGER
        )
    "#,
        )
        .await?;

    // Analyze SELECT
    let result = analyzer
        .analyze("SELECT id, name, price, in_stock FROM products")
        .await?;
    assert_eq!(result.columns.len(), 4);

    // SQLite return types from analyze can differ from table definition depending on how sqlx infers them.
    // But usually for prepared statements it infers from schema if available.

    assert_eq!(result.columns[0].name, "id");
    assert_eq!(result.columns[0].data_type, DataType::BigInt);

    assert_eq!(result.columns[1].name, "name");
    assert_eq!(result.columns[1].data_type, DataType::Text);

    assert_eq!(result.columns[2].name, "price");
    assert_eq!(result.columns[2].data_type, DataType::Double);

    assert_eq!(result.columns[3].name, "in_stock");
    assert_eq!(result.columns[3].data_type, DataType::BigInt);

    Ok(())
}
