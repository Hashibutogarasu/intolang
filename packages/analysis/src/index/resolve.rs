use std::path::{Path, PathBuf};

use ipc::SymbolKind;
use moduler::ImportTarget;
use types::{Span, Token, TokenKind};

use super::symbol::{Index, NativeRef, Symbol};
use crate::analyzer::Analysis;

const ROOT_CLASS: &str = "Object";
const MAX_PARENTS: usize = 16;

pub enum Resolved<'a> {
    Symbol {
        /// The module that declares the symbol; `None` for the document itself.
        module: Option<usize>,
        symbol: &'a Symbol,
        token: Span,
    },
    Module {
        target: ImportTarget,
        token: Span,
    },
    Native {
        native: &'a NativeRef,
        root: PathBuf,
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
            module(analysis, token, token, packages_dir?)
        }
        TokenKind::Identifier(word) if matches!(word.as_str(), "import" | "export") => {
            let uri = tokens
                .get(position + 1)
                .filter(|next| matches!(next.kind, TokenKind::String(_)))?;
            module(analysis, token, uri, packages_dir?)
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
    packages_dir: &Path,
) -> Option<Resolved<'a>> {
    let within = |decl: &&ast::UseDecl| {
        decl.uri_span.0.start <= uri.span.start && uri.span.end <= decl.uri_span.0.end
    };
    let target = match analysis.program.imports().find(within) {
        Some(decl) => moduler::import_target(decl, packages_dir),
        None => moduler::export_target(analysis.program.exports().find(within)?, packages_dir),
    };
    Some(Resolved::Module {
        target: target.ok()??,
        token: token.span,
    })
}

fn is_member(tokens: &[Token], position: usize) -> bool {
    position >= 1 && tokens[position - 1].kind == TokenKind::Symbol('.')
}

type Found<'a> = Option<(Option<usize>, &'a Symbol)>;

struct Scope<'a> {
    analysis: &'a Analysis,
}

impl<'a> Scope<'a> {
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
        candidates.next().is_none().then_some(first)
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
        let mut current = self.root_type(segments[0], line)?;
        for segment in &segments[1..] {
            let (_, symbol) = self.member(&current, segment)?;
            current = base_type(symbol.ty.as_deref()?);
        }
        Some(current)
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
