import { loader } from "@monaco-editor/react";
import * as monaco from "monaco-editor";

/** Makes the editor load the Monaco of this bundle, with its worker, rather than a copy from a CDN. */
export function configureMonaco() {
  loader.config({ monaco });
  window.MonacoEnvironment = {
    getWorker: () =>
      new Worker(new URL("monaco-editor/editor/editor.worker.js", import.meta.url), { type: "module" }),
  };
}
