import { MouseModifiers, parseAnsiMouseEvent } from "./parser.ts";
import { assertEquals } from "jsr:@std/assert@1";

const noMod: MouseModifiers = { alt: false, ctrl: false };

Deno.test({
  name: "scroll",
  fn() {
    // little m not supported
    assertEquals(parseAnsiMouseEvent("\x1b[<65;1;1m"), null);
    assertEquals(parseAnsiMouseEvent("\x1b[<64;1;1m"), null);

    assertEquals(parseAnsiMouseEvent("\x1b[<65;1;1M"), {
      type: "scroll",
      direction: "scrolldown",
      x: 1,
      y: 1,
      ...noMod,
    });
    assertEquals(parseAnsiMouseEvent("\x1b[<64;1;1M"), {
      type: "scroll",
      direction: "scrollup",
      x: 1,
      y: 1,
      ...noMod,
    });
    assertEquals(parseAnsiMouseEvent("\x1b[<65;4000;42M"), {
      type: "scroll",
      direction: "scrolldown",
      x: 4000,
      y: 42,
      ...noMod,
    });
  },
});
Deno.test({
  name: "drag",
  fn() {
    // we don't care about the m or M
    assertEquals(parseAnsiMouseEvent("\x1b[<32;1;1m")?.type, "move");

    assertEquals(parseAnsiMouseEvent("\x1b[<32;1;1M"), {
      type: "move",
      button: "left",
      state: "pressed",
      x: 1,
      y: 1,
      ...noMod,
    });
    assertEquals(parseAnsiMouseEvent("\x1b[<32;4000;42M"), {
      type: "move",
      button: "left",
      state: "pressed",
      x: 4000,
      y: 42,
      ...noMod,
    });
  },
});
Deno.test({
  name: "move",
  fn() {
    assertEquals(parseAnsiMouseEvent("\x1b[<35;1;1M")?.type, "move");
    assertEquals(parseAnsiMouseEvent("\x1b[<35;1;1m"), null);

    assertEquals(parseAnsiMouseEvent("\x1b[<35;1;1M"), {
      type: "move",
      button: "none",
      state: "pressed",
      x: 1,
      y: 1,
      ...noMod,
    });
    assertEquals(parseAnsiMouseEvent("\x1b[<35;4000;42M"), {
      type: "move",
      button: "none",
      state: "pressed",
      x: 4000,
      y: 42,
      ...noMod,
    });
  },
});
Deno.test({
  name: "click",
  fn() {
    assertEquals(parseAnsiMouseEvent("\x1b[<0;1;1m"), {
      type: "click",
      button: "left",
      state: "released",
      x: 1,
      y: 1,
      ...noMod,
    });

    assertEquals(parseAnsiMouseEvent("\x1b[<0;1;1M"), {
      type: "click",
      button: "left",
      state: "pressed",
      x: 1,
      y: 1,
      ...noMod,
    });
    assertEquals(parseAnsiMouseEvent("\x1b[<0;4000;42M"), {
      type: "click",
      button: "left",
      state: "pressed",
      x: 4000,
      y: 42,
      ...noMod,
    });
  },
});
