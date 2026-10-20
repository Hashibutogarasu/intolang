use ast::{
    ClassDecl, Declaration, EnumDecl, EventDecl, Expr, FunctionDecl, Item, Param, Program, Stmt,
    StmtKind,
};
use ipc::SymbolKind;
use types::{Literal, Span};

use super::signature;
use super::symbol::{Index, NativeRef, Symbol};
use crate::docs::DocComments;

impl Index {
    pub fn build(program: &Program, docs: &DocComments) -> Self {
        let mut builder = Builder {
            docs,
            index: Index::default(),
            owner: 0,
        };
        for (owner, item) in program.items.iter().enumerate() {
            builder.owner = owner;
            builder.index.item_lines.push(item_line(item));
            builder.item(item);
        }
        builder.index
    }
}

fn item_line(item: &Item) -> usize {
    match item {
        Item::Import(decl) | Item::Export(decl) => decl.line,
        Item::Enum(decl) => decl.line,
        Item::Class(decl) => decl.line,
        Item::Function(decl) => decl.line,
        Item::Event(decl) => decl.line,
        Item::Macro(decl) => decl.line,
        Item::Stmt(stmt) => stmt.line,
    }
}

struct Builder<'a> {
    docs: &'a DocComments,
    index: Index,
    owner: usize,
}

impl Builder<'_> {
    fn item(&mut self, item: &Item) {
        match item {
            Item::Import(_) | Item::Export(_) => {}
            Item::Enum(decl) => self.enumeration(decl),
            Item::Class(decl) => self.class(decl),
            Item::Function(decl) => self.function(decl, None),
            Item::Event(decl) => self.event(decl, None),
            Item::Macro(decl) => {
                self.push(Symbol {
                    signature: signature::mac(decl),
                    public: decl.is_public,
                    ..self.symbol(&decl.name, SymbolKind::Macro, decl.name_span.0, decl.line)
                });
            }
            Item::Stmt(stmt) => self.statement(stmt, false),
        }
    }

    fn symbol(&self, name: &str, kind: SymbolKind, name_span: Span, line: usize) -> Symbol {
        Symbol {
            name: name.to_owned(),
            kind,
            container: None,
            name_span,
            line,
            signature: String::new(),
            docs: self.docs.for_declaration(line).map(str::to_owned),
            ty: None,
            parent: None,
            public: false,
            local: false,
            owner: self.owner,
            native: None,
        }
    }

    fn push(&mut self, symbol: Symbol) {
        self.index.symbols.push(symbol);
    }

    fn enumeration(&mut self, decl: &EnumDecl) {
        self.push(Symbol {
            signature: format!("enum {}", decl.name),
            public: true,
            ..self.symbol(&decl.name, SymbolKind::Enum, decl.name_span.0, decl.line)
        });
        for variant in &decl.variants {
            let span = variant.name_span.0;
            let value = variant
                .value
                .map_or(String::new(), |value| format!(" = {value}"));
            self.push(Symbol {
                container: Some(decl.name.clone()),
                signature: format!("{}.{}{value}", decl.name, variant.name),
                ty: Some(decl.name.clone()),
                public: true,
                ..self.symbol(&variant.name, SymbolKind::Variant, span, span.line)
            });
        }
    }

    fn class(&mut self, decl: &ClassDecl) {
        self.push(Symbol {
            signature: signature::class(decl),
            parent: decl.parent.clone(),
            public: decl.is_public,
            ..self.symbol(&decl.name, SymbolKind::Class, decl.name_span.0, decl.line)
        });
        for field in &decl.fields {
            self.push(Symbol {
                container: Some(decl.name.clone()),
                signature: signature::field(field),
                ty: Some(field.ty.to_string()),
                public: true,
                ..self.symbol(
                    &field.name,
                    SymbolKind::Field,
                    field.name_span.0,
                    field.line,
                )
            });
        }
        for method in &decl.methods {
            self.function(method, Some(&decl.name));
        }
        for event in &decl.events {
            self.event(event, Some(&decl.name));
        }
    }

    fn function(&mut self, decl: &FunctionDecl, container: Option<&str>) {
        let native = native_reference(decl);
        let mut symbol = Symbol {
            container: container.map(str::to_owned),
            signature: signature::function(decl),
            ty: decl.return_type.as_ref().map(ToString::to_string),
            public: decl.is_public || container.is_some(),
            ..self.symbol(
                &decl.name,
                SymbolKind::Function,
                decl.name_span.0,
                decl.line,
            )
        };
        if let Some(native) = &native {
            let origin = format!(
                "Native: `{}` `{}` `{}`",
                native.language, native.source_path, native.target
            );
            symbol.docs = Some(match symbol.docs.take() {
                Some(docs) => format!("{docs}\n\n{origin}"),
                None => origin,
            });
        }
        symbol.native = native;
        self.push(symbol);
        self.params(&decl.params, decl.line);
        if let Some(body) = &decl.body {
            self.block(body);
        }
    }

    fn event(&mut self, decl: &EventDecl, container: Option<&str>) {
        self.push(Symbol {
            container: container.map(str::to_owned),
            signature: signature::event(decl),
            public: decl.is_public || container.is_some(),
            ..self.symbol(&decl.name, SymbolKind::Event, decl.name_span.0, decl.line)
        });
        self.params(&decl.params, decl.line);
        self.block(&decl.body);
    }

    fn params(&mut self, params: &[Param], line: usize) {
        for param in params {
            self.push(Symbol {
                signature: signature::param(param),
                ty: param.ty.as_ref().map(ToString::to_string),
                local: true,
                ..self.symbol(&param.name, SymbolKind::Variable, param.name_span.0, line)
            });
        }
    }

    fn block(&mut self, body: &[Stmt]) {
        for stmt in body {
            self.statement(stmt, true);
        }
    }

    fn statement(&mut self, stmt: &Stmt, local: bool) {
        match &stmt.kind {
            StmtKind::Declaration(declaration) => {
                let ty = declared_type(declaration);
                self.push(Symbol {
                    signature: signature::local(declaration, ty.as_deref()),
                    ty,
                    local,
                    ..self.symbol(
                        &declaration.name,
                        SymbolKind::Variable,
                        declaration.name_span.0,
                        stmt.line,
                    )
                });
            }
            StmtKind::If {
                then_body,
                else_body,
                ..
            } => {
                self.block(then_body);
                if let Some(body) = else_body {
                    self.block(body);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Init(body) => self.block(body),
            _ => {}
        }
    }
}

fn native_reference(decl: &FunctionDecl) -> Option<NativeRef> {
    let attribute = decl
        .attributes
        .iter()
        .find(|attribute| attribute.name == "link")?;
    let [language, source_path, target] = attribute.string_args()?[..] else {
        return None;
    };
    Some(NativeRef {
        language: language.to_owned(),
        source_path: source_path.to_owned(),
        target: target.to_owned(),
    })
}

fn declared_type(declaration: &Declaration) -> Option<String> {
    if let Some(annotation) = &declaration.annotation {
        return Some(annotation.to_string());
    }
    match declaration.value.as_ref()? {
        Expr::Literal(Literal::Int(_)) => Some("Int32".to_owned()),
        Expr::Literal(Literal::Float(_)) => Some("Float64".to_owned()),
        Expr::Literal(Literal::String(_)) => Some("String".to_owned()),
        Expr::Literal(Literal::Bool(_)) => Some("Boolean".to_owned()),
        Expr::Literal(Literal::Char(_)) => Some("Char".to_owned()),
        Expr::Call { callee, .. } => callee.as_ident().map(str::to_owned),
        _ => None,
    }
}
