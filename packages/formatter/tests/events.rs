use formatter::format_source;

#[test]
fn keeps_the_arrow_as_one_operator() {
    assert_eq!(
        format_source("example.on((args)=>{println(\"got\");});"),
        "example.on((args) => {\n    println(\"got\");\n});\n"
    );
}

#[test]
fn formats_multi_statement_lambdas() {
    let source = "example.on((a,b)=>{into x=a;println(x);});";
    assert_eq!(
        format_source(source),
        "example.on((a, b) => {\n    into x = a;\n    println(x);\n});\n"
    );
}

#[test]
fn formats_lambdas_nested_in_blocks() {
    let source = "init {\nexample.on((args)=>{\nif (args == 1) {\nprintln(args);\n}\n});\n}\n";
    assert_eq!(
        format_source(source),
        "init {\n    example.on((args) => {\n        if (args == 1) {\n            println(args);\n        }\n    });\n}\n"
    );
}

#[test]
fn formats_event_declarations() {
    assert_eq!(
        format_source("pub  event example(args: Args){}"),
        "pub event example(args: Args) {\n}\n"
    );
    assert_eq!(
        format_source("pub override event onCast<Char,Hex>(from: Char,to: Hex){to.value=1;}"),
        "pub override event onCast<Char, Hex>(from: Char, to: Hex) {\n    to.value = 1;\n}\n"
    );
}
