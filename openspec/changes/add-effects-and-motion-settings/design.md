## Context

The frontend already has a mature dual-theme system: a `data-theme` attribute on
`<html>`, a Zustand `themeStore` that persists the choice, and a pre-paint bootstrap
in `index.html` that sets the attribute before React mounts (no flash-of-wrong-theme).
Persisted UI preferences follow a consistent pattern in `uiStore.ts` — a storage key,
`loadPersisted*` / `persist*` helpers, validation against a known set, and an
`__uiStoreInternals` export for tests (`botSpeed`, `railCollapsed`,
`deckBuilderView`, `graphicsPreset` all do this).

The animation surface is split across two files: the game board / in-game library
lives in `index.css` (`@keyframes` + `animate-*` classes for card play, digivolve
banner, battle slash/shake, security reveal/break, phase banner), and the app-shell
atmosphere lives in `design/components/components.css` (`.ds-backdrop` scanlines /
dot-grid + the `ds-crt-scan` looping animation). Critically, only `ds-crt-scan` is
wrapped in a `@media (prefers-reduced-motion: reduce)` guard; everything in
`index.css` animates unconditionally.

This change is the foundation for a roadmap of "make it alive" changes
(`add-cursor-follow-lighting`, `add-live-theme-atmosphere`,
`animate-board-atmosphere`, `upgrade-digivolve-cutin`). Those need one shared,
user-controllable, accessibility-aware motion gate to read from.

## Goals / Non-Goals

**Goals:**
- A single persisted Motion preference (`full` / `reduced` / `off`) defaulting from
  the OS `prefers-reduced-motion` setting.
- A persisted Live-background toggle (consumed later; defined now so the gate and
  control exist before atmosphere ships).
- A global `data-motion` attribute on `<html>`, set pre-paint, mirroring the theme
  bootstrap, so animation gating is pure CSS where possible (no JS flash).
- Retrofit the existing animation library so ambient/looping effects stop at
  `reduced`/`off` while functional one-shot feedback survives `reduced`.
- A shared way for downstream features to read the effective motion level.

**Non-Goals:**
- Building any live background, cursor lighting, or new VFX (those are later changes).
- Per-effect granularity / an "effects gallery." One coarse Motion level + one
  Live-background switch is the whole surface for now.
- A general settings framework. We extend `uiStore` + the existing Graphics Settings
  page, nothing more.
- Audio / SFX (deferred to its own change).
- Touching the engine, gameplay, RL, or network layers.

## Decisions

### Decision: Three-level Motion preference, not a single boolean
A boolean "reduce motion" can't express the difference between "I want functional
feedback but no ambient noise" (`reduced`) and "I want the app dead still" (`off`).
Three levels map cleanly onto the two classes of animation we already have (ambient
vs functional) plus a full-stop. Alternative considered: a boolean plus a separate
"background on/off" — rejected because it conflates the accessibility axis (motion)
with the cosmetic axis (live background), and gives no path to `off`.

### Decision: `data-motion` attribute on `<html>`, set pre-paint
Mirror the proven `data-theme` mechanism exactly. The pre-paint bootstrap in
`index.html` resolves the effective level (persisted value → else OS query → else
`full`) and writes `document.documentElement.dataset.motion` before React mounts.
This lets the bulk of gating be CSS selectors (`[data-motion="off"] .animate-*`,
`:where([data-motion="full"]) .ds-backdrop::after { animation: … }`), which is
cheaper and flash-free versus toggling classes from React after hydration.
Alternative considered: a React context that conditionally renders/animates —
rejected for the hydration flash and because CSS-only gating keeps the animation
definitions co-located with their styles.

### Decision: CSS-first gating, with the store as the source of truth
The store owns the value and persistence; the attribute is its projection (the
`ThemeProvider`/bootstrap pattern). Animations are gated in CSS keyed on
`data-motion`. The `prefers-reduced-motion` media query is folded into the *default
derivation*, not used as the runtime gate — once a user picks a level we honor their
explicit choice over the OS hint (but the OS hint seeds the first-run default). The
existing lone `ds-crt-scan` `@media` guard is replaced by the `data-motion` gate so
there is one mechanism, not two.

### Decision: Classify the existing animation library into ambient vs functional
Each `@keyframes`/`animate-*` is tagged as either *ambient/looping* (CRT scan, any
idle pulse/glow loops, the binary/atmosphere layers once they animate) or *functional
one-shot* (card-enter, card-play-in, security-reveal, digivolve banner, battle slash,
phase banner). Gating rule: ambient stops at `reduced`; functional stops only at
`off`. This classification is documented in the change and encoded as CSS so later
changes inherit the convention. Alternative considered: gate everything at `reduced`
— rejected because losing card-play / security-reveal feedback harms game legibility
for users who only wanted less *ambient* noise.

### Decision: Live background is a gated cosmetic toggle, not an accessibility control
`liveBackground` is stored independently but its *effective* state is
`motion === 'full' && liveBackground`. This keeps the accessibility guarantee
(reduced/off ⇒ static background) regardless of the cosmetic toggle, and means the
later atmosphere change reads one resolved boolean.

## Risks / Trade-offs

- [Retrofitting a large existing animation library risks missing a keyframe or
  regressing a functional animation] → Enumerate every `animate-*` class and
  `@keyframes` in `index.css` + `components.css` in the task list and classify each
  explicitly; a visual pass per theme at each motion level before completion.
- [CSS `data-motion` gate and a stale JS path could disagree] → Single source of
  truth in the store; attribute is its projection; a bootstrap-parity test asserts
  the `index.html` storage key matches the store literal (same pattern as the theme
  bootstrap parity test).
- [Pre-paint bootstrap runs before the bundle and must not throw] → Keep it tiny and
  defensive (try/catch around `localStorage` and `matchMedia`, like the theme
  bootstrap), defaulting to `full` on any error.
- [Over-suppressing at `off` could hide state changes (e.g. a card silently moving
  zones)] → `off` removes the *animation*, not the state change; transitions collapse
  to instant, never to "nothing happened."
- [Scope creep into a full effects-settings panel] → Hard non-goal; exactly two
  controls this change.

## Migration Plan

Frontend-only, additive, no data migration. First run with no persisted value derives
the default from the OS; existing users get `full` unless their OS requests reduced
motion. Rollback is reverting the change — the attribute and store fields are new and
unread by anything pre-existing. Ship behind no flag; the controls simply appear in
Graphics Settings.

## Open Questions

- Should the Motion control be a 3-way segmented control or a dropdown? (UI detail;
  resolve during implementation to match the existing Graphics Settings row styling.)
- Does `reduced` need to also dampen non-looping but *long* transitions (e.g. the
  1.4s digivolve banner), or only kill loops? Leaning: `reduced` keeps it but a later
  cut-in change may shorten it; revisit when `upgrade-digivolve-cutin` lands.
