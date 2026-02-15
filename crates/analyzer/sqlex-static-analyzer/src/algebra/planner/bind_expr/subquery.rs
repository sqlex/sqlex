use sqlex_analyzer::extension::DataTypeExt;
use sqlex_common::{dialect::Dialect, types::DataType};
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Select, SelectItem, SetExpr,
    TableFactor,
};

use crate::{
    algebra::{planner::Algebraizer, scalar::BoundLiteral},
    catalog::{
        model::Catalog,
        normalize::{normalize_ident, normalize_object_name},
    },
};

impl Algebraizer {
    pub(crate) fn infer_scalar_subquery_result(
        &self,
        query: &sqlparser::ast::Query,
        catalog: &Catalog,
    ) -> (DataType, bool) {
        let SetExpr::Select(select) = &*query.body else {
            return (DataType::Custom("unknown".to_string()), true);
        };
        if select.projection.len() != 1 {
            return (DataType::Custom("unknown".to_string()), true);
        }

        let item_expr = match &select.projection[0] {
            SelectItem::UnnamedExpr(expr) => Some(expr),
            SelectItem::ExprWithAlias { expr, .. } => Some(expr),
            _ => None,
        };
        let Some(item_expr) = item_expr else {
            return (DataType::Custom("unknown".to_string()), true);
        };

        if let Expr::Function(function) = item_expr {
            let function_name = normalize_object_name(&function.name, self.dialect);
            let function_name = function_name.to_ascii_lowercase();
            match function_name.as_str() {
                "count" => {
                    return (DataType::BigInt, false);
                },
                "max" | "min" => {
                    if let Some(arg_expr) = first_function_expr_arg(function) {
                        if let Some((data_type, _)) =
                            self.resolve_scalar_subquery_expr_type(select, arg_expr, catalog)
                        {
                            return (data_type, true);
                        }
                    }
                },
                "sum" => {
                    if let Some(arg_expr) = first_function_expr_arg(function) {
                        if let Some((arg_type, _)) =
                            self.resolve_scalar_subquery_expr_type(select, arg_expr, catalog)
                        {
                            let data_type = match self.dialect {
                                Dialect::Postgres => {
                                    if arg_type.is_integer() {
                                        DataType::BigInt
                                    } else {
                                        arg_type
                                    }
                                },
                                Dialect::MySQL | Dialect::SQLite => {
                                    if arg_type.is_numeric() {
                                        arg_type
                                    } else {
                                        DataType::Double
                                    }
                                },
                            };
                            return (data_type, true);
                        }
                    }
                },
                "avg" => {
                    return (DataType::Decimal, true);
                },
                _ => {},
            }
        }

        if let Some((data_type, _)) =
            self.resolve_scalar_subquery_expr_type(select, item_expr, catalog)
        {
            return (data_type, true);
        }

        (DataType::Custom("unknown".to_string()), true)
    }

    pub(crate) fn resolve_scalar_subquery_expr_type(
        &self,
        select: &Select,
        expr: &Expr,
        catalog: &Catalog,
    ) -> Option<(DataType, bool)> {
        match expr {
            Expr::Identifier(identifier) => {
                self.resolve_subquery_column(select, None, &identifier.value, catalog)
            },
            Expr::CompoundIdentifier(idents) => {
                if idents.len() < 2 {
                    return None;
                }
                let qualifier = idents[..idents.len() - 1]
                    .iter()
                    .map(|ident| normalize_ident(ident, self.dialect))
                    .collect::<Vec<_>>()
                    .join(".");
                let column_name = normalize_ident(idents.last()?, self.dialect);
                self.resolve_subquery_column(select, Some(&qualifier), &column_name, catalog)
            },
            Expr::Value(value) => {
                let bound_literal = self.bind_literal(value, false).ok()?;
                match bound_literal {
                    BoundLiteral::Null => Some((DataType::Custom("null".to_string()), true)),
                    BoundLiteral::Bool(_) => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Bool,
                            Dialect::MySQL | Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Int {
                        raw, assignment, ..
                    } => Some((
                        match self.dialect {
                            Dialect::Postgres => DataType::Int,
                            Dialect::MySQL => {
                                if assignment
                                    && crate::algebra::planner::bind_expr::literal::mysql_integer_literal_should_be_int(&raw)
                                {
                                    DataType::Int
                                } else {
                                    DataType::BigInt
                                }
                            },
                            Dialect::SQLite => DataType::BigInt,
                        },
                        false,
                    )),
                    BoundLiteral::Float(_) => Some((
                        match self.dialect {
                            Dialect::SQLite => DataType::Double,
                            Dialect::MySQL | Dialect::Postgres => DataType::Decimal,
                        },
                        false,
                    )),
                    BoundLiteral::String(_) => Some((
                        match self.dialect {
                            Dialect::MySQL => DataType::Varchar,
                            Dialect::Postgres | Dialect::SQLite => DataType::Text,
                        },
                        false,
                    )),
                    BoundLiteral::Placeholder(_) => {
                        Some((DataType::Custom("unknown".to_string()), false))
                    },
                }
            },
            _ => None,
        }
    }

    fn resolve_subquery_column(
        &self,
        select: &Select,
        qualifier: Option<&str>,
        column_name: &str,
        catalog: &Catalog,
    ) -> Option<(DataType, bool)> {
        if select.from.len() != 1 {
            return None;
        }

        let from_item = &select.from[0];
        if !from_item.joins.is_empty() {
            return None;
        }

        let TableFactor::Table { name, alias, .. } = &from_item.relation else {
            return None;
        };

        let normalized_table_name = normalize_object_name(name, self.dialect);
        let table = catalog.table(&normalized_table_name)?;

        if let Some(qualifier) = qualifier {
            let mut visible_names = vec![normalized_table_name.clone()];
            if let Some(last_segment) = normalized_table_name.split('.').next_back() {
                if !visible_names.iter().any(|name| name == last_segment) {
                    visible_names.push(last_segment.to_string());
                }
            }
            if let Some(alias) = alias {
                visible_names.push(normalize_ident(&alias.name, self.dialect));
            }

            if !visible_names.iter().any(|name| name == qualifier) {
                return None;
            }
        }

        let normalized_column_name = normalize_column_name(column_name, self.dialect);
        let column = table
            .columns
            .iter()
            .find(|column| column.name == normalized_column_name)?;
        Some((column.data_type.clone(), column.nullable))
    }
}

fn first_function_expr_arg(function: &Function) -> Option<&Expr> {
    let FunctionArguments::List(argument_list) = &function.args else {
        return None;
    };
    let first = argument_list.args.first()?;
    let arg_expr = match first {
        FunctionArg::Named { arg, .. } => arg,
        FunctionArg::ExprNamed { arg, .. } => arg,
        FunctionArg::Unnamed(arg) => arg,
    };
    match arg_expr {
        FunctionArgExpr::Expr(expr) => Some(expr),
        FunctionArgExpr::Wildcard | FunctionArgExpr::QualifiedWildcard(_) => None,
    }
}

fn normalize_column_name(name: &str, dialect: Dialect) -> String {
    if matches!(dialect, Dialect::Postgres) {
        name.to_ascii_lowercase()
    } else {
        name.to_string()
    }
}
