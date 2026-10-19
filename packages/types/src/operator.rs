#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticOperator {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOperator {
    And,
    Or,
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
    Arrow,
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
