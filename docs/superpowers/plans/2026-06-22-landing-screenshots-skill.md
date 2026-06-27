# Landing-page Desktop Screenshots + Companion Skill — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a committed, CRT-styled screenshot gallery (both themes) to the landing page, plus a `update-landing-screenshots` skill that launches the real desktop app, drives it page-by-page via a dev-only bridge hook, captures each window with `PrintWindow`, and publishes.

**Architecture:** A dev-only `/navigate` verb on the existing feature-gated debug bridge emits a `debug:navigate` window event; a dev+desktop-only React listener routes + sets theme. PowerShell `PrintWindow` captures the real window (client area only); a Python/PIL step converts to WebP. A guided SKILL.md orchestrates launch → navigate/stage → capture → wire `index.html` → commit + push.

**Tech Stack:** Rust (axum, tauri Emitter), React + react-router + Zustand, PowerShell (System.Drawing / PrintWindow), Python (Pillow/WebP), static HTML/CSS.

**Reference:** spec `docs/superpowers/specs/2026-06-22-landing-screenshots-skill-design.md`. Companion to `cut-desktop-release`. Reuses the `run-desktop` launch recipe and the debug bridge (`code/src-tauri/src/debug_bridge.rs`).

**Pre-flight (once, before Task 1):** Stop the trial's `cargo tauri dev` and dev server so the `src-tauri` build lock is free:
```bash
# stop the background cargo-tauri-dev + dev-server tasks (or close the app window)
tasklist | grep -i digimon-tcg.exe   # then taskkill //F //PID <pid> if present
```

---

### Task 1: Add `/navigate` to the debug bridge (Rust)

**Files:**
- Modify: `code/src-tauri/src/debug_bridge.rs` (router list ~line 85-96; add handler + `NavigateBody`; add test in the `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test** — append to `mod tests` in `debug_bridge.rs`:

```rust
    #[test]
    fn navigate_body_parses_route_and_optional_theme() {
        let b: NavigateBody =
            serde_json::from_value(json!({ "route": "/deckbuilder", "theme": "dark" })).unwrap();
        assert_eq!(b.route, "/deckbuilder");
        assert_eq!(b.theme.as_deref(), Some("dark"));

        let b2: NavigateBody = serde_json::from_value(json!({ "route": "/" })).unwrap();
        assert_eq!(b2.route, "/");
        assert!(b2.theme.is_none(), "theme is optional");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml --lib --features debug-bridge navigate_body_parses`
Expected: FAIL — `cannot find type NavigateBody in this scope`.

- [ ] **Step 3: Add the body type + handler.** After the `StepBody` struct/handler block (near the other `#[derive(Deserialize)] struct *Body`), add:

```rust
#[derive(Deserialize)]
struct NavigateBody {
    route: String,
    theme: Option<String>,
}

/// Dev-only: drive the desktop window's client-side router (+ optional theme)
/// for the screenshot skill. Emits a `debug:navigate` window event the React
/// `DebugBridgeNav` listener consumes; the engine state is untouched.
async fn navigate(State(s): State<BridgeState>, Json(b): Json<NavigateBody>) -> BridgeResult {
    s.app
        .emit("debug:navigate", json!({ "route": b.route, "theme": b.theme }))
        .map_err(bad)?;
    Ok(Json(json!({ "ok": true })))
}
```

- [ ] **Step 4: Register the route.** In `maybe_spawn`, add to the `Router::new()` chain (next to `.route("/step", post(step))`):

```rust
        .route("/navigate", post(navigate))
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml --lib --features debug-bridge navigate_body_parses`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/src-tauri/src/debug_bridge.rs
git commit -m "feat(debug-bridge): add dev-only /navigate verb for screenshot skill"
```

---

### Task 2: Frontend `DebugBridgeNav` listener (dev + desktop only)

**Files:**
- Create: `code/frontend/src/components/desktop/DebugBridgeNav.tsx`
- Create: `code/frontend/src/components/desktop/DebugBridgeNav.test.tsx`
- Modify: `code/frontend/src/App.tsx` (import + mount inside `<BrowserRouter>`, near `UpdaterBridge` at line 192)

- [ ] **Step 1: Write the failing test** — `DebugBridgeNav.test.tsx`:

```tsx
import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect } from 'vitest';
import { DebugBridgeNav } from './DebugBridgeNav';

describe('DebugBridgeNav', () => {
  it('renders nothing and does not throw outside a desktop build', () => {
    const { container } = render(
      <MemoryRouter><DebugBridgeNav /></MemoryRouter>,
    );
    // Web/test build: IS_DESKTOP is false, so the effect early-returns and no
    // Tauri event API is imported. Component must render null.
    expect(container.firstChild).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd code/frontend && npx vitest run src/components/desktop/DebugBridgeNav.test.tsx`
Expected: FAIL — cannot resolve `./DebugBridgeNav`.

- [ ] **Step 3: Create the component** — `DebugBridgeNav.tsx`:

```tsx
import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useThemeStore, type Theme } from '@/design/theme/themeStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

/**
 * Dev-only bridge between the desktop debug bridge and the app's client-side
 * router + theme store. The screenshot skill (`update-landing-screenshots`)
 * POSTs `/navigate {route, theme}` to the bridge, which emits a `debug:navigate`
 * window event this component consumes to drive the real window page-by-page.
 *
 * Mounted only when `IS_DESKTOP && import.meta.env.DEV`, so production desktop
 * builds (`vite build --mode desktop`) tree-shake it out. The Tauri event API
 * is imported lazily so the web/test build never touches it.
 */
export function DebugBridgeNav() {
  const navigate = useNavigate();
  useEffect(() => {
    if (!IS_DESKTOP || !import.meta.env.DEV) return;
    let un: (() => void) | undefined;
    let cancelled = false;
    void (async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<{ route?: string; theme?: Theme }>(
        'debug:navigate',
        (e) => {
          const { route, theme } = e.payload ?? {};
          if (theme === 'dark' || theme === 'light') {
            useThemeStore.getState().setTheme(theme);
          }
          if (route) navigate(route);
        },
      );
      if (cancelled) unlisten();
      else un = unlisten;
    })();
    return () => {
      cancelled = true;
      un?.();
    };
  }, [navigate]);
  return null;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd code/frontend && npx vitest run src/components/desktop/DebugBridgeNav.test.tsx`
Expected: PASS.

- [ ] **Step 5: Mount it in `App.tsx`.** Add the import beside the other component imports (e.g. after the `UpdaterBridge` import at line 23):

```tsx
import { DebugBridgeNav } from '@/components/desktop/DebugBridgeNav';
```

Then inside the `<BrowserRouter>` block, immediately after `<UpdaterBridge />` (line 192), add:

```tsx
        {IS_DESKTOP && import.meta.env.DEV && <DebugBridgeNav />}
```

- [ ] **Step 6: Typecheck**

Run: `cd code/frontend && npx tsc -b --noEmit`
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add code/frontend/src/components/desktop/DebugBridgeNav.tsx code/frontend/src/components/desktop/DebugBridgeNav.test.tsx code/frontend/src/App.tsx
git commit -m "feat(frontend): dev-only DebugBridgeNav for bridge-driven routing + theme"
```

---

### Task 3: Integration smoke — bridge nav drives the real window

This validates Tasks 1+2 together before building capture tooling on top. Manual integration (no unit test).

**Files:** none (uses the running app).

- [ ] **Step 1: Launch the app with the bridge** (two background processes), per `run-desktop`:

```bash
cd code/frontend && npm run dev:desktop          # background; wait for :5173 LISTEN
cd code/src-tauri && DIGIMON_DEBUG_BRIDGE=1 cargo tauri dev --features debug-bridge \
  --config '{"build":{"beforeDevCommand":""}}'   # background; wait for the window
```

- [ ] **Step 2: Find the bridge port**

Run: `cat ~/AppData/Roaming/digimon-tcg/debug_bridge.json`
Expected: JSON like `{"port":5174,"base_url":"http://127.0.0.1:5174"}`.

- [ ] **Step 3: Drive navigation + theme** (use the port from Step 2):

```bash
curl -s -X POST http://127.0.0.1:5174/navigate -H 'Content-Type: application/json' \
  -d '{"route":"/deckbuilder","theme":"light"}'
```

Expected: `{"ok":true}`, and the **window** switches to the Deck Library in the light theme. Try `{"route":"/models","theme":"dark"}` and `{"route":"/","theme":"dark"}` to confirm routing + theme both respond.

- [ ] **Step 4:** If navigation does not move the window, confirm `DebugBridgeNav` is mounted (dev build, `import.meta.env.DEV` true under `cargo tauri dev`) and the event name matches (`debug:navigate`). Fix and re-verify before continuing. No commit (verification only).

---

### Task 4: `capture_window.ps1` — client-area PrintWindow capture

**Files:**
- Create: `.claude/skills/update-landing-screenshots/scripts/capture_window.ps1`

- [ ] **Step 1: Write the script** (captures the webview **client** area only — excludes the native title bar — via `PrintWindow(PW_RENDERFULLCONTENT)` then crops to the client rect):

```powershell
# Capture a window's CLIENT area (no native title bar) to PNG via PrintWindow.
# PrintWindow works even when the window is not foreground/occluded.
param(
  [string]$ProcName = "digimon-tcg",
  [Parameter(Mandatory=$true)][string]$OutPath
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class WinCap {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@
[void][WinCap]::SetProcessDPIAware()

$p = Get-Process -Name $ProcName -ErrorAction SilentlyContinue |
     Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Error "no '$ProcName' window"; exit 2 }
$h = $p.MainWindowHandle

$wr = New-Object WinCap+RECT; [void][WinCap]::GetWindowRect($h, [ref]$wr)
$cr = New-Object WinCap+RECT; [void][WinCap]::GetClientRect($h, [ref]$cr)
$origin = New-Object WinCap+POINT; $origin.X = 0; $origin.Y = 0
[void][WinCap]::ClientToScreen($h, [ref]$origin)
$ww = $wr.Right - $wr.Left; $wh = $wr.Bottom - $wr.Top
$cw = $cr.Right - $cr.Left; $ch = $cr.Bottom - $cr.Top
$offX = $origin.X - $wr.Left; $offY = $origin.Y - $wr.Top

# Full-window PrintWindow into a bitmap, then crop to the client rect.
$full = New-Object System.Drawing.Bitmap $ww, $wh
$g = [System.Drawing.Graphics]::FromImage($full); $hdc = $g.GetHdc()
[void][WinCap]::PrintWindow($h, $hdc, 0x2)   # PW_RENDERFULLCONTENT (WebView2)
$g.ReleaseHdc($hdc); $g.Dispose()

$client = New-Object System.Drawing.Bitmap $cw, $ch
$g2 = [System.Drawing.Graphics]::FromImage($client)
$g2.DrawImage($full, (New-Object System.Drawing.Rectangle 0,0,$cw,$ch),
              $offX, $offY, $cw, $ch, [System.Drawing.GraphicsUnit]::Pixel)
$g2.Dispose(); $full.Dispose()
$client.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png); $client.Dispose()
Write-Output "saved $OutPath (${cw}x${ch})"
```

- [ ] **Step 2: Verify against the running app** (from Task 3 it's still up; navigate to `/` first):

```bash
curl -s -X POST http://127.0.0.1:5174/navigate -d '{"route":"/","theme":"dark"}' -H 'Content-Type: application/json'
powershell -File .claude/skills/update-landing-screenshots/scripts/capture_window.ps1 -OutPath "$PWD/.trial/cap_test.png"
```

Then `Read` `.trial/cap_test.png`. Expected: the Launcher in dark theme, **no native title bar** (client area only).

- [ ] **Step 3: Commit**

```bash
git add .claude/skills/update-landing-screenshots/scripts/capture_window.ps1
git commit -m "feat(skill): client-area PrintWindow capture script"
```

---

### Task 5: `to_webp.py` — crop margins + resize + WebP

**Files:**
- Create: `.claude/skills/update-landing-screenshots/scripts/to_webp.py`
- Create: `.claude/skills/update-landing-screenshots/scripts/test_to_webp.py`

- [ ] **Step 1: Write the failing test** — `test_to_webp.py`:

```python
import subprocess, sys, pathlib
from PIL import Image

HERE = pathlib.Path(__file__).parent

def test_converts_png_to_webp_at_target_width(tmp_path):
    src = tmp_path / "in.png"
    Image.new("RGB", (1200, 720), (10, 20, 16)).save(src)
    out = tmp_path / "out.webp"
    subprocess.run(
        [sys.executable, str(HERE / "to_webp.py"), str(src), str(out), "--width", "600"],
        check=True,
    )
    assert out.exists()
    im = Image.open(out)
    assert im.format == "WEBP"
    assert im.width == 600
    assert im.height == 360  # aspect preserved
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest .claude/skills/update-landing-screenshots/scripts/test_to_webp.py -v`
Expected: FAIL — `to_webp.py` missing.

- [ ] **Step 3: Write `to_webp.py`:**

```python
"""Crop + resize a capture PNG and save as WebP for the landing-page gallery.

Usage: to_webp.py IN.png OUT.webp [--width 960] [--crop L T R B] [--quality 82]
--crop trims that many pixels off each edge (e.g. residual letterbox); omit for none.
"""
import argparse
from PIL import Image


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("src")
    ap.add_argument("dst")
    ap.add_argument("--width", type=int, default=960)
    ap.add_argument("--crop", type=int, nargs=4, metavar=("L", "T", "R", "B"))
    ap.add_argument("--quality", type=int, default=82)
    a = ap.parse_args()

    im = Image.open(a.src).convert("RGB")
    if a.crop:
        l, t, r, b = a.crop
        im = im.crop((l, t, im.width - r, im.height - b))
    if a.width and im.width != a.width:
        h = round(im.height * a.width / im.width)
        im = im.resize((a.width, h), Image.LANCZOS)
    im.save(a.dst, "WEBP", quality=a.quality, method=6)
    print(f"wrote {a.dst} {im.width}x{im.height}")


if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest .claude/skills/update-landing-screenshots/scripts/test_to_webp.py -v`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add .claude/skills/update-landing-screenshots/scripts/to_webp.py .claude/skills/update-landing-screenshots/scripts/test_to_webp.py
git commit -m "feat(skill): PNG->WebP crop/resize converter + test"
```

---

### Task 6: `hero-board.json` staging fixture

**Files:**
- Create: `.claude/skills/update-landing-screenshots/fixtures/hero-board.json`
- Modify: `code/src-tauri/src/debug_bridge.rs` (`mod tests`: add a test that the fixture stages legally)

- [ ] **Step 1: Create the fixture.** Reuse the **proven-legal** deck + stack data from the existing `stage_into_installs_a_board_that_round_trips` test in `debug_bridge.rs` (decks of `ST1-01`×5 + `ST1-03`×45; a 3-deep digivolution stack `BT12-022 → BT12-050 → AD1-011` already shown legal there) so the hero board looks alive (a tall evolved stack) and stages first-try. Schema matches `stage_into`'s `decks`+`state`+`zones`:

```json
{
  "schema_version": 1,
  "decks": {
    "1": ["ST1-01","ST1-01","ST1-01","ST1-01","ST1-01","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03"],
    "2": ["ST1-01","ST1-01","ST1-01","ST1-01","ST1-01","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03","ST1-03"]
  },
  "seed": 7,
  "state": { "memory": 2, "phase": "Main", "turn": 4, "first_player": 1 },
  "zones": {
    "1": { "field": [ { "stack": ["BT12-022","BT12-050","AD1-011"], "is_suspended": true, "turn_played": 1 } ] },
    "2": { "field": [ { "stack": ["BT12-022","BT12-050"], "is_suspended": false, "turn_played": 1 } ] }
  },
  "assertions": { "engine": [], "ui": [] }
}
```

> NOTE: this mirrors data already proven legal by the existing test, so it should stage first-try. If the Step-3 test reports an illegal stack/id, read the `stage_into` diagnostic and adjust ids against `data/tested_cards.json` — that validator is the gate.

- [ ] **Step 2: Write the failing test** — append to `mod tests` in `debug_bridge.rs`:

```rust
    #[test]
    fn hero_fixture_stages_into_a_legal_board() {
        let raw = include_str!(
            "../../../.claude/skills/update-landing-screenshots/fixtures/hero-board.json"
        );
        let fixture: serde_json::Value = serde_json::from_str(raw).expect("fixture is valid JSON");
        let mut world = EngineWorld::default();
        let dto = stage_into(&mut world, &fixture).expect("hero fixture must stage legally");
        assert!(dto.get("players").is_some());
        assert!(world.game.is_some(), "a game must be installed");
    }
```

- [ ] **Step 3: Run test to verify it fails, then passes**

Run: `cargo test --manifest-path code/src-tauri/Cargo.toml --lib --features debug-bridge hero_fixture_stages`
Expected: initially may FAIL if a card id is illegal/unimplemented — read the diagnostic, fix ids in `hero-board.json` (use known-good Lv.3→Lv.4 pairs from `data/tested_cards.json`), re-run until PASS.

- [ ] **Step 4: Commit**

```bash
git add .claude/skills/update-landing-screenshots/fixtures/hero-board.json code/src-tauri/src/debug_bridge.rs
git commit -m "feat(skill): hero-board staging fixture + legality test"
```

---

### Task 7: Verify the game-board capture path end-to-end

Manual integration: confirm staging + navigating renders the board so the hero shot works. (Restart the app to pick up Task 6's nothing-frontend changes is unnecessary; the fixture is consumed at runtime.)

**Files:** none.

- [ ] **Step 1:** With the app running (Task 3) and bridge port known, stage then navigate:

```bash
PORT=5174  # from debug_bridge.json
curl -s -X POST http://127.0.0.1:$PORT/stage -H 'Content-Type: application/json' \
  --data-binary @.claude/skills/update-landing-screenshots/fixtures/hero-board.json
curl -s -X POST http://127.0.0.1:$PORT/navigate -H 'Content-Type: application/json' \
  -d '{"route":"/game/rust-local","theme":"dark"}'
```

- [ ] **Step 2: Capture + inspect**

```bash
powershell -File .claude/skills/update-landing-screenshots/scripts/capture_window.ps1 -OutPath "$PWD/.trial/board_test.png"
```

`Read` `.trial/board_test.png`. Expected: the live board (two Digimon stacks, memory gauge, security) in dark theme — not the pre-game setup form.

- [ ] **Step 3:** If the setup form shows instead of the board: the order matters — `/stage` must precede the `/game/rust-local` navigation so `GamePage`'s mount-time fetch (`store.setGameId(urlGameId)` at `GamePage.tsx:287`) finds the staged `world.game`. If it still fails, add a brief settle (`sleep 1`) between stage and navigate, or navigate first then stage (the `debug:state-changed` refetch at `GamePage.tsx:347` will pull it in). Document the working order in the SKILL.md (Task 9). No commit (verification).

---

### Task 8: Landing-page gallery section (`code/landing/index.html`)

**Files:**
- Modify: `code/landing/index.html` (add CSS in `<style>`; add a `<section>` after the "system capabilities" features section ~line 362; add a toggle `<script>` before `</body>`)

- [ ] **Step 1: Add gallery CSS** — inside the existing `<style>` block, before the `/* ── footer */` comment (~line 270):

```css
  /* ── screenshot gallery ─────────────────────────────────────── */
  .shotgrid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  @media (max-width: 640px) { .shotgrid { grid-template-columns: 1fr; } }
  .gallery-bar { display: flex; align-items: center; gap: 12px; margin-bottom: 16px; font-size: 13px; color: var(--ink-dim); }
  .skin-toggle { display: inline-flex; align-items: center; gap: 9px; cursor: pointer; user-select: none;
    text-transform: uppercase; letter-spacing: 0.18em; font-size: 12px; }
  .skin-toggle .track { width: 46px; height: 20px; border: 1px solid var(--phosphor-dim); background: var(--phosphor-faint); position: relative; }
  .skin-toggle .knob { position: absolute; top: 1px; left: 1px; width: 21px; height: 16px; background: var(--phosphor);
    box-shadow: 0 0 8px var(--phosphor); transition: left 120ms steps(3); }
  .shotgrid-wrap[data-skin="light"] .knob { left: 23px; background: var(--amber); box-shadow: 0 0 8px var(--amber); }
  .skin-toggle .on-d { color: var(--phosphor); } .shotgrid-wrap[data-skin="light"] .on-d { color: var(--ink-dim); }
  .skin-toggle .on-l { color: var(--ink-dim); } .shotgrid-wrap[data-skin="light"] .on-l { color: var(--amber); }
  .shot { border: 1px solid var(--phosphor-dim); background: #000; position: relative;
    box-shadow: 0 0 0 1px rgba(0,0,0,0.6), 0 18px 40px -24px rgba(57,255,136,0.35); }
  .shot::after { content: ""; position: absolute; right: -1px; bottom: -1px; width: 14px; height: 14px;
    border-right: 2px solid var(--amber); border-bottom: 2px solid var(--amber); }
  .shot .frame { position: relative; aspect-ratio: 1280 / 768; overflow: hidden; }
  .shot .frame img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .shot .frame img.l { display: none; }
  .shotgrid-wrap[data-skin="light"] .shot .frame img.d { display: none; }
  .shotgrid-wrap[data-skin="light"] .shot .frame img.l { display: block; }
  .shot .frame::before { content: ""; position: absolute; inset: 0; z-index: 2; pointer-events: none;
    background: repeating-linear-gradient(to bottom, transparent 0 2px, rgba(0,0,0,0.20) 3px 4px); }
  .shot .cap { font-size: 11px; color: var(--phosphor-dim); padding: 7px 10px; letter-spacing: 0.1em; }
```

- [ ] **Step 2: Add the section** — after the features `</section>` (the "system capabilities" block, ~line 362), insert:

```html
  <!-- ── screenshot gallery ── -->
  <section>
    <h2>visual feed</h2>
    <div class="shotgrid-wrap" data-skin="dark" id="shotgrid-wrap">
      <div class="gallery-bar">
        <span>// captured from the desktop client</span>
        <span class="skin-toggle" onclick="toggleSkin()" role="button" tabindex="0">
          <span class="on-d">dark</span>
          <span class="track"><span class="knob"></span></span>
          <span class="on-l">light</span>
        </span>
      </div>
      <div class="shotgrid">
        <div class="shot"><div class="frame">
          <img class="d" loading="lazy" alt="Launcher (dark)" src="assets/screenshots/launcher-dark.webp">
          <img class="l" loading="lazy" alt="Launcher (light)" src="assets/screenshots/launcher-light.webp">
        </div><div class="cap">// launcher</div></div>
        <div class="shot"><div class="frame">
          <img class="d" loading="lazy" alt="Game board (dark)" src="assets/screenshots/game-board-dark.webp">
          <img class="l" loading="lazy" alt="Game board (light)" src="assets/screenshots/game-board-light.webp">
        </div><div class="cap">// game_board</div></div>
        <div class="shot"><div class="frame">
          <img class="d" loading="lazy" alt="Deck builder (dark)" src="assets/screenshots/deck-builder-dark.webp">
          <img class="l" loading="lazy" alt="Deck builder (light)" src="assets/screenshots/deck-builder-light.webp">
        </div><div class="cap">// deck_builder</div></div>
        <div class="shot"><div class="frame">
          <img class="d" loading="lazy" alt="Deck library (dark)" src="assets/screenshots/deck-library-dark.webp">
          <img class="l" loading="lazy" alt="Deck library (light)" src="assets/screenshots/deck-library-light.webp">
        </div><div class="cap">// deck_library</div></div>
        <div class="shot"><div class="frame">
          <img class="d" loading="lazy" alt="AI models (dark)" src="assets/screenshots/ai-models-dark.webp">
          <img class="l" loading="lazy" alt="AI models (light)" src="assets/screenshots/ai-models-light.webp">
        </div><div class="cap">// ai_models</div></div>
      </div>
    </div>
  </section>
```

- [ ] **Step 3: Add the toggle script** — just before the existing download `<script>` (or right after `<body>`'s `</footer>`/`</main>`), add a small script:

```html
<script>
  function toggleSkin() {
    var w = document.getElementById("shotgrid-wrap");
    w.dataset.skin = w.dataset.skin === "dark" ? "light" : "dark";
  }
</script>
```

- [ ] **Step 4: Verify structure** (images 404 until Task 9 generates them — that's expected; check the markup parses and toggle works with placeholders):

Run: `python -c "import pathlib,re; h=pathlib.Path('code/landing/index.html').read_text(encoding='utf-8'); assert h.count('assets/screenshots/')==10, h.count('assets/screenshots/'); assert 'toggleSkin' in h; print('gallery markup ok: 10 image refs')"`
Expected: `gallery markup ok: 10 image refs`.

- [ ] **Step 5: Commit**

```bash
git add code/landing/index.html
git commit -m "feat(landing): CRT screenshot gallery section with light/dark toggle"
```

---

### Task 9: The skill recipe (`SKILL.md`)

**Files:**
- Create: `.claude/skills/update-landing-screenshots/SKILL.md`

- [ ] **Step 1: Write `SKILL.md`** with this content (frontmatter description is trigger-tuned; body is the verified recipe):

````markdown
---
name: update-landing-screenshots
description: Recapture the desktop-app screenshots on the landing page and republish them. Launches the REAL Tauri desktop app (not a browser — backend-fed pages like decks/models/a live game only populate in the real app), drives it through the mainstay pages in both light and dark themes via the dev-only debug-bridge navigate hook, captures each window with PrintWindow, writes the WebP assets under code/landing/assets/screenshots/, and commits + pushes (triggering the landing-page Pages deploy). Use WHENEVER the user wants to update / refresh / regenerate the landing-page screenshots or gallery, recapture the desktop app shots, or "take new screenshots for the site" — and as a companion to cut-desktop-release so the gallery tracks the shipped build. NOT for the hosted API or the desktop release itself.
---

# Update the landing-page screenshots

Companion to `cut-desktop-release`. Recaptures the gallery in `code/landing/`
from the **real** desktop app. The capture is fully scriptable (PowerShell
`PrintWindow`); navigation + theme are driven through the dev-only debug bridge
(`/navigate`). This is an **outward-facing** action — the final step pushes to
`main` and publishes to the live site.

## Pages captured (5 × 2 themes = 10 WebP)

| asset stem | route | notes |
|---|---|---|
| `launcher` | `/` | front door, populated decks |
| `game-board` | `/game/rust-local` | **stage `fixtures/hero-board.json` first** |
| `deck-builder` | `/deckbuilder/new` | |
| `deck-library` | `/deckbuilder` | |
| `ai-models` | `/models` | desktop-only (Tauri invoke) |

## Recipe

### 1. Preconditions
- On `main`'s tip (assets should reflect the shipped build); `gh` authenticated.
- Frontend deps present: `test -d code/frontend/node_modules || (cd code/frontend && npm install)`.
- Free the `src-tauri` build lock (no other `cargo tauri dev` running).

### 2. Launch the app with the bridge (reuse `run-desktop`)
```bash
cd code/frontend && npm run dev:desktop      # background; wait for :5173 LISTEN
cd code/src-tauri && DIGIMON_DEBUG_BRIDGE=1 cargo tauri dev --features debug-bridge \
  --config '{"build":{"beforeDevCommand":""}}'   # background; wait for the window
```
Bridge port: read `~/AppData/Roaming/digimon-tcg/debug_bridge.json` (default 5174).

### 3. Capture loop
For `theme` in `dark`, `light`; for each page in the table:
```bash
# menu pages:
curl -s -X POST http://127.0.0.1:$PORT/navigate -H 'Content-Type: application/json' \
  -d "{\"route\":\"$ROUTE\",\"theme\":\"$THEME\"}"
sleep 1
# game board ONLY (stage before navigating so GamePage's mount-fetch finds it):
curl -s -X POST http://127.0.0.1:$PORT/stage -H 'Content-Type: application/json' \
  --data-binary @.claude/skills/update-landing-screenshots/fixtures/hero-board.json
curl -s -X POST http://127.0.0.1:$PORT/navigate -d "{\"route\":\"/game/rust-local\",\"theme\":\"$THEME\"}" -H 'Content-Type: application/json'
sleep 1
# capture + convert:
powershell -File .claude/skills/update-landing-screenshots/scripts/capture_window.ps1 -OutPath "$TMP/$STEM-$THEME.png"
python .claude/skills/update-landing-screenshots/scripts/to_webp.py "$TMP/$STEM-$THEME.png" \
  "code/landing/assets/screenshots/$STEM-$THEME.webp" --width 960
```
**`Read` each WebP** and confirm the right page + theme + a clean crop before moving on (the no-approximations habit: verify, don't assume). Re-capture any that are wrong (wrong theme = settle longer; bad crop = pass `--crop L T R B`).

### 4. Wire `index.html` (first run only)
The gallery section references the fixed asset paths above. If it's absent
(first run), add it per the plan/Task 8. On recaptures, only the WebPs change.

### 5. Publish
```bash
git add code/landing/assets/screenshots/*.webp code/landing/index.html
git commit -m "chore(landing): refresh desktop screenshots"
git push origin HEAD:main      # triggers .github/workflows/landing-page.yml
```
Confirm the deploy: `gh run list --workflow=landing-page.yml --limit 1`.

### 6. Teardown
Stop `cargo tauri dev` + the dev server.

## Gotchas
- **Browser ≠ desktop:** a Playwright shot of `:5173` renders the right chrome but empty backend data (decks/models) and can't show invoke-only pages. Always use the real app.
- **Game board shows the setup form:** `/stage` must precede the `/game/rust-local` navigate; add a settle if needed.
- **Native title bar in the shot:** `capture_window.ps1` already crops to the client area; if a sliver remains, pass `--crop` to `to_webp.py`.
- **Stale exe:** never screenshot a prebuilt `digimon-tcg.exe` — it may bundle an old web dist. Always launch via `cargo tauri dev` so it loads the current `:5173` desktop frontend.
````

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/update-landing-screenshots/SKILL.md
git commit -m "feat(skill): update-landing-screenshots recipe"
```

---

### Task 10: Cross-link from `cut-desktop-release`

**Files:**
- Modify: `.claude/skills/cut-desktop-release/SKILL.md` (add a pointer near the end, before "## Reference")

- [ ] **Step 1: Add the pointer** — insert this block before the `## Reference` heading:

```markdown
## After publishing

Consider running the `update-landing-screenshots` skill so the landing-page
gallery reflects this build's UI. It launches the real desktop app, recaptures
each mainstay page in both themes, and republishes the gallery.
```

- [ ] **Step 2: Commit**

```bash
git add .claude/skills/cut-desktop-release/SKILL.md
git commit -m "docs(skill): point cut-desktop-release at update-landing-screenshots"
```

---

### Task 11: End-to-end run — generate all 10 assets + verify

**Files:**
- Create (generated): `code/landing/assets/screenshots/*.webp` (10 files)

- [ ] **Step 1:** Execute the skill recipe (Task 9) against the running app: capture all 5 pages × 2 themes into `code/landing/assets/screenshots/`.

- [ ] **Step 2: Verify every asset.** `Read` each of the 10 WebP files; confirm correct page, correct theme, clean crop. Re-capture any that fail.

- [ ] **Step 3: Verify the page renders with real images:**

Run: `python -c "import pathlib; d=pathlib.Path('code/landing/assets/screenshots'); n=len(list(d.glob('*.webp'))); print('webp count:', n); assert n==10"`
Expected: `webp count: 10`.

- [ ] **Step 4: Commit the assets** (on the feature branch; the skill's own run is what pushes to `main` in real use):

```bash
git add code/landing/assets/screenshots/*.webp
git commit -m "chore(landing): initial desktop screenshot assets (10, both themes)"
```

- [ ] **Step 5: Teardown** — stop `cargo tauri dev` + the dev server.

---

## Notes for the implementer

- **Build isolation (rule 31):** if you hit a compile error in a file you didn't touch, suspect shared-target contamination — prefix cargo with `CARGO_TARGET_DIR='D:/cargo-target-wt/priceless-vaughan-28cff2'` (or the base target `D:/cargo-target/digimon-deck-list-builder-1` to reuse the warm cache).
- **Don't run `cargo tauri dev` and a `src-tauri` `cargo test` at once** — two concurrent crate builds corrupt the build. Stop the app before Task 1/6 test runs.
- **DCGO / rules** are not involved here.
- The `.trial/` scratch dir and `.superpowers/` are gitignored (this session); `.trial/` holds the comparison PNGs if useful for reference.
