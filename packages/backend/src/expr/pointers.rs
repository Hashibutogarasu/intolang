use ast::Expr;
use native::{MemoryLoad, MemoryStore, Operand};
use types::Type;

use crate::context::Lowerer;
use crate::emit;

/// The element types a pointer can be read and written as, by the suffix of
/// `loadInt32` and `storeInt32`.
const ELEMENTS: [(&str, Type); 9] = [
    ("Int8", Type::Int8),
    ("Int16", Type::Int16),
    ("Int32", Type::Int32),
    ("Int64", Type::Int64),
    ("Float32", Type::Float32),
    ("Float64", Type::Float64),
    ("Char", Type::Char),
    ("Boolean", Type::Boolean),
    ("Pointer", Type::Pointer),
];

impl Lowerer<'_> {
    /// The element type and the operation of a call such as `p.load(i)`,
    /// `p.storeInt32(i, v)` on a pointer.
    fn pointer_access(receiver_type: &Type, method: &str) -> Option<(Type, bool)> {
        let (store, rest) = match method.strip_prefix("load") {
            Some(rest) => (false, rest),
            None => (true, method.strip_prefix("store")?),
        };
        let pointer = receiver_type.referent();
        if !pointer.is_pointer() {
            return None;
        }
        if rest.is_empty() {
            return match pointer {
                Type::PointerTo(element) => Some(((**element).clone(), store)),
                _ => None,
            };
        }
        ELEMENTS
            .iter()
            .find(|(name, _)| *name == rest)
            .map(|(_, element)| (element.clone(), store))
    }

    /// Lower `load` and `store` (and `loadInt32`, `storeFloat64`, ...) on a
    /// pointer: read or write the item at an index. Returns `None` when the
    /// call is not one of those on a pointer; otherwise the type of the value
    /// that goes to `assigned_to`, if the call has one.
    pub fn lower_pointer_method(
        &mut self,
        receiver: &str,
        receiver_type: &Type,
        method: &str,
        args: &[Expr],
        assigned_to: &str,
    ) -> Result<Option<Option<Type>>, String> {
        let Some((element, store)) = Self::pointer_access(receiver_type, method) else {
            return Ok(None);
        };
        let expected = if store { 2 } else { 1 };
        if args.len() != expected {
            return Err(format!(
                "`{method}` takes {expected} argument(s) but received {}",
                args.len()
            ));
        }
        self.ensure_usable(receiver)?;
        let index = Operand::Slot(self.scalar_slot(&args[0], &Type::Int64)?);
        if store {
            let value = self.pointer_value(&args[1], &element)?;
            emit::memory_store(
                &mut self.state.output,
                &MemoryStore {
                    pointer: receiver.to_owned(),
                    index,
                    ty: element.name(),
                    value,
                },
            );
            return Ok(Some(None));
        }
        if assigned_to.is_empty() {
            return Err(format!("`{method}` gives a value that has to be used"));
        }
        emit::memory_load(
            &mut self.state.output,
            &MemoryLoad {
                target: assigned_to.to_owned(),
                pointer: receiver.to_owned(),
                index,
                ty: element.name(),
            },
        );
        Ok(Some(Some(element)))
    }

    /// The operand that writes `value` as an item of type `element`.
    fn pointer_value(&mut self, value: &Expr, element: &Type) -> Result<Operand, String> {
        if *element == Type::Pointer {
            let name = value
                .as_ident()
                .ok_or_else(|| format!("Expected a pointer but received `{value}`"))?;
            self.ensure_usable(name)?;
            return Ok(Operand::Slot(name.to_owned()));
        }
        Ok(Operand::Slot(self.scalar_slot(value, element)?))
    }
}
