use std::path::{Path, PathBuf};

use ipc::SymbolKind;
use types::{Span, Token, TokenKind};

use super::symbol::{Index, Symbol};
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
        path: PathBuf,
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
            let packages_dir = packages_dir?;
            let decl = analysis.program.imports().find(|decl| {
                decl.uri_span.0.start <= token.span.start && token.span.end <= decl.uri_span.0.end
            })?;
            let path = moduler::import_path(decl, packages_dir).ok()??;
            Some(Resolved::Module {
                path,
                token: token.span,
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
