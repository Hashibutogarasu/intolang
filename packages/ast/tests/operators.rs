use ast::Expr;
use types::{ArithmeticOperator, ComparisonOperator, LogicalOperator};

fn number(value: i64) -> Box<Expr> {
    Box::new(Expr::int(value))
}

fn name(text: &str) -> Box<Expr> {
    Box::new(Expr::ident(text))
}

fn binary(left: Box<Expr>, op: ArithmeticOperator, right: Box<Expr>) -> Box<Expr> {
    Box::new(Expr::Binary { left, op, right })
}

fn logical(left: Box<Expr>, op: LogicalOperator, right: Box<Expr>) -> Box<Expr> {
    Box::new(Expr::Logical { left, op, right })
}

fn compare(left: Box<Expr>, op: ComparisonOperator, right: Box<Expr>) -> Box<Expr> {
    Box::new(Expr::Compare { left, op, right })
}

fn parse(source: &str) -> Expr {
    source.parse().unwrap()
}

#[test]
fn multiplication_binds_tighter_than_addition() {
    use ArithmeticOperator::{Add, Mul};
    assert_eq!(
        parse("1 + 2 * 3"),
        *binary(number(1), Add, binary(number(2), Mul, number(3)))
    );
    assert_eq!(
        parse("1 * 2 + 3"),
        *binary(binary(number(1), Mul, number(2)), Add, number(3))
    );
}

#[test]
fn remainder_has_the_precedence_of_multiplication() {
    use ArithmeticOperator::{Add, Rem};
    assert_eq!(
        parse("a % 4 + 2"),
        *binary(binary(name("a"), Rem, number(4)), Add, number(2))
    );
}

#[test]
fn operators_of_one_level_are_left_associative() {
    use ArithmeticOperator::Sub;
    assert_eq!(
        parse("10 - 3 - 2"),
        *binary(binary(number(10), Sub, number(3)), Sub, number(2))
    );
}

#[test]
fn shifts_bind_looser_than_addition_and_tighter_than_bit_and() {
    use ArithmeticOperator::{Add, BitAnd, ShiftLeft};
    assert_eq!(
        parse("a << 1 + 2"),
        *binary(name("a"), ShiftLeft, binary(number(1), Add, number(2)))
    );
    assert_eq!(
        parse("a & b << 1"),
        *binary(name("a"), BitAnd, binary(name("b"), ShiftLeft, number(1)))
    );
}

#[test]
fn bit_operators_bind_in_the_order_and_xor_or() {
    use ArithmeticOperator::{BitAnd, BitOr, BitXor};
    assert_eq!(
        parse("a | b ^ c & d"),
        *binary(
            name("a"),
            BitOr,
            binary(name("b"), BitXor, binary(name("c"), BitAnd, name("d")))
        )
    );
}

#[test]
fn shifts_are_not_comparisons() {
    use ArithmeticOperator::{ShiftLeft, ShiftRight};
    assert_eq!(
        parse("a << 1 >> 2"),
        *binary(
            binary(name("a"), ShiftLeft, number(1)),
            ShiftRight,
            number(2)
        )
    );
    assert_eq!(
        parse("a < b"),
        *compare(name("a"), ComparisonOperator::Less, name("b"))
    );
    assert_eq!(
        parse("a >= b"),
        *compare(name("a"), ComparisonOperator::GreaterEqual, name("b"))
    );
}

#[test]
fn a_comparison_binds_looser_than_arithmetic_and_tighter_than_logic() {
    use ArithmeticOperator::Add;
    use LogicalOperator::{And, Or};
    assert_eq!(
        parse("a + 1 < b && c == 2 || d"),
        *logical(
            logical(
                compare(
                    binary(name("a"), Add, number(1)),
                    ComparisonOperator::Less,
                    name("b")
                ),
                And,
                compare(name("c"), ComparisonOperator::Equal, number(2))
            ),
            Or,
            name("d")
        )
    );
}

#[test]
fn and_binds_tighter_than_or() {
    use LogicalOperator::{And, Or};
    assert_eq!(
        parse("a || b && c"),
        *logical(name("a"), Or, logical(name("b"), And, name("c")))
    );
}

#[test]
fn single_symbols_are_bit_operators_and_double_symbols_are_logical() {
    use ArithmeticOperator::{BitAnd, BitOr};
    use LogicalOperator::{And, Or};
    assert_eq!(parse("a & b"), *binary(name("a"), BitAnd, name("b")));
    assert_eq!(parse("a && b"), *logical(name("a"), And, name("b")));
    assert_eq!(parse("a | b"), *binary(name("a"), BitOr, name("b")));
    assert_eq!(parse("a || b"), *logical(name("a"), Or, name("b")));
}

#[test]
fn parentheses_override_the_precedence() {
    use ArithmeticOperator::{Add, Mul};
    assert_eq!(
        parse("(1 + 2) * 3"),
        *binary(binary(number(1), Add, number(2)), Mul, number(3))
    );
}

#[test]
fn the_expressions_print_with_their_operators() {
    assert_eq!(parse("a % 4 << 1").to_string(), "a % 4 << 1");
    assert_eq!(parse("a && b || !c").to_string(), "a && b || !c");
}
