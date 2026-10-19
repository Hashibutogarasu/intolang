use ast::Expr;
use native::{CastAssign, NativeLink};
use types::{Literal, MessageTag, Type};

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
