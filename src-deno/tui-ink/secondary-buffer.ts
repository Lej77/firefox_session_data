import { disableMouseEvents } from "./mouse/context.tsx";
import { writeAllSync } from "jsr:@std/io@^0.225.2/write-all";

/** ANSI escape code to tell terminal to switch back to primary buffer */
const USE_PRIMARY_BUFFER = "\x1b[?1049l";
/** ANSI escape code to tell terminal to use secondary buffer */
const USE_SECONDARY_BUFFER = "\x1b[?1049h";
/** ANSI escape code to clear screen  */
const CLEAR_SCREEN = `\x1b[2J`;
const MOVE_HOME = "\x1b[H";
/** ANSI escape code to hide terminal cursor  */
const HIDE_CURSOR = `\x1b[?25l`;
/** ANSI escape code to show terminal cursor  */
const SHOW_CURSOR = `\x1b[?25h`;
/** ANSI escape code to disable mouse handling */
const DISABLE_MOUSE = "\x1b[?1000l\x1b[?1002l\x1b[?1005l\x1b[?1006l";

/** This ensures any console history from before this app was opened remains
 * undisturbed while this app is running. */
export function switchToSecondaryTerminalBuffer() {
  const cleanup = () => {
    Deno.removeSignalListener("SIGBREAK", cleanup);
    Deno.removeSignalListener("SIGINT", cleanup);
    if (Deno.build.os !== "windows") {
      Deno.removeSignalListener("SIGTERM", cleanup);
    }
    removeEventListener("unload", cleanup);
    removeEventListener("unhandledrejection", cleanup);

    // Clear secondary buffer then switch back to the main buffer where the user's previous content is shown.
    writeAllSync(
      Deno.stdout,
      new TextEncoder().encode(
        USE_SECONDARY_BUFFER + CLEAR_SCREEN + MOVE_HOME + USE_PRIMARY_BUFFER +
          SHOW_CURSOR + DISABLE_MOUSE,
      ),
    );
    // Disable some extra things since we are quitting anyway:
    disableMouseEvents();
    Deno.stdin.setRaw(false);
  };

  Deno.addSignalListener("SIGBREAK", cleanup);
  Deno.addSignalListener("SIGINT", cleanup);
  if (Deno.build.os !== "windows") {
    Deno.addSignalListener("SIGTERM", cleanup);
  }

  globalThis.addEventListener("unload", cleanup);
  globalThis.addEventListener("unhandledrejection", cleanup);

  // Switch to secondary buffer and clear it (this will leave the main terminal content untouched while our UI is rendered.)
  writeAllSync(
    Deno.stdout,
    new TextEncoder().encode(
      USE_SECONDARY_BUFFER + HIDE_CURSOR + CLEAR_SCREEN + MOVE_HOME,
    ),
  );
}

/** Check if some Deno permissions needs to be prompted for and if so switch to
 * the main buffer before doing it. After all permissions have been requested
 * switch back to the secondary buffer.
 *
 * @returns `true` if the permissions were granted, `false` otherwise.
 */
export function requestPermissionOnMainBuffer(
  desc: Deno.PermissionDescriptor | Deno.PermissionDescriptor[],
): boolean {
  if (!Array.isArray(desc)) {
    desc = [desc];
  }
  const toRequest: Deno.PermissionDescriptor[] = [];
  for (const d of desc) {
    try {
      const info = Deno.permissions.querySync(d);
      if (info.state === "denied") {
        return false;
      } else if (info.state === "prompt") {
        toRequest.push(d);
      }
    } catch {
      // likely incorrect permission request (such as empty string as path in read permission)
      return false;
    }
  }

  if (toRequest.length === 0) return true;

  writeAllSync(
    Deno.stdout,
    new TextEncoder().encode(
      USE_SECONDARY_BUFFER + CLEAR_SCREEN + MOVE_HOME + USE_PRIMARY_BUFFER +
        SHOW_CURSOR,
    ),
  );
  Deno.stdin.setRaw(false);
  try {
    for (const d of toRequest) {
      try {
        const result = Deno.permissions.requestSync(d);
        if (result.state !== "granted") {
          return false;
        }
      } catch {
        return false;
      }
    }
  } finally {
    Deno.stdin.setRaw(true);
    writeAllSync(
      Deno.stdout,
      new TextEncoder().encode(
        USE_SECONDARY_BUFFER + HIDE_CURSOR + CLEAR_SCREEN + MOVE_HOME,
      ),
    );
  }
  return true;
}
