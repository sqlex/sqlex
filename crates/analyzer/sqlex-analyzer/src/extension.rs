use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::ObjectName;

/// Extension trait for sqlparser ObjectName
pub trait ObjectNameExt {
    /// Convert ObjectName to a dotted string (e.g. "schema.table")
    fn to_dotted_string(&self) -> String;

    /// Convert ObjectName to a normalized dotted string according to dialect rules
    /// For PostgreSQL: unquoted identifiers are converted to lowercase
    /// For MySQL/SQLite: identifiers are used as-is
    fn to_normalized_string(&self, dialect: Dialect) -> String;
}

impl ObjectNameExt for ObjectName {
    fn to_dotted_string(&self) -> String {
        self.0
            .iter()
            .map(|ident| ident.value.clone())
            .collect::<Vec<_>>()
            .join(".")
    }

    fn to_normalized_string(&self, dialect: Dialect) -> String {
        self.0
            .iter()
            .map(|ident| {
                match dialect {
                    Dialect::Postgres => {
                        // PostgreSQL: unquoted identifiers are case-insensitive (converted to lowercase)
                        if ident.quote_style.is_none() {
                            ident.value.to_lowercase()
                        } else {
                            ident.value.clone()
                        }
                    },
                    Dialect::MySQL | Dialect::SQLite => {
                        // MySQL and SQLite: use identifier as-is
                        ident.value.clone()
                    },
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

/// Extension trait for common DataType helpers.
pub trait DataTypeExt {
    fn is_numeric(&self) -> bool;
    fn is_integer(&self) -> bool;
    fn is_text_like(&self) -> bool;
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
                | DataType::UnsignedTinyInt
                | DataType::SmallInt
                | DataType::UnsignedSmallInt
                | DataType::Int
                | DataType::UnsignedInt
                | DataType::BigInt
                | DataType::UnsignedBigInt
                | DataType::Float
                | DataType::Double
                | DataType::Decimal
        )
    }

    fn is_integer(&self) -> bool {
        matches!(
            self,
            DataType::TinyInt
                | DataType::UnsignedTinyInt
                | DataType::SmallInt
                | DataType::UnsignedSmallInt
                | DataType::Int
                | DataType::UnsignedInt
                | DataType::BigInt
                | DataType::UnsignedBigInt
        )
    }

    fn is_text_like(&self) -> bool {
        matches!(
            self,
            DataType::Char | DataType::Varchar | DataType::Text | DataType::Binary
        )
    }

    fn promote_numeric(&self, other: &DataType) -> DataType {
        match (self, other) {
            (DataType::Double, _) | (_, DataType::Double) => DataType::Double,
            (DataType::Float, _) | (_, DataType::Float) => DataType::Float,
            (DataType::Decimal, _) | (_, DataType::Decimal) => DataType::Decimal,
            _ => {
                let left = match self {
                    DataType::TinyInt => Some((1, false)),
                    DataType::UnsignedTinyInt => Some((1, true)),
                    DataType::SmallInt => Some((2, false)),
                    DataType::UnsignedSmallInt => Some((2, true)),
                    DataType::Int => Some((3, false)),
                    DataType::UnsignedInt => Some((3, true)),
                    DataType::BigInt => Some((4, false)),
                    DataType::UnsignedBigInt => Some((4, true)),
                    _ => None,
                };
                let right = match other {
                    DataType::TinyInt => Some((1, false)),
                    DataType::UnsignedTinyInt => Some((1, true)),
                    DataType::SmallInt => Some((2, false)),
                    DataType::UnsignedSmallInt => Some((2, true)),
                    DataType::Int => Some((3, false)),
                    DataType::UnsignedInt => Some((3, true)),
                    DataType::BigInt => Some((4, false)),
                    DataType::UnsignedBigInt => Some((4, true)),
                    _ => None,
                };

                if let (Some((left_rank, left_unsigned)), Some((right_rank, right_unsigned))) =
                    (left, right)
                {
                    let rank = left_rank.max(right_rank);
                    let unsigned = left_unsigned && right_unsigned;
                    return match (rank, unsigned) {
                        (1, false) => DataType::TinyInt,
                        (1, true) => DataType::UnsignedTinyInt,
                        (2, false) => DataType::SmallInt,
                        (2, true) => DataType::UnsignedSmallInt,
                        (3, false) => DataType::Int,
                        (3, true) => DataType::UnsignedInt,
                        (_, false) => DataType::BigInt,
                        (_, true) => DataType::UnsignedBigInt,
                    };
                }
                self.clone()
            },
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
