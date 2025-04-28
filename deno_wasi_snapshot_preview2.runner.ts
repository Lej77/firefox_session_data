import * as wasi from 'npm:@bytecodealliance/preview2-shim@^0.17.1'

if (Deno.args.length === 0) {
    console.error(`First argument must be the path to the WebAssembly module`);
    Deno.exit(2);
}
const binary = await Deno.readFile(Deno.args[0]);
const module = await WebAssembly.compile(binary); // FIXME: currently fails to compile...
const instance = await WebAssembly.instantiate(module, {
    ...wasi,
});

const { _start, _initialize, memory } = instance.exports;

if (!(memory instanceof WebAssembly.Memory)) {
    throw new TypeError("WebAssembly.instance must provide a memory export");
}

if (typeof _initialize === "function") {
    throw new TypeError(
        "WebAssembly.instance export _initialize must not be a function",
    );
}

if (typeof _start !== "function") {
    throw new TypeError(
        "WebAssembly.Instance export _start must be a function",
    );
}

_start();