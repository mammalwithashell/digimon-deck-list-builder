/** Route-matching helper shared by the desktop `MenuShell` rail and the
 *  hosted-web `NavBar`. The current URL is the single source of truth for
 *  which nav item is "active" — never page-managed state, which can desync
 *  from the route. */

export interface NavMatch {
  /** The route this nav item links to. */
  to: string;
  /** When true, only an exact pathname match counts (e.g. Home `/`). When
   *  false/absent, the item is also active for any nested route under it
   *  (e.g. `/play` is active on `/play/deck`). */
  exact?: boolean;
}

export function isActivePath(pathname: string, item: NavMatch): boolean {
  if (item.exact) return pathname === item.to;
  return pathname === item.to || pathname.startsWith(`${item.to}/`);
}
