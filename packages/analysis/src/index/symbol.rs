use ipc::SymbolKind;
use types::Span;

#[derive(Debug, Clone)]
pub struct NativeRef {
    pub language: String,
    pub source_path: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub container: Option<String>,
    pub name_span: Span,
    pub line: usize,
    pub signature: String,
    pub docs: Option<String>,
    pub ty: Option<String>,
    pub parent: Option<String>,
    pub public: bool,
    pub local: bool,
    pub owner: usize,
    pub native: Option<NativeRef>,
}

#[derive(Debug, Default)]
pub struct Index {
    pub symbols: Vec<Symbol>,
    pub(super) item_lines: Vec<usize>,
}

impl Index {
    /// The top-level item that contains the 1 based `line`.
    pub fn owner_at(&self, line: usize) -> usize {
        self.item_lines
            .partition_point(|start| *start <= line)
            .saturating_sub(1)
    }

    pub fn at(&self, offset: usize) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|symbol| symbol.name_span.start <= offset && offset < symbol.name_span.end)
    }

    /// The value, parameter or declaration visible at `line` under `name`.
    pub fn local(&self, name: &str, line: usize) -> Option<&Symbol> {
        let owner = self.owner_at(line);
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.local && symbol.owner == owner && symbol.name == name && symbol.line <= line
            })
            .max_by_key(|symbol| symbol.line)
    }

    pub fn top_level(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|symbol| !symbol.local && symbol.container.is_none() && symbol.name == name)
    }

    pub fn member(&self, container: &str, name: &str) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|symbol| symbol.container.as_deref() == Some(container) && symbol.name == name)
    }
}
