use formatter::format_source;

#[test]
fn keeps_the_spacing_of_the_new_arithmetic_operators() {
    assert_eq!(
        format_source("into a:Int32=b % 3 | c<<2;"),
        r#"into a: Int32 = b % 3 | c<<2;
"#
    );
    assert_eq!(
        format_source("into a:Int32 = b >> 1 ^ c & d;"),
        r#"into a: Int32 = b >> 1 ^ c & d;
"#
    );
}

#[test]
fn spaces_the_logical_operators_like_the_other_compound_operators() {
    assert_eq!(
        format_source("if(a>1&&b<2||!c){x=1;}"),
        r#"if (a>1 && b<2 || !c) {
    x = 1;
}
"#
    );
}

#[test]
fn formatting_the_new_operators_twice_changes_nothing() {
    let once = format_source("while(n<10&&n%2==0){n=n<<1;}");
    assert_eq!(format_source(&once), once);
}
