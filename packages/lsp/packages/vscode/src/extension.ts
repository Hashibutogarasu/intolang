import { ExtensionContext, workspace } from "vscode";
import {
  Executable,
  LanguageClient,
  LanguageClientOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
  const settings = workspace.getConfiguration("into");
  const server: Executable = {
    command: settings.get<string>("lsp.path") || "into-lsp",
    transport: TransportKind.stdio,
  };
  const options: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "into" }],
    initializationOptions: {
      analyzerPath: settings.get<string>("analyzer.path") || undefined,
      packagesDir: settings.get<string>("packagesDir") || undefined,
    },
  };
  client = new LanguageClient("into", "Into Language Server", server, options);
  context.subscriptions.push({ dispose: () => void client?.stop() });
  void client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
