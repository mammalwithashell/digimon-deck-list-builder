import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react';

/**
 * Lets a page contribute contextual sub-navigation into the persistent
 * `MenuShell` rail (e.g. the deck library's Folders/Formats). The shell
 * owns a single "section" slot; the active page sets it on mount and
 * clears it on unmount, so the rail never shows a stale page's sub-nav.
 *
 * The contract is intentionally tiny — one slot, one setter — so the shell
 * stays ignorant of any page's internals.
 */

interface RailContextValue {
  section: ReactNode;
  setSection: (node: ReactNode) => void;
}

const RailContext = createContext<RailContextValue | null>(null);

export function RailProvider({ children }: { children: ReactNode }) {
  const [section, setSection] = useState<ReactNode>(null);
  return (
    <RailContext.Provider value={{ section, setSection }}>
      {children}
    </RailContext.Provider>
  );
}

/** Read the currently-contributed rail section (used by `MenuShell`). */
export function useRailSectionSlot(): ReactNode {
  return useContext(RailContext)?.section ?? null;
}

/**
 * Contribute `node` into the rail. The node is (re)published whenever it
 * changes and cleared when the calling component unmounts. Pages that
 * derive the node from changing state (filters, counts) should memoize it
 * so it only re-publishes when the rendered sub-nav actually changes.
 *
 * Safe to call outside a `RailProvider` (e.g. in the hosted-web build or a
 * unit test) — it simply does nothing.
 */
export function useRailSection(node: ReactNode): void {
  const setSection = useContext(RailContext)?.setSection;

  // Publish (and re-publish) the contributed section as it changes.
  useEffect(() => {
    setSection?.(node);
  }, [setSection, node]);

  // Clear on unmount only — kept separate from the publish effect so a
  // changing node updates in place instead of flickering through null.
  useEffect(() => {
    return () => setSection?.(null);
  }, [setSection]);
}
