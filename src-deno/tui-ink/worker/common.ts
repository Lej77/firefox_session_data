import WasmContext, {
  ContextOptions as WasmContextOptions,
} from "../../wasi-snapshot-preview1.ts";

export type MessageFromWorker = {
  type: "command-success";
  result: WasmCommandResultSimple;
} | {
  type: "command-failed";
  error: string;
} | {
  type: "stderr-line";
  line: string;
};
export type MessageToWorker = {
  type: "wasm-init";
  context: WasmContextOptions;
  wasmData: Uint8Array;
} | {
  type: "run-command";
  options: Omit<WasmCommandRunOptions, "onStderrLine">;
  sendStderrLines: boolean;
};
export function messageToWorker(message: MessageToWorker): MessageToWorker {
  return message;
}
export function messageFromWorker(
  message: MessageFromWorker,
): MessageFromWorker {
  return message;
}

export type WasmCommandRunOptions = {
  /** Overrides the arguments in the {@link WasmCommandOptions.wasmContextOptions} */
  args: string[];
  stdin?: Uint8Array | string;
  onStderrLine?: (line: string) => void;
};
export type WasmCommandOptions = {
  wasmContextOptions: WasmContextOptions;
  wasmModule: WebAssembly.Module;
} & WasmCommandRunOptions;
export type WasmCommandResult = Awaited<ReturnType<typeof runWasmCommand>>;
export type WasmCommandResultSimple = Omit<
  WasmCommandResult,
  "stdoutString" | "stderrString"
>;

export async function runWasmCommand(options: WasmCommandOptions) {
  const { wasmContextOptions, wasmModule, args, stdin: stdinOption } = options;
  const decoder = new TextDecoder();
  let stderrLen = 0;
  let stderr = new Uint8Array(4048);
  let stdoutLen = 0;
  let stdout = new Uint8Array(4048);
  let stdin: Uint8Array;
  if (typeof stdinOption === "string") {
    stdin = new TextEncoder().encode(stdinOption);
  } else if (stdinOption) {
    stdin = stdinOption;
  } else {
    stdin = new Uint8Array(0);
  }

  const context = new WasmContext({
    ...wasmContextOptions,
    args: [(wasmContextOptions.args ?? ["program"])[0], ...args],
    exitOnReturn: false,
    stderr: {
      writeSync(buf) {
        if (stderrLen + buf.length > stderr.length) {
          const larger = new Uint8Array(
            Math.max(stderr.length * 2, stderrLen + buf.length),
          );
          larger.set(stderr.slice(0, stderrLen));
          stderr = larger;
        }
        stderr.set(buf, stderrLen);
        stderrLen += buf.length;
        if (options.onStderrLine) {
          while (true) {
            const lineIx = stderr.slice(0, stderrLen).indexOf(
              "\n".charCodeAt(0),
            );
            if (lineIx < 0) break;
            const lineArr = stderr.slice(0, lineIx);
            options.onStderrLine(new TextDecoder().decode(lineArr));
            stderr.copyWithin(0, lineIx + 1, stderrLen);
            stderrLen -= lineIx + 1;
          }
        }

        return buf.length;
      },
    },
    stdout: {
      writeSync(buf) {
        if (stdoutLen + buf.length > stdout.length) {
          const larger = new Uint8Array(
            Math.max(stdout.length * 2, stdoutLen + buf.length),
          );
          larger.set(stdout.slice(0, stdoutLen));
          stdout = larger;
        }
        stdout.set(buf, stdoutLen);
        stdoutLen += buf.length;
        return buf.length;
      },
    },
    stdin: {
      readSync(buf) {
        if (stdin.length === 0) return null;
        const source = stdin.slice(0, Math.min(buf.length, stdin.length));
        buf.set(source);
        stdin = stdin.slice(source.length);
        return source.length;
      },
    },
  });
  const instance = await WebAssembly.instantiate(wasmModule, {
    "wasi_snapshot_preview1": context.exports,
  });
  const exitCode = context.start(instance) ?? 0;

  if (stdout.length !== stdoutLen) {
    stdout = new Uint8Array(stdout.slice(0, stdoutLen));
  }
  if (stderr.length !== stderrLen) {
    stderr = new Uint8Array(stderr.slice(0, stderrLen));
  }
  return {
    stdout: stdout,
    get stdoutString() {
      return decoder.decode(stdout);
    },
    stderr,
    get stderrString() {
      return decoder.decode(stderr);
    },
    exitCode,
  };
}

export function withStdioStringMethods(
  data: WasmCommandResultSimple,
): WasmCommandResult {
  const decoder = new TextDecoder();
  return {
    ...data,
    get stdoutString() {
      return decoder.decode(this.stdout);
    },
    get stderrString() {
      return decoder.decode(this.stderr);
    },
  };
}
