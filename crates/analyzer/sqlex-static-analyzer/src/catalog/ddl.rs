//! DDL parser for building and maintaining the catalog
//!
//! Parses CREATE TABLE, ALTER TABLE, and DROP TABLE statements
//! to build and maintain the catalog.

use sqlex_analyzer::{AnalyzerError, ObjectNameExt};
use sqlex_common::{DataType, Dialect};
use sqlparser::{
    ast::{
        AlterTableOperation, CharacterLength, ColumnOption, CreateTable, DataType as SqlDataType,
        Statement, TableConstraint,
    },
    dialect::{Dialect as SqlParserDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect},
    parser::Parser,
};

use super::{Catalog, ColumnDef, ForeignKeyDef, ReferentialAction, TableDef};

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

        let table_name = name.to_dotted_string();
        let mut table = TableDef::new(&table_name);

        // Parse columns
        for col in columns {
            table.columns.push(parse_column_def(col)?);
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
                            foreign_table.to_dotted_string(),
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
            parse_table_constraint(constraint, &mut table)?;
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
            let table_name = name.to_dotted_string();
            if let Some(table) = self.tables.get_mut(&table_name) {
                for op in operations {
                    match op {
                        AlterTableOperation::AddColumn { column_def, .. } => {
                            table.columns.push(parse_column_def(column_def)?);
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
                            parse_table_constraint(constraint, table)?;
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
        if let Statement::Drop { names, .. } = stmt {
            for name in names {
                let table_name = name.to_dotted_string();
                self.tables.remove(&table_name);
            }
        }
        Ok(())
    }
}

/// Parse a column definition from sqlparser AST
pub fn parse_column_def(col: &sqlparser::ast::ColumnDef) -> Result<ColumnDef> {
    let name = col.name.value.clone();
    let data_type = map_data_type(&col.data_type);
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
pub fn parse_table_constraint(constraint: &TableConstraint, table: &mut TableDef) -> Result<()> {
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
                foreign_table.to_dotted_string(),
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

fn map_referential_action(
    action: &Option<sqlparser::ast::ReferentialAction>,
) -> Option<ReferentialAction> {
    action.as_ref().map(|a| match a {
        sqlparser::ast::ReferentialAction::Cascade => ReferentialAction::Cascade,
        sqlparser::ast::ReferentialAction::SetNull => ReferentialAction::SetNull,
        sqlparser::ast::ReferentialAction::SetDefault => ReferentialAction::SetDefault,
        sqlparser::ast::ReferentialAction::Restrict => ReferentialAction::Restrict,
        sqlparser::ast::ReferentialAction::NoAction => ReferentialAction::NoAction,
    })
}

/// Map sqlparser DataType to our DataType
pub fn map_data_type(sql_type: &SqlDataType) -> DataType {
    match sql_type {
        SqlDataType::Boolean => DataType::Bool,
        SqlDataType::TinyInt(_) => DataType::TinyInt,
        SqlDataType::SmallInt(_) => DataType::SmallInt,
        SqlDataType::Int(_) | SqlDataType::Integer(_) => DataType::Int,
        SqlDataType::BigInt(_) => DataType::BigInt,
        SqlDataType::Float(_) => DataType::Float,
        SqlDataType::Real | SqlDataType::Double(_) | SqlDataType::DoublePrecision => {
            DataType::Double
        },
        SqlDataType::Decimal(_) | SqlDataType::Numeric(_) | SqlDataType::Dec(_) => {
            DataType::Decimal
        },
        // Match Option<CharacterLength>
        SqlDataType::Char(n) => match n {
            Some(CharacterLength::IntegerLength { length, .. }) => {
                DataType::Char(Some(*length as u32))
            },
            _ => DataType::Char(None),
        },
        SqlDataType::Varchar(n) => match n {
            Some(CharacterLength::IntegerLength { length, .. }) => {
                DataType::Varchar(Some(*length as u32))
            },
            _ => DataType::Varchar(None),
        },
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
                sqlparser::ast::ArrayElemTypeDef::None => DataType::Custom("ANY".to_string()),
                sqlparser::ast::ArrayElemTypeDef::AngleBracket(t) => map_data_type(t),
                sqlparser::ast::ArrayElemTypeDef::SquareBracket(t, _) => map_data_type(t),
                sqlparser::ast::ArrayElemTypeDef::Parenthesis(t) => map_data_type(t),
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
