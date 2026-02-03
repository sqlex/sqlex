pub mod control;
mod extension;
pub mod funcs;
pub mod ops;
mod order_by;
pub mod values;

use std::{any::Any, fmt::Debug};

// Re-export from other submodules
pub use control::{CaseExpr, CastExpr};
pub use extension::ExprExt;
// Re-export from funcs
pub use funcs::{
    AggregateFunctionExpr, AggregateFunctionName, ScalarFunction, ScalarFunctionExpr, WindowFrame,
    WindowFrameBound, WindowFrameUnits, WindowFunctionExpr, WindowFunctionName,
};
pub use ops::{BinaryExpr, UnaryExpr};
pub use order_by::OrderByExpr;
use sqlex_common::DataType;
pub use values::{ColumnExpr, LiteralExpr};

// Legacy re-exports (for compatibility if needed, though we moved types to funcs)
// If there was an AggregateExpr struct in the old aggregate.rs, we need to handle it.
// Wait, AggregateExpr (legacy) is NOT in funcs/aggregate.rs (only AggregateFunctionExpr is).
// We deleted aggregate.rs so we lost AggregateExpr.
// Let's bring AggregateExpr back into funcs/aggregate.rs or a legacy module if it's still needed.
// Checking previous steps: AggregateExpr was used in builder.rs and nodes/aggregate.rs.
// I must define AggregateExpr somewhere.
// Let's put legacy AggregateExpr in funcs/aggregate.rs temporarily or cleaner: typed.rs?
// Or just put it in funcs/aggregate.rs adjacent to the new one.

/// The core logic trait that concrete expressions implement
pub trait ExpressionNode: Debug + Clone + Send + Sync + 'static {
    fn data_type(&self) -> DataType;
    fn nullable(&self) -> bool;
}

/// The object-safe trait for dynamic dispatch
pub trait Expression: Debug + Send + Sync + 'static {
    fn data_type(&self) -> DataType;
    fn nullable(&self) -> bool;

    fn as_any(&self) -> &dyn Any;
    fn box_clone(&self) -> Box<dyn Expression>;
}

// Blanket implementation reduces boilerplate
impl<T: ExpressionNode> Expression for T {
    fn data_type(&self) -> DataType {
        ExpressionNode::data_type(self)
    }

    fn nullable(&self) -> bool {
        ExpressionNode::nullable(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn box_clone(&self) -> Box<dyn Expression> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Expression> {
    fn clone(&self) -> Box<dyn Expression> {
        self.box_clone()
    }
}
