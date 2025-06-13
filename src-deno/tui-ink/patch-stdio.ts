import process from "node:process";
import { isMouseEvent } from "./mouse/parser.ts";

let hasPatchedStdin = false;
/**
 * # Fix Ctrl+Z issue
 *
 *  Pressing Ctrl+Z on Windows causes a read of zero characters which deno
 * `node:process` compatibility layer mishandles and internally closes its file
 * handle. This issue occurs whenever either `process.stdin.read()` or
 * `process.stdin.on('data', ...)` is used.
 *
 * # Hide mouse events from `useInput`
 *
 * Hide mouse events from `stdin.read()` but show them on `stdin.on('data',
 * chunk => {})`. This way the events won't be visible to `ink` but will still
 * be read by `ink-mouse`.
 */
export function patchStdinObject() {
  if (hasPatchedStdin) return;
  hasPatchedStdin = true;

  const readChunks: string[] = [];

  type StdinListener = (data: string) => void;
  const callbacks: StdinListener[] = [];
  let state: "started" | "stopped" | "stopping" = "stopped";

  const maxbuffer = new Uint8Array(1024);
  const textDecoder = new TextDecoder();
  const backgroundRead = async () => {
    state = "started";
    try {
      while (state === "started") {
        const size = await Deno.stdin.read(maxbuffer);
        let data: string | null = null;
        if (size) {
          const view = maxbuffer.subarray(0, size ?? 0);
          data = textDecoder.decode(view);
          readChunks.push(data);
        }

        try {
          if (data !== null) {
            for (const callback of callbacks.slice()) {
              callback(data);
            }
          }
        } catch {
          // ignore callback errors
        }
      }
    } finally {
      state = "stopped";
    }
  };

  const addListener = (listener: StdinListener) => {
    callbacks.push(listener);
    if (state === "stopped") {
      backgroundRead();
    }
    state = "started";
  };
  const removeListener = (listener: StdinListener) => {
    let ix: number;
    while ((ix = callbacks.indexOf(listener)) > 0) {
      callbacks.splice(ix, 1);
    }
    if (callbacks.length === 0) {
      if (state !== "stopped") {
        state = "stopping";
      }
    }
  };

  function patchMethod(
    name: keyof typeof process.stdin,
    // deno-lint-ignore no-explicit-any
    newMethod: (original: (...args: any[]) => any) => (...args: any[]) => any,
  ) {
    const original = process.stdin[name];
    // deno-lint-ignore no-explicit-any
    (process.stdin as any)[name] = newMethod(original as any);
  }

  patchMethod("on", (rawOn) =>
    function on(...args) {
      if (args[0] !== "data") {
        rawOn(...args);
      } else {
        addListener(args[1]);
      }
    });
  patchMethod("off", (rawOff) =>
    function off(...args) {
      if (args[0] !== "data") {
        rawOff(...args);
      } else {
        removeListener(args[1]);
      }
    });

  patchMethod(
    "addListener",
    (rawAddListener) =>
      function patchedAddListener(...args) {
        if (args[0] === "readable") {
          addListener(args[1]);
        } else {
          return rawAddListener(...args);
        }
      },
  );
  patchMethod(
    "removeListener",
    (rawRemoveListener) =>
      function patchedRemoveListener(...args) {
        if (args[0] === "readable") {
          removeListener(args[1]);
        } else {
          return rawRemoveListener(...args);
        }
      },
  );

  // Don't forward mouse events to ink. Instead send them only to ink-mouse (by not returning them from `stdin.read()`).
  patchMethod("read", (_rawRead) =>
    function patchedRead() {
      while (true) {
        const chunk = readChunks.shift();
        if (!chunk) return null;
        if (isMouseEvent(chunk)) continue;
        return chunk;
      }
    });
  patchMethod(
    "setRawMode",
    (_rawSetRawMode) =>
      function patchedSetRawMode(value: boolean) {
        if (value) {
          Deno.stdin.setRaw(true, { cbreak: Deno.build.os !== "windows" });
        } else {
          Deno.stdin.setRaw(false);
        }
      },
  );
}

let hasPatchedStdout = false;
/** Patch stdout object so that writes aren't partially completed when they are
 * too long. This caused issues where large terminal sizes weren't completely
 * drawn. */
export function patchStdoutObject() {
  if (hasPatchedStdout) return;
  hasPatchedStdout = true;

  const encoder = new TextEncoder();
  Object.defineProperty(process.stdout, "write", {
    get() {
      return function write(data: string | Uint8Array) {
        const buffer: Uint8Array = typeof data === "string"
          ? encoder.encode(data)
          : data;
        let n = 0;
        while (n < buffer.length) {
          n += Deno.stdout.writeSync(buffer.slice(n));
        }
      };
    },
  });
}
