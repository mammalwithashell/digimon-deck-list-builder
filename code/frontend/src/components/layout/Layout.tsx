import { Outlet } from 'react-router-dom';
import { NavBar } from './NavBar';

// The desktop build navigates via the launcher sidebar, so the legacy top
// NavBar header is dropped there (add-desktop-design-system). The hosted web
// build keeps it as its primary navigation.
const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

export function Layout() {
  // `h-full` sizes us to the fixed CanvasScaler box (1080px) in the
  // desktop build. `min-h-screen` provides the same guarantee in the
  // web build where there's no canvas — `100%` of body is 0 otherwise.
  return (
    <div className="h-full min-h-screen bg-[var(--surface-bg)] text-[var(--ink-0)] flex flex-col">
      {!IS_DESKTOP && <NavBar />}
      <main className="flex-1 min-h-0">
        <Outlet />
      </main>
    </div>
  );
}
