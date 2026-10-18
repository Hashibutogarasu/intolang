#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticOperator {
    Add,
    Sub,
    Mul,
    Div,
}

/// An operator written with two adjacent symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompoundOperator {
    Equal,
    NotEqual,
    LessEqual,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
