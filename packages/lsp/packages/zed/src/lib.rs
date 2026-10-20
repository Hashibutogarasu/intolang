use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, Command, LanguageServerId, Result, Worktree, serde_json};

const SERVER: &str = "into-lsp";

struct IntoExtension;

impl zed::Extension for IntoExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let settings = LspSettings::for_worktree(id.as_ref(), worktree).ok();
        let configured = settings
            .and_then(|settings| settings.binary)
            .and_then(|binary| binary.path);
        let command = configured
            .map(|path| {
                if path.contains('/') && !path.starts_with('/') {
                    format!("{}/{path}", worktree.root_path())
                } else {
                    path
                }
            })
            .or_else(|| worktree.which(SERVER))
            .ok_or_else(|| format!("{SERVER} was not found on the PATH"))?;
        Ok(Command {
            command,
            args: Vec::new(),
            env: worktree.shell_env(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<serde_json::Value>> {
        Ok(LspSettings::for_worktree(id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.initialization_options))
    }
}

zed::register_extension!(IntoExtension);
