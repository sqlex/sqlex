#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionArgType {
    TextLike,
    Numeric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionArgTypeRule {
    pub(crate) index: usize,
    pub(crate) expected: FunctionArgType,
}

impl FunctionArgTypeRule {
    pub(crate) const fn text(index: usize) -> Self {
        Self {
            index,
            expected: FunctionArgType::TextLike,
        }
    }

    pub(crate) const fn numeric(index: usize) -> Self {
        Self {
            index,
            expected: FunctionArgType::Numeric,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionCoercionProfile {
    Strict,
    Permissive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionReturnTypeRule {
    TextLikeOrDefaultText,
    LengthInteger,
    NumericUnary,
    NumericBinaryCommon,
    CoalesceCommonType,
    NullIfFirstArg,
    Count,
    Sum,
    Avg,
    MinMax,
    Ranking,
    LeadLag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionNullabilityRule {
    AnyArg,
    AllArgs,
    Always,
    Never,
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) category: FunctionCategory,
    pub(crate) min_arity: usize,
    pub(crate) max_arity: Option<usize>,
    pub(crate) return_type_rule: FunctionReturnTypeRule,
    pub(crate) nullability_rule: FunctionNullabilityRule,
    pub(crate) coercion_profile: FunctionCoercionProfile,
    pub(crate) arg_type_rules: Vec<FunctionArgTypeRule>,
}

impl FunctionSignature {
    pub(crate) fn new(
        category: FunctionCategory,
        min_arity: usize,
        max_arity: Option<usize>,
        return_type_rule: FunctionReturnTypeRule,
        nullability_rule: FunctionNullabilityRule,
    ) -> Self {
        Self {
            category,
            min_arity,
            max_arity,
            return_type_rule,
            nullability_rule,
            coercion_profile: FunctionCoercionProfile::Permissive,
            arg_type_rules: Vec::new(),
        }
    }
}
