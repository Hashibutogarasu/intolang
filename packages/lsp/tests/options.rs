use lsp::Options;
use lsp_types::InitializeParams;
use serde_json::json;

fn parse(value: serde_json::Value) -> InitializeParams {
    serde_json::from_value(value).unwrap()
}

#[test]
fn the_options_come_from_the_initialization_options() {
    let options = Options::from_initialize(&parse(json!({
        "capabilities": {},
        "workspaceFolders": [{"uri": "file:///project", "name": "project"}],
        "initializationOptions": {"analyzerPath": "/bin/analyzer", "packagesDir": "/packages"},
    })));
    assert_eq!(options.root_uri.as_deref(), Some("file:///project"));
    assert_eq!(options.analyzer, "/bin/analyzer");
    assert_eq!(options.packages_dir.as_deref(), Some("/packages"));
}

#[test]
fn the_root_uri_is_used_without_workspace_folders() {
    let options = Options::from_initialize(&parse(json!({
        "capabilities": {},
        "rootUri": "file:///other",
    })));
    assert_eq!(options.root_uri.as_deref(), Some("file:///other"));
    assert_eq!(options.packages_dir, None);
}
