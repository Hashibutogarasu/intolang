use ast::UseDecl;
use moduler::{ExportSpec, ImportSpec};

use crate::context::Lowerer;
use crate::emit;

impl Lowerer<'_> {
    /// `import "scheme://module";`: when the URI names a resolved module, a
    /// `Module` instance describing it is created in the lowered message.
    pub fn lower_import(&mut self, decl: &UseDecl) {
        if let Ok(spec) = ImportSpec::try_from(decl) {
            self.emit_module_where(|module| module.alias == spec.alias && module.name == spec.module);
        }
    }

    /// `export "alias://path";`: like an import, for the exported module.
    pub fn lower_export(&mut self, decl: &UseDecl) {
        if let Ok(spec) = ExportSpec::try_from(decl) {
            self.emit_module_where(|module| module.alias == spec.alias && module.name == spec.path);
        }
    }

    fn emit_module_where(&mut self, matches: impl Fn(&moduler::Module) -> bool) {
        let modules = &self.env.modules;
        if let Some(index) = modules.iter().position(matches) {
            emit::module(&mut self.state.output, index, &modules[index]);
        }
    }
}
