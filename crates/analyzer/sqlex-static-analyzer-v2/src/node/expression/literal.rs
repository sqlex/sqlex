#[derive(Debug, Clone)]
pub enum LiteralValue {
    Null,
    Bool(bool),
    Int { value: i64, raw: String },
    Float(f64),
    String(String),
}

#[derive(Debug, Clone)]
pub struct LiteralExpression {
    pub value: LiteralValue,
}

impl LiteralExpression {
    pub fn new(value: LiteralValue) -> Self {
        Self { value }
    }
}
