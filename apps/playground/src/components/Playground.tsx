"use client";

import { useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { useRunner } from "@/hooks/useRunner";
import { sampleProgram } from "@/lib/sample";
import ProgramEditor from "./ProgramEditor";

export default function Playground() {
  const [stdin, setStdin] = useState("");
  const readSource = useRef<() => string>(() => "");
  const { running, output, failure, run } = useRunner();

  return (
    <div className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b px-4 py-2">
        <h1 className="text-sm font-medium">Into Playground</h1>
        <Button onClick={() => void run(readSource.current(), stdin)} disabled={running}>
          {running && <Spinner data-icon="inline-start" />}
          {running ? "Running" : "Run"}
        </Button>
      </header>
      <main className="grid min-h-0 flex-1 grid-rows-2 md:grid-cols-2 md:grid-rows-1">
        <section className="min-h-0">
          <ProgramEditor
            defaultValue={sampleProgram()}
            onReady={(read) => {
              readSource.current = read;
            }}
          />
        </section>
        <section className="flex min-h-0 flex-col gap-2 overflow-auto p-3">
          <label className="flex flex-col gap-1 text-sm">
            Input
            <textarea
              className="min-h-24 resize-y rounded-md border bg-transparent p-2 font-mono text-sm"
              value={stdin}
              onChange={(event) => setStdin(event.target.value)}
              spellCheck={false}
            />
          </label>
          <label className="flex flex-1 flex-col gap-1 text-sm">
            Output
            <pre className="min-h-24 flex-1 rounded-md border p-2 font-mono text-sm whitespace-pre-wrap">{output}</pre>
          </label>
          {failure !== null && (
            <pre className="rounded-md border border-destructive p-2 font-mono text-sm whitespace-pre-wrap text-destructive">
              {failure}
            </pre>
          )}
        </section>
      </main>
    </div>
  );
}
