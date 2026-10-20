use errors::Result;

use super::evaluation::Evaluation;
use super::package::Package;
use super::workspace::Workspace;

/// Evaluate Into source with the builtin `std` package available, so
/// `import "intolang://builtin/std";` provides `print` and `println`.
pub fn eval(source: &str) -> Result<Evaluation> {
    eval_with(source, &[])
}

/// Evaluate Into source with extra packages available for `import`. A package
/// aliased `builtin` replaces the default builtin `std` package.
pub fn eval_with(source: &str, packages: &[Package]) -> Result<Evaluation> {
    let mut all = packages.to_vec();
    if !all
        .iter()
        .any(|package| package.alias == Package::BUILTIN_ALIAS)
    {
        all.push(Package::builtin_std());
    }
    let workspace = Workspace::with_packages(&all)?;
    let message = backend::lower_with_packages(source, Some(workspace.path()))?;
    Ok(Evaluation { message })
}
