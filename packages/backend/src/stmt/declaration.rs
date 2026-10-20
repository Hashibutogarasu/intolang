use ast::Declaration;
use errors::Result;
use native::OwnershipChange;
use types::{Storage, Type};

use crate::context::{Lowerer, Tracking, Variable};
use crate::emit;
use crate::expr::parse_type;

impl Lowerer<'_> {
    /// Lower `[const] [now|late] into [*]name[: Type] [= value];`.
    pub fn lower_declaration(&mut self, line: usize, declaration: &Declaration) -> Result<()> {
        let env = self.env;
        let memory_backed = declaration.storage != Storage::Local;
        let declared_in_init = self.state.in_init;
        let Some(rhs) = &declaration.value else {
            if declaration.pointer {
                let name = &declaration.name;
                emit::assign(&mut self.state.output, name, &Type::Pointer, "n", "0");
                self.state.values.insert(
                    name.clone(),
                    Variable {
                        value: name.clone(),
                        ty: Type::Pointer,
                        memory_backed,
                        allocated: true,
                        lazy: false,
                        unloaded: false,
                        declared_in_init,
                        native: true,
                        constant: declaration.constant,
                        tracking: Tracking::default(),
                    },
                );
            }
            return Ok(());
        };
        let annotation = declaration
            .annotation
            .as_ref()
            .map(|ty| parse_type(ty.as_str(), &env.type_params))
            .transpose()
            .map_err(|error| self.diagnostic(line, error))?;
        let annotation = match (declaration.pointer, annotation) {
            (true, None) => Some(Type::Pointer),
            (_, annotation) => annotation,
        };
        self.state.pending_tracking = None;
        let (value, ty, native) = self
            .declaration_value(rhs, annotation, &env.type_params, &declaration.name)
            .map_err(|error| self.diagnostic(line, error))?;
        let mut tracking = self.state.pending_tracking.take().unwrap_or_default();
        tracking.reference |= ty.is_reference();
        self.state.values.insert(
            declaration.name.clone(),
            Variable {
                value,
                ty,
                memory_backed,
                allocated: declaration.storage == Storage::Now,
                lazy: declaration.storage == Storage::Late,
                unloaded: false,
                declared_in_init,
                native,
                constant: declaration.constant,
                tracking,
            },
        );
        self.emit_change(OwnershipChange::Declare {
            name: declaration.name.clone(),
            storage: declaration.storage,
        });
        self.declare_in_scope(&declaration.name);
        Ok(())
    }
}
