import { useEffect, useRef } from 'react';
import { useTheme } from '@/design/theme/ThemeProvider';
import { useEffectiveLiveBackground } from '@/stores/uiStore';
import './LiveAtmosphere.css';

/**
 * Live theme atmosphere (add-live-theme-atmosphere).
 *
 * A reusable, non-interactive background layer with two theme variants:
 *  - dark "Digi-OS": a calm, slow digital rain (canvas) + a faint drifting grid;
 *  - light "Adventure '99": an idle laptop desktop (breathing gradient,
 *    parallaxing dot-grid, blinking cursor, occasional analyzer sweep — CSS).
 *
 * It animates only when effective live-background is on (motion `full` AND the
 * Live-background toggle on); otherwise it renders a static version of the same
 * scene. The canvas loop is frame-rate capped and pauses while the tab is
 * hidden. The `surface` prop lets the in-game board reuse the same engine with
 * board-appropriate intensity.
 */

// Calm-by-default tunables — ambient weather, not a screensaver.
const RAIN_FONT = 14; // px glyph size === cell size
const RAIN_SPEED = 0.4; // cells advanced per drawn frame
const RAIN_DENSITY = 0.5; // fraction of columns populated
const RAIN_FPS = 20; // capped frame rate
const BOARD_SCALE = 0.7; // board atmosphere is subtler than menus
const GLYPHS = '01ｱｲｳｴｵｶｷｸｹｺﾊﾐﾋｰﾂDIGIMON$+*=';

export type AtmosphereSurface = 'menu' | 'board';

function hexA(hex: string, a: number): string {
  const m = hex.trim().replace('#', '');
  if (m.length === 6) {
    const r = parseInt(m.slice(0, 2), 16);
    const g = parseInt(m.slice(2, 4), 16);
    const b = parseInt(m.slice(4, 6), 16);
    if (!Number.isNaN(r + g + b)) return `rgba(${r}, ${g}, ${b}, ${a})`;
  }
  return hex || `rgba(7, 11, 9, ${a})`;
}

function DarkRain({ live, surface }: { live: boolean; surface: AtmosphereSurface }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext('2d');
    if (!canvas || !ctx) return;

    const cs = getComputedStyle(document.documentElement);
    const bg = cs.getPropertyValue('--surface-bg') || '#070b09';
    const body = cs.getPropertyValue('--accent') || '#39ff88';
    const head = cs.getPropertyValue('--player') || '#ff7a18';
    const density = surface === 'board' ? RAIN_DENSITY * BOARD_SCALE : RAIN_DENSITY;

    let drops: number[] = [];
    let active: boolean[] = [];
    let w = 0;
    let h = 0;

    const seedColumns = (cols: number) => {
      // Preserve existing drop positions so the rain stays CONTINUOUS across
      // board re-layouts (action bar appearing, prompts, played cards) — those
      // resize the canvas and previously re-seeded every column back to the
      // top. Only fill newly added columns; drop removed ones.
      const nextDrops: number[] = [];
      const nextActive: boolean[] = [];
      for (let i = 0; i < cols; i++) {
        if (i < drops.length) {
          nextDrops[i] = drops[i]!;
          nextActive[i] = active[i]!;
        } else {
          nextDrops[i] = Math.random() * -40;
          nextActive[i] = Math.random() < density;
        }
      }
      drops = nextDrops;
      active = nextActive;
    };

    const size = () => {
      const nextW = canvas.clientWidth;
      const nextH = canvas.clientHeight;
      // Ignore no-op resize callbacks so the rain doesn't restart needlessly.
      if (nextW === w && nextH === h) return;
      w = nextW;
      h = nextH;
      canvas.width = w;
      canvas.height = h;
      seedColumns(Math.max(1, Math.floor(w / RAIN_FONT)));
      ctx.font = `${RAIN_FONT}px monospace`;
    };

    const drawFrame = () => {
      ctx.fillStyle = hexA(bg, 0.1);
      ctx.fillRect(0, 0, w, h);
      ctx.font = `${RAIN_FONT}px monospace`;
      for (let i = 0; i < drops.length; i++) {
        if (!active[i]) continue;
        const ch = GLYPHS[Math.floor(Math.random() * GLYPHS.length)]!;
        const y = drops[i]! * RAIN_FONT;
        ctx.fillStyle = Math.random() > 0.97 ? head : body;
        ctx.fillText(ch, i * RAIN_FONT, y);
        drops[i] = drops[i]! + RAIN_SPEED;
        if (y > h && Math.random() > 0.99) {
          drops[i] = 0;
          active[i] = Math.random() < density;
        }
      }
    };

    const drawStatic = () => {
      ctx.fillStyle = (bg || '#070b09').trim();
      ctx.fillRect(0, 0, w, h);
      ctx.font = `${RAIN_FONT}px monospace`;
      ctx.fillStyle = hexA(body, 0.5);
      const rows = Math.max(1, Math.floor(h / RAIN_FONT));
      for (let i = 0; i < drops.length; i++) {
        if (!active[i]) continue;
        const top = (i * 7) % rows;
        for (let k = 0; k < 6; k++) {
          ctx.fillText(GLYPHS[(i + k) % GLYPHS.length]!, i * RAIN_FONT, (top + k) * RAIN_FONT);
        }
      }
    };

    size();
    const ro = new ResizeObserver(() => {
      size();
      if (!live) drawStatic();
    });
    ro.observe(canvas);

    if (!live) {
      drawStatic();
      return () => ro.disconnect();
    }

    let raf = 0;
    let last = 0;
    let stopped = false;
    const interval = 1000 / RAIN_FPS;
    const loop = (t: number) => {
      if (stopped) return;
      raf = requestAnimationFrame(loop);
      if (t - last < interval) return;
      last = t;
      drawFrame();
    };
    const onVisibility = () => {
      if (document.hidden) {
        stopped = true;
        if (raf) cancelAnimationFrame(raf);
      } else if (stopped) {
        stopped = false;
        last = 0;
        raf = requestAnimationFrame(loop);
      }
    };
    document.addEventListener('visibilitychange', onVisibility);
    raf = requestAnimationFrame(loop);

    return () => {
      stopped = true;
      if (raf) cancelAnimationFrame(raf);
      ro.disconnect();
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [live, surface]);

  return <canvas ref={canvasRef} className="live-atmosphere__rain" />;
}

function LightDesktop() {
  return (
    <>
      <div className="live-atmosphere__teal" />
      <div className="live-atmosphere__dots" />
      <div className="live-atmosphere__sweep" />
      <div className="live-atmosphere__cursor" />
    </>
  );
}

export function LiveAtmosphere({ surface = 'menu' }: { surface?: AtmosphereSurface }) {
  const { theme } = useTheme();
  const live = useEffectiveLiveBackground();

  return (
    <div
      className={`live-atmosphere${live ? ' is-live' : ''}`}
      data-surface={surface}
      aria-hidden="true"
    >
      {theme === 'dark' ? (
        <DarkRain live={live} surface={surface} />
      ) : (
        <LightDesktop />
      )}
    </div>
  );
}
