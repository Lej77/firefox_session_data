import { ANSI_RESPONSE_CODES } from "./constants.ts";

export type MouseButton = "right" | "middle" | "left";
export type ButtonState = "pressed" | "released";
export type ScrollDirection = "scrollup" | "scrolldown";

export type MousePosition = {
  x: number;
  y: number;
};
export type MouseModifiers = {
  /** The Alt key was held down when the mouse event occurred. */
  alt: boolean;
  /** The Ctrl key was held down when the mouse event occurred. */
  ctrl: boolean;
};
export type ScrollEvent = {
  /** The scroll wheel was turned. */
  type: "scroll";
  /** The direction the wheel was turned. */
  direction: ScrollDirection;
};
export type MoveEvent = {
  /** The mouse's position was changed. Some terminals seem to only send this
   * while a button is held. */
  type: "move";
  /** The held mouse button. */
  button: MouseButton | "none";
  /** The button's current state. */
  state: ButtonState;
};
export type ClickEvent = {
  /** A mouse button was pressed or released. */
  type: "click";
  /** Which mouse button was clicked. */
  button: MouseButton;
  /** The button's new state. */
  state: ButtonState;
};
export type MouseEvent =
  & MousePosition
  & MouseModifiers
  & (ScrollEvent | MoveEvent | ClickEvent);

const scrollEvents = Object.values(ANSI_RESPONSE_CODES.scroll);
const moveEvents = Object.values(ANSI_RESPONSE_CODES.move);
const clickEvents = Object.values(ANSI_RESPONSE_CODES.click);
const allEvents = [...scrollEvents, ...moveEvents, ...clickEvents];

export function isMouseEvent(input: string): boolean {
  if (!input.startsWith(ANSI_RESPONSE_CODES.prefix)) return false;
  input = input.slice(ANSI_RESPONSE_CODES.prefix.length);

  const len = input.indexOf(";");
  if (len < 0) return false;

  const code = parseInt(input.slice(0, len), 10);
  if (isNaN(code)) return false;

  const eventCode = code & ANSI_RESPONSE_CODES.mask;
  return allEvents.includes(eventCode);
}

/** Parse an ANSI mouse response code.
 *
 * Adapted from code at:
 * <https://github.com/zenobi-us/ink-mouse/blob/406716cfcdbcef910eeeece84851a5ce1659023a/src/ink-mouse/ansiParser.ts>
 */
export function parseAnsiMouseEvent(input: string): null | MouseEvent {
  if (!input.startsWith(ANSI_RESPONSE_CODES.prefix)) return null;
  input = input.slice(ANSI_RESPONSE_CODES.prefix.length);

  const len = input.indexOf(";");
  if (len < 0) return null;

  const code = parseInt(input.slice(0, len), 10);
  if (isNaN(code)) return null;
  input = input.slice(len + 1);

  const eventCode = code & ANSI_RESPONSE_CODES.mask;

  let eventInfo:
    & Partial<MousePosition>
    & Partial<MouseModifiers>
    & (
      | ScrollEvent
      | (Omit<MoveEvent, "state"> & Partial<MoveEvent>)
      | (Omit<ClickEvent, "state"> & Partial<ClickEvent>)
    );

  switch (eventCode) {
    case ANSI_RESPONSE_CODES.scroll.up:
      eventInfo = { type: "scroll", direction: "scrollup" };
      break;
    case ANSI_RESPONSE_CODES.scroll.down:
      eventInfo = { type: "scroll", direction: "scrolldown" };
      break;

    case ANSI_RESPONSE_CODES.move.none:
      eventInfo = { type: "move", button: "none" };
      break;
    case ANSI_RESPONSE_CODES.move.right:
      eventInfo = { type: "move", button: "right" };
      break;
    case ANSI_RESPONSE_CODES.move.left:
      eventInfo = { type: "move", button: "left" };
      break;
    case ANSI_RESPONSE_CODES.move.middle:
      eventInfo = { type: "move", button: "middle" };
      break;

    case ANSI_RESPONSE_CODES.click.right:
      eventInfo = { type: "click", button: "right" };
      break;
    case ANSI_RESPONSE_CODES.click.middle:
      eventInfo = { type: "click", button: "middle" };
      break;
    case ANSI_RESPONSE_CODES.click.left:
      eventInfo = { type: "click", button: "left" };
      break;
    default:
      /** Unknown mouse event. */
      return null;
  }

  eventInfo.ctrl = (code & ANSI_RESPONSE_CODES.flags.ctrl) != 0;
  eventInfo.alt = (code & ANSI_RESPONSE_CODES.flags.alt) != 0;

  const xEnd = input.indexOf(";");
  const yEnd = input.toLowerCase().indexOf("m", xEnd + 1);
  const x = xEnd <= 0 ? 0 : parseInt(input.slice(0, xEnd), 10);
  const y = yEnd <= 0 ? 0 : parseInt(input.slice(xEnd + 1, yEnd));
  eventInfo.x = isNaN(x) ? 0 : x;
  eventInfo.y = isNaN(y) ? 0 : y;

  if (eventInfo.type === "scroll") {
    if (yEnd < 0 || input[yEnd] !== "M") {
      return null;
    } else {
      return eventInfo as MouseEvent;
    }
  }

  const state: ButtonState = yEnd < 0
    ? "released"
    : input[yEnd] === "M"
    ? "pressed"
    : "released";

  eventInfo.state = state;

  if (eventInfo.button === "none" && state === "released") {
    return null;
  }

  return eventInfo as MouseEvent;
}
