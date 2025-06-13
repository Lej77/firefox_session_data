import {
    ContextOptions,
    default as Context,
} from "./wasi-snapshot-preview1.ts";

/** Keep in sync with GitHub releases. Should match the version of the most recent git tag. */
export const VERSION = "0.1.1";

export const GITHUB_RELEASE_COMPRESSED_WASM =
    `https://github.com/Lej77/firefox_session_data/releases/download/v${VERSION}/firefox-session-data-wasm32-wasip1.tar.gz`;
export const GITHUB_RELEASE_JSON_WITH_WASM =
    `https://github.com/Lej77/firefox_session_data/releases/download/v${VERSION}/firefox-session-data-wasm32-wasip1.json`;

/** Get the path of the firefox profile directory which will contain a different
 * directory for each Firefox profile. */
export function findFirefoxProfilesDirectory(env: { [index: string]: string }) {
    if (Deno.build.os === "windows") {
        if ("APPDATA" in env) {
            return `${env.APPDATA}\\Mozilla\\Firefox\\Profiles`;
        } else if ("USERNAME" in env) {
            return `C:\\Users\\${env.USERNAME}\\AppData\\Roaming\\Mozilla\\Firefox\\Profiles`;
        }
    }
    return null;
}

/** Setup Wasm environment */
export async function prepareWasiContextArguments(
    cliArgs: string[],
    options?: { preopenFirefoxProfilesDir?: boolean },
): Promise<ContextOptions> {
    let env: { [index: string]: string } = {};
    try {
        env = Deno.env.toObject();
    } catch (error) {
        console.error(
            `Failed to get environment variables so running without them: `,
            error,
        );
    }

    try {
        env["CD"] = Deno.cwd();
    } catch (error) {
        console.error(
            `Failed to get current directory so running as if in root directory: `,
            error,
        );
    }

    let preopens: { [key: string]: string } | undefined = undefined;
    if (options?.preopenFirefoxProfilesDir !== false) {
        // Make firefox's profile directory available:
        const firefoxProfileDir: string | null = findFirefoxProfilesDirectory(
            env,
        );
        if (firefoxProfileDir !== null) {
            const status = await Deno.permissions.request({
                name: "read",
                path: firefoxProfileDir,
            });
            if (status.state === "granted") {
                preopens = { [firefoxProfileDir]: firefoxProfileDir };
            }
        }
    }

    return {
        args: ["firefox-session-data", ...cliArgs],
        env,
        skipFsCheck: true,
        preopens,
    };
}

/** Download WebAssembly CLI code from GitHub. */
export async function fetchWasmBlob(
    url: string = GITHUB_RELEASE_COMPRESSED_WASM,
): Promise<Uint8Array> {
    const { UntarStream } = await import("jsr:@std/tar@^0.1.6/untar-stream");

    const response = await fetch(url);
    if (response.body === null) {
        throw new Error(`No response when downloading WASM blob`);
    }
    const decompressed = response.body.pipeThrough(
        new DecompressionStream("gzip"),
    ).pipeThrough(new UntarStream());
    const files = decompressed[Symbol.asyncIterator]();
    const fileEntry = await files.next();
    if (fileEntry.done) {
        throw new Error("No file entries inside the tar archive");
    }
    const binary = await new Response(fileEntry.value.readable).bytes();
    if (!(await files.next()).done) {
        throw new Error(
            "Found several files inside the tar archive but expected a single one",
        );
    }
    return binary;
}

/** Download WebAssembly CLI code from GitHub using dynamic `import` instead of
 * `fetch` to not require the `--allow-net` permission. */
export async function importWasmBlob(
    url: string = GITHUB_RELEASE_JSON_WITH_WASM,
) {
    const { decodeBase64 } = await import("jsr:@std/encoding@^1.0.0/base64");

    const json = (await import(url, {
        with: { type: "json" },
    })).default;

    if (!json || typeof json !== "object") {
        throw new Error(
            `Expected json to contain an object but found ${typeof json}`,
        );
    }
    if (!("wasmGzippedBase64" in json)) {
        throw new Error(
            `Expected json to contain the key "wasmGzippedBase64" but it only contained the keys: ${
                JSON.stringify(Object.keys(json))
            }`,
        );
    }
    if (typeof json.wasmGzippedBase64 !== "string") {
        throw new Error(`Expected "wasmGzippedBase64" key to contain a string`);
    }

    // Base64 decode
    const gzippedWasm = decodeBase64(json.wasmGzippedBase64);
    //const gzippedWasm = Uint8Array.from(atob(json.wasmGzippedBase64), (c) => c.charCodeAt(0));

    // Decompress (gunzip)
    return new Response(new Response(gzippedWasm.buffer).body!.pipeThrough(
        new DecompressionStream("gzip"),
    )).bytes();
}

/** Gracefully support multiple different ways of specifying where to get the
 * WebAssembly module from. */
export async function getWasm(from: string): Promise<Uint8Array> {
    try {
        if (from === "DOWNLOAD") {
            return await fetchWasmBlob();
        } else if (from === "IMPORT") {
            return await importWasmBlob();
        } else if (from.toLowerCase().startsWith("http")) {
            if (from.toLowerCase().endsWith(".json")) {
                return await importWasmBlob(from);
            } else {
                return await fetchWasmBlob(from);
            }
        } else if (from.toLowerCase().endsWith(".json")) {
            // Assume JSON with gzipped base64 data:
            const path = await import("jsr:@std/path@^1.1.0");

            return await importWasmBlob(
                path.toFileUrl(
                    path.resolve(Deno.cwd(), from.replaceAll("\\", "/")),
                ).toString(),
            );
        } else {
            // Assume wasm file
            return await Deno.readFile(Deno.args[0]);
        }
    } catch (cause) {
        throw new Error(`Failed to get WebAssembly module from "${from}"`, {
            cause,
        });
    }
}

export async function main() {
    if (Deno.args.length === 0) {
        console.error(
            `First argument must be the path to the WebAssembly module`,
        );
        Deno.exit(2);
    }

    const context = new Context(
        await prepareWasiContextArguments(Deno.args.slice(1)),
    );

    const binary: Uint8Array = await getWasm(Deno.args[0]);

    const module = await WebAssembly.compile(binary);
    const instance = await WebAssembly.instantiate(module, {
        "wasi_snapshot_preview1": context.exports,
    });
    context.start(instance);
}

if (import.meta.main) {
    await main();
}
