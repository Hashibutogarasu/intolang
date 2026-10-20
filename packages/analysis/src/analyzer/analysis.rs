use std::path::{Path, PathBuf};

use ast::Program;
use errors::Result;
use moduler::SourceProvider;
use package::Package;
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
    pub root: Option<PathBuf>,
    pub modules: Vec<ModuleAnalysis>,
}

const BUILTIN_FUNCTIONS: &str = "src/main/builtin";

/// The functions the compiler provides without an import, such as `println`,
/// declared one to a file in the builtin package.
fn builtin_functions(packages_dir: &Path, sources: &dyn SourceProvider) -> Vec<ModuleAnalysis> {
    let Ok(Some(builtin)) = Package::find_by_alias(packages_dir, moduler::BUILTIN_ALIAS) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(builtin.join(BUILTIN_FUNCTIONS)) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "into")
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let contents = sources.read(&path)?;
            let program = contents.parse::<Program>().ok()?;
            Some(ModuleAnalysis {
                index: Index::build(&program, &DocComments::new(&contents)),
                line_index: LineIndex::new(&contents),
                path: real(&path),
            })
        })
        .collect()
}

impl Analysis {
    pub fn build(
        text: &str,
        path: Option<&Path>,
        packages_dir: Option<&Path>,
        sources: &dyn SourceProvider,
    ) -> Result<Self> {
        let tokens: Tokens = text.parse()?;
        let program = Program::try_from(&tokens)?;
        let index = Index::build(&program, &DocComments::new(text));
        let mut modules: Vec<ModuleAnalysis> = packages_dir
            .and_then(|dir| moduler::collect_modules_from(&program, dir, sources).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|module| ModuleAnalysis {
                index: Index::build(&module.program, &DocComments::new(&module.contents)),
                line_index: LineIndex::new(&module.contents),
                path: real(&module.module.source_path),
            })
            .collect();
        if let Some(dir) = packages_dir {
            modules.extend(builtin_functions(dir, sources));
        }
        Ok(Self {
            program,
            tokens,
            index,
            root: path
                .and_then(|path| Package::enclosing(&real(path)))
                .map(|package| package.root),
            modules,
        })
    }
}
