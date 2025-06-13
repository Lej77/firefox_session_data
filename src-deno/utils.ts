import { encodeBase64 } from "jsr:@std/encoding@^1.0.0/base64";

export function findDownloadsFolder() {
    if (Deno.build.os === "windows") {
        try {
            const userProfile = Deno.env.get("USERPROFILE");
            if (userProfile) {
                return userProfile + "\\Downloads";
            }
        } catch {
            // likely permission error
        }
        try {
            const name = Deno.env.get("USERNAME");
            if (name) {
                return "C:/Users/" + name + "/Downloads";
            }
        } catch {
            // likely permission error
        }
    }
    return null;
}

/**
 * XTerm and a few other terminal emulators recognize the [OSC 52] escape
 * sequence. [VS Code supports it since June 2024]  When run in such a terminal
 * emulator, this program writes text to the clipboard without any permissions.
 *
 * Code copied from: <https://github.com/denoland/deno/issues/3450#issuecomment-2207908260>
 *
 * [OSC 52]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h4-Operating-System-Commands:OSC-Ps;Pt-ST:Ps-=-5-2.101B
 * [VS Code supports it since June 2024]: https://code.visualstudio.com/updates/v1_91#_support-for-copy-and-paste-escape-sequence-osc-52
 */
export async function writeTextToClipboard(
    data: string,
    options?: { sync?: boolean },
): Promise<void> {
    const encoder = new TextEncoder();
    const osc = encoder.encode(`\x1b]52;c;${encodeBase64(`${data}`)}\x07`);
    if (options?.sync) {
        let n = 0;
        while (n < osc.length) {
            n += Deno.stdout.writeSync(osc.slice(n));
        }
    } else {
        const writer = Deno.stdout.writable.getWriter();
        const promise = writer.write(osc);
        writer.releaseLock();
        try {
            await promise;
        } catch {
            // ignored
        }
    }
}
