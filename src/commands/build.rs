use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Package {
    name: String,
    #[serde(default)]
    alias: Option<String>,
    #[serde(rename = "builtConfig", default)]
    built_config: BuildConfig,
    runs: String,
}

#[derive(Deserialize, Default)]
struct BuildConfig {
    #[serde(default = "default_dist")]
    dist: String,
}

fn default_dist() -> String { "./dist".into() }

pub(super) fn run(package_path: PathBuf, test_mode: bool) -> Result<()> {
    let manifest = if package_path.is_dir() {
        package_path.join("into-package.json")
    } else {
        package_path
    };
    let manifest = std::fs::canonicalize(&manifest)
        .with_context(|| format!("Could not find package manifest: {}", manifest.display()))?;
    let package_dir = manifest.parent().context("Manifest has no parent directory")?;
    let package: Package = serde_json::from_slice(&std::fs::read(&manifest)?)
        .with_context(|| format!("Invalid package manifest: {}", manifest.display()))?;
    let embedded_packages = compiler::builtin_packages_dir();
    if test_mode {
        return run_tests(package_dir, &embedded_packages);
    }

    let entry = package_dir.join(&package.runs);
    let source = std::fs::read_to_string(&entry)
        .with_context(|| format!("Could not read package entry point: {}", entry.display()))?;
    let output_dir = package_dir.join(&package.built_config.dist)
        .join(std::env::consts::OS).join(std::env::consts::ARCH);
    let executable = package.alias.as_deref().unwrap_or(&package.name).replace(' ', "-");
    #[cfg(windows)]
    let executable = format!("{executable}.exe");
    let output = output_dir.join(executable);
    if output.exists() { std::fs::remove_file(&output)?; }

    let message = backend::lower_file(&source, &entry, Some(&embedded_packages))
        .context("Could not compile Into entry point")?;
    std::fs::create_dir_all(&output_dir)?;
    compiler::compile(&message, &output).context("Compilation failed")?;
    println!("Built {}", output.display());
    Ok(())
}

fn run_tests(package_dir: &Path, embedded_packages: &Path) -> Result<()> {
    let test_root = package_dir.join("src/tests");
    let mut test_files = Vec::new();
    collect_into_files(&test_root, &mut test_files)?;
    if test_files.is_empty() { anyhow::bail!("No Into test files found under {}", test_root.display()); }
    let mut passed = 0;
    let mut failed = 0;
    for file in test_files {
        let source = std::fs::read_to_string(&file)?;
        let result = backend::run_tests(&source, &file, embedded_packages)
            .with_context(|| format!("Failed running tests in {}", file.display()))?;
        passed += result.passed;
        failed += result.failed;
    }
    println!("test result: {}. {passed} passed; {failed} failed", if failed == 0 { "ok" } else { "FAILED" });
    if failed > 0 { anyhow::bail!("{failed} Into test(s) failed"); }
    Ok(())
}

fn collect_into_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() { return Ok(()); }
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_into_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "into") {
            files.push(path);
        }
    }
    files.sort();
    Ok(())
}
