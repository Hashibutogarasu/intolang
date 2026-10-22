use errors::Result;
use moduler::Project;
use progress::Progress;
use std::fmt;
use std::path::Path;

/// The kind of work a [`PackageTask`] does.
#[derive(Clone, Copy)]
pub(super) enum Operation {
    Install,
    Uninstall,
}

/// Where a package or a whole task stands. Every message the install and
/// uninstall commands print is a status, so none of them is spelled twice.
#[derive(Clone, Copy)]
pub(super) enum PackageStatus {
    /// The task has started.
    Begin(Operation),
    Installing,
    Restoring,
    Installed,
    UpToDate,
    Removing,
    Removed,
    /// Every package of the task has been handled.
    Complete(Operation),
}

impl fmt::Display for PackageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Begin(Operation::Install) => "Starting the install",
            Self::Begin(Operation::Uninstall) => "Starting the uninstall",
            Self::Installing => "Installing",
            Self::Restoring => "Restoring",
            Self::Installed => "Installed",
            Self::UpToDate => "Up to date",
            Self::Removing => "Removing",
            Self::Removed => "Removed",
            Self::Complete(Operation::Install) => "Install complete",
            Self::Complete(Operation::Uninstall) => "Uninstall complete",
        };
        f.write_str(text)
    }
}

/// A status together with the package it is about.
pub(super) struct Outcome {
    pub status: PackageStatus,
    pub subject: String,
}

impl Outcome {
    pub fn new(status: PackageStatus, subject: impl Into<String>) -> Self {
        Self {
            status,
            subject: subject.into(),
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status, self.subject)
    }
}

/// A command that installs or removes the packages of the current project.
///
/// Both commands share one flow, [`PackageTask::run`]: announce the start,
/// detect the packages, handle each of them while printing its outcome, then
/// announce the end.
pub(super) trait PackageTask {
    type Item;
    fn operation(&self) -> Operation;
    fn detect(&self, project: &Project, embedded: &Path) -> Result<Vec<Self::Item>>;
    fn label(&self, item: &Self::Item) -> Outcome;
    fn handle(&self, project: &Project, embedded: &Path, item: Self::Item) -> Result<Outcome>;

    fn run(&self, project: &Project, embedded: &Path) -> Result<()> {
        let operation = self.operation();
        let progress = Progress::start(&PackageStatus::Begin(operation).to_string(), 1);
        let items = self.detect(project, embedded)?;
        progress.steps(items.len() as u64);
        for item in items {
            progress.step(&self.label(&item).to_string());
            let outcome = self.handle(project, embedded, item)?;
            progress.println(&outcome.to_string());
        }
        progress.finish(&PackageStatus::Complete(operation).to_string());
        Ok(())
    }
}
