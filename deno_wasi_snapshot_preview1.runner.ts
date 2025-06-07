import Context from './deno_wasi_snapshot_preview1.ts'

/** Setup Wasm environment */
export async function prepareContext(args: string[]) {
    let env: { [index: string]: string } = {};
    try {
        env = Deno.env.toObject();
    } catch (error) {
        console.error(`Failed to get environment variables so running without them: `, error);
    }

    try {
        env['CD'] = Deno.cwd();
    } catch (error) {
        console.error(`Failed to get current directory so running as if in root directory: `, error);
    }

    // Make firefox's profile directory available:
    let preopens: { [key: string]: string } | undefined = undefined;
    if ('USERNAME' in env) {
        const firefoxProfileWindows = `C:\\Users\\${env.USERNAME}\\AppData\\Roaming\\Mozilla\\Firefox\\Profiles`;
        const status = await Deno.permissions.request({ name: 'read', path: firefoxProfileWindows });
        if (status.state === 'granted') {
            preopens = { [firefoxProfileWindows]: firefoxProfileWindows, };
        }
    }

    return new Context({
        args: ['firefox-session-data', ...args],
        env,
        skipFsCheck: true,
        preopens,
    });
}

/** Download WebAssembly CLI code from GitHub. */
export async function downloadWasmBlobFromGitHubRelease() {
    const { UntarStream } = await import("jsr:@std/tar@^0.1.6/untar-stream");

    const response = await fetch('https://github.com/Lej77/firefox_session_data/releases/download/v0.1.0/firefox-session-data-wasm32-wasip1.tar.gz')
    if (response.body === null) {
        throw new Error(`No response when downloading WASM blob`);
    }
    const decompressed = response.body.pipeThrough(new DecompressionStream("gzip")).pipeThrough(new UntarStream());
    const files = decompressed[Symbol.asyncIterator]();
    const fileEntry = await files.next();
    if (fileEntry.done) {
        throw new Error("No file entries inside the tar archive");
    }
    const binary = await new Response(fileEntry.value.readable).bytes();
    if (!(await files.next()).done) {
        throw new Error("Found several files inside the tar archive but expected a single one");
    }
    return binary;
}

export async function main() {
    if (Deno.args.length === 0) {
        console.error(`First argument must be the path to the WebAssembly module`);
        Deno.exit(2);
    }

    const context = await prepareContext(Deno.args.slice(1));

    const binary: Uint8Array = Deno.args[0] === 'DOWNLOAD' ?
        await downloadWasmBlobFromGitHubRelease() :
        await Deno.readFile(Deno.args[0]);

    const module = await WebAssembly.compile(binary);
    const instance = await WebAssembly.instantiate(module, {
        "wasi_snapshot_preview1": context.exports,
    });
    context.start(instance);
}

if (import.meta.main) {
    await main();
}
