use formatter::format_source;
use tokenizer::Tokens;
use types::TokenKind;

const SAMPLES: [&str; 4] = [
    "import \"intolang://builtin/std\";\ninto mem1 = 0;\nprint(mem1); // compile error\ninit {\nprint(mem1);\n}\nconst now into mem3 = 3;\nunload mem5;\n",
    "enum JsonKind { Null, Boolean, Number }\npub JsonObject() {\n    pub into: Int32 kind = 0;\n    pub into: String text = \"\";\n}\n#[link(\"cpp\", \"src/a.cpp\", \"sym\")]\npub cpp fun readFile(path: String, format: Format): JsonObject;\n",
    "pub fun create(maximizable: Boolean = true): TitleBar {\nif (maximizable) {\nthis.allowMaximize();\n} else {\nthis.preventMaximize();\n}\nreturn this;\n}\n",
    "init {\nwhile (reading >= 0) {\nreading = readAudioFrame(format, packet);\nnow into* pcm = audio.buffer();\nnow into n = produced * channels;\n}\n}\n",
];

fn kinds(source: &str) -> Vec<TokenKind> {
    Tokens::with_comments(source)
        .unwrap()
        .into_iter()
        .map(|token| token.kind)
        .collect()
}

#[test]
fn formatting_preserves_tokens() {
    for sample in SAMPLES {
        assert_eq!(kinds(sample), kinds(&format_source(sample)), "{sample}");
    }
}

#[test]
fn formatting_is_idempotent_on_samples() {
    for sample in SAMPLES {
        let once = format_source(sample);
        assert_eq!(format_source(&once), once, "{sample}");
    }
}
