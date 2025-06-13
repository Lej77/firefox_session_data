import { ink, React } from "./deps.ts";
import { useConsoleSize } from "./console-size.tsx";
import { getElementRegion } from "./position.ts";

const { useEffect, useState, useRef, useMemo } = React;
const { Box, Text } = ink;

export interface Region {
  left: number;
  top: number;
  width: number;
  height: number;
}
export interface OverlayElement {
  enabled: boolean;

  /** The area that should be considered "covered" by the overlay and so content
   * from previous layers should be cleared from this area. */
  getRegionsToClear(): Region[];

  /** Render the overlay. This node will be placed at the very top of a new
   * layer. If you want it to placed elsewhere then use margin to position it
   * correctly.
   *
   * If you use location info from a ref to an element rendered here when
   * determining areas to clear then ensure you call `checkClearRegions` once
   * the ref has been set. {@link React.Ref} can be a function so this should be
   * quite easy. */
  renderOverlay(checkClearRegions: () => void): React.ReactNode;
}
interface OverlayCollection {
  addOverlay(overlay: OverlayElement): void;
  removeOverlay(overlay: OverlayElement): void;
  overlays: OverlayElement[];
  forceUpdates: (() => void)[];
}

function Layer(
  props: { overlay: OverlayElement },
) {
  const { overlay } = props;

  const consoleSize = useConsoleSize();
  const [, setCount] = useState(0);
  const refClearRegions = useRef<string>(null);

  // Draw empty spaces over certain parts of the previous layer:
  const erasedAreas: React.ReactNode[] = [];
  const regionsToClear = overlay.getRegionsToClear();

  refClearRegions.current = JSON.stringify(regionsToClear);

  for (const region of regionsToClear) {
    const maxHeight = Math.max(
      0,
      consoleSize.rows - 1 - Math.max(region.top, 0),
    );
    const height = Math.max(0, Math.min(maxHeight, region.height));

    const maxWidth = Math.max(
      0,
      consoleSize.columns - 1 - Math.max(region.left, 0),
    );
    const width = Math.max(0, Math.min(maxWidth, region.width));
    const x = Math.max(0, region.left);
    const y = Math.max(0, region.top);
    erasedAreas.push(
      <Box
        marginTop={y}
        height={height}
        marginBottom={-height - y}
        marginLeft={x}
        width={width}
      >
        <Text truncate-end>
          {" ".repeat(width * height)}
        </Text>
      </Box>,
    );
  }

  const checkClearRegions = () => {
    // Redraw the clear regions if they have changed:
    const regions = overlay.getRegionsToClear();
    if (regions.length === 0) return;

    const regionsToClearJson = JSON.stringify(regions);
    if (refClearRegions.current !== regionsToClearJson) {
      setCount((prev) => prev + 1);
    }
  };

  // Draw the actual overlay on top of the erased parts:
  const overlayUi = (
    <CurrentOverlayContext.Provider value={overlay}>
      <Box
        width={consoleSize.columns - 1}
        height={consoleSize.rows - 1}
        marginBottom={-(consoleSize.rows - 1)}
        overflow="hidden"
      >
        {overlay.renderOverlay(checkClearRegions)}
      </Box>
    </CurrentOverlayContext.Provider>
  );

  return (
    <>
      {...erasedAreas}
      {overlayUi}
    </>
  );
}

const CurrentOverlayContext = React.createContext<OverlayElement | null>(null);
const OverlayContext = React.createContext<OverlayCollection | null>(null);

export function OverlayProvider(
  props: React.PropsWithChildren & ink.BoxProps,
) {
  const { children, ...boxProps } = props;

  const consoleSize = useConsoleSize();
  const [overlays, setOverlays] = useState<OverlayElement[]>([]);
  const collection = useMemo<OverlayCollection>(() => {
    return {
      addOverlay(overlay: OverlayElement): void {
        setOverlays((prev) =>
          prev.includes(overlay) ? prev : prev.concat(overlay)
        );
        for (const update of collection.forceUpdates.slice()) {
          update();
        }
      },
      removeOverlay(overlay: OverlayElement): void {
        setOverlays((prev) => prev.filter((o) => o !== overlay));
        for (const update of collection.forceUpdates.slice()) {
          update();
        }
      },
      renderingOverlay: null,
      overlays: [],
      forceUpdates: [],
    };
  }, []);
  collection.overlays = overlays;

  const firstLayer = (
    <Box
      {...boxProps}
      width={consoleSize.columns - 1}
      height={consoleSize.rows - 1}
      marginBottom={-(consoleSize.rows - 1)}
      overflow="hidden"
    >
      {children}
    </Box>
  );

  // Overlays:
  const otherLayers: React.ReactNode[] = [];
  for (const overlay of overlays) {
    otherLayers.push(<Layer overlay={overlay} />);
  }

  return (
    <OverlayContext.Provider value={collection}>
      <Box
        width={consoleSize.columns - 1}
        height={consoleSize.rows - 1}
        overflow="hidden"
        flexDirection="column"
      >
        {firstLayer}
        {...otherLayers}
      </Box>
    </OverlayContext.Provider>
  );
}

/** Specify a new layer that will be rendered on top of all other components.
 * Other components will be disabled while the new layer is enabled. */
export function useOverlay(overlay: OverlayElement) {
  const context = React.useContext(OverlayContext);
  if (!context) {
    throw new Error(
      "useOverlay must only be used within children of <OverlayProvider />",
    );
  }

  // Create object that lives for the duration of the component:
  const refInfo = useRef<OverlayElement>(null);
  if (!refInfo.current) {
    refInfo.current = { ...overlay };
  }
  // Ensure we always use the latest callbacks:
  Object.assign(refInfo.current, overlay);

  // Register/unregister overlay:
  useEffect(() => {
    const info = refInfo.current;
    if (info === null || !overlay.enabled) return;
    context.addOverlay(info);
    return () => context.removeOverlay(info);
  }, [context, refInfo.current, overlay.enabled]);
}

export type OverlayInfoHookOptions = {
  /** Force the component to be re-rendered if the current top layer changes.
   * Useful to update other hooks such as: `useFocus({ isActive: isTopLayer, })`
   */
  updateOnLayerChange?: boolean;
};

/** Get info about the layer that the current component is a part of. */
export function useOverlayInfo(options?: OverlayInfoHookOptions) {
  const { updateOnLayerChange = true } = options || {};

  const collection = React.useContext(OverlayContext);
  const current = React.useContext(CurrentOverlayContext);

  const [, setCount] = useState(0); // <- force update
  useEffect(() => {
    if (!collection || !updateOnLayerChange) return;
    const forceUpdate = () => setCount((c) => c + 1);
    collection.forceUpdates.push(forceUpdate);
    return () => {
      const index = collection.forceUpdates.indexOf(forceUpdate);
      if (index >= 0) collection.forceUpdates.splice(index, 1);
    };
  }, [collection, Boolean(updateOnLayerChange)]);

  return {
    isTopLayer() {
      return Boolean(
        collection === null || collection.overlays.length === 0 ||
          collection.overlays[collection.overlays.length - 1] === current,
      );
    },
  };
}

export type AsOverlayOptions<P> = {
  /** Properties for the rendered component that should be given
   * {@link React.RefCallback<ink.DOMElement | null>}. The referenced elements
   * will be considered the non-transparent part of the overlay and so the
   * region behind those elements will be cleared before the component is rendered.
   *
   * The default name is `refOverlay`. */
  refPropKeys?: (keyof P)[];
};

/** Transform a React component into an overlay. The `refOverlay` property should be set to the element that won't be transparent. */
export function asOverlay<P>(
  Render: React.FunctionComponent<P>,
  options?: AsOverlayOptions<P>,
): React.FunctionComponent<P & { isOverlayOpen: boolean }> {
  const { refPropKeys = ["refOverlay"] } = options || {};
  return function RenderOverlay(props) {
    const ref = useRef<Map<string, ink.DOMElement | null>>(null);
    if (!ref.current) {
      ref.current = new Map();
    }

    useOverlay({
      enabled: props.isOverlayOpen,
      getRegionsToClear() {
        if (!ref.current) return [];

        const locations = [];
        for (const element of ref.current.values()) {
          const location = getElementRegion(element);
          if (location) locations.push(location);
        }
        return locations;
      },
      renderOverlay(checkClearRegions) {
        const refProps: {
          [key: string]: React.RefCallback<ink.DOMElement | null>;
        } = {};
        for (const refName of refPropKeys) {
          refProps[refName as string] = (node) => {
            if (!ref.current) return;
            ref.current.set(refName as string, node);
            checkClearRegions();
          };
        }
        return (
          <Render
            {...props}
            {...refProps}
          />
        );
      },
    });
    return null;
  };
}
