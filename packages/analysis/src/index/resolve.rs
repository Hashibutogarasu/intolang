use std::path::{Path, PathBuf};

use ipc::SymbolKind;
use moduler::ImportTarget;
use types::{Span, Token, TokenKind};

use super::symbol::{Index, NativeRef, Symbol};
use crate::analyzer::Analysis;

const ROOT_CLASS: &str = "Object";
const MAX_PARENTS: usize = 16;

/// Where in the address of an `import` or `export` the cursor lies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModulePart {
    Whole,
    Scheme,
    Path { prefix: String },
}

pub enum Resolved<'a> {
    Symbol {
        module: Option<usize>,
        symbol: &'a Symbol,
        token: Span,
    },
    Module {
        target: ImportTarget,
        token: Span,
        part: ModulePart,
    },
    Native {
        native: &'a NativeRef,
        root: PathBuf,
        token: Span,
    },
    /// A keyword that continues a statement begun elsewhere: `break` and the
    /// `while` it leaves, `else` and `else if` and the branch they follow.
    Keyword {
        label: &'static str,
        target: Span,
        token: Span,
    },
}

/// What the name at the character `offset` refers to.
pub fn resolve<'a>(
    analysis: &'a Analysis,
    offset: usize,
    packages_dir: Option<&Path>,
) -> Option<Resolved<'a>> {
    let tokens = &analysis.tokens;
    let position = tokens
        .iter()
        .position(|token| token.span.start <= offset && offset < token.span.end)
        .or_else(|| {
            tokens.iter().position(|token| {
                offset > 0
                    && token.span.end == offset
                    && matches!(token.kind, TokenKind::Identifier(_))
            })
        })?;
    let token = &tokens[position];
    match &token.kind {
        TokenKind::String(_) => {
            if let Some(native) = link_attribute(analysis, position) {
                return Some(native);
            }
            module(analysis, token, token, offset, packages_dir?)
        }
        TokenKind::Identifier(word) if matches!(word.as_str(), "import" | "export") => {
            let uri = tokens
                .get(position + 1)
                .filter(|next| matches!(next.kind, TokenKind::String(_)))?;
            module(analysis, token, uri, offset, packages_dir?)
        }
        TokenKind::Identifier(word) if word == "break" => {
            enclosing_loop(tokens, position).map(|keyword| Resolved::Keyword {
                label: "break",
                target: tokens[keyword].span,
                token: token.span,
            })
        }
        TokenKind::Identifier(word) if matches!(word.as_str(), "else" | "if") => {
            let (first, last) = branch(tokens, position)?;
            let (previous, previous_last) = previous_branch(tokens, first)?;
            Some(Resolved::Keyword {
                label: if first == last { "else" } else { "else if" },
                target: join(tokens[previous].span, tokens[previous_last].span),
                token: join(tokens[first].span, tokens[last].span),
            })
        }
        TokenKind::Identifier(word) => {
            let scope = Scope { analysis };
            if let Some(symbol) = analysis.index.at(token.span.start) {
                return Some(Resolved::Symbol {
                    module: None,
                    symbol,
                    token: token.span,
                });
            }
            let found = if is_member(tokens, position) {
                scope.member_access(tokens, position, word, token.span.line)
            } else {
                scope.name(word, token.span.line)
            };
            found.map(|(module, symbol)| Resolved::Symbol {
                module,
                symbol,
                token: token.span,
            })
        }
        _ => None,
    }
}

fn is_word(token: &Token, expected: &str) -> bool {
    matches!(&token.kind, TokenKind::Identifier(word) if word == expected)
}

/// The tokens `first..=last` of the `else` or `else if` that holds the token at
/// `position`. `else if` is one keyword whatever lies between its two words;
/// a plain `if` begins a statement and is no branch.
fn branch(tokens: &[Token], position: usize) -> Option<(usize, usize)> {
    let before = position
        .checked_sub(1)
        .filter(|&before| is_word(&tokens[before], "else"));
    if is_word(&tokens[position], "if") {
        return before.map(|before| (before, position));
    }
    let after = position + 1;
    Some(match tokens.get(after) {
        Some(next) if is_word(next, "if") => (position, after),
        _ => (position, position),
    })
}

/// The `if`, `else if` or `else` whose block ends right before the token
/// `first`: the branch that `first` continues.
fn previous_branch(tokens: &[Token], first: usize) -> Option<(usize, usize)> {
    if tokens[first.checked_sub(1)?].kind != TokenKind::Symbol('}') {
        return None;
    }
    let mut closed = 0;
    let open = (0..first).rev().find(|&index| {
        match tokens[index].kind {
            TokenKind::Symbol('}') => closed += 1,
            TokenKind::Symbol('{') => closed -= 1,
            _ => {}
        }
        closed == 0
    })?;
    if open > 0 && is_word(&tokens[open - 1], "else") {
        return Some((open - 1, open - 1));
    }
    let keyword = (0..open)
        .rev()
        .find(|&index| is_word(&tokens[index], "if"))?;
    match keyword.checked_sub(1) {
        Some(before) if is_word(&tokens[before], "else") => Some((before, keyword)),
        _ => Some((keyword, keyword)),
    }
}

/// The span from the start of `first` to the end of `last`.
fn join(first: Span, last: Span) -> Span {
    Span {
        line: first.line,
        end_line: last.end_line,
        start: first.start,
        end: last.end,
    }
}

/// The `while` whose body holds the token at `position`, the innermost one.
fn enclosing_loop(tokens: &[Token], position: usize) -> Option<usize> {
    let mut closed = 0;
    for index in (0..position).rev() {
        match tokens[index].kind {
            TokenKind::Symbol('}') => closed += 1,
            TokenKind::Symbol('{') if closed > 0 => closed -= 1,
            TokenKind::Symbol('{') => {
                if let Some(keyword) = loop_keyword(tokens, index) {
                    return Some(keyword);
                }
            }
            _ => {}
        }
    }
    None
}

/// The `while` of `while (condition) {` when `open` is that brace.
fn loop_keyword(tokens: &[Token], open: usize) -> Option<usize> {
    let mut parentheses = 0;
    let start = (0..open).rev().find(|&index| {
        match tokens[index].kind {
            TokenKind::Symbol(')') => parentheses += 1,
            TokenKind::Symbol('(') => parentheses -= 1,
            _ => {}
        }
        parentheses == 0
    })?;
    let keyword = start.checked_sub(1)?;
    (tokens[open - 1].kind == TokenKind::Symbol(')')
        && tokens[start].kind == TokenKind::Symbol('(')
        && matches!(&tokens[keyword].kind, TokenKind::Identifier(word) if word == "while"))
    .then_some(keyword)
}

/// The foreign function of the `#[link("language", "source", "symbol")]`
/// attribute that holds the string token at `position`.
fn link_attribute<'a>(analysis: &'a Analysis, position: usize) -> Option<Resolved<'a>> {
    let tokens = &analysis.tokens;
    let open = (0..position).rev().find(|&index| {
        !matches!(
            tokens[index].kind,
            TokenKind::String(_) | TokenKind::Symbol(',')
        )
    })?;
    if tokens[open].kind != TokenKind::Symbol('(')
        || !matches!(&tokens[open.checked_sub(1)?].kind, TokenKind::Identifier(word) if word == "link")
    {
        return None;
    }
    let arguments = tokens[open + 1..]
        .iter()
        .map_while(|token| match &token.kind {
            TokenKind::String(value) => Some(Some(value.as_str())),
            TokenKind::Symbol(',') => Some(None),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    let [language, source_path, target] = arguments[..] else {
        return None;
    };
    let native = analysis.index.symbols.iter().find_map(|symbol| {
        symbol.native.as_ref().filter(|native| {
            native.language == language
                && native.source_path == source_path
                && native.target == target
        })
    })?;
    Some(Resolved::Native {
        native,
        root: analysis.root.clone()?,
        token: tokens[position].span,
    })
}

/// The package and file named by the `import` or `export` whose address is the
/// string token `uri`; `token` is what the answer is about.
fn module<'a>(
    analysis: &Analysis,
    token: &Token,
    uri: &Token,
    offset: usize,
    packages_dir: &Path,
) -> Option<Resolved<'a>> {
    let within = |decl: &&ast::UseDecl| {
        decl.uri_span.0.start <= uri.span.start && uri.span.end <= decl.uri_span.0.end
    };
    let (decl, target) = match analysis.program.imports().find(within) {
        Some(decl) => (decl, moduler::import_target(decl, packages_dir)),
        None => {
            let decl = analysis.program.exports().find(within)?;
            (decl, moduler::export_target(decl, packages_dir))
        }
    };
    let part = if token.span == uri.span {
        module_part(&decl.uri, offset.saturating_sub(uri.span.start + 1))
    } else {
        ModulePart::Whole
    };
    Some(Resolved::Module {
        target: target.ok()??,
        token: token.span,
        part,
    })
}

/// The part of the address `uri` that the character `at` lies in.
fn module_part(uri: &str, at: usize) -> ModulePart {
    let Some((scheme, rest)) = uri.split_once("://") else {
        return ModulePart::Scheme;
    };
    let mut scheme_length = scheme.chars().count() + "://".len();
    let mut path = rest;
    if scheme == moduler::BUILTIN_SCHEME {
        let alias = rest.split('/').next().unwrap_or_default();
        scheme_length += alias.chars().count();
        path = rest[alias.len()..].strip_prefix('/').unwrap_or("");
        if rest.len() > alias.len() {
            scheme_length += 1;
        }
    }
    if at < scheme_length || path.is_empty() {
        return ModulePart::Scheme;
    }
    let path: Vec<char> = path.chars().collect();
    let inside = at - scheme_length;
    let end = path
        .iter()
        .skip(inside)
        .position(|character| *character == '/')
        .map_or(path.len(), |distance| inside + distance);
    ModulePart::Path {
        prefix: path[..end.min(path.len())].iter().collect(),
    }
}

fn is_member(tokens: &[Token], position: usize) -> bool {
    position >= 1 && tokens[position - 1].kind == TokenKind::Symbol('.')
}

type Found<'a> = Option<(Option<usize>, &'a Symbol)>;

pub struct Scope<'a> {
    analysis: &'a Analysis,
}

impl<'a> Scope<'a> {
    pub fn new(analysis: &'a Analysis) -> Self {
        Self { analysis }
    }

    fn indexes(&self) -> impl Iterator<Item = (Option<usize>, &'a Index)> {
        std::iter::once((None, &self.analysis.index)).chain(
            self.analysis
                .modules
                .iter()
                .enumerate()
                .map(|(number, module)| (Some(number), &module.index)),
        )
    }

    fn name(&self, name: &str, line: usize) -> Found<'a> {
        if let Some(symbol) = self.analysis.index.local(name, line) {
            return Some((None, symbol));
        }
        self.indexes().find_map(|(module, index)| {
            let symbol = index.top_level(name)?;
            (module.is_none() || symbol.public).then_some((module, symbol))
        })
    }

    fn member(&self, class: &str, name: &str) -> Found<'a> {
        let mut current = Some(class.to_owned());
        for _ in 0..MAX_PARENTS {
            let Some(class) = current else { break };
            if let Some(found) = self
                .indexes()
                .find_map(|(module, index)| Some((module, index.member(&class, name)?)))
            {
                return Some(found);
            }
            current = self.parent_of(&class);
        }
        self.indexes()
            .find_map(|(module, index)| Some((module, index.member(ROOT_CLASS, name)?)))
    }

    fn parent_of(&self, class: &str) -> Option<String> {
        self.indexes()
            .filter_map(|(_, index)| index.top_level(class))
            .find(|symbol| symbol.kind == SymbolKind::Class)
            .and_then(|symbol| symbol.parent.clone())
    }

    fn member_access(
        &self,
        tokens: &[Token],
        position: usize,
        word: &str,
        line: usize,
    ) -> Found<'a> {
        if let Some(receiver) = self.receiver_type(tokens, position, line) {
            return self.member(&receiver, word);
        }
        let mut candidates = self.indexes().flat_map(|(module, index)| {
            index
                .symbols
                .iter()
                .filter(|symbol| symbol.container.is_some() && symbol.name == word)
                .map(move |symbol| (module, symbol))
        });
        let first = candidates.next()?;
        if candidates.next().is_none() {
            return Some(first);
        }
        self.member(ROOT_CLASS, word)
    }

    fn receiver_type(&self, tokens: &[Token], position: usize, line: usize) -> Option<String> {
        let mut segments = Vec::new();
        let mut dot = position - 1;
        loop {
            if dot == 0 || tokens[dot].kind != TokenKind::Symbol('.') {
                return None;
            }
            let TokenKind::Identifier(name) = &tokens[dot - 1].kind else {
                return None;
            };
            segments.push(name.as_str());
            if dot >= 2 && tokens[dot - 2].kind == TokenKind::Symbol('.') {
                dot -= 2;
            } else {
                break;
            }
        }
        segments.reverse();
        self.type_of_path(&segments, line)
    }

    /// The class of the value `a.b.c` names, found from the declared types.
    pub fn type_of_path<S: AsRef<str>>(&self, segments: &[S], line: usize) -> Option<String> {
        let (first, rest) = segments.split_first()?;
        let mut current = self.root_type(first.as_ref(), line)?;
        for segment in rest {
            let (_, symbol) = self.member(&current, segment.as_ref())?;
            current = base_type(symbol.ty.as_deref()?);
        }
        Some(current)
    }

    /// The member of the class `class` or of its parents named `name`.
    pub fn member_of(&self, class: &str, name: &str) -> Found<'a> {
        self.member(class, name)
    }

    /// The symbol `name` stands for at the 1 based `line`.
    pub fn named(&self, name: &str, line: usize) -> Found<'a> {
        self.name(name, line)
    }

    /// The members of the class `class`, its parents and `Object`.
    pub fn members_of(&self, class: &str) -> Vec<(Option<usize>, &'a Symbol)> {
        let mut found = Vec::new();
        let mut current = Some(class.to_owned());
        for _ in 0..MAX_PARENTS {
            let Some(class) = current else { break };
            found.extend(self.indexes().flat_map(|(module, index)| {
                index.members(&class).map(move |symbol| (module, symbol))
            }));
            current = self.parent_of(&class);
        }
        found.extend(
            self.indexes()
                .flat_map(|(module, index)| index.members(ROOT_CLASS).map(move |s| (module, s))),
        );
        found
    }

    /// Every symbol a name may stand for at the 1 based `line`: the locals, the
    /// top-level names of the document and the public ones of its modules.
    pub fn visible(&self, line: usize) -> Vec<(Option<usize>, &'a Symbol)> {
        let mut found: Vec<_> = self
            .analysis
            .index
            .visible_at(line)
            .map(|symbol| (None, symbol))
            .collect();
        found.extend(self.indexes().flat_map(|(module, index)| {
            index
                .top_levels()
                .filter(move |symbol| module.is_none() || symbol.public)
                .map(move |symbol| (module, symbol))
        }));
        found
    }

    fn root_type(&self, name: &str, line: usize) -> Option<String> {
        let (_, symbol) = self.name(name, line)?;
        match symbol.kind {
            SymbolKind::Class | SymbolKind::Enum => Some(symbol.name.clone()),
            _ => symbol.ty.as_deref().map(base_type),
        }
    }
}

fn base_type(ty: &str) -> String {
    ty.split('<')
        .next()
        .unwrap_or_default()
        .trim_end_matches(['&', '*'])
        .to_owned()
}
