use formatter::format_source;

#[test]
fn keeps_the_spacing_of_the_new_arithmetic_operators() {
    assert_eq!(
        format_source("into a:Int32=b % 3 | c<<2;"),
        "into a: Int32 = b % 3 | c<<2;\n"
    );
    assert_eq!(
        format_source("into a:Int32 = b >> 1 ^ c & d;"),
        "into a: Int32 = b >> 1 ^ c & d;\n"
    );
}

#[test]
fn spaces_the_logical_operators_like_the_other_compound_operators() {
    assert_eq!(
        format_source("if(a>1&&b<2||!c){x=1;}"),
        "if (a>1 && b<2 || !c) {\n    x = 1;\n}\n"
    );
}

#[test]
fn formatting_the_new_operators_twice_changes_nothing() {
    let once = format_source("while(n<10&&n%2==0){n=n<<1;}");
    assert_eq!(format_source(&once), once);
}
