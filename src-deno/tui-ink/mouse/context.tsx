import { React } from "../deps.ts";
import { ANSI_CODES } from "./constants.ts";
import {
  ClickEvent,
  MouseButton,
  MouseEvent,
  MouseModifiers,
  MousePosition,
  MoveEvent,
  parseAnsiMouseEvent,
  ScrollEvent,
} from "./parser.ts";
import process from "node:process";
import EventEmitter from "node:events";

const { useEffect, useMemo } = React;

export function enableMouseEvents() {
  process.stdout.write(
    ANSI_CODES.mouseButton.on +
      ANSI_CODES.mouseMotion.on +
      ANSI_CODES.mouseMotionOthers.on +
      ANSI_CODES.mouseSGR.on,
  );
}
export function disableMouseEvents() {
  process.stdout.write(
    ANSI_CODES.mouseMotion.off +
      ANSI_CODES.mouseSGR.off +
      ANSI_CODES.mouseMotionOthers.off +
      ANSI_CODES.mouseButton.off,
  );
}

export interface MouseEventMap {
  "all": [event: MouseEvent];
  "position": [position: MousePosition];
  "drag": [
    position: MousePosition,
    event: MoveEvent & MouseModifiers & MousePosition & { button: MouseButton },
  ];
  "click": [
    position: MousePosition,
    event: (ClickEvent & MouseModifiers & MousePosition) | null,
  ];
  "scroll": [
    position: MousePosition,
    event: (ScrollEvent & MouseModifiers & MousePosition) | null,
  ];
}
// listen to "data" event:  process.stdin.on("data", handleEvent);   process.stdin.off("data", handleEvent);
// send null event after time to forget mouse click: setTimeout(() => { onClick(null); }, 100);

const MouseContext = React.createContext<Mouse | null>(
  null,
);

export interface Mouse {
  events: EventEmitter<MouseEventMap>;
  position: MousePosition;
}

export function MouseProvider(props: React.PropsWithChildren) {
  const context = React.useContext(MouseContext);
  const mouse = useMemo<Mouse>(() => ({
    events: new EventEmitter<MouseEventMap>(),
    position: { x: 0, y: 0 },
  }), []);
  useEffect(() => {
    if (context) return; // already registered in parent provider.

    let clickTimeoutId: number | null = null;
    let scrollTimeoutId: number | null = null;

    const handleEvent = (data: string) => {
      const info = parseAnsiMouseEvent(data);
      if (!info) return;
      mouse.events.emit("all", info);
      mouse.position = { x: info.x, y: info.y };

      // Fancier events (don't really need these):
      mouse.events.emit("position", mouse.position);
      switch (info.type) {
        case "move":
          if (info.button !== "none") {
            // deno-lint-ignore no-explicit-any
            mouse.events.emit("drag", mouse.position, info as any);
          }
          break;
        case "click":
          mouse.events.emit("click", mouse.position, info);
          if (clickTimeoutId !== null) {
            clearTimeout(clickTimeoutId);
          }
          clickTimeoutId = setTimeout(() => {
            clickTimeoutId = null;
            mouse.events.emit("click", mouse.position, null);
          }, 100);
          break;
        case "scroll":
          mouse.events.emit("scroll", mouse.position, info);
          if (scrollTimeoutId !== null) {
            clearTimeout(scrollTimeoutId);
          }
          scrollTimeoutId = setTimeout(() => {
            scrollTimeoutId = null;
            mouse.events.emit("scroll", mouse.position, null);
          }, 100);
          break;
        default: {
          const _exhaustive: never = info;
        }
      }
    };
    const cleanup = () => {
      globalThis.removeEventListener("unload", cleanup);
      process.stdin.off("data", handleEvent);
      disableMouseEvents();

      if (clickTimeoutId !== null) {
        clearTimeout(clickTimeoutId);
        clickTimeoutId = null;
      }
      if (scrollTimeoutId !== null) {
        clearTimeout(scrollTimeoutId);
        scrollTimeoutId = null;
      }
    };
    globalThis.addEventListener("unload", cleanup);
    process.stdin.on("data", handleEvent);
    enableMouseEvents();
    return cleanup;
  }, [context]);

  return (
    <MouseContext.Provider value={context ?? mouse}>
      {props.children}
    </MouseContext.Provider>
  );
}

export function useMouse() {
  const context = React.useContext(MouseContext);
  if (!context) {
    throw new Error(
      "useMouse must only be used within children of <MouseProvider/>",
    );
  }

  return context;
}
