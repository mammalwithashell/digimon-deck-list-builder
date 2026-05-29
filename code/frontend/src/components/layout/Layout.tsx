import { Outlet } from 'react-router-dom';
import { NavBar } from './NavBar';

export function Layout() {
  // `h-full` sizes us to the fixed CanvasScaler box (1080px) in the
  // desktop build. `min-h-screen` provides the same guarantee in the
  // web build where there's no canvas — `100%` of body is 0 otherwise.
  // Both can coexist; whichever is larger wins per the CSS spec.
  return (
    <div className="h-full min-h-screen bg-gray-900 text-gray-100 flex flex-col">
      <NavBar />
      <main className="flex-1 min-h-0">
        <Outlet />
      </main>
    </div>
  );
}
