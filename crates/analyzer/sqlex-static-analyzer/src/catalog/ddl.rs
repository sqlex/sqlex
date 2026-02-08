//! DDL parser for building and maintaining the catalog
//!
//! Parses CREATE TABLE, ALTER TABLE, and DROP TABLE statements
//! to build and maintain the catalog.

use sqlex_analyzer::{AnalyzerError, extension::ObjectNameExt};
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::{
    ast::{
        self, AlterTableOperation, ColumnOption, CreateTable, DataType as SqlDataType, Statement,
        TableConstraint,
    },
    dialect::{Dialect as SqlParserDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use crate::catalog::{
    Catalog,
    types::{ColumnDef, ForeignKeyDef, ReferentialAction, TableDef},
};

type Result<T> = std::result::Result<T, AnalyzerError>;

impl Catalog {
    /// Parse and execute DDL statement(s)
    pub fn apply_ddl(&mut self, sql: &str) -> Result<()> {
        let dialect: Box<dyn SqlParserDialect> = match self.dialect {
            Dialect::Postgres => Box::new(PostgreSqlDialect {}),
            Dialect::MySQL => Box::new(MySqlDialect {}),
            Dialect::SQLite => Box::new(SQLiteDialect {}),
        };
        let statements = Parser::parse_sql(dialect.as_ref(), sql)
            .map_err(|e| AnalyzerError::AnalysisError(e.to_string()))?;

        for stmt in statements {
            self.execute_statement(&stmt)?;
        }

        self.rebuild_fk_index();
        Ok(())
    }

    /// Execute a single parsed statement
    fn execute_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::CreateTable(c) => self.handle_create_table(c),
            Statement::AlterTable { .. } => self.handle_alter_table(stmt),
            Statement::Drop { .. } => self.handle_drop(stmt),
            _ => Ok(()), // Ignore other statements
        }
    }

    /// Handle CREATE TABLE statement
    fn handle_create_table(&mut self, create_table: &CreateTable) -> Result<()> {
        let CreateTable {
            name,
            columns,
            constraints,
            ..
        } = create_table;

        let table_name = name.to_normalized_string(self.dialect);
        let mut table = TableDef::new(&table_name);

        // Parse columns
        for col in columns {
            table.columns.push(parse_column_def(self.dialect, col)?);
            // Also handle inline constraints (PRIMARY KEY, UNIQUE, REFERENCES)
            for option in &col.options {
                match &option.option {
                    ColumnOption::Unique {
                        is_primary: true, ..
                    } => {
                        table.primary_key = Some(vec![col.name.value.clone()]);
                        // PK implies NOT NULL
                        if let Some(c) = table.columns.last_mut() {
                            c.nullable = false;
                        }
                    },
                    ColumnOption::Unique {
                        is_primary: false, ..
                    } => {
                        table.unique_constraints.push(vec![col.name.value.clone()]);
                    },
                    ColumnOption::ForeignKey {
                        foreign_table,
                        referred_columns,
                        ..
                    } => {
                        let fk = ForeignKeyDef::new(
                            vec![col.name.value.clone()],
                            foreign_table.to_normalized_string(self.dialect),
                            referred_columns.iter().map(|id| id.value.clone()).collect(),
                        );
                        table.foreign_keys.push(fk);
                    },
                    _ => {},
                }
            }
        }

        // Parse table constraints
        for constraint in constraints {
            parse_table_constraint(self.dialect, constraint, &mut table)?;
        }

        self.add_table(table);

        Ok(())
    }

    /// Handle ALTER TABLE statement
    fn handle_alter_table(&mut self, stmt: &Statement) -> Result<()> {
        if let Statement::AlterTable {
            name, operations, ..
        } = stmt
        {
            let table_name = name.to_normalized_string(self.dialect);
            if let Some(table) = self.tables.get_mut(&table_name) {
                for op in operations {
                    match op {
                        AlterTableOperation::AddColumn { column_def, .. } => {
                            table
                                .columns
                                .push(parse_column_def(self.dialect, column_def)?);
                        },
                        AlterTableOperation::DropColumn {
                            column_name,
                            if_exists,
                            ..
                        } => {
                            let col_name = column_name.value.clone();
                            if let Some(pos) = table.columns.iter().position(|c| c.name == col_name)
                            {
                                table.columns.remove(pos);
                            } else if !*if_exists {
                                return Err(AnalyzerError::AnalysisError(format!(
                                    "Column {} does not exist in table {}",
                                    col_name, table_name
                                )));
                            }
                        },
                        AlterTableOperation::AddConstraint(constraint) => {
                            if self.dialect == Dialect::SQLite {
                                return Err(AnalyzerError::AnalysisError(
                                    "SQLite does not support ALTER TABLE ADD CONSTRAINT"
                                        .to_string(),
                                ));
                            }
                            parse_table_constraint(self.dialect, constraint, table)?;
                        },
                        _ => {
                            // Ignore other operations for now
                        },
                    }
                }
            } else {
                return Err(AnalyzerError::AnalysisError(format!(
                    "Table {} does not exist",
                    table_name
                )));
            }
        }
        Ok(())
    }

    /// Handle DROP statement
    fn handle_drop(&mut self, stmt: &Statement) -> Result<()> {
        if let Statement::Drop {
            object_type,
            names,
            cascade,
            ..
        } = stmt
        {
            if *object_type != ast::ObjectType::Table {
                return Ok(());
            }
            for name in names {
                let table_name = name.to_normalized_string(self.dialect);
                if self.dialect != Dialect::SQLite && !*cascade {
                    let referencing: Vec<String> = self
                        .tables
                        .values()
                        .filter(|table| {
                            table
                                .foreign_keys
                                .iter()
                                .any(|fk| fk.ref_table == table_name)
                        })
                        .map(|table| table.name.clone())
                        .collect();
                    if !referencing.is_empty() {
                        return Err(AnalyzerError::AnalysisError(format!(
                            "Cannot drop table {} referenced by foreign key constraints: {}",
                            table_name,
                            referencing.join(", ")
                        )));
                    }
                }
                self.tables.remove(&table_name);
                if *cascade {
                    for table in self.tables.values_mut() {
                        table.foreign_keys.retain(|fk| fk.ref_table != table_name);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Parse a column definition from sqlparser AST
pub fn parse_column_def(dialect: Dialect, col: &ast::ColumnDef) -> Result<ColumnDef> {
    let name = col.name.value.clone();
    let data_type = map_data_type(dialect, &col.data_type);
    let mut column_def = ColumnDef::new(name, data_type);

    for option in &col.options {
        match &option.option {
            ColumnOption::NotNull => {
                column_def.nullable = false;
            },
            ColumnOption::Default(expr) => {
                column_def.default = Some(expr.to_string());
            },
            _ => {},
        }
    }

    Ok(column_def)
}

/// Parse a table constraint from sqlparser AST
pub fn parse_table_constraint(
    dialect: Dialect,
    constraint: &TableConstraint,
    table: &mut TableDef,
) -> Result<()> {
    match constraint {
        TableConstraint::PrimaryKey { columns, .. } => {
            let pk_cols: Vec<String> = columns.iter().map(|c| c.value.clone()).collect();
            // PK columns are implicitly NOT NULL
            for col_name in &pk_cols {
                if let Some(col) = table.columns.iter_mut().find(|c| c.name == *col_name) {
                    col.nullable = false;
                }
            }
            table.primary_key = Some(pk_cols);
        },
        TableConstraint::Unique { columns, .. } => {
            table
                .unique_constraints
                .push(columns.iter().map(|c| c.value.clone()).collect());
        },
        TableConstraint::ForeignKey {
            columns,
            foreign_table,
            referred_columns,
            on_delete,
            on_update,
            ..
        } => {
            let mut fk = ForeignKeyDef::new(
                columns.iter().map(|c| c.value.clone()).collect(),
                foreign_table.to_normalized_string(dialect),
                referred_columns.iter().map(|c| c.value.clone()).collect(),
            );
            fk.on_delete = map_referential_action(on_delete);
            fk.on_update = map_referential_action(on_update);
            table.foreign_keys.push(fk);
        },
        _ => {},
    }
    Ok(())
}

fn map_referential_action(action: &Option<ast::ReferentialAction>) -> Option<ReferentialAction> {
    action.as_ref().map(|a| match a {
        ast::ReferentialAction::Cascade => ReferentialAction::Cascade,
        ast::ReferentialAction::SetNull => ReferentialAction::SetNull,
        ast::ReferentialAction::SetDefault => ReferentialAction::SetDefault,
        ast::ReferentialAction::Restrict => ReferentialAction::Restrict,
        ast::ReferentialAction::NoAction => ReferentialAction::NoAction,
    })
}

/// Map sqlparser DataType to our DataType
pub fn map_data_type(dialect: Dialect, sql_type: &SqlDataType) -> DataType {
    if dialect == Dialect::SQLite
        && matches!(
            sql_type,
            SqlDataType::TinyInt(_)
                | SqlDataType::SmallInt(_)
                | SqlDataType::Int(_)
                | SqlDataType::Integer(_)
                | SqlDataType::BigInt(_)
                | SqlDataType::UnsignedTinyInt(_)
                | SqlDataType::UnsignedSmallInt(_)
                | SqlDataType::UnsignedInt2(_)
                | SqlDataType::UnsignedInt(_)
                | SqlDataType::UnsignedInt4(_)
                | SqlDataType::UnsignedInteger(_)
                | SqlDataType::UnsignedMediumInt(_)
                | SqlDataType::UnsignedBigInt(_)
                | SqlDataType::UnsignedInt8(_)
        )
    {
        return DataType::BigInt(false);
    }

    match sql_type {
        SqlDataType::Boolean => DataType::Bool,
        SqlDataType::TinyInt(_) => DataType::TinyInt(false),
        SqlDataType::SmallInt(_) => DataType::SmallInt(false),
        SqlDataType::Int(_) | SqlDataType::Integer(_) => DataType::Int(false),
        SqlDataType::BigInt(_) => DataType::BigInt(false),
        SqlDataType::UnsignedTinyInt(_) => DataType::TinyInt(true),
        SqlDataType::UnsignedSmallInt(_) | SqlDataType::UnsignedInt2(_) => DataType::SmallInt(true),
        SqlDataType::UnsignedInt(_)
        | SqlDataType::UnsignedInt4(_)
        | SqlDataType::UnsignedInteger(_) => DataType::Int(true),
        SqlDataType::UnsignedMediumInt(_) => DataType::Int(true),
        SqlDataType::UnsignedBigInt(_) | SqlDataType::UnsignedInt8(_) => DataType::BigInt(true),
        SqlDataType::Float(_) => DataType::Float,
        SqlDataType::Real | SqlDataType::Double(_) | SqlDataType::DoublePrecision => {
            DataType::Double
        },
        SqlDataType::Decimal(_) | SqlDataType::Numeric(_) | SqlDataType::Dec(_) => {
            DataType::Decimal
        },
        SqlDataType::Char(_) => DataType::Char,
        SqlDataType::Varchar(_) => DataType::Varchar,
        SqlDataType::Text => DataType::Text,
        SqlDataType::Date => DataType::Date,
        SqlDataType::Time(_, _) => DataType::Time,
        SqlDataType::Timestamp(_, _) => DataType::Timestamp,
        SqlDataType::Uuid => DataType::Uuid,
        SqlDataType::JSON => DataType::Json,
        SqlDataType::Binary(_) | SqlDataType::Varbinary(_) | SqlDataType::Blob(_) => {
            DataType::Binary
        },
        SqlDataType::Array(inner) => {
            let inner_type = match inner {
                ast::ArrayElemTypeDef::None => DataType::Custom("ANY".to_string()),
                ast::ArrayElemTypeDef::AngleBracket(t) => map_data_type(dialect, t),
                ast::ArrayElemTypeDef::SquareBracket(t, _) => map_data_type(dialect, t),
                ast::ArrayElemTypeDef::Parenthesis(t) => map_data_type(dialect, t),
            };
            DataType::Array(Box::new(inner_type))
        },
        // Custom / Other types
        other => {
            // Try to map dialect specific types or fallback
            let s = other.to_string().to_uppercase();
            match s.as_str() {
                "JSONB" => DataType::Json,
                _ => DataType::Custom(s),
            }
        },
    }
}
