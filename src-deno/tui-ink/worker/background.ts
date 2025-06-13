// Links about Web Workers in Deno:
// https://docs.deno.com/examples/web_workers/
// https://stackoverflow.com/questions/71736369/deno-on-multi-core-machines
// https://medium.com/deno-the-complete-reference/communicate-with-workers-in-deno-5ca5381e5979

// Correct auto completion:
/// <reference no-default-lib="true"/>
/// <reference lib="deno.worker" />

import {
  messageFromWorker,
  MessageToWorker,
  runWasmCommand,
  WasmCommandOptions,
} from "./common.ts";

let options:
  | Pick<WasmCommandOptions, "wasmContextOptions" | "wasmModule">
  | null = null;

let prev: Promise<void> | null;

self.onmessage = (e: MessageEvent<MessageToWorker>) => {
  switch (e.data.type) {
    case "wasm-init":
      {
        const data = e.data;
        prev = (async () => {
          options = {
            wasmContextOptions: data.context,
            wasmModule: await WebAssembly.compile(data.wasmData),
          };
        })();
      }
      break;
    case "run-command":
      {
        const actualPrev = prev;
        const data = e.data;
        prev = (async () => {
          await actualPrev;
          if (!options) {
            self.postMessage(messageFromWorker({
              type: "command-failed",
              error: "Must initiate worker before running a command",
            }));
            return;
          }
          try {
            const result = await runWasmCommand({
              ...data.options,
              onStderrLine: data.sendStderrLines
                ? (line) => {
                  self.postMessage(messageFromWorker({
                    type: "stderr-line",
                    line,
                  }));
                }
                : undefined,
              ...options,
            });
            self.postMessage(
              messageFromWorker({
                type: "command-success",
                result,
              }),
              // Transfer ownership so we don't have two copies of the data:
              [result.stderr.buffer, result.stdout.buffer],
            );
          } catch (error) {
            self.postMessage(
              messageFromWorker({
                type: "command-failed",
                error: String(error),
              }),
            );
          }
        })();
      }
      break;
    default: {
      const _exhaustive: never = e.data;
    }
  }
};
