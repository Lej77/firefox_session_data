// Info about re-exports:
// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/export

// @ts-types="npm:@types/react@^19.1.6"
import * as React from "npm:react@18";
export { React };

export * as ink from "npm:ink@^5.0.0";
export * as ink_input from "npm:ink-text-input@^6.0.0"; // <- want to fork this since it doesn't handle the "delete" key correctly (delete is not the same as backspace)

// Used by wrap-text
export * as cli_truncate from 'npm:cli-truncate@^4.0.0';
export * as wrap_ansi from 'npm:wrap-ansi@^9.0.0';

export * as path_extname from 'jsr:@std/path@^1.1.0/extname'
