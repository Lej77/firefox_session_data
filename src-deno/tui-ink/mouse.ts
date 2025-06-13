import { ink, React } from "./deps.ts";
import { useConsoleSize } from "./console-size.tsx";
import { useOverlayInfo } from "./overlay.tsx";
import { getElementRegion, isIntersecting, Position } from "./position.ts";
import { useMouse } from "./mouse/context.tsx";
import {
  ClickEvent,
  MouseEvent,
  ScrollDirection,
  ScrollEvent,
} from "./mouse/parser.ts";

const { useEffect, useRef } = React;

/** Listen for mouse clicks on the specified element. */
export function useOnMouseClick(
  ref: React.RefObject<ink.DOMElement | null>,
  onChange: (event: boolean, position: Position) => void,
): void {
  const mouse = useMouse();
  const overlayInfo = useOverlayInfo();

  const latestOnChange = useRef(onChange);
  latestOnChange.current = onChange;

  const handler = (position: Position, event: ClickEvent | null) => {
    latestOnChange.current(
      event?.state === "pressed" && overlayInfo.isTopLayer() && isIntersecting({
        region: getElementRegion(ref.current),
        position: position,
      }),
      position,
    );
  };

  useEffect(
    function HandleIntersection() {
      const events = mouse.events;

      events.on("click", handler);
      return () => {
        events.off("click", handler);
      };
    },
    [ref],
  );
}

/** Listen for mouse movement above a specific element. */
export function useOnMouseHover(
  ref: React.RefObject<ink.DOMElement | null>,
  onChange: (event: boolean) => void,
): void {
  const mouse = useMouse();
  const overlayInfo = useOverlayInfo();

  const latestOnChange = useRef(onChange);
  latestOnChange.current = onChange;

  const handler = (position: Position) => {
    const intersecting = overlayInfo.isTopLayer() && isIntersecting({
      region: getElementRegion(ref.current),
      position: position,
    });

    latestOnChange.current(intersecting);
  };
  useEffect(function HandleIntersection() {
    const events = mouse.events;

    events.on("position", handler);
    return () => {
      events.off("position", handler);
    };
  }, [ref]);

  // Re-check hover after console size changes since that might have affected position (and maybe size?)
  const consoleSize = useConsoleSize();
  useEffect(() => handler(mouse.position), [consoleSize]);
}

/** Listen to mouse scroll events that occur over a specific element. */
export function useOnMouseScroll(
  ref: React.RefObject<ink.DOMElement | null>,
  onChange: (direction: ScrollDirection) => void,
): void {
  const mouse = useMouse();
  const overlayInfo = useOverlayInfo();

  const latestOnChange = useRef(onChange);
  latestOnChange.current = onChange;

  const handler = (position: Position, event: ScrollEvent | null) => {
    if (!event) return;
    const intersecting = overlayInfo.isTopLayer() && isIntersecting({
      region: getElementRegion(ref.current),
      position: position,
    });

    if (intersecting) {
      latestOnChange.current(event.direction);
    }
  };

  useEffect(function HandleIntersection() {
    const events = mouse.events;

    events.on("scroll", handler);
    return () => {
      events.off("scroll", handler);
    };
  }, [ref]);
}

/** Listen to raw mouse events. */
export function useOnMouseEvent(
  onEvent: (event: MouseEvent) => void,
): void {
  const mouse = useMouse();

  const latestOnEvent = useRef(onEvent);
  latestOnEvent.current = onEvent;

  const handler = (event: MouseEvent) => {
    latestOnEvent.current(event);
  };
  useEffect(function HandleIntersection() {
    const events = mouse.events;

    events.on("all", handler);
    return () => {
      events.off("all", handler);
    };
  });
}
