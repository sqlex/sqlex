//! Query plan node definitions
//!
//! Defines the tree structure for representing SQL queries
//! in a form suitable for type and nullability inference.

/// Query plan node representing the logical structure of a SQL query
use std::any::Any;
use std::{collections::HashMap, fmt::Debug};

use sqlex_common::DataType;

/// CTE columns context for resolving CTERef nodes
/// Column information specific to the internal query plan
#[derive(Debug, Clone)]
pub struct PlanNodeColumn {
    pub name: String,
    pub data_type: DataType,
    pub nullability: bool,
    pub origin_table: Option<String>,
    pub origin_column: Option<String>,
}

pub type CTEContext = HashMap<String, Vec<PlanNodeColumn>>;

// ============================================================================
//  Traits & Macros
// ============================================================================

/// Core logic trait that concrete nodes must implement.
///
/// This trait defines the specific behavior of a logical operator,
/// such as how to infer its output schema.
pub trait LogicalNode: Debug + Clone + Send + Sync + 'static {
    /// Recursively derive the output columns of this plan node.
    fn columns(&self) -> &[PlanNodeColumn];
}

/// The main object-safe trait for query plan nodes.
///
/// This trait is automatically implemented for any type that implements `LogicalNode`.
/// It provides dynamic dispatch capabilities (`as_any`, `box_clone`) and
/// exposes the core logic methods.
pub trait PlanNode: Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
    fn columns(&self) -> &[PlanNodeColumn];
    fn box_clone(&self) -> Box<dyn PlanNode>;
}

/// Blanket implementation: Any `LogicalNode` is a `PlanNode`.
impl<T> PlanNode for T
where
    T: LogicalNode,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn columns(&self) -> &[PlanNodeColumn] {
        self.columns()
    }

    fn box_clone(&self) -> Box<dyn PlanNode> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn PlanNode> {
    fn clone(&self) -> Box<dyn PlanNode> {
        self.box_clone()
    }
}

/// Helper macro to simplify downcasting `PlanNode` trait objects.
///
/// # Usage
/// ```ignore
/// match_plan!(node, {
///     scan: TableScanNode => { ... },
///     proj: ProjectNode => { ... },
///     _ => { ... }
/// })
/// ```
#[macro_export]
macro_rules! match_plan {
    ($node:expr, {
        $( $var:ident : $type:ty => $body:expr ),*,
        _ => $default:expr
    }) => {
        {
            let node_ref = $node.as_any();
            if false {
                unreachable!()
            }
            $(
                else if let Some($var) = node_ref.downcast_ref::<$type>() {
                    $body
                }
            )*
            else {
                $default
            }
        }
    };
}
