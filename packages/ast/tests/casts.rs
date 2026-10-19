use ast::{Expr, TypeRef};
use types::{ArithmeticOperator, Literal};

fn parse(source: &str) -> Expr {
    source.parse().unwrap()
}

fn cast(value: Expr, ty: &str) -> Expr {
    Expr::Cast {
        value: Box::new(value),
        ty: TypeRef::from(ty),
    }
}

#[test]
fn a_character_literal_is_an_expression() {
    assert_eq!(parse("'A'"), Expr::Literal(Literal::Char('A')));
}

#[test]
fn as_converts_the_value_to_a_type() {
    assert_eq!(
        parse("'A' as Int32"),
        cast(Expr::Literal(Literal::Char('A')), "Int32")
    );
}

#[test]
fn as_binds_tighter_than_arithmetic() {
    assert_eq!(
        parse("a as Int64 + 1"),
        Expr::Binary {
            left: Box::new(cast(Expr::ident("a"), "Int64")),
            op: ArithmeticOperator::Add,
            right: Box::new(Expr::int(1)),
        }
    );
}

#[test]
fn a_star_after_the_type_is_a_multiplication() {
    assert_eq!(
        parse("a as Int32 * 2"),
        Expr::Binary {
            left: Box::new(cast(Expr::ident("a"), "Int32")),
            op: ArithmeticOperator::Mul,
            right: Box::new(Expr::int(2)),
        }
    );
}

#[test]
fn casts_chain_from_left_to_right() {
    assert_eq!(
        parse("c as Int32 as Float64"),
        cast(cast(Expr::ident("c"), "Int32"), "Float64")
    );
}

#[test]
fn parentheses_group_a_cast_operand() {
    assert_eq!(
        parse("(('0' as Int32) + d) as Char"),
        cast(
            Expr::Binary {
                left: Box::new(cast(Expr::Literal(Literal::Char('0')), "Int32")),
                op: ArithmeticOperator::Add,
                right: Box::new(Expr::ident("d")),
            },
            "Char"
        )
    );
}

#[test]
fn expressions_print_with_their_casts_and_characters() {
    assert_eq!(parse("'A' as Int32 + 1").to_string(), "'A' as Int32 + 1");
    assert_eq!(parse("'\\n'").to_string(), "'\\n'");
}
