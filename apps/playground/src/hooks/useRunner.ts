import { useCallback, useEffect, useRef, useState } from "react";

import { Engine, runProgram } from "@/lib/engine";

/**
 * Runs programs in a worker of their own, and ends the worker of a program
 * that has not finished after `timeLimitMs`.
 */
export function useRunner(timeLimitMs = 10_000) {
  const [running, setRunning] = useState(false);
  const [output, setOutput] = useState("");
  const [failure, setFailure] = useState<string | null>(null);
  const engine = useRef<Engine | null>(null);

  useEffect(() => () => engine.current?.terminate(), []);

  const run = useCallback(
    async (source: string, stdin: string) => {
      const current = (engine.current ??= new Engine());
      setRunning(true);
      setOutput("");
      setFailure(null);
      let timer: ReturnType<typeof setTimeout> | undefined;
      try {
        const stopped = new Promise<never>((_, reject) => {
          timer = setTimeout(() => {
            current.terminate();
            engine.current = null;
            reject(new Error(`Stopped after ${timeLimitMs / 1000} seconds.`));
          }, timeLimitMs);
        });
        const outcome = await Promise.race([runProgram(current, source, stdin), stopped]);
        setOutput(outcome.stdout);
        setFailure(outcome.error);
      } catch (error) {
        setFailure(error instanceof Error ? error.message : String(error));
      } finally {
        clearTimeout(timer);
        setRunning(false);
      }
    },
    [timeLimitMs],
  );

  return { running, output, failure, run };
}
