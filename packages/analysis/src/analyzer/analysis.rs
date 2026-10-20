use std::path::{Path, PathBuf};

use ast::Program;
use errors::Result;
use moduler::SourceProvider;
use tokenizer::Tokens;

use crate::docs::DocComments;
use crate::index::Index;
use crate::text::LineIndex;
use crate::workspace::real;

pub struct ModuleAnalysis {
    pub path: PathBuf,
    pub index: Index,
    pub line_index: LineIndex,
}

/// What the tools know about a document that parsed: its tree, its names and
/// the modules it imports.
pub struct Analysis {
    pub program: Program,
    pub tokens: Tokens,
    pub index: Index,
    pub modules: Vec<ModuleAnalysis>,
}

impl Analysis {
    pub fn build(
        text: &str,
        packages_dir: Option<&Path>,
        sources: &dyn SourceProvider,
    ) -> Result<Self> {
        let tokens: Tokens = text.parse()?;
        let program = Program::try_from(&tokens)?;
        let index = Index::build(&program, &DocComments::new(text));
        let modules = packages_dir
            .and_then(|dir| moduler::collect_modules_from(&program, dir, sources).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|module| ModuleAnalysis {
                index: Index::build(&module.program, &DocComments::new(&module.contents)),
                line_index: LineIndex::new(&module.contents),
                path: real(&module.module.source_path),
            })
            .collect();
        Ok(Self {
            program,
            tokens,
            index,
            modules,
        })
    }
}
