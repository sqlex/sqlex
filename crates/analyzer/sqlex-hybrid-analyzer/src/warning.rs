use sqlex_common::types::DataType;

/// Warnings emitted when static and database analyzers produce different results.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisWarning {
    /// Column count mismatch between static and database analyzers.
    ColumnCountMismatch {
        static_count: usize,
        db_count: usize,
    },

    /// Column name mismatch at a specific index.
    ColumnNameMismatch {
        index: usize,
        static_name: String,
        db_name: String,
    },

    /// Data type mismatch for a column.
    DataTypeMismatch {
        index: usize,
        column_name: String,
        static_type: DataType,
        db_type: DataType,
    },

    /// Nullability mismatch for a column.
    NullabilityMismatch {
        index: usize,
        column_name: String,
        static_nullability: bool,
        db_nullability: bool,
    },

    /// Table count mismatch between static and database analyzers.
    TableCountMismatch {
        static_count: usize,
        db_count: usize,
    },

    /// Table found in one analyzer but not in the other.
    TableMissing {
        table_name: String,
        found_in_static: bool,
    },
}

impl std::fmt::Display for AnalysisWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisWarning::ColumnCountMismatch {
                static_count,
                db_count,
            } => {
                write!(
                    f,
                    "Column count mismatch: static analyzer found {} columns, database analyzer found {} columns",
                    static_count, db_count
                )
            },
            AnalysisWarning::ColumnNameMismatch {
                index,
                static_name,
                db_name,
            } => {
                write!(
                    f,
                    "Column name mismatch at index {}: static='{}', database='{}'",
                    index, static_name, db_name
                )
            },
            AnalysisWarning::DataTypeMismatch {
                index,
                column_name,
                static_type,
                db_type,
            } => {
                write!(
                    f,
                    "Data type mismatch for column '{}' at index {}: static={:?}, database={:?}",
                    column_name, index, static_type, db_type
                )
            },
            AnalysisWarning::NullabilityMismatch {
                index,
                column_name,
                static_nullability,
                db_nullability,
            } => {
                write!(
                    f,
                    "Nullability mismatch for column '{}' at index {}: static={:?}, database={:?}",
                    column_name, index, static_nullability, db_nullability
                )
            },
            AnalysisWarning::TableCountMismatch {
                static_count,
                db_count,
            } => {
                write!(
                    f,
                    "Table count mismatch: static analyzer found {} tables, database analyzer found {} tables",
                    static_count, db_count
                )
            },
            AnalysisWarning::TableMissing {
                table_name,
                found_in_static,
            } => {
                if *found_in_static {
                    write!(
                        f,
                        "Table '{}' found in static analyzer but not in database analyzer",
                        table_name
                    )
                } else {
                    write!(
                        f,
                        "Table '{}' found in database analyzer but not in static analyzer",
                        table_name
                    )
                }
            },
        }
    }
}
