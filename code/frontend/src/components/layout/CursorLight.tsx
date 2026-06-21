import { useEffect, useRef } from 'react';
import { useMotion } from '@/stores/uiStore';

/**
 * Cursor-follow light (add-cursor-follow-lighting).
 *
 * A non-interactive layer in the menu shell whose soft glow tracks the pointer.
 * The pointer position is written to CSS custom properties in a
 * `requestAnimationFrame`-throttled handler, so there is no React re-render per
 * move; a fixed-size glow element is moved via `transform` (compositor-friendly)
 * rather than repainting a gradient center every frame. Rendered only at motion
 * `full` — at `reduced`/`off` the component returns null and attaches no
 * listener (CSS also hides it as defense-in-depth).
 *
 * Mounted once inside `MenuShell`, so it covers menu routes only; the full-bleed
 * in-game board route is not wrapped by the shell and therefore excluded.
 */
export function CursorLight() {
  const motion = useMotion();
  const glowRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (motion !== 'full') return;
    const glow = glowRef.current;
    const host = glow?.parentElement;
    if (!glow || !host) return;

    let x = 0;
    let y = 0;
    let queued = false;
    let raf = 0;

    const apply = () => {
      queued = false;
      // Coordinates are relative to the layer (its own containing block), so
      // the glow lines up regardless of the rail width / title bar offset.
      const rect = host.getBoundingClientRect();
      glow.style.setProperty('--cursor-x', `${x - rect.left}px`);
      glow.style.setProperty('--cursor-y', `${y - rect.top}px`);
    };

    const onMove = (e: PointerEvent) => {
      x = e.clientX;
      y = e.clientY;
      if (!queued) {
        queued = true;
        raf = requestAnimationFrame(apply);
      }
    };

    window.addEventListener('pointermove', onMove, { passive: true });
    return () => {
      window.removeEventListener('pointermove', onMove);
      if (raf) cancelAnimationFrame(raf);
    };
  }, [motion]);

  if (motion !== 'full') return null;

  return (
    <div className="cursor-light" aria-hidden="true">
      <div className="cursor-light__glow" ref={glowRef} />
    </div>
  );
}
