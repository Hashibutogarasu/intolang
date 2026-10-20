use moduler::BUILTIN_ALIAS;
use package::Package;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

const PACKAGES_VARIABLE: &str = "INTOLANG_PACKAGES";
const BUILTIN_PACKAGES: &str = "packages";

/// A directory of package links the way a build sets one up: the builtin
/// packages and the packages installed in the project.
pub struct Environment {
    dir: PathBuf,
    /// Whether `dir` is a directory of links this environment made, rather
    /// than the packages themselves.
    linked: bool,
}

impl Environment {
    /// `builtin` is the directory that holds the builtin packages; it is looked
    /// up from `INTOLANG_PACKAGES` and from the `packages` directory above the
    /// project when it is not given.
    pub fn build(root: Option<&Path>, builtin: Option<&Path>) -> Option<Self> {
        if !package::is_real() {
            // Without a disk there is nothing to link; the packages stand
            // where they were put.
            return builtin.map(|dir| Self {
                dir: dir.to_path_buf(),
                linked: false,
            });
        }
        let number = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("intolang-analysis-{}-{number}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).ok()?;
        let environment = Self { dir, linked: true };
        let builtin = builtin
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os(PACKAGES_VARIABLE).map(PathBuf::from))
            .or_else(|| root.and_then(find_builtin));
        if let Some(builtin) = builtin {
            environment.link_packages(&builtin);
        }
        if let Some(root) = root {
            environment.link_packages(&root.join(".into/packages"));
        }
        Some(environment)
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    fn link_packages(&self, source: &Path) {
        let Ok(packages) = Package::children(source) else {
            return;
        };
        for package in packages {
            if let Some(name) = package.root.file_name() {
                let _ = moduler::link_directory(&package.root, &self.dir.join(name));
            }
            for (name, path) in &package.manifest.modules {
                let _ = moduler::link_directory(&package.join(path), &self.dir.join(name));
            }
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        if self.linked {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

fn find_builtin(root: &Path) -> Option<PathBuf> {
    root.ancestors()
        .map(|ancestor| ancestor.join(BUILTIN_PACKAGES))
        .find(|packages| Package::find_by_alias(packages, BUILTIN_ALIAS).is_ok_and(|p| p.is_some()))
}
