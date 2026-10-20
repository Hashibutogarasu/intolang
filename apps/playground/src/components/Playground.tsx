"use client";

import { useRef, useState } from "react";

import { useRunner } from "@/hooks/useRunner";
import { sampleProgram } from "@/lib/sample";
import ProgramEditor from "./ProgramEditor";

export default function Playground() {
  const [stdin, setStdin] = useState("");
  const readSource = useRef<() => string>(() => "");
  const { running, output, failure, run } = useRunner();

  return (
    <div className="playground">
      <header className="bar">
        <h1>Into Playground</h1>
        <button type="button" onClick={() => void run(readSource.current(), stdin)} disabled={running}>
          {running ? "Running…" : "Run"}
        </button>
      </header>
      <main className="panes">
        <section>
          <ProgramEditor
            defaultValue={sampleProgram()}
            onReady={(read) => {
              readSource.current = read;
            }}
          />
        </section>
        <section className="side">
          <label>
            Input
            <textarea value={stdin} onChange={(event) => setStdin(event.target.value)} spellCheck={false} />
          </label>
          <label>
            Output
            <pre className="output">{output}</pre>
          </label>
          {failure !== null && <pre className="failure">{failure}</pre>}
        </section>
      </main>
    </div>
  );
}
