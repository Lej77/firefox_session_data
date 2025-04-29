# Firefox Session Data CLI

<!-- Badge style inspired by https://github.com/dnaka91/advent-of-code/blob/de37024ba3b385694e14f79c849370c0f605f054/README.md -->

<!-- [![Build Status][build-img]][build-url] -->
[![Documentation][doc-img]][doc-url]

<!--
[build-img]: https://img.shields.io/github/actions/workflow/status/Lej77/firefox_session_data/ci.yml?branch=main&style=for-the-badge
[build-url]: https://github.com/Lej77/firefox_session_data/actions/workflows/ci.yml
 -->
<!-- https://shields.io/badges/static-badge -->
[doc-img]: https://img.shields.io/badge/docs.rs-firefox_session_data-4d76ae?style=for-the-badge
[doc-url]: https://lej77.github.io/firefox_session_data

This repository contains a CLI tool for interacting with Firefox's session data which stores information about the browser's currently open windows and tabs.

There is a couple of GUI applications that provides some of the functionality that the CLI exposes:

- <https://github.com/Lej77/firefox-session-ui> ([Web demo](https://lej77.github.io/firefox-session-ui/)) (Built with web technology using the [`Dioxus`](https://crates.io/crates/dioxus) and [`Tauri`](https://crates.io/crates/tauri) frameworks)
- <https://github.com/Lej77/firefox-session-ui-gtk4> (Built using the [`GTK4`](https://crates.io/crates/gtk4) UI library)
- <https://github.com/Lej77/firefox-session-ui-iced> ([Bugged web demo](https://lej77.github.io/firefox-session-ui-iced/)) (Built using the [`iced`](https://crates.io/crates/iced) UI library)

## Platform support

Currently only Windows and WebAssembly are supported, but it should be easy to port to other platforms and might already compile without issues.

The optional HTML to PDF converters would probably require quite a bit of work in order to allow them to compile on all platforms, the code can be found at [Lej77/html_to_pdf: Rust code for different HTML to PDF conversion methods](https://github.com/Lej77/html_to_pdf). These aren't enabled by default though so they can be left alone for now.

Otherwise it should mostly be code related to finding where Firefox stores its profile folders on each platform.

## Usage

Just compile using [`Cargo`](https://www.rust-lang.org/tools/install):

```bash
cargo run --release -- --help
cargo run --release -- tabs-to-links --firefox-profile=default-release --output=./my-links
```

Note: currently only Windows is supported but it should not be hard to port the program to other operating systems.

### WebAssembly

This crate can be compile to WebAssembly and executed with limited functionality. You need a runtime like [`wasmtime`](https://crates.io/crates/wasmtime-cli) (`cargo install wasmtime-cli`) to run it.

We can use [`cargo-wasi`](https://crates.io/crates/cargo-wasi) (`cargo install cargo-wasi`) to easily compile and run the program:

```cmd
cargo wasi run -- tabs-to-links --compressed --stdin --stdout --format=text >.temp.txt <%AppData%/Mozilla/Firefox/Profiles/XXXXXXXX.default-release/sessionstore-backups/recovery.jsonlz4
REM Note: XXXXXXXX is some unique prefix generated for your Firefox profile.
```

We can also manually compile the program for WebAssembly and then run it using a runtime like `wasmtime`:

```cmd
rustup target add wasm32-wasip1

cargo build --release --target wasm32-wasip1
REM Built wasm file is now at:
REM ./target/wasm32-wasip1/release/firefox-session-data.wasm


wasmtime ./target/wasm32-wasip1/release/firefox-session-data.wasm tabs-to-links --compressed --stdin --stdout --format=text >.temp.txt <%AppData%/Mozilla/Firefox/Profiles/XXXXXXXX.default-release/sessionstore-backups/recovery.jsonlz4
REM Note: XXXXXXXX is some unique prefix generated for your Firefox profile.


REM if we preopen the Firefox profile folder we can even let the CLI find the input file's exact path:
wasmtime -S inherit-env=y --dir "%AppData%\Mozilla\Firefox\Profiles" ./target/wasm32-wasip1/release/firefox-session-data.wasm tabs-to-links --firefox-profile=default-release --stdout --format=text >.temp.txt


REM We can also run the WebAssembly module using Deno v2:
deno run --allow-env --allow-read deno_wasi_snapshot_preview1.runner.ts  ./target/wasm32-wasip1/release/firefox-session-data.wasm tabs-to-links --compressed --stdin --stdout --format=text >.temp.txt <%AppData%/Mozilla/Firefox/Profiles/XXXXXXXX.default-release/sessionstore-backups/recovery.jsonlz4
REM Note: XXXXXXXX is some unique prefix generated for your Firefox profile.
REM Note: seems like detecting if a directory entry is a file or folder is broken in deno_wasi_snapshot_preview1.ts so we can't let the CLI find paths
```

## License

This project is released under either:

- [MIT License](./LICENSE-MIT)
- [Apache License (Version 2.0)](./LICENSE-APACHE)

at your choosing.

Note that some optional dependencies might be under different licenses.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
