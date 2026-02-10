use sqlex_common::{
    dialect::Dialect,
    types::{Cardinality, DataType},
};

use crate::{
    catalog::Catalog,
    diagnostics::Diagnostic,
    ir::{
        output::{OutputColumn, OutputSchema},
        relational::RelationalExpr,
    },
};

mod cardinality;
mod operators;
mod scalar;

pub struct InferResult {
    pub output: Option<OutputSchema>,
    #[allow(dead_code)]
    pub diagnostics: Vec<Diagnostic>,
}

/// Metadata inferred for a relational expression node.
#[derive(Debug, Clone)]
pub(super) struct RelationalMetadata {
    pub(super) columns: Vec<ColumnMetadata>,
    pub(super) cardinality: Cardinality,
}

#[derive(Debug, Clone)]
pub(super) struct ColumnMetadata {
    /// Table or alias this column originates from (for qualified reference resolution)
    pub(super) table: Option<String>,
    pub(super) name: String,
    pub(super) data_type: DataType,
    pub(super) nullable: bool,
}

/// Type information for a scalar expression.
#[derive(Debug, Clone)]
pub(super) struct TypeInfo {
    pub(super) data_type: DataType,
    pub(super) nullable: bool,
}

impl RelationalMetadata {
    fn to_output_schema(&self) -> OutputSchema {
        OutputSchema {
            columns: self
                .columns
                .iter()
                .map(|c| OutputColumn {
                    name: c.name.clone(),
                    data_type: c.data_type.clone(),
                    nullability: c.nullable,
                })
                .collect(),
            cardinality: self.cardinality,
        }
    }
}

pub(crate) struct Inferrer<'a> {
    pub(super) dialect: Dialect,
    pub(super) catalog: &'a Catalog,
    pub(super) diagnostics: Vec<Diagnostic>,
}

impl<'a> Inferrer<'a> {
    pub(crate) fn new(dialect: Dialect, catalog: &'a Catalog) -> Self {
        Self {
            dialect,
            catalog,
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn infer(mut self, expr: &RelationalExpr) -> InferResult {
        let metadata = self.infer_expr(expr);
        let output = Some(metadata.to_output_schema());
        InferResult {
            output,
            diagnostics: self.diagnostics,
        }
    }
}
