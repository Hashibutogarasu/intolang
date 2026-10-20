type Call =
  | { kind: "run"; payload: { source: string; stdin: string } }
  | { kind: "handle" | "notify" | "readBuiltin"; payload: string };

type Pending = { resolve: (result: string | null) => void; reject: (error: Error) => void };

/** The wasm module in a worker of its own. */
export class Engine {
  private readonly worker = new Worker("/into-worker.js", { type: "module" });
  private readonly pending = new Map<number, Pending>();
  private next = 0;

  constructor() {
    this.worker.onmessage = ({ data }: MessageEvent<{ id: number; result?: string | null; error?: string }>) => {
      const call = this.pending.get(data.id);
      this.pending.delete(data.id);
      if (data.error !== undefined) {
        call?.reject(new Error(data.error));
      } else {
        call?.resolve(data.result ?? null);
      }
    };
    this.worker.onerror = (event) => this.fail(new Error(event.message || "the worker failed"));
  }

  request(call: Call): Promise<string | null> {
    const id = this.next++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ id, ...call });
    });
  }

  /** Ends the worker; what is waiting for it fails. */
  terminate() {
    this.worker.terminate();
    this.fail(new Error("the worker was stopped"));
  }

  private fail(error: Error) {
    for (const call of this.pending.values()) {
      call.reject(error);
    }
    this.pending.clear();
  }
}

export type RunOutcome = { stdout: string; error: string | null };

export async function runProgram(engine: Engine, source: string, stdin: string): Promise<RunOutcome> {
  const answer = await engine.request({ kind: "run", payload: { source, stdin } });
  return JSON.parse(answer ?? "null") as RunOutcome;
}
