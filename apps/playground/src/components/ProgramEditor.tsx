import Editor, { type OnMount } from "@monaco-editor/react";
import type * as monaco from "monaco-editor";
import { useEffect, useRef, useState } from "react";

import { useDarkScheme } from "@/hooks/useColorScheme";
import { Engine } from "@/lib/engine";
import { DOCUMENT_URI, LANGUAGE, followDocument, registerLanguage } from "@/lib/language";
import { configureMonaco } from "@/lib/monaco";

type Props = {
  defaultValue: string;
  /** Receives a function that reads the program as it is now. */
  onReady: (readSource: () => string) => void;
};

/** The editor of one program, with the language service of Into behind it. */
export default function ProgramEditor({ defaultValue, onReady }: Props) {
  useState(configureMonaco);
  const dark = useDarkScheme();
  const cleanup = useRef<(() => void) | null>(null);

  useEffect(() => () => cleanup.current?.(), []);

  const mount: OnMount = (editor, monacoApi) => {
    onReady(() => editor.getValue());
    const model = editor.getModel();
    if (!model) {
      return;
    }
    const engine = new Engine();
    const disposables: monaco.IDisposable[] = [];
    let disposed = false;
    void registerLanguage(monacoApi, engine).then((registration) => {
      if (disposed) {
        registration.dispose();
        return;
      }
      disposables.push(registration);
      monacoApi.editor.setModelLanguage(model, LANGUAGE);
      disposables.push(followDocument(monacoApi, engine, model));
    });
    cleanup.current = () => {
      disposed = true;
      disposables.forEach((disposable) => disposable.dispose());
      engine.terminate();
    };
  };

  return (
    <Editor
      path={DOCUMENT_URI}
      defaultLanguage="plaintext"
      defaultValue={defaultValue}
      theme={dark ? "vs-dark" : "light"}
      onMount={mount}
      options={{
        minimap: { enabled: false },
        fontSize: 14,
        tabSize: 4,
        automaticLayout: true,
        "semanticHighlighting.enabled": true,
      }}
    />
  );
}
