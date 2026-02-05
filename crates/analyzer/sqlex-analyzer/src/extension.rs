use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::ObjectName;

/// Extension trait for sqlparser ObjectName
pub trait ObjectNameExt {
    /// Convert ObjectName to a dotted string (e.g. "schema.table")
    fn to_dotted_string(&self) -> String;
}

impl ObjectNameExt for ObjectName {
    fn to_dotted_string(&self) -> String {
        self.0
            .iter()
            .map(|ident| ident.value.clone())
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// Extension trait for common DataType helpers.
pub trait DataTypeExt {
    fn is_numeric(&self) -> bool;
    fn promote_numeric(&self, other: &DataType) -> DataType;
    fn merge_common_type(
        dialect: Dialect,
        existing: Option<DataType>,
        next: &DataType,
    ) -> Option<DataType>;
    fn common_type(dialect: Dialect, arg_types: &[DataType]) -> Option<DataType>;
}

impl DataTypeExt for DataType {
    fn is_numeric(&self) -> bool {
        matches!(
            self,
            DataType::TinyInt
                | DataType::SmallInt
                | DataType::Int
                | DataType::BigInt
                | DataType::Float
                | DataType::Double
                | DataType::Decimal
        )
    }

    fn promote_numeric(&self, other: &DataType) -> DataType {
        match (self, other) {
            (DataType::Double, _) | (_, DataType::Double) => DataType::Double,
            (DataType::Float, _) | (_, DataType::Float) => DataType::Float,
            (DataType::Decimal, _) | (_, DataType::Decimal) => DataType::Decimal,
            (DataType::BigInt, _) | (_, DataType::BigInt) => DataType::BigInt,
            (DataType::Int, _) | (_, DataType::Int) => DataType::Int,
            (DataType::SmallInt, _) | (_, DataType::SmallInt) => DataType::SmallInt,
            (DataType::TinyInt, DataType::TinyInt) => DataType::TinyInt,
            _ => self.clone(),
        }
    }

    fn merge_common_type(
        dialect: Dialect,
        existing: Option<DataType>,
        next: &DataType,
    ) -> Option<DataType> {
        if matches!(next, DataType::Custom(_)) {
            return existing;
        }

        match existing {
            None => Some(next.clone()),
            Some(current) => {
                if current == *next {
                    return Some(current);
                }

                if current.is_numeric() && next.is_numeric() {
                    return Some(current.promote_numeric(next));
                }

                // Placeholder for dialect-specific common-type rules.
                // For now we keep the current type for non-numeric mismatches.
                match dialect {
                    Dialect::SQLite => Some(current),
                    Dialect::MySQL | Dialect::Postgres => Some(current),
                }
            },
        }
    }

    fn common_type(dialect: Dialect, arg_types: &[DataType]) -> Option<DataType> {
        let mut merged = None;
        for data_type in arg_types {
            merged = DataType::merge_common_type(dialect, merged, data_type);
        }
        merged
    }
}
