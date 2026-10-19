use formatter::format_source;
use tokenizer::Tokens;
use types::TokenKind;

fn kinds(source: &str) -> Vec<TokenKind> {
    Tokens::with_comments(source)
        .unwrap()
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

#[test]
fn formatting_preserves_tokens() {
    for sample in [
        r#"import "intolang://builtin/std";
into mem1 = 0;
print(mem1); // compile error
init {
print(mem1);
}
const now into mem3 = 3;
unload mem5;
"#,
        r#"enum JsonKind { Null, Boolean, Number }
pub JsonObject() {
    pub into: Int32 kind = 0;
    pub into: String text = "";
}
#[link("cpp", "src/a.cpp", "sym")]
pub cpp fun readFile(path: String, format: Format): JsonObject;
"#,
        r#"pub fun create(maximizable: Boolean = true): TitleBar {
if (maximizable) {
this.allowMaximize();
} else {
this.preventMaximize();
}
return this;
}
"#,
        r#"init {
while (reading >= 0) {
reading = readAudioFrame(format, packet);
now into* pcm = audio.buffer();
now into n = produced * channels;
}
}
"#,
    ] {
        assert_eq!(kinds(sample), kinds(&format_source(sample)), "{sample}");
    }
}

#[test]
fn formatting_is_idempotent_on_samples() {
    for sample in [
        r#"import "intolang://builtin/std";
into mem1 = 0;
print(mem1); // compile error
init {
print(mem1);
}
const now into mem3 = 3;
unload mem5;
"#,
        r#"enum JsonKind { Null, Boolean, Number }
pub JsonObject() {
    pub into: Int32 kind = 0;
    pub into: String text = "";
}
#[link("cpp", "src/a.cpp", "sym")]
pub cpp fun readFile(path: String, format: Format): JsonObject;
"#,
        r#"pub fun create(maximizable: Boolean = true): TitleBar {
if (maximizable) {
this.allowMaximize();
} else {
this.preventMaximize();
}
return this;
}
"#,
        r#"init {
while (reading >= 0) {
reading = readAudioFrame(format, packet);
now into* pcm = audio.buffer();
now into n = produced * channels;
}
}
"#,
    ] {
        let once = format_source(sample);
        assert_eq!(format_source(&once), once, "{sample}");
    }
}
