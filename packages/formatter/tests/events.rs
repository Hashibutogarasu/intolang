use formatter::format_source;

#[test]
fn keeps_the_arrow_as_one_operator() {
    assert_eq!(
        format_source(r#"example.on((args)=>{println("got");});"#),
        r#"example.on((args) => {
    println("got");
});
"#
    );
}

#[test]
fn formats_multi_statement_lambdas() {
    assert_eq!(
        format_source("example.on((a,b)=>{into x=a;println(x);});"),
        r#"example.on((a, b) => {
    into x = a;
    println(x);
});
"#
    );
}

#[test]
fn formats_lambdas_nested_in_blocks() {
    assert_eq!(
        format_source(
            r#"init {
example.on((args)=>{
if (args == 1) {
println(args);
}
});
}
"#
        ),
        r#"init {
    example.on((args) => {
        if (args == 1) {
            println(args);
        }
    });
}
"#
    );
}

#[test]
fn formats_event_declarations() {
    assert_eq!(
        format_source("pub  event example(args: Args){}"),
        r#"pub event example(args: Args) {
}
"#
    );
    assert_eq!(
        format_source("pub override event onCast<Char,Hex>(from: Char,to: Hex){to.value=1;}"),
        r#"pub override event onCast<Char, Hex>(from: Char, to: Hex) {
    to.value = 1;
}
"#
    );
}
