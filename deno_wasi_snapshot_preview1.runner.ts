import Context from './deno_wasi_snapshot_preview1.ts'

if (Deno.args.length === 0) {
    console.error(`First argument must be the path to the WebAssembly module`);
    Deno.exit(2);
}

let env: { [index: string]: string } = {};
try {
    env = Deno.env.toObject();
} catch (error) {
    console.error(`Failed to get environment variables so running without them: `, error);
}
if (!('CD' in env)) {
    try {
        env['CD'] = Deno.cwd();
    } catch (error) {
        console.error(`Failed to get current directory so running as if in root directory: `, error);
    }
}

const firefoxProfileWindows = `C:\\Users\\${env.USERNAME}\\AppData\\Roaming\\Mozilla\\Firefox\\Profiles`;
const context = new Context({
    args: Deno.args,
    env,
    skipFsCheck: true,
    preopens: {
        [firefoxProfileWindows]: firefoxProfileWindows,
    }
});

const binary = await Deno.readFile(Deno.args[0]);
const module = await WebAssembly.compile(binary);
const instance = await WebAssembly.instantiate(module, {
    "wasi_snapshot_preview1": context.exports,
});
context.start(instance);
