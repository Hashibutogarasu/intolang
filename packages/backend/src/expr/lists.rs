use ast::{Declaration, Expr, Path, Stmt, StmtKind, TypeRef};
use native::{CastAssign, NativeLink};
use types::{
    ArithmeticOperator, ComparisonOperator, Literal, MessageTag, Storage as Declared, Type,
};

use super::type_name::parse_type;
use crate::context::{Lowerer, Variable};
use crate::emit;

/// How the elements of a list are stored: the list keeps 64-bit items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Storage {
    Integer,
    Float,
    Text,
    Pointer,
}

impl Storage {
    fn of(element: &Type) -> Self {
        match element {
            ty if ty.is_integer() || matches!(ty, Type::Boolean | Type::Char) => Storage::Integer,
            ty if ty.is_float() => Storage::Float,
            Type::String | Type::Path => Storage::Text,
            _ => Storage::Pointer,
        }
    }

    /// The suffix of the list methods that move items of this storage.
    fn suffix(self) -> &'static str {
        match self {
            Storage::Integer => "Int",
            Storage::Float => "Float",
            Storage::Text => "String",
            Storage::Pointer => "Pointer",
        }
    }

    /// The type native code sees an item as, for scalars.
    fn scalar(self) -> Option<Type> {
        match self {
            Storage::Integer => Some(Type::Int64),
            Storage::Float => Some(Type::Float64),
            Storage::Text | Storage::Pointer => None,
        }
    }
}

impl<'a> Lowerer<'a> {
    fn list_link(&self, method: &str) -> Result<&'a NativeLink, String> {
        let env = self.env;
        let key = format!("List.{method}");
        env.native_links
            .get(&key)
            .ok_or_else(|| format!("`{key}` is not available; import \"intolang://builtin/std\""))
    }

    /// The element type of a list, if the type is one.
    fn list_element(ty: &Type) -> Option<&Type> {
        match ty.referent() {
            Type::List(element) => Some(element),
            _ => None,
        }
    }

    /// The expression that stands for `item` stored as an item of a list of
    /// `element`: a converted slot for a scalar, the text, or the binding.
    fn list_item(&mut self, item: &Expr, element: &Type) -> Result<Expr, String> {
        let storage = Storage::of(element);
        match storage {
            Storage::Integer | Storage::Float => {
                let slot = self.scalar_slot(item, element)?;
                let stored = storage.scalar().unwrap_or(Type::Int64);
                if *element == stored {
                    return Ok(Expr::ident(slot));
                }
                let converted = self.fresh_slot(&stored);
                emit::cast_assign(
                    &mut self.state.output,
                    &CastAssign {
                        target: converted.clone(),
                        source: slot,
                        from: element.name(),
                        to: stored.name(),
                    },
                );
                Ok(Expr::ident(converted))
            }
            Storage::Text => {
                if !self.is_text(item) {
                    return Err(format!(
                        "Expected a `{}` item but received `{item}`",
                        element.name()
                    ));
                }
                Ok(match self.lower_text(item, None)? {
                    super::Text::Const(text) => Expr::Literal(Literal::String(text)),
                    super::Text::Slot(slot) => Expr::ident(slot),
                })
            }
            Storage::Pointer => match item.as_ident() {
                Some(name) => {
                    self.ensure_usable(name)?;
                    Ok(item.clone())
                }
                None => Err(format!("Unsupported list item `{item}`")),
            },
        }
    }

    /// The element type an array literal has: the annotation when it names
    /// one, otherwise the type of its first item.
    fn array_element(&self, items: &[Expr], annotation: Option<&Type>) -> Result<Type, String> {
        if let Some(element) = annotation.and_then(Self::list_element) {
            return Ok(element.clone());
        }
        let first = items
            .first()
            .ok_or_else(|| "An empty array needs a type, such as `[Int32]`".to_owned())?;
        self.type_of(first)
    }

    /// The type of an array literal.
    pub(super) fn array_type(&self, items: &[Expr]) -> Result<Type, String> {
        Ok(Type::List(Box::new(self.array_element(items, None)?)))
    }

    /// Lower an array literal to a new list held by the slot `assigned_to`.
    pub fn lower_array(
        &mut self,
        items: &[Expr],
        annotation: Option<&Type>,
        assigned_to: &str,
    ) -> Result<Type, String> {
        let element = self.array_element(items, annotation)?;
        let list = Type::List(Box::new(element.clone()));
        let link = self.list_link("new")?;
        self.emit_link_call(MessageTag::LinkAssign, link, Vec::new(), assigned_to, false)?;
        self.state
            .values
            .insert(assigned_to.to_owned(), Variable::temporary(list.clone()));
        let storage = Storage::of(&element);
        let push = self.list_link(&format!("push{}", storage.suffix()))?;
        for item in items {
            let stored = self.list_item(item, &element)?;
            self.emit_link_call(
                MessageTag::LinkAssign,
                push,
                vec![Expr::ident(assigned_to), stored],
                "",
                true,
            )?;
        }
        Ok(list)
    }

    /// The statements `list.forEach(action)` stands for: a loop over the
    /// indices that hands each item to the lambda body, which also sees the
    /// index when it declares a second parameter. A binding that holds a
    /// function is called with each item instead.
    fn list_for_each(
        &mut self,
        list: &str,
        element: &Type,
        args: &[Expr],
    ) -> Result<Vec<Stmt>, String> {
        let [action] = args else {
            return Err(format!(
                "`forEach` takes 1 argument but received {}",
                args.len()
            ));
        };
        let counter = format!("__index{}", self.state.temp_counter);
        self.state.temp_counter += 1;
        let statement = |kind| Stmt { line: 0, kind };
        let declare = |name: &str, annotation: Option<TypeRef>, value: Expr| {
            statement(StmtKind::Declaration(Declaration {
                storage: Declared::Local,
                constant: false,
                pointer: false,
                name: name.to_owned(),
                annotation,
                value: Some(value),
            }))
        };
        let item_value = Expr::Call {
            callee: Path::from(vec![list.to_owned(), "get".to_owned()]),
            args: vec![Expr::ident(counter.clone())],
        };
        let borrowed = self.is_class_type(element);
        let mut body = Vec::new();
        match action {
            Expr::Lambda {
                params,
                body: lambda,
            } => {
                if params.is_empty() || params.len() > 2 {
                    return Err(format!(
                        "The lambda of `forEach` takes the item and optionally the index, not {} parameter(s)",
                        params.len()
                    ));
                }
                let written = match &params[0].ty {
                    Some(written) => {
                        let own = parse_type(written.as_str(), &self.env.type_params)
                            .map_err(|error| error.to_string())?;
                        if !element.is_assignable_to(&own) {
                            return Err(format!(
                                "The parameter `{}` is a `{}` but the items are `{}`",
                                params[0].name,
                                own.name(),
                                element.name()
                            ));
                        }
                        written.clone()
                    }
                    None if borrowed => TypeRef::from(format!("{}&", element.name()).as_str()),
                    None => TypeRef::from(element.name().as_str()),
                };
                body.push(declare(&params[0].name, Some(written), item_value));
                if let Some(index) = params.get(1) {
                    body.push(declare(
                        &index.name,
                        Some(TypeRef::from("Int32")),
                        Expr::ident(counter.clone()),
                    ));
                }
                body.extend(lambda.iter().cloned());
            }
            other => match other.as_ident() {
                Some(name) => {
                    let item = format!("__item{}", self.state.temp_counter);
                    self.state.temp_counter += 1;
                    let written = if borrowed {
                        TypeRef::from(format!("{}&", element.name()).as_str())
                    } else {
                        TypeRef::from(element.name().as_str())
                    };
                    body.push(declare(&item, Some(written), item_value));
                    body.push(statement(StmtKind::Expr(Expr::Call {
                        callee: Path::single(name),
                        args: vec![Expr::ident(item)],
                    })));
                }
                None => {
                    return Err(format!(
                        "`forEach` expects a lambda or a function, not `{other}`"
                    ));
                }
            },
        }
        body.push(statement(StmtKind::Assign {
            target: Path::single(counter.clone()),
            value: Expr::Binary {
                left: Box::new(Expr::ident(counter.clone())),
                op: ArithmeticOperator::Add,
                right: Box::new(Expr::int(1)),
            },
        }));
        let length = Expr::Call {
            callee: Path::from(vec![list.to_owned(), "length".to_owned()]),
            args: Vec::new(),
        };
        Ok(vec![
            declare(&counter, Some(TypeRef::from("Int32")), Expr::int(0)),
            statement(StmtKind::While {
                condition: Expr::Compare {
                    left: Box::new(Expr::ident(counter)),
                    op: ComparisonOperator::Less,
                    right: Box::new(length),
                },
                body,
            }),
        ])
    }

    /// Lower `push`, `get` and `set` on a list, which pick the methods that
    /// fit the element type. Returns `None` when the call is not one of those
    /// on a list; otherwise the type of the value, if the call has one. The
    /// value goes to `assigned_to`, unless that is empty.
    pub fn lower_list_method(
        &mut self,
        receiver: &str,
        receiver_type: &Type,
        method: &str,
        args: &[Expr],
        assigned_to: &str,
    ) -> Result<Option<Option<Type>>, String> {
        let Some(element) = Self::list_element(receiver_type).cloned() else {
            return Ok(None);
        };
        if method == "forEach" {
            let statements = self.list_for_each(receiver, &element, args)?;
            self.lower_block(&statements)
                .map_err(|error| error.to_string())?;
            return Ok(Some(None));
        }
        if !matches!(method, "push" | "get" | "set") {
            return Ok(None);
        }
        let expected = if method == "set" { 2 } else { 1 };
        if args.len() != expected {
            return Err(format!(
                "`{method}` takes {expected} argument(s) but received {}",
                args.len()
            ));
        }
        let storage = Storage::of(&element);
        let link = self.list_link(&format!("{method}{}", storage.suffix()))?;
        let list = Expr::ident(receiver);
        match method {
            "push" => {
                let stored = self.list_item(&args[0], &element)?;
                self.emit_link_call(MessageTag::LinkAssign, link, vec![list, stored], "", true)?;
                Ok(Some(None))
            }
            "set" => {
                let index = Expr::ident(self.scalar_slot(&args[0], &Type::Int32)?);
                let stored = self.list_item(&args[1], &element)?;
                self.emit_link_call(
                    MessageTag::LinkAssign,
                    link,
                    vec![list, index, stored],
                    "",
                    true,
                )?;
                Ok(Some(None))
            }
            _ => {
                let index = Expr::ident(self.scalar_slot(&args[0], &Type::Int32)?);
                let combined = vec![list, index];
                match storage.scalar() {
                    Some(stored) if stored != element && !assigned_to.is_empty() => {
                        let raw = self.fresh_slot(&stored);
                        self.emit_link_call(MessageTag::LinkAssign, link, combined, &raw, true)?;
                        emit::cast_assign(
                            &mut self.state.output,
                            &CastAssign {
                                target: assigned_to.to_owned(),
                                source: raw,
                                from: stored.name(),
                                to: element.name(),
                            },
                        );
                    }
                    _ => {
                        self.emit_link_call(
                            MessageTag::LinkAssign,
                            link,
                            combined,
                            assigned_to,
                            true,
                        )?;
                    }
                }
                Ok(Some(Some(element)))
            }
        }
    }
}
