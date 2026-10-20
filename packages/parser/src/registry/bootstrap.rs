use types::Keyword;

use super::definition::{KeywordDefinition, ValueDefinition};
use super::store::Registry;

/// Registers the grammar. Every reserved word is spelled here and nowhere else.
pub(super) fn bootstrap(registry: &mut Registry) {
    for (keyword, spelling, detail) in [
        (Keyword::Init, "init", "Run a block when the module loads"),
        (Keyword::Fun, "fun", "Declare a function"),
        (Keyword::Pub, "pub", "Make a declaration public"),
        (
            Keyword::Static,
            "static",
            "Declare a member of the class itself",
        ),
        (Keyword::Macro, "macro", "Declare a macro"),
        (Keyword::Into, "into", "Declare a variable"),
        (Keyword::Now, "now", "Declare a variable set at once"),
        (Keyword::Late, "late", "Declare a variable set later"),
        (Keyword::Const, "const", "Declare a constant"),
        (Keyword::Unload, "unload", "Release a variable"),
        (Keyword::Enum, "enum", "Declare an enumeration"),
        (Keyword::If, "if", "Run a block when a condition holds"),
        (
            Keyword::Else,
            "else",
            "Run a block when the condition does not hold",
        ),
        (
            Keyword::While,
            "while",
            "Repeat a block while a condition holds",
        ),
        (Keyword::Break, "break", "Leave the enclosing loop"),
        (Keyword::Return, "return", "Return from the function"),
        (Keyword::Throw, "throw", "Raise an error"),
        (Keyword::Import, "import", "Import a module"),
        (Keyword::Export, "export", "Export a module"),
        (Keyword::Event, "event", "Declare an event"),
        (Keyword::Thread, "thread", "Run a block on a new thread"),
        (Keyword::Await, "await", "Wait for a thread to finish"),
        (
            Keyword::Abstract,
            "abstract",
            "Declare a class or member without a body",
        ),
        (Keyword::Default, "default", "Give a member a default body"),
        (
            Keyword::Override,
            "override",
            "Replace a member of the parent class",
        ),
        (Keyword::As, "as", "Reserved"),
    ] {
        registry.register_keyword(KeywordDefinition {
            keyword,
            spelling,
            detail,
        });
    }
    for (spelling, detail) in [
        ("true", "The true boolean"),
        ("false", "The false boolean"),
        ("this", "The instance a method works on"),
    ] {
        registry.register_value(ValueDefinition { spelling, detail });
    }
}
