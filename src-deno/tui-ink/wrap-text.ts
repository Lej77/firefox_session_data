import { cli_truncate, ink, wrap_ansi } from "./deps.ts";

/** Handle text wrapping in terminal.
 *
 * Adapted from <https://github.com/vadimdemedes/ink/blob/7f2bc3cad6ce21d9f127a15cb58008e02ba48b11/src/wrap-text.ts>
 */
export function wrapText(
    text: string,
    maxWidth: number,
    wrapType: ink.TextProps["wrap"],
): string {
    if (wrapType === "wrap") {
        return wrap_ansi.default(text, maxWidth, {
            trim: false,
            hard: true,
        });
    } else if (wrapType!.startsWith("truncate")) {
        let position: "end" | "middle" | "start" = "end";

        if (wrapType === "truncate-middle") {
            position = "middle";
        }

        if (wrapType === "truncate-start") {
            position = "start";
        }

        return cli_truncate.default(text, maxWidth, { position });
    } else {
        return text;
    }
}

/** Same as {@link wrapText} but preforms heavy work inside a WebWorker. */
export async function asyncWrapText(
    text: string,
    maxWidth: number,
    wrapType: ink.TextProps["wrap"],
    abortSignal?: AbortSignal,
): Promise<string> {
    if (wrapType === "wrap") {
        const code = `
import wrapAnsi from 'npm:wrap-ansi';
self.onmessage = (msg) => {
    self.postMessage(wrapAnsi(msg.data.text, msg.data.maxWidth, { trim: false, hard: true }));
    self.close();
}`;
        const worker = new Worker("data:," + code, { type: "module" });
        worker.postMessage({ text, maxWidth });
        const abortWorker = () => worker.terminate();
        abortSignal?.addEventListener("abort", abortWorker);
        try {
            return await new Promise<string>((resolve, reject) => {
                worker.onmessage = (msg: MessageEvent<string>) =>
                    resolve(msg.data);
                worker.onerror = reject;
                worker.onmessageerror = reject;
            });
        } finally {
            worker.terminate();
            abortSignal?.removeEventListener("abort", abortWorker);
        }
    } else if (wrapType!.startsWith("truncate")) {
        let position: "end" | "middle" | "start" = "end";

        if (wrapType === "truncate-middle") {
            position = "middle";
        }

        if (wrapType === "truncate-start") {
            position = "start";
        }

        const code = `
export cliTruncate from 'npm:cli-truncate';
self.onmessage = (msg) => {
    self.postMessage(cliTruncate(msg.data.text, msg.data.maxWidth, { position: msg.data.position }));
    self.close();
}`;
        const worker = new Worker("data:," + code, { type: "module" });
        worker.postMessage({ text, maxWidth, position });
        const abortWorker = () => worker.terminate();
        abortSignal?.addEventListener("abort", abortWorker);
        try {
            return await new Promise<string>((resolve, reject) => {
                worker.onmessage = (msg: MessageEvent<string>) =>
                    resolve(msg.data);
                worker.onerror = reject;
                worker.onmessageerror = reject;
            });
        } finally {
            worker.terminate();
            abortSignal?.removeEventListener("abort", abortWorker);
        }
    } else {
        return text;
    }
}
