// ink will try to read lots of env vars to determine terminal capabilities. Better to request all env access up front.
Deno.permissions.requestSync({ name: "env" });

if (Deno.build.os === "windows") {
  // Workaround for color detection on Windows 10 which otherwise requires --allow-sys from `(await import('node:os')).release()` in ink's code.
  if (
    "granted" !==
      Deno.permissions.querySync({ name: "sys", kind: "osRelease" }).state
  ) {
    Deno.env.set("FORCE_COLOR", "3");
    Deno.env.set("TERM", "dumb");
  }
}
