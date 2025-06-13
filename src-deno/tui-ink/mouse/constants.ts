/** Mouse related ansi codes.
 *
 * Copied from: <https://github.com/zenobi-us/ink-mouse/blob/406716cfcdbcef910eeeece84851a5ce1659023a/src/ink-mouse/constants.ts#L1-L39>
 */
export const ANSI_CODES = {
  // SET_X10_MOUSE
  mouseX10: { on: "\x1b[?9h", off: "\x1b[?9l" },

  // Terminal will send event on button pressed with mouse position
  // SET_VT200_MOUSE
  mouseButton: { on: "\x1b[?1000h", off: "\x1b[?1000l" },

  // Terminal will send position of the column highlighted
  // SET_VT200_HIGHLIGHT_MOUSE
  mouseHighlight: { on: "\x1b[?1001h", off: "\x1b[?1001l" },

  // Terminal will send event on button pressed and mouse motion as long as a button is down, with mouse position
  // SET_BTN_EVENT_MOUSE
  mouseDrag: { on: "\x1b[?1002h", off: "\x1b[?1002l" },

  // Terminal will send event on button pressed and motion
  // SET_ANY_EVENT_MOUSE
  mouseMotion: { on: "\x1b[?1003h", off: "\x1b[?1003l" },

  // SET_FOCUS_EVENT_MOUSE
  mouseFocus: { on: "\x1b[?1004h", off: "\x1b[?1004l" },

  // SET_EXT_MODE_MOUSE
  mouseUtf8: { on: "\x1b[?1005h", off: "\x1b[?1005l" },

  // Another mouse protocol that extend coordinate mapping (without it, it supports only 223 rows and columns)
  // SET_SGR_EXT_MODE_MOUSE
  mouseSGR: { on: "\x1b[?1006h", off: "\x1b[?1006l" },

  // SET_ALTERNATE_SCROLL
  alternateScroll: { on: "\x1b[?1007h", off: "\x1b[?1007l" },

  // SET_URXVT_EXT_MODE_MOUSE
  mouseMotionOthers: { on: "\x1b[?1015h", off: "\x1b[?1015l" },

  // SET_PIXEL_POSITION_MOUSE
  mousePixelMode: { on: "\x1b[?1016h", off: "\x1b[?1016l" },
};

/** Ansi codes written to stdin when mouse events are enabled.
 *
 * Adapted from:
 * <https://github.com/zenobi-us/ink-mouse/blob/406716cfcdbcef910eeeece84851a5ce1659023a/src/ink-mouse/constants.ts#L41-L46>
 */
export const ANSI_RESPONSE_CODES = {
  /** Masks the bits that determine the type of mouse events. */
  mask: 0b1100011,

  /** Leading text of mouse events that are written to stdin. */
  prefix: "\x1b[<",

  flags: {
    ctrl: 16,
    alt: 8,
  },

  /** Mouse wheel events. */
  scroll: {
    /** Scrolled up. This is also mentioned at:
     * <https://github.com/zenobi-us/ink-mouse/blob/406716cfcdbcef910eeeece84851a5ce1659023a/src/ink-mouse/ansiParser.ts#L31>
     */
    up: 64,
    down: 65,
  },

  /** Mouse movement events (also occur when clicking and then holding the
   * mouse button and moving the cursor around). */
  move: {
    none: 35,
    right: 34,
    middle: 33,
    left: 32,
  },

  /* Mouse click events. */
  click: {
    right: 2,
    middle: 1,
    left: 0,
  },
};
