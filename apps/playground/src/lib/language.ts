import type * as Monaco from "monaco-editor";

import type { Engine } from "./engine";
import { loadHighlighter } from "./highlight";

export const LANGUAGE = "into";
export const DOCUMENT_URI = "file:///playground/main.into";

type Position = { line: number; column: number };
type Range = { start: Position; end: Position };
type Location = { uri: string; range: Range };

type CompletionItem = {
  label: string;
  kind: string;
  detail: string | null;
  documentation: { markdown: string } | null;
  range: Range;
  text: string;
  snippet: boolean;
};

type Reply<T> = { ok: { type: string; value: T } } | { error: { code: number; message: string } };

/** The text of line `lineNumber` of the model, or "" past its end. */
function lineOf(model: Monaco.editor.ITextModel, lineNumber: number): string {
  return lineNumber >= 1 && lineNumber <= model.getLineCount() ? model.getLineContent(lineNumber) : "";
}

/** Monaco counts UTF-16 units from 1; the analyzer counts characters from 0. */
function toAnalyzer(model: Monaco.editor.ITextModel, position: Monaco.Position): Position {
  const before = lineOf(model, position.lineNumber).slice(0, position.column - 1);
  return { line: position.lineNumber - 1, column: Array.from(before).length };
}

function toMonaco(model: Monaco.editor.ITextModel, position: Position): Monaco.IPosition {
  const characters = Array.from(lineOf(model, position.line + 1));
  return {
    lineNumber: position.line + 1,
    column: characters.slice(0, position.column).join("").length + 1,
  };
}

function toMonacoRange(model: Monaco.editor.ITextModel, range: Range): Monaco.IRange {
  const start = toMonaco(model, range.start);
  const end = toMonaco(model, range.end);
  return {
    startLineNumber: start.lineNumber,
    startColumn: start.column,
    endLineNumber: end.lineNumber,
    endColumn: end.column,
  };
}

/**
 * Teaches Monaco the language. The colours come from the tree-sitter grammar
 * and the highlight queries the Zed extension uses, so the two cannot drift
 * apart.
 */
export async function registerLanguage(monaco: typeof Monaco, engine: Engine): Promise<Monaco.IDisposable> {
  const highlighter = await loadHighlighter();
  const disposables: Monaco.IDisposable[] = [];
  monaco.languages.register({ id: LANGUAGE, extensions: [".into"] });
  disposables.push(
    monaco.languages.setLanguageConfiguration(LANGUAGE, {
      comments: { lineComment: "//" },
      brackets: [["{", "}"], ["[", "]"], ["(", ")"]],
    }),
    monaco.languages.registerDocumentSemanticTokensProvider(LANGUAGE, {
      getLegend: () => ({ tokenTypes: highlighter.legend, tokenModifiers: [] }),
      provideDocumentSemanticTokens: (model) => ({ data: highlighter.tokens(model.getValue()) }),
      releaseDocumentSemanticTokens: () => undefined,
    }),
  );

  /** The model a location names, made from the standard package when it is one of its files. */
  const modelOf = async (uri: string): Promise<Monaco.editor.ITextModel | null> => {
    const monacoUri = monaco.Uri.parse(uri);
    const existing = monaco.editor.getModel(monacoUri);
    if (existing) {
      return existing;
    }
    const text = await engine.request({ kind: "readBuiltin", payload: decodeURIComponent(new URL(uri).pathname) });
    return text === null ? null : monaco.editor.createModel(text, LANGUAGE, monacoUri);
  };

  const ask = async <T>(method: string, model: Monaco.editor.ITextModel, position?: Monaco.Position) => {
    const answer = await engine.request({
      kind: "handle",
      payload: JSON.stringify({
        method,
        params: {
          uri: DOCUMENT_URI,
          position: position ? toAnalyzer(model, position) : { line: 0, column: 0 },
        },
      }),
    });
    const reply = JSON.parse(answer ?? "null") as Reply<T>;
    return "ok" in reply ? reply.ok.value : null;
  };

  const isProgram = (model: Monaco.editor.ITextModel) => model.uri.toString() === monaco.Uri.parse(DOCUMENT_URI).toString();

  disposables.push(
    monaco.languages.registerHoverProvider(LANGUAGE, {
      async provideHover(model, position) {
        if (!isProgram(model)) {
          return null;
        }
        const hover = await ask<{ signature: string; documentation: { markdown: string } | null; range: Range } | null>(
          "analysis/hover",
          model,
          position,
        );
        if (!hover) {
          return null;
        }
        const contents = [{ value: "```into\n" + hover.signature + "\n```" }];
        if (hover.documentation) {
          contents.push({ value: hover.documentation.markdown });
        }
        return { contents, range: toMonacoRange(model, hover.range) };
      },
    }),
    monaco.languages.registerDefinitionProvider(LANGUAGE, {
      async provideDefinition(model, position) {
        if (!isProgram(model)) {
          return null;
        }
        const found = (await ask<Location[]>("analysis/definition", model, position)) ?? [];
        const locations: Monaco.languages.Location[] = [];
        for (const location of found) {
          const target = await modelOf(location.uri);
          if (target) {
            locations.push({ uri: target.uri, range: toMonacoRange(target, location.range) });
          }
        }
        return locations;
      },
    }),
    monaco.languages.registerDocumentSymbolProvider(LANGUAGE, {
      async provideDocumentSymbols(model) {
        if (!isProgram(model)) {
          return [];
        }
        const symbols =
          (await ask<{ name: string; kind: string; container: string | null; range: Range; name_range: Range }[]>(
            "analysis/documentSymbol",
            model,
          )) ?? [];
        const kinds = monaco.languages.SymbolKind;
        const kindOf: Record<string, Monaco.languages.SymbolKind> = {
          function: kinds.Function,
          class: kinds.Class,
          field: kinds.Field,
          enum: kinds.Enum,
          variant: kinds.EnumMember,
          event: kinds.Event,
          macro: kinds.Function,
          variable: kinds.Variable,
        };
        return symbols.map((symbol) => ({
          name: symbol.name,
          detail: symbol.container ?? "",
          kind: kindOf[symbol.kind] ?? kinds.Variable,
          tags: [],
          range: toMonacoRange(model, symbol.range),
          selectionRange: toMonacoRange(model, symbol.name_range),
        }));
      },
    }),
    monaco.languages.registerDocumentFormattingEditProvider(LANGUAGE, {
      async provideDocumentFormattingEdits(model) {
        if (!isProgram(model)) {
          return [];
        }
        const edits = (await ask<{ range: Range; text: string }[]>("analysis/format", model)) ?? [];
        return edits.map((edit) => ({ range: toMonacoRange(model, edit.range), text: edit.text }));
      },
    }),
    monaco.languages.registerCompletionItemProvider(LANGUAGE, {
      triggerCharacters: [".", "/", ":", '"'],
      async provideCompletionItems(model, position) {
        if (!isProgram(model)) {
          return { suggestions: [] };
        }
        const items = (await ask<CompletionItem[]>("analysis/completion", model, position)) ?? [];
        const kinds = monaco.languages.CompletionItemKind;
        const kindOf: Record<string, Monaco.languages.CompletionItemKind> = {
          keyword: kinds.Keyword,
          function: kinds.Function,
          class: kinds.Class,
          field: kinds.Field,
          enum: kinds.Enum,
          variant: kinds.EnumMember,
          event: kinds.Event,
          macro: kinds.Function,
          variable: kinds.Variable,
          module: kinds.Module,
          folder: kinds.Folder,
          snippet: kinds.Snippet,
        };
        return {
          suggestions: items.map((item) => ({
            label: item.label,
            kind: kindOf[item.kind] ?? kinds.Text,
            detail: item.detail ?? undefined,
            documentation: item.documentation ? { value: item.documentation.markdown } : undefined,
            insertText: item.text,
            insertTextRules: item.snippet ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
            range: toMonacoRange(model, item.range),
            command: item.text.endsWith("/") ? { id: "editor.action.triggerSuggest", title: "Suggest" } : undefined,
          })),
        };
      },
    }),
  );

  return { dispose: () => disposables.forEach((disposable) => disposable.dispose()) };
}

type Diagnostic = {
  range: Range;
  severity: "error" | "warning" | "information" | "hint";
  message: string;
  code: number | null;
  unnecessary?: boolean;
};

function markerSeverity(monaco: typeof Monaco, severity: Diagnostic["severity"]): Monaco.MarkerSeverity {
  switch (severity) {
    case "error":
      return monaco.MarkerSeverity.Error;
    case "warning":
      return monaco.MarkerSeverity.Warning;
    case "information":
      return monaco.MarkerSeverity.Info;
    case "hint":
      return monaco.MarkerSeverity.Hint;
  }
}

/**
 * Keeps the analyzer's copy of the program and the red lines of the editor in
 * step with the model.
 */
export function followDocument(monaco: typeof Monaco, engine: Engine, model: Monaco.editor.ITextModel): Monaco.IDisposable {
  let version = 0;
  let opened = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const send = async () => {
    version += 1;
    const answer = await engine.request({
      kind: "notify",
      payload: JSON.stringify({
        method: opened ? "document/change" : "document/open",
        params: { uri: DOCUMENT_URI, version, text: model.getValue() },
      }),
    });
    opened = true;
    const caused = JSON.parse(answer ?? "[]") as { method: string; params: { diagnostics: Diagnostic[] } }[];
    const diagnostics = caused.find((notification) => notification.method === "analysis/diagnostics")?.params.diagnostics ?? [];
    monaco.editor.setModelMarkers(
      model,
      "into",
      diagnostics.map((diagnostic) => ({
        ...toMonacoRange(model, diagnostic.range),
        severity: markerSeverity(monaco, diagnostic.severity),
        tags: diagnostic.unnecessary ? [monaco.MarkerTag.Unnecessary] : undefined,
        message: diagnostic.message,
        code: diagnostic.code === null ? undefined : String(diagnostic.code),
      })),
    );
  };

  const schedule = () => {
    clearTimeout(timer);
    timer = setTimeout(() => void send().catch(() => undefined), 150);
  };
  schedule();
  const subscription = model.onDidChangeContent(schedule);
  return {
    dispose: () => {
      clearTimeout(timer);
      subscription.dispose();
    },
  };
}
