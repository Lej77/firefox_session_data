import { useConsoleSize } from "./console-size.tsx";
import { ink, ink_input, React } from "./deps.ts";
import {
  useOnMouseClick,
  useOnMouseEvent,
  useOnMouseHover,
  useOnMouseScroll,
} from "./mouse.ts";
import { asOverlay, useOverlayInfo } from "./overlay.tsx";
import { getElementRegion, isIntersecting, Position } from "./position.ts";
import { asyncWrapText } from "./wrap-text.ts";

const { useMemo, useRef, useState, useContext, useEffect } = React;
const {
  Box,
  Spacer,
  Text,
  useFocus,
  useInput,
} = ink;
const { default: TextInput } = ink_input;

export function useRandomId(customId?: string) {
  // Copied from the useFocused hook.
  const id = useMemo(() => {
    return customId ?? Math.random().toString().slice(2, 7);
  }, [customId]);
  return id;
}

export type ScrollableRef = ReturnType<typeof createScrollableRef>;

function createScrollableRef() {
  return {
    /** The max value for {@link ScrollableRef.scrollY}. */
    maxScrollY: 0,
    /** The top row inside the inner view that is visible. */
    scrollY: 0,
    /** Height of the inner view as seen in the terminal. */
    outerViewHeight: 0,
    /** Ensure a value is between 0 and {@link ScrollableRef.maxScrollY}. */
    asValidScrollY(value: number) {
      return Math.max(0, Math.min(this.maxScrollY, value));
    },
    setScrollY: (_scroll: number) => {},
    applyDeltaY(value: number) {
      this.setScrollY(this.asValidScrollY(this.scrollY + value));
    },
    ensureVisibleY(row: number) {
      if (row < this.scrollY) {
        this.setScrollY(Math.max(0, row));
      } else if (this.scrollY + this.outerViewHeight <= row) {
        this.setScrollY(this.asValidScrollY(row - this.outerViewHeight + 1));
      }
    },
    ensureElementIsVisible(_element: ink.DOMElement | null, _margin?: number) {
    },
  };
}
type ScrollbarProps = {
  value: number;
  maxValue: number;
  /** Height of the whole scrollbar track, same as the height of the scrollable view. */
  fullHeight: number;
  setScroll(value: number): void;
} & Required<Pick<ScrollableProps, "showScrollbar">>;
function Scrollbar(props: ScrollbarProps) {
  const refHandle = useRef(null);
  const refTrack = useRef(null);

  const { value, maxValue, fullHeight, showScrollbar } = props;
  const scrollOfFull = value / Math.max(maxValue, value);
  const handleHeight = Math.max(fullHeight > 16 ? 3 : 1, fullHeight - maxValue);
  const maxOffset = Math.max(0, fullHeight - handleHeight);
  const handleOffset = Math.floor(scrollOfFull * maxOffset);

  const [hovering, setHovering] = useState(false);
  const [dragging, setDragging] = useState(false);

  useOnMouseEvent((event) => {
    if (event.type === "scroll") return;
    const region = getElementRegion(refHandle.current);
    const onBar = isIntersecting({ position: event, region });
    setHovering(onBar);
    if (event.type === "click" && event.button === "left") {
      setDragging(onBar && event.state === "pressed");
    } else if (event.button === "left" && dragging) {
      const trackRegion = getElementRegion(refTrack.current);
      if (trackRegion) {
        const halfHandle = Math.round(handleHeight / 2);
        const wantedOffset = Math.min(
          maxOffset,
          Math.max(0, event.y - trackRegion.top - halfHandle),
        );
        const wantedValue = Math.ceil(maxValue * (wantedOffset / maxOffset));
        props.setScroll(Math.min(Math.max(maxValue, value), wantedValue));
      }
    }
  });

  const scrollbarEnabled = showScrollbar !== "never";
  const scrollbarIsFullSize = handleHeight >= fullHeight;
  const scrollbarIsVisible = showScrollbar === "auto"
    ? !scrollbarIsFullSize
    : scrollbarEnabled;

  return (
    <Box
      ref={refTrack}
      flexGrow={0}
      flexShrink={0}
      width={scrollbarIsVisible ? 1 : 0}
      flexDirection="column"
      overflow="hidden"
    >
      <Box
        ref={refHandle}
        width={1}
        marginTop={handleOffset}
      >
        <Text color={dragging ? "cyan" : hovering ? "white" : "grey"}>
          {"█".repeat(
            showScrollbar === "auto-invisible" && scrollbarIsFullSize
              ? 0
              : handleHeight,
          )}
        </Text>
      </Box>
    </Box>
  );
}

export type ScrollableProps = React.PropsWithChildren & {
  refOuter?: React.Ref<ink.DOMElement>;
  refInner?: React.Ref<ink.DOMElement>;
  refScroll?: React.Ref<ScrollableRef>;
  onScrollY?: (scrollY: number) => void;
  innerBoxProps?: ink.BoxProps;
  showScrollbar?: "always" | "auto" | "auto-invisible" | "never";
};

export function Scrollable(props: ScrollableProps) {
  const {
    refInner: propsRefInner,
    refOuter: propsRefOuter,
    refScroll: propsRefScroll,
    innerBoxProps,
    children,
    onScrollY,
    showScrollbar = "auto",
  } = props;
  const refOuter = useRef<ink.DOMElement>(null);
  const refInner = useRef<ink.DOMElement>(null);
  const refScroll = useRef<ScrollableRef>(null);

  const [scrollY, setScrollY] = useState(0);
  const [, setUpdateCount] = useState(0);

  // Remember scroll info:
  if (!refScroll.current) {
    refScroll.current = createScrollableRef();
  }
  function setScrollYAndNotify(value: number) {
    setScrollY(value);
    onScrollY?.(value);
  }
  refScroll.current.setScrollY = setScrollYAndNotify;
  refScroll.current.scrollY = scrollY;
  refScroll.current.ensureElementIsVisible = function (element, margin = 0) {
    if (!refInner.current) return;
    const location = getElementRegion(element, refInner.current);
    if (!location) return;
    const wantedTop = Math.max(0, location.top - margin);
    if (wantedTop < this.scrollY) {
      // Ensure top row of element is visible:
      this.setScrollY(wantedTop);
      return;
    }
    // top + min(height, viewHeight) should be visible:
    const lastVisibleRow = location.top +
      Math.min(location.height + margin, this.outerViewHeight) - 1;
    if (this.scrollY + this.outerViewHeight <= lastVisibleRow) {
      // Ensure bottom row of element is visible:
      this.setScrollY(
        this.asValidScrollY(lastVisibleRow - this.outerViewHeight + 1),
      );
    }
  };

  function updateMaxScroll(forceUpdateOnChange: boolean = true) {
    const heightOuter = refOuter.current
      ? ink.measureElement(refOuter.current).height
      : 0;
    const heightInner = refInner.current
      ? ink.measureElement(refInner.current).height
      : 0;
    const newMaxScrollY = Math.max(heightInner - heightOuter, 0);

    if (refScroll.current === null) return;

    refScroll.current.outerViewHeight = heightOuter;

    if (newMaxScrollY !== refScroll.current.maxScrollY) {
      refScroll.current.maxScrollY = newMaxScrollY;
      if (forceUpdateOnChange) {
        setUpdateCount((c) => c + 1);
      }
    }
  }
  updateMaxScroll(false);

  useOnMouseScroll(refOuter, (direction) => {
    updateMaxScroll();
    if (direction === "scrollup") {
      setScrollYAndNotify(Math.max(scrollY - 1, 0));
    } else {
      setScrollYAndNotify(
        Math.min(scrollY + 1, refScroll.current?.maxScrollY ?? 0),
      );
    }
  });

  // Provide scroll info to parent:
  if (propsRefScroll) {
    if (typeof propsRefScroll === "function") {
      propsRefScroll(refScroll.current);
    } else {
      propsRefScroll.current = refScroll.current;
    }
  }
  if (scrollY > refScroll.current.maxScrollY) {
    setScrollYAndNotify(refScroll.current.maxScrollY);
  }

  // Re-check max scroll periodically to account for too large scroll (after
  // resize) and for scrollbar size changes.
  useEffect(() => {
    const intervalId = setInterval(() => updateMaxScroll(true), 500);
    return () => clearInterval(intervalId);
  }, []);

  return (
    <Box
      ref={(element) => {
        refOuter.current = element;
        if (propsRefOuter) {
          if (typeof propsRefOuter === "function") {
            propsRefOuter(element);
          } else {
            propsRefOuter.current = element;
          }
        }
      }}
      flexDirection="row"
      overflow="hidden"
    >
      <Box
        flexDirection="column"
        overflow="hidden"
        flexGrow={1}
      >
        <Box
          {...innerBoxProps}
          ref={(element) => {
            refInner.current = element;
            if (propsRefInner) {
              if (typeof propsRefInner === "function") {
                propsRefInner(element);
              } else {
                propsRefInner.current = element;
              }
            }
          }}
          flexShrink={0} // <- Needed to ensure elements don't shrink to sizes like 0.5 rows (which will hide them from view).
          marginTop={-scrollY}
          marginBottom={scrollY} //<- ensure we take the same total amount of space
          overflow="visible"
        >
          {children}
        </Box>
      </Box>
      <Scrollbar
        value={scrollY}
        maxValue={refScroll.current.maxScrollY}
        fullHeight={refScroll.current.outerViewHeight}
        showScrollbar={showScrollbar}
        setScroll={setScrollYAndNotify}
      />
    </Box>
  );
}

export type ButtonProps = {
  label: string;
  focusId?: string;
  onClick?: () => void;
} & Pick<NonNullable<Parameters<typeof useFocus>[0]>, "autoFocus">;
export function Button(props: ButtonProps) {
  const { label, focusId: customFocusId, autoFocus, onClick } = props;
  const ref = useRef<ink.DOMElement>(null);

  const [hovering, setHovering] = useState(false);
  const [clicking, setClicking] = useState(false);

  const overlayInfo = useOverlayInfo();
  const focusId = useRandomId(customFocusId);
  const { isFocused, focus } = useFocus({
    id: focusId,
    autoFocus,
    isActive: overlayInfo.isTopLayer(),
  });

  useInput((input, key) => {
    if (!isFocused || key.ctrl || key.shift || key.meta) return;
    if (key.return || input == " ") {
      setClicking(true);
      Promise.resolve().then(() => setClicking(false));
      if (typeof onClick === "function") {
        onClick();
      }
    }
  });
  useOnMouseClick(ref, (event) => {
    setClicking(event);
    if (event) {
      focus(focusId);
    }
    if (event && typeof onClick === "function") {
      onClick();
    }
  });
  useOnMouseHover(ref, setHovering);

  const border: ink.BoxProps["borderStyle"] = clicking
    ? "double"
    : hovering
    ? "singleDouble"
    : "single";

  return (
    <Box
      gap={1}
      paddingX={1}
      ref={ref}
      borderStyle={border}
      borderColor={isFocused ? "blue" : undefined}
      flexShrink={0}
    >
      <Text>{label}</Text>
    </Box>
  );
}

export type TextFieldProps = {
  /* Text to display when `value` is empty. */
  readonly placeholder?: string;

  /** Listen to user's input. Useful in case there are multiple input components
   * at the same time and input must be "routed" to a specific component. */
  readonly focus?: boolean; // eslint-disable-line react/boolean-prop-naming

  /** Replace all chars and mask the value. Useful for password inputs. */
  readonly mask?: string;

  /** Whether to show cursor and allow navigation inside text input with arrow keys. */
  readonly showCursor?: boolean; // eslint-disable-line react/boolean-prop-naming

  /** Highlight pasted text */
  readonly highlightPastedText?: boolean; // eslint-disable-line react/boolean-prop-naming

  /** Value to display in a text input. */
  readonly value: string;

  /** Function to call when value updates. If `null` then field is read only. */
  readonly onChange: null | ((value: string) => void);

  /** Function to call when `Enter` is pressed, where first argument is a value of the input. */
  readonly onSubmit?: (value: string) => void;
};

export function TextField(props: TextFieldProps) {
  const ref = useRef<ink.DOMElement>(null);

  const overlayInfo = useOverlayInfo();
  const focusId = useRandomId();
  const { isFocused, focus } = useFocus({
    id: focusId,
    isActive: overlayInfo.isTopLayer(),
  });
  useOnMouseClick(ref, (event) => {
    if (event) {
      focus(focusId);
    }
  });
  if (props.onChange === null) {
    props = { ...props, onChange() {} };
  }
  // deno-lint-ignore no-explicit-any
  const fixedProps = props as any;

  return (
    <Box
      ref={ref}
      borderStyle="round"
      flexDirection="row"
      height={3}
      flexGrow={1}
      overflow="hidden"
      borderColor={isFocused ? "blue" : undefined}
    >
      <TextInput {...fixedProps} focus={isFocused} />
    </Box>
  );
}

export type TextAreaProps = Omit<TextFieldProps, "onSubmit" | "mask"> & {
  outerBoxProps?: ink.BoxProps;
};

export function TextArea(props: TextAreaProps) {
  const ref = useRef<ink.DOMElement>(null);
  const refInner = useRef<ink.DOMElement>(null);

  const [scroll, setScroll] = useState(0);
  const refScroll = useRef<ScrollableRef>(null);

  const overlayInfo = useOverlayInfo();
  const focusId = useRandomId();
  const { isFocused, focus } = useFocus({
    id: focusId,
    isActive: overlayInfo.isTopLayer(),
  });
  useOnMouseClick(ref, (event) => {
    if (event) {
      focus(focusId);
    }
  });
  useInput((_input, key) => {
    if (!isFocused || key.ctrl || key.shift || key.meta) return;
    if (key.upArrow) {
      refScroll.current?.applyDeltaY(-1);
    } else if (key.downArrow) {
      refScroll.current?.applyDeltaY(1);
    } else if (key.pageUp) {
      refScroll.current?.applyDeltaY(-refScroll.current.outerViewHeight);
    } else if (key.pageDown) {
      refScroll.current?.applyDeltaY(refScroll.current.outerViewHeight);
    }
  });

  const refLatestWidth = useRef(1000);
  const [, setUpdateCount] = useState(0);
  function updateLatestWidth(forceUpdate = true) {
    if (!refInner.current) return;
    const width = Math.max(ink.measureElement(refInner.current).width, 5);
    if (width !== refLatestWidth.current) {
      refLatestWidth.current = width;
      if (forceUpdate) {
        setUpdateCount((c) => c + 1);
      }
    }
  }
  updateLatestWidth(false);

  // Prevent slow down by disabling word wrap and using virtual scroll, i.e. not
  // rendering text outside the current view:
  const virtualScroll = props.value.length > 500_00; // <-- about 500 lines if each line is 100 characters.

  const refWrappedText = useRef<{ width: number; wrappedText: string } | null>(
    null,
  );
  if (
    !virtualScroll || refWrappedText.current?.width !== refLatestWidth.current
  ) {
    refWrappedText.current = null;
  }
  const lines = (refWrappedText.current?.wrappedText ?? props.value).split(
    "\n",
  );

  useEffect(() => {
    if (!virtualScroll) return;
    const abortSource = new AbortController();
    const width = refLatestWidth.current;
    asyncWrapText(
      props.value,
      width,
      "wrap",
      abortSource.signal,
    )
      .then((wrappedText) => {
        refWrappedText.current = { wrappedText, width };
        setUpdateCount((c) => c + 1);
      }).catch(() => {});
    return () => abortSource.abort();
  }, [virtualScroll, refLatestWidth.current, props.value]);

  let inner: React.ReactNode;
  let virtualHeight = 0;
  if (virtualScroll) {
    const textChunks: React.ReactNode[] = [];
    let lineIndex = 0;
    let scrollPos = 0;
    let rowCount = 0;
    let text: string[] = [];
    while (true) {
      if (rowCount >= 100 || (lineIndex >= lines.length && rowCount > 0)) {
        const extraDrawMargin = 5; // <- slight delay before hidden text is shown sometimes, so add a few rows outside the view to be safe.

        const notVisible = scrollPos + rowCount + extraDrawMargin < scroll ||
          scrollPos - extraDrawMargin >=
            scroll + (refScroll.current?.outerViewHeight ?? 0);

        textChunks.push(
          <Box
            key={textChunks.length}
            flexDirection="column"
            minHeight={rowCount}
          >
            {notVisible
              ? null
              : text.map((line, ix) => (
                <Text key={ix} wrap="truncate-end">{line || " "}</Text>
              ))}
          </Box>,
        );
        scrollPos += rowCount;
        rowCount = 0;
        text = [];
      }
      if (lineIndex >= lines.length) break;
      const line = lines[lineIndex];
      // Word wrapping was done previously:
      if (refWrappedText.current !== null) {
        text.push(line);
        rowCount++;
      } else {
        // Otherwise we do naive manual wrap of long lines (might be slightly off for non-ascii):
        for (let ix = 0; ix <= line.length; ix += refLatestWidth.current) {
          // TODO: maybe use something like https://www.npmjs.com/package/widest-line to ensure lines are never too long
          text.push(
            line.slice(
              ix,
              Math.min(line.length, ix + refLatestWidth.current),
            ),
          );
          rowCount++;
        }
      }
      lineIndex++;
    }
    virtualHeight = scrollPos;
    inner = textChunks;
  } else {
    inner = <Text>{props.value || " "}</Text>;
  }

  useEffect(() => {
    if (!virtualScroll) return;
    const intervalId = setInterval(() => updateLatestWidth(), 1000);
    return () => clearInterval(intervalId);
  }, [virtualScroll]);

  return (
    <Box
      {...props.outerBoxProps}
      borderStyle="round"
      flexDirection="column"
      overflow="hidden"
      borderColor={isFocused ? "blue" : undefined}
    >
      <Scrollable
        refOuter={ref}
        refInner={(element) => {
          refInner.current = element;
          updateLatestWidth();
        }}
        refScroll={refScroll}
        onScrollY={setScroll}
        innerBoxProps={{
          height: virtualScroll ? virtualHeight : undefined,
          flexDirection: "column",
        }}
      >
        {inner}
      </Scrollable>
    </Box>
  );
}

export type CheckboxStyle =
  | "simple"
  | "large"
  | "green"
  | "purple"
  | "mark"
  | "thick-mark";

export type CheckboxProps = React.PropsWithChildren & {
  /** If the checkbox is currently checked. */
  checked: boolean;
  /** Set the new state for the checkbox. If `null` then the checkbox is readonly and won't be focusable using the tab key (but can still be focused using the mouse). */
  setChecked: null | ((value: boolean) => void);
  /** A convenient shorthand for a single child of the type: `<Text>Label text here</Text>`. */
  label?: string;
  /** What style to use for the checkbox. */
  style?: CheckboxStyle;
} & ink.BoxProps;
export function Checkbox(props: CheckboxProps) {
  const { checked, setChecked, label, style, ...other } = props;

  const ref = useRef<ink.DOMElement>(null);

  const currentCheck = useRef(checked);
  currentCheck.current = checked;

  const overlayInfo = useOverlayInfo();
  const focusId = useRandomId();
  const { isFocused, focus } = useFocus({
    id: focusId,
    isActive: Boolean(setChecked) && overlayInfo.isTopLayer(),
  });

  useInput((input, key) => {
    if (!isFocused || key.ctrl || key.shift || key.meta) return;
    if (key.return || input == " ") {
      if (setChecked) {
        setChecked(!currentCheck.current);
      }
    }
  });
  useOnMouseClick(ref, (event) => {
    if (event) {
      focus(focusId);
      if (setChecked) {
        setChecked(!currentCheck.current);
      }
    }
  });

  let large = false;
  let onChar: string;
  let offChar: string;
  let highlightLabel = false;
  switch (style) {
    default:
    case "simple":
      onChar = "☑";
      offChar = "☐ ";
      break;
    case "green":
      onChar = "✅";
      offChar = "❎";
      highlightLabel = true;
      break;
    case "purple":
      onChar = "✔️  ";
      offChar = "✖️  ";
      highlightLabel = true;
      break;
    case "mark":
      onChar = "✓ ";
      offChar = "✗ ";
      break;
    case "thick-mark":
      onChar = "✔";
      offChar = "✘ ";
      break;
    case "large":
      onChar = " ✔ ";
      offChar = "    ";
      large = true;
      break;
  }

  const checkmark = (
    <Text color={isFocused && !large ? "blue" : undefined}>
      {checked ? onChar : offChar}
    </Text>
  );
  return (
    <Box {...other} alignItems="center" ref={ref}>
      {large
        ? ( // Large:
          <Box
            borderStyle="round"
            borderColor={isFocused ? "blue" : undefined}
            marginRight={1}
            flexShrink={0}
            overflow="hidden"
          >
            {checkmark}
          </Box>
        )
        // Simple:
        : (
          <Box alignSelf="flex-start" flexShrink={0} overflow="hidden">
            {checkmark}
          </Box>
        )}
      {label
        ? (
          <Text color={highlightLabel && isFocused ? "blue" : undefined}>
            {label}
          </Text>
        )
        : undefined}
      {props.children}
    </Box>
  );
}

interface SelectableListItemState {
  /** The item's element. Used to scroll to the active item. */
  element: React.RefObject<ink.DOMElement | null>;
  /** The id of this item. Will never be modified. */
  id: string;
  /** Latest name of a selectable item. Used to show a selected item's name elsewhere such as in a {@link DropDown}. */
  name: React.RefObject<string>;
  /** This is called by {@link SelectableList} whenever the active/selected state of this item has changed. */
  update(): void;
}
interface SelectableListState {
  /** All {@link SelectableListItem} inside the {@link SelectableList} */
  items: SelectableListItemState[];
  /** The currently selected items. */
  selectedIds: string[];
  /** The item that has keyboard focus. This state is not exposed outside of the {@link SelectableList} itself. */
  activeId: string | null;
  /** Move keyboard focus to a different item. */
  setActiveId(id: string | null): void;
  /** Toggle if a specific id is selected. */
  toggleSelected(id: string, wasDestroyed: boolean): void;
  /** Set the selected items. */
  setSelectedIds(ids: string[], wasDestroyed: boolean): void;
  /** Called by a {@link SelectableListItem} if its name changes. Also initially when first registered. */
  onNameChange(id: string, name: string): void;
}
const SelectableListContext = React.createContext<SelectableListState | null>(
  null,
);

export type SelectableListItemProps = {
  name: string;
  id: string;
  onClick?: () => void;
};
export function SelectableListItem(props: SelectableListItemProps) {
  const { id, name, onClick } = props;

  const ref = useRef<ink.DOMElement>(null);
  const refName = useRef("");
  const [, setUpdate] = useState(0);
  const context = useContext(SelectableListContext);
  if (!context) {
    throw new Error(
      `SelectableListItem component can only be used inside a SelectableList`,
    );
  }
  if (context.activeId === null) {
    context.activeId = id;
  }
  const selected = context.selectedIds.includes(id);
  const active = context.activeId === id;

  const update = () => setUpdate((c) => c + 1);
  useEffect(() => {
    const obj: SelectableListItemState = {
      element: ref,
      id,
      name: refName,
      update,
    };
    context.items.push(obj);
    return () => {
      const index = context.items.indexOf(obj);
      if (index >= 0) context.items.splice(index, 1);
      if (context.activeId === id) {
        context.activeId = context.items[Math.max(0, index - 1)]?.id ?? null;
      }
      if (context.selectedIds.includes(id)) {
        context.toggleSelected(id, true);
      }
    };
  }, [id]);

  if (refName.current !== name) {
    refName.current = name;
    Promise.resolve().then(() => context.onNameChange(id, name));
  }

  useOnMouseClick(ref, (event) => {
    if (!event) return;
    context.setActiveId(id);
    context.toggleSelected(id, false);
    update();
    if (typeof onClick === "function") {
      onClick();
    }
  });
  const [hovering, setHovering] = useState(false);
  useOnMouseHover(ref, setHovering);

  let backgroundColor: ink.TextProps["backgroundColor"];
  if (hovering) {
    backgroundColor = selected ? "blueBright" : "cyan";
  } else if (selected) {
    backgroundColor = active ? "blueBright" : "blue";
  } else {
    backgroundColor = active ? "blackBright" : undefined;
  }
  return (
    <Box
      ref={ref}
      height={1}
    >
      <Text backgroundColor={backgroundColor}>{name}</Text>
    </Box>
  );
}

export type SelectableListRef = {
  /** Get the name of a selectable item given its id.
   *
   * If the list item hasn't been registered yet then `null` is returned. In that case the name might be provided later by the {@link SelectableListProps.onNameChange}
   */
  getName(id: string): string | null;
  /** Move keyboard selection to a specific item. */
  keyboardSelectItem(id: string): boolean;
};

export type SelectableListProps = React.PropsWithChildren & {
  selectedIds: string[];
  setSelectedIds: (ids: string[], wasDestroyed: boolean) => void;
  /** When selecting using the keyboard ensure this many rows are visible below
   * and above the active item. */
  scrollMargin?: number;
  outerBoxProps?: ink.BoxProps;
  innerBoxProps?: ink.BoxProps;
  /** Get access to imperative state for the selectable list. */
  refSelectable?: React.Ref<SelectableListRef>;
  /** This will be called when new items are registered and whenever an existing item changes its name. */
  onNameChange?: (id: string, name: string) => void;
} & Pick<NonNullable<Parameters<typeof useFocus>[0]>, "autoFocus">;

export function SelectableList(
  props: SelectableListProps,
) {
  const {
    selectedIds,
    setSelectedIds,
    scrollMargin = 1,
    autoFocus,
    innerBoxProps = {},
    outerBoxProps = {},
    refSelectable,
    onNameChange = () => {},
    children,
  } = props;

  const refOuter = useRef<ink.DOMElement>(null);
  const refInner = useRef<ink.DOMElement>(null);
  const refScroll = useRef<ScrollableRef>(null);
  const refList = useRef<SelectableListState>(null);
  const refSelectableInner = useRef<SelectableListRef>(null);
  const overlayInfo = useOverlayInfo();

  const [activeId, setActiveId] = useState<string | null>(null);

  function toggleSelected(id: string, wasDestroyed: boolean): void {
    if (!refList.current) return;
    let list = refList.current.selectedIds;
    if (list.includes(id)) {
      list = list.filter((id2) => id2 !== id);
    } else {
      list = list.concat(id);
    }
    refList.current.setSelectedIds(list, wasDestroyed);
  }

  if (!refList.current) {
    refList.current = {
      activeId: null,
      items: [],
      selectedIds: [],
      setActiveId,
      toggleSelected,
      setSelectedIds,
      onNameChange,
    };
  }
  refList.current.activeId = activeId;
  refList.current.selectedIds = selectedIds;
  refList.current.setSelectedIds = setSelectedIds;
  refList.current.onNameChange = onNameChange;
  const list = refList.current;

  function getActiveIndex() {
    if (list.activeId === null) return null;
    for (let i = 0; i < list.items.length; i++) {
      if (list.items[i].id === list.activeId) {
        return i;
      }
    }
    return null;
  }
  function changeActiveId(
    newActiveItem: SelectableListItemState,
    prevIndex?: number | null,
  ) {
    if (prevIndex === undefined) {
      prevIndex = getActiveIndex();
    }
    setActiveId(newActiveItem.id);
    list.activeId = newActiveItem.id;
    if (prevIndex !== null) {
      list.items[prevIndex].update();
    }
    newActiveItem.update();
    refScroll.current?.ensureElementIsVisible(
      newActiveItem.element.current,
      scrollMargin,
    );
  }

  if (!refSelectableInner.current) {
    refSelectableInner.current = {
      getName(id) {
        for (const item of list.items) {
          if (item.id === id) {
            return item.name.current;
          }
        }
        return null;
      },
      keyboardSelectItem(id) {
        let newActiveItem: SelectableListItemState | null = null;
        for (const item of list.items) {
          if (item.id === id) {
            newActiveItem = item;
            break;
          }
        }
        if (newActiveItem === null) return false;
        changeActiveId(newActiveItem);
        return true;
      },
    };
  }

  const focusId = useRandomId();
  const { isFocused, focus } = useFocus({
    id: focusId,
    isActive: overlayInfo.isTopLayer(),
    autoFocus,
  });

  useInput((input, key) => {
    if (!isFocused) return;
    if (key.upArrow) {
      const prev = getActiveIndex();
      let newActiveItem: SelectableListItemState;
      if (prev === null) {
        if (list.items.length > 0) {
          newActiveItem = list.items[0];
        } else {
          return;
        }
      } else if (prev <= 0) {
        return;
      } else {
        newActiveItem = list.items[prev - 1];
      }
      changeActiveId(newActiveItem, prev);
    } else if (key.downArrow) {
      const prev = getActiveIndex();
      let newActiveItem: SelectableListItemState;
      if (prev === null) {
        if (list.items.length > 0) {
          newActiveItem = list.items[0];
        } else {
          return;
        }
      } else if (prev >= list.items.length - 1) {
        return;
      } else {
        newActiveItem = list.items[prev + 1];
      }
      changeActiveId(newActiveItem, prev);
    } else if (key.return || input == " ") {
      if (list.activeId !== null) {
        toggleSelected(list.activeId, false);
      }
    } else if (key.pageUp) {
      refScroll.current?.applyDeltaY(-refScroll.current.outerViewHeight);
    } else if (key.pageDown) {
      refScroll.current?.applyDeltaY(refScroll.current.outerViewHeight);
    }
  });
  useOnMouseClick(refOuter, (event) => {
    if (event) {
      focus(focusId);
    }
  });

  if (refSelectable) {
    if (typeof refSelectable === "function") {
      refSelectable(refSelectableInner.current);
    } else {
      refSelectable.current = refSelectableInner.current;
    }
  }

  return (
    <SelectableListContext.Provider value={refList.current}>
      <Box
        paddingX={3}
        borderStyle="round"
        borderColor={isFocused ? "blue" : "grey"}
        {...outerBoxProps}
      >
        <Scrollable
          refOuter={refOuter}
          refScroll={refScroll}
          refInner={refInner}
          innerBoxProps={{
            flexDirection: "column",
            ...innerBoxProps,
          }}
        >
          {children}
        </Scrollable>
      </Box>
    </SelectableListContext.Provider>
  );
}

type DropDownPopupProps = {
  selectedId: string;
  overlapButton: boolean;
  onOverlayClose: (id: string | null) => void;
  /** The list that is shown when the drop down is pressed. */
  refListPopup?: React.Ref<ink.DOMElement>;
  /** The drop down button that the popup should be attached to. */
  anchorRef: React.RefObject<ink.DOMElement | null>;
} & React.PropsWithChildren;
function DropDownPopup(props: DropDownPopupProps) {
  const {
    selectedId,
    overlapButton,
    onOverlayClose,
    refListPopup,
    anchorRef,
    children,
  } = props;

  const refBackground = useRef<ink.DOMElement>(null);
  const refWizard = useRef<ink.DOMElement>(null);

  useOnMouseClick(refBackground, (event, mousePos) => {
    if (
      // Click on background:
      event &&
      // Not click on wizard:
      !isIntersecting({
        region: getElementRegion(refWizard.current),
        position: mousePos,
      })
    ) {
      onOverlayClose(null);
    }
  });
  useInput((_input, key) => {
    if (!key.ctrl && !key.shift && key.escape) {
      onOverlayClose(null);
    }
  });

  /** Handle selected item inside popup. */
  function setSelected(ids: string[], wasDestroyed: boolean) {
    if (wasDestroyed) return; // Drop down is being closed
    if (ids.length === 0) {
      // Deselected current item by clicking it again => want to keep the current selection.
      onOverlayClose(null);
    } else {
      // Newest selection at the end of the array.
      onOverlayClose(ids[ids.length - 1]);
    }
  }

  // Track popup position:
  const consoleSize = useConsoleSize();
  const latestPos = useRef<Position & { above: boolean }>(null);
  const [, setUpdate] = useState(0);
  function updatePosition(needUpdate = true) {
    const anchorRegion = getElementRegion(anchorRef.current);
    if (!anchorRegion) return;
    const shouldBeAbove = anchorRegion.top + 3 > consoleSize.rows / 2;
    if (
      latestPos.current?.x === anchorRegion.left &&
      latestPos.current?.y === anchorRegion.top &&
      latestPos.current?.above === shouldBeAbove
    ) {
      return;
    }
    latestPos.current = {
      x: anchorRegion.left,
      y: anchorRegion.top,
      above: shouldBeAbove,
    };
    if (needUpdate) setUpdate((c) => c + 1);
  }
  useEffect(() => {
    const intervalId = setInterval(() => updatePosition(), 100);
    return () => clearInterval(intervalId);
  }, [anchorRef, consoleSize]);
  updatePosition(false);

  const hasSetActiveItem = useRef(false);

  const above = Boolean(latestPos.current?.above);
  return (
    <Box
      ref={refBackground}
      height="100%"
      width="100%"
      alignItems="flex-start"
      flexDirection="column"
    >
      {above ? <Spacer /> : null}
      <Box
        ref={(element) => {
          refWizard.current = element;
          if (refListPopup) {
            if (typeof refListPopup === "function") {
              refListPopup(element);
            } else {
              refListPopup.current = element;
            }
          }
        }}
        marginLeft={latestPos.current?.x ?? 0}
        marginTop={above
          ? 0
          : (latestPos.current?.y ?? 0) + (overlapButton ? 0 : 3)}
        marginBottom={above
          ? consoleSize.rows - 1 - (latestPos.current?.y ?? 0) -
            (overlapButton ? 3 : 0)
          : 0}
        flexDirection="column"
        alignItems="center"
      >
        <SelectableList
          selectedIds={[selectedId]}
          setSelectedIds={setSelected}
          autoFocus
          outerBoxProps={{ borderStyle: "round" }}
          innerBoxProps={{ flexDirection: "column" }}
          refSelectable={(refSelectable) => {
            // This ensures keyboard is focused on the currently selected item when the drop down is first opened:
            if (!refSelectable) return;
            Promise.resolve().then(() => {
              if (!refSelectable) return;
              if (!hasSetActiveItem.current) {
                hasSetActiveItem.current = refSelectable.keyboardSelectItem(
                  selectedId,
                );
              }
            });
          }}
        >
          {children}
        </SelectableList>
      </Box>
    </Box>
  );
}
const DropDownOverlay = asOverlay(DropDownPopup, {
  refPropKeys: ["refListPopup"],
});

export type DropDownProps = React.PropsWithChildren & {
  selectedId: string;
  setSelectedId: (id: string) => void;
  /** If true the opened menu will completely cover the button itself. */
  overlapWithButton?: boolean;
};

export function DropDown(props: DropDownProps) {
  const { children, selectedId, setSelectedId, overlapWithButton = true } =
    props;
  const ref = useRef<ink.DOMElement>(null);
  const refSelectable = useRef<SelectableListRef>(null);
  const refLastSelectedId = useRef(props.selectedId);

  const [hovering, setHovering] = useState(false);
  const [clicking, setClicking] = useState(false);
  const [isOpen, setIsOpen] = useState(false);
  const [selectedName, setSelectedName] = useState("");

  const overlayInfo = useOverlayInfo();
  const focusId = useRandomId();
  const { isFocused, focus } = useFocus({
    id: focusId,
    isActive: overlayInfo.isTopLayer(),
  });

  useInput((input, key) => {
    if (!isFocused || key.ctrl || key.shift || key.meta) return;
    if (key.return || input == " ") {
      setClicking(true);
      setIsOpen(!isOpen);
      Promise.resolve().then(() => setClicking(false));
    }
  });
  useOnMouseClick(ref, (event) => {
    setClicking(event);
    if (event) {
      focus(focusId);
      setIsOpen(!isOpen);
    }
  });
  useOnMouseHover(ref, setHovering);

  if (refLastSelectedId.current !== selectedId) {
    refLastSelectedId.current = selectedId;
    if (refSelectable.current) {
      setSelectedName(refSelectable.current.getName(selectedId) ?? "");
    } else {
      setSelectedName("");
    }
  }

  const border: ink.BoxProps["borderStyle"] = clicking
    ? "double"
    : hovering
    ? "singleDouble"
    : "single";
  return (
    <Box
      gap={1}
      paddingX={1}
      ref={ref}
      borderStyle={border}
      borderColor={isFocused ? "blue" : undefined}
      flexShrink={0}
    >
      <Box flexDirection="column">
        <Text>{selectedName}</Text>
        <SelectableList // <- this list ensures that the button has the width of the largest item in the list.
          selectedIds={[]}
          setSelectedIds={() => {}}
          outerBoxProps={{ height: 0, paddingX: 0, borderStyle: undefined }}
          refSelectable={refSelectable} // <- use this list since the other one is only sometimes rendered.
          onNameChange={(id, name) => {
            if (id === selectedId) {
              setSelectedName(name);
            }
          }}
        >
          {children}
        </SelectableList>
      </Box>
      <Box width={1} marginLeft={1} overflow="hidden">
        <Text>▼</Text>
      </Box>
      <DropDownOverlay
        selectedId={selectedId}
        overlapButton={overlapWithButton}
        isOverlayOpen={isOpen}
        onOverlayClose={(id) => {
          setIsOpen(false);
          focus(focusId);
          if (id !== null) {
            setSelectedId(id);
          }
        }}
        anchorRef={ref}
      >
        {children}
      </DropDownOverlay>
    </Box>
  );
}
