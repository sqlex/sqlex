use sqlex_common::{dialect::Dialect, types::DataType as CommonDataType};
use sqlparser::ast::DataType as SqlDataType;

pub(crate) fn map_sql_data_type(dialect: Dialect, sql_type: &SqlDataType) -> CommonDataType {
    map_type_name(dialect, &sql_type.to_string())
}

fn map_type_name(dialect: Dialect, raw_type: &str) -> CommonDataType {
    let normalized = raw_type.to_ascii_lowercase();

    match dialect {
        Dialect::Postgres => map_postgres_type(&normalized),
        Dialect::MySQL => map_mysql_type(&normalized),
        Dialect::SQLite => map_sqlite_type(&normalized),
    }
}

fn map_postgres_type(value: &str) -> CommonDataType {
    if value == "bool" || value == "boolean" {
        return CommonDataType::Bool;
    }
    if value == "smallint" || value == "int2" {
        return CommonDataType::SmallInt;
    }
    if value == "integer" || value == "int4" || value == "int" {
        return CommonDataType::Int;
    }
    if value == "bigint" || value == "int8" {
        return CommonDataType::BigInt;
    }
    if value == "real" || value == "float4" {
        return CommonDataType::Float;
    }
    if value == "double precision" || value == "float8" {
        return CommonDataType::Double;
    }
    if value.starts_with("numeric") || value.starts_with("decimal") {
        return CommonDataType::Decimal;
    }
    if value.starts_with("varchar") || value.starts_with("character varying") {
        return CommonDataType::Varchar;
    }
    if value.starts_with("char") || value.starts_with("character") || value.starts_with("bpchar") {
        return CommonDataType::Char;
    }
    if value == "text" {
        return CommonDataType::Text;
    }
    if value == "date" {
        return CommonDataType::Date;
    }
    if value.starts_with("time") || value == "timetz" {
        return CommonDataType::Time;
    }
    if value == "timestamptz" || value.contains("timestamp with time zone") {
        return CommonDataType::Timestamp;
    }
    if value.starts_with("timestamp") {
        return CommonDataType::DateTime;
    }
    if value == "uuid" {
        return CommonDataType::Uuid;
    }
    if value == "json" || value == "jsonb" {
        return CommonDataType::Json;
    }
    if value == "bytea" {
        return CommonDataType::Binary;
    }

    CommonDataType::Custom(value.to_string())
}

fn map_mysql_type(value: &str) -> CommonDataType {
    let unsigned = value.contains("unsigned");

    if value.starts_with("tinyint") {
        return if unsigned {
            CommonDataType::UnsignedTinyInt
        } else {
            CommonDataType::TinyInt
        };
    }
    if value.starts_with("smallint") {
        return if unsigned {
            CommonDataType::UnsignedSmallInt
        } else {
            CommonDataType::SmallInt
        };
    }
    if value.starts_with("int") || value.starts_with("integer") {
        return if unsigned {
            CommonDataType::UnsignedInt
        } else {
            CommonDataType::Int
        };
    }
    if value.starts_with("bigint") {
        return if unsigned {
            CommonDataType::UnsignedBigInt
        } else {
            CommonDataType::BigInt
        };
    }

    if value.starts_with("bool") || value.starts_with("boolean") {
        return CommonDataType::Bool;
    }
    if value.starts_with("float") {
        return CommonDataType::Float;
    }
    if value.starts_with("double") {
        return CommonDataType::Double;
    }
    if value.starts_with("decimal") || value.starts_with("numeric") {
        return CommonDataType::Decimal;
    }
    if value.starts_with("varchar") {
        return CommonDataType::Varchar;
    }
    if value.starts_with("char") {
        return CommonDataType::Char;
    }
    if value.contains("text") {
        return CommonDataType::Text;
    }
    if value.starts_with("date") {
        return CommonDataType::Date;
    }
    if value.starts_with("datetime") {
        return CommonDataType::DateTime;
    }
    if value.starts_with("timestamp") {
        return CommonDataType::Timestamp;
    }
    if value.starts_with("json") {
        return CommonDataType::Json;
    }
    if value.contains("blob") || value.starts_with("binary") || value.starts_with("varbinary") {
        return CommonDataType::Binary;
    }

    CommonDataType::Custom(value.to_string())
}

fn map_sqlite_type(value: &str) -> CommonDataType {
    if value.contains("int") {
        return CommonDataType::BigInt;
    }
    if value.contains("char") {
        if value.contains("varchar") {
            return CommonDataType::Varchar;
        }
        return CommonDataType::Char;
    }
    if value.contains("text") || value.contains("clob") {
        return CommonDataType::Text;
    }
    if value.contains("blob") {
        return CommonDataType::Binary;
    }
    if value.contains("real") || value.contains("double") || value.contains("float") {
        return CommonDataType::Double;
    }
    if value.contains("numeric") || value.contains("decimal") {
        return CommonDataType::Decimal;
    }
    if value.contains("bool") {
        return CommonDataType::Bool;
    }
    if value.contains("datetime") {
        return CommonDataType::DateTime;
    }
    if value.contains("timestamp") {
        return CommonDataType::Timestamp;
    }
    if value.contains("date") {
        return CommonDataType::Date;
    }
    if value.contains("time") {
        return CommonDataType::Time;
    }
    if value.is_empty() {
        return CommonDataType::Custom("unknown".to_string());
    }

    CommonDataType::Custom(value.to_string())
}
