use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{DataType as SqlDataType, TimezoneInfo};

/// Extension trait for common DataType helpers.
pub trait DataTypeExt {
    fn from_sql_data_type(dialect: Dialect, sql_type: &SqlDataType) -> DataType;
    fn is_numeric(&self) -> bool;
    fn is_integer(&self) -> bool;
    fn is_unsigned_integer(&self) -> bool;
    fn is_text_like(&self) -> bool;
    fn promote_numeric(&self, dialect: Dialect, other: &DataType) -> DataType;
    fn merge_common_type(
        dialect: Dialect,
        existing: Option<DataType>,
        next: &DataType,
    ) -> Option<DataType>;
    fn common_type(dialect: Dialect, arg_types: &[DataType]) -> Option<DataType>;
}

impl DataTypeExt for DataType {
    fn from_sql_data_type(dialect: Dialect, sql_type: &SqlDataType) -> DataType {
        fn map_integer_type(dialect: Dialect, data_type: DataType) -> DataType {
            if dialect == Dialect::SQLite {
                DataType::BigInt
            } else {
                data_type
            }
        }
        match sql_type {
            SqlDataType::Bool | SqlDataType::Boolean => DataType::Bool,
            SqlDataType::TinyInt(_) => map_integer_type(dialect, DataType::TinyInt),
            SqlDataType::UnsignedTinyInt(_) => map_integer_type(dialect, DataType::UnsignedTinyInt),
            SqlDataType::Int2(_) | SqlDataType::SmallInt(_) => {
                map_integer_type(dialect, DataType::SmallInt)
            },
            SqlDataType::UnsignedInt2(_) | SqlDataType::UnsignedSmallInt(_) => {
                map_integer_type(dialect, DataType::UnsignedSmallInt)
            },
            SqlDataType::MediumInt(_)
            | SqlDataType::Int(_)
            | SqlDataType::Int4(_)
            | SqlDataType::Integer(_) => map_integer_type(dialect, DataType::Int),
            SqlDataType::UnsignedMediumInt(_)
            | SqlDataType::UnsignedInt(_)
            | SqlDataType::UnsignedInt4(_)
            | SqlDataType::UnsignedInteger(_) => map_integer_type(dialect, DataType::UnsignedInt),
            SqlDataType::BigInt(_) | SqlDataType::Int8(_) => {
                map_integer_type(dialect, DataType::BigInt)
            },
            SqlDataType::UnsignedBigInt(_) | SqlDataType::UnsignedInt8(_) => {
                map_integer_type(dialect, DataType::UnsignedBigInt)
            },

            // SQLite integer affinity fallback for explicitly-typed extended integers.
            SqlDataType::UInt8
            | SqlDataType::UInt16
            | SqlDataType::UInt32
            | SqlDataType::UInt64
            | SqlDataType::UInt128
            | SqlDataType::UInt256
            | SqlDataType::Int16
            | SqlDataType::Int32
            | SqlDataType::Int64
            | SqlDataType::Int128
            | SqlDataType::Int256 => map_integer_type(dialect, DataType::BigInt),

            SqlDataType::Float(_) | SqlDataType::Float4 | SqlDataType::Float32 => match dialect {
                Dialect::Postgres => DataType::Float,
                Dialect::MySQL | Dialect::SQLite => DataType::Double,
            },
            SqlDataType::Real
            | SqlDataType::Double(_)
            | SqlDataType::DoublePrecision
            | SqlDataType::Float8
            | SqlDataType::Float64 => DataType::Double,
            SqlDataType::Numeric(_)
            | SqlDataType::Decimal(_)
            | SqlDataType::Dec(_)
            | SqlDataType::BigNumeric(_)
            | SqlDataType::BigDecimal(_) => DataType::Decimal,

            SqlDataType::Varchar(_)
            | SqlDataType::Nvarchar(_)
            | SqlDataType::CharacterVarying(_)
            | SqlDataType::CharVarying(_) => DataType::Varchar,
            SqlDataType::Char(_) | SqlDataType::Character(_) => DataType::Char,
            SqlDataType::Text
            | SqlDataType::TinyText
            | SqlDataType::MediumText
            | SqlDataType::LongText
            | SqlDataType::String(_)
            | SqlDataType::Clob(_)
            | SqlDataType::CharacterLargeObject(_)
            | SqlDataType::CharLargeObject(_) => DataType::Text,

            SqlDataType::Date => DataType::Date,
            SqlDataType::Time(_, _) => DataType::Time,
            SqlDataType::Datetime(_) => DataType::DateTime,
            SqlDataType::Timestamp(_, timezone) => match dialect {
                Dialect::Postgres => match timezone {
                    TimezoneInfo::WithTimeZone | TimezoneInfo::Tz => DataType::Timestamp,
                    TimezoneInfo::None | TimezoneInfo::WithoutTimeZone => DataType::DateTime,
                },
                Dialect::MySQL | Dialect::SQLite => DataType::Timestamp,
            },

            SqlDataType::Uuid => DataType::Uuid,
            SqlDataType::JSON | SqlDataType::JSONB => DataType::Json,

            SqlDataType::Binary(_)
            | SqlDataType::Varbinary(_)
            | SqlDataType::Blob(_)
            | SqlDataType::TinyBlob
            | SqlDataType::MediumBlob
            | SqlDataType::LongBlob
            | SqlDataType::Bytes(_)
            | SqlDataType::Bytea => DataType::Binary,

            SqlDataType::Custom(name, modifiers) => {
                let base = name.to_string().to_ascii_lowercase();
                let custom_with_modifiers = if modifiers.is_empty() {
                    base.clone()
                } else {
                    let normalized_modifiers = modifiers
                        .iter()
                        .map(|modifier| modifier.to_ascii_lowercase())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{base}({normalized_modifiers})")
                };

                if dialect == Dialect::MySQL
                    && matches!(
                        base.as_str(),
                        "unsigned" | "unsigned integer" | "unsigned int"
                    )
                {
                    DataType::UnsignedBigInt
                } else if dialect == Dialect::MySQL
                    && matches!(base.as_str(), "signed" | "signed integer" | "signed int")
                {
                    DataType::BigInt
                } else {
                    DataType::Custom(custom_with_modifiers)
                }
            },
            _ => DataType::Custom(sql_type.to_string().to_ascii_lowercase()),
        }
    }

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

    fn is_unsigned_integer(&self) -> bool {
        matches!(
            self,
            DataType::UnsignedTinyInt
                | DataType::UnsignedSmallInt
                | DataType::UnsignedInt
                | DataType::UnsignedBigInt
        )
    }

    fn is_text_like(&self) -> bool {
        matches!(
            self,
            DataType::Char | DataType::Varchar | DataType::Text | DataType::Binary
        )
    }

    fn promote_numeric(&self, dialect: Dialect, other: &DataType) -> DataType {
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
                    // MySQL: arithmetic on integers always promotes to (Unsigned)BigInt
                    if dialect == Dialect::MySQL {
                        let unsigned = left_unsigned || right_unsigned;
                        return if unsigned {
                            DataType::UnsignedBigInt
                        } else {
                            DataType::BigInt
                        };
                    }

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
                    return Some(current.promote_numeric(dialect, next));
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
