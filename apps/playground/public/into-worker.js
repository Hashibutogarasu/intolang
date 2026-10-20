// Runs the wasm module off the page, so that a program that does not end
// leaves the editor alone. The page ends the worker when it has waited enough.
import init, { handle, notify, readBuiltin, run } from "/wasm/wasm.js";

const ready = init();

const calls = {
  run: ({ source, stdin }) => run(source, stdin),
  handle: (request) => handle(request),
  notify: (notification) => notify(notification),
  readBuiltin: (path) => readBuiltin(path) ?? null,
};

self.onmessage = async ({ data: { id, kind, payload } }) => {
  try {
    await ready;
    self.postMessage({ id, result: calls[kind](payload) });
  } catch (error) {
    self.postMessage({ id, error: String(error) });
  }
};
