import { ink } from "./deps.ts";

export type Position = { x: number; y: number };

/** A region of the terminal. Usually the region covered by a specific {@link ink.DOMElement} */
export type Region = {
  /** Distance above the region to the top of the screen. */
  top: number;
  /** Distance to the left of the region to the left edge of the screen. */
  left: number;
  /** Width of the region. */
  width: number;
  /** Height of the region. */
  height: number;
};

/** Check if a position is inside a region. Useful to check if a mouse click was
 * inside a specific element.
 *
 *  Adapted from code at
 * <https://github.com/zenobi-us/ink-mouse/blob/master/src/ink-mouse/isIntersecting.ts>
 */
export function isIntersecting(params: {
  position: Position;
  region: Region | null;
}) {
  let { region, position: { x, y } } = params;
  if (!region) {
    return false;
  }

  // Position seems slightly off for some reason:
  y--;
  x--;

  const { left, top, width, height } = region;
  const isOutsideHorizontally = x < left || x > left + width - 1;
  const isOutsideVertically = y < top || y > top + height - 1;

  return !isOutsideHorizontally && !isOutsideVertically;
}

export function emptyRegion(): Region {
  return { top: 0, left: 0, width: 0, height: 0 };
}

/**
 * Get the position of the element.
 *
 * Adapted from code at:
 * <https://github.com/zenobi-us/ink-mouse/blob/406716cfcdbcef910eeeece84851a5ce1659023a/src/ink-mouse/useElementPosition.ts#L50-L126>
 *
 * @export
 * @param {(ink.DOMElement | null)} node The child not to be measured.
 * @param {(ink.DOMElement | null)} [relativeToParent] If specified then
 * {@link Area.top} and  {@link Area.left} will be relative to this parent node.
 * @return {Area} The child node's location relative to the parent node or the
 * terminal's top left corner if no parent is specified.
 */
export function getElementRegion(
  node: ink.DOMElement | null,
  relativeToParent?: ink.DOMElement | null,
) {
  if (!node || !node.yogaNode) {
    return null;
  }
  const elementLayout = node.yogaNode.getComputedLayout();

  const parent = walkParentPosition(node, relativeToParent);

  const location = {
    left: elementLayout.left + parent.x,
    top: elementLayout.top + parent.y,
    width: elementLayout.width,
    height: elementLayout.height,
  };

  return location;
}

/**
 * Walk the parent ancestry to get the position of the element.
 *
 * Since InkNodes are relative by default and because Ink does not
 * provide precomputed x and y values, we need to walk the parent and
 * accumulate the x and y values.
 *
 * I only discovered this by debugging the getElementPosition before
 * and after wrapping the element in a Box with padding:
 *
 *  - before padding: { left: 0, top: 0, width: 10, height: 1 }
 *  - after padding: { left: 2, top: 0, width: 10, height: 1 }
 *
 * It's still a mystery why padding on a parent results in the child
 * having a different top value. `#todo`
 */
function walkParentPosition(
  node: ink.DOMElement,
  topParent?: ink.DOMElement | null,
) {
  let parent = node.parentNode;
  let x = 0;
  let y = 0;

  while (parent) {
    if (!parent.yogaNode || parent === topParent) {
      return { x, y };
    }

    const layout = parent.yogaNode.getComputedLayout();
    x += layout.left;
    y += layout.top;

    parent = parent.parentNode;
  }
  return { x, y };
}
