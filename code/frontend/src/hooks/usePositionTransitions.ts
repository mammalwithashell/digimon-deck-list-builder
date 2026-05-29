import { useLayoutEffect, useRef } from 'react';

/**
 * FLIP-animation hook (First, Last, Invert, Play) for a keyed set of
 * DOM nodes that may move between grid cells across renders.
 *
 * Usage: in the parent, populate `keys` with stable identity strings
 * in render order, then attach the returned `registerNode(key)` ref
 * callback to each rendered element. Between renders the hook captures
 * each node's bounding rect, applies an inverse `translate` after the
 * commit, and animates back to zero translation — producing a smooth
 * slide for elements that changed grid position.
 *
 * Elements whose key didn't exist last render don't animate (the parent
 * can use its own enter animation for those). Elements whose key
 * disappears are forgotten on the next render.
 *
 * Design constraint: this hook does *not* control opacity, enter, or
 * exit animations. It only smooths position changes for elements that
 * persisted across renders. Combine with `.animate-card-play-in` for
 * the enter case.
 */
export function usePositionTransitions(
  keys: string[],
  options: { durationMs?: number; easing?: string } = {},
) {
  const durationMs = options.durationMs ?? 250;
  const easing = options.easing ?? 'cubic-bezier(0.2, 0.8, 0.2, 1)'; // ease-out
  const nodesRef = useRef<Map<string, HTMLElement>>(new Map());
  const prevRectsRef = useRef<Map<string, DOMRect>>(new Map());

  // Capture current rects BEFORE React commits the new render (using
  // useLayoutEffect's pre-paint phase on the next render is sufficient
  // — we capture in the *current* render's layout effect, then on the
  // next render's layout effect we have both the old rects (from this
  // run) and the new rects (from the new run).
  useLayoutEffect(() => {
    const prevRects = prevRectsRef.current;
    const nodes = nodesRef.current;

    // For every node that exists now AND existed before, animate from
    // its old position to its new position.
    for (const key of keys) {
      const node = nodes.get(key);
      if (!node) continue;
      const prevRect = prevRects.get(key);
      const newRect = node.getBoundingClientRect();
      if (prevRect) {
        const dx = prevRect.left - newRect.left;
        const dy = prevRect.top - newRect.top;
        if (Math.abs(dx) > 0.5 || Math.abs(dy) > 0.5) {
          // Skip the animation if the node is currently mid-play-in
          // (the parent's enter animation owns this case).
          if (node.classList.contains('animate-card-play-in')) {
            // No FLIP on first appearance.
          } else {
            // Invert: jump back to the old position with no transition.
            node.style.transition = 'none';
            node.style.transform = `translate(${dx}px, ${dy}px)`;
            // Play: on next frame, animate the inverse away.
            requestAnimationFrame(() => {
              node.style.transition = `transform ${durationMs}ms ${easing}`;
              node.style.transform = '';
              // Clear inline transition after it completes so subsequent
              // re-renders don't inherit a stale transition spec.
              window.setTimeout(() => {
                if (node.style.transition.includes('transform')) {
                  node.style.transition = '';
                }
              }, durationMs + 50);
            });
          }
        }
      }
      prevRects.set(key, newRect);
    }

    // Forget rects for keys that no longer exist.
    const keySet = new Set(keys);
    for (const oldKey of Array.from(prevRects.keys())) {
      if (!keySet.has(oldKey)) {
        prevRects.delete(oldKey);
        nodes.delete(oldKey);
      }
    }
  }, [keys, durationMs, easing]);

  /** Ref callback for a single key. Pass to `ref={registerNode(key)}`. */
  const registerNode = (key: string) => (node: HTMLElement | null) => {
    if (node) {
      nodesRef.current.set(key, node);
    } else {
      nodesRef.current.delete(key);
    }
  };

  return { registerNode };
}
