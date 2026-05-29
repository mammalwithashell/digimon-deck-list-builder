## ADDED Requirements

### Requirement: Graphics Settings page exposes resolution presets and fullscreen toggle

The desktop client SHALL provide a Graphics Settings page that lists
exactly eight resolution presets — 1024×576, 1280×720, 1600×900,
1920×1080, 2560×1440, 3440×1440, 3840×2160, 5160×2160 — plus a
fullscreen on/off toggle. The presets MUST appear in the order listed.
The page MUST be reachable from the desktop client's main navigation.

#### Scenario: User opens Graphics Settings

- **WHEN** the user navigates to the Graphics Settings page
- **THEN** the page renders eight preset buttons in the specified order
- **AND** a fullscreen toggle is visible above the preset list
- **AND** the currently active preset is visually marked as selected

#### Scenario: Browser build does not show the page

- **WHEN** the application is running in a non-desktop build (i.e.,
  `VITE_BUILD_TARGET !== 'desktop'`)
- **THEN** the Graphics Settings page route is not reachable
- **AND** no canvas scaling is applied

### Requirement: Selecting a preset resizes the window immediately

When the user clicks a resolution preset, the desktop window SHALL be
resized to the preset's width and height via Tauri's window API. The
change MUST take effect without an "Apply" step and without a process
restart.

#### Scenario: User clicks 1920×1080

- **WHEN** the user clicks the 1920×1080 preset
- **THEN** the application calls `appWindow.setSize` with logical size
  1920×1080
- **AND** the window immediately resizes to 1920×1080
- **AND** the canvas scales to fit the new window dimensions

#### Scenario: User toggles fullscreen on

- **WHEN** the user enables the fullscreen toggle
- **THEN** the application calls `appWindow.setFullscreen(true)`
- **AND** the window enters fullscreen on the current monitor
- **AND** the canvas scales to fit the fullscreen dimensions

#### Scenario: User toggles fullscreen off

- **WHEN** the user disables the fullscreen toggle while fullscreen
- **THEN** the application calls `appWindow.setFullscreen(false)`
- **AND** the window returns to the last selected preset's dimensions

### Requirement: Selected preset persists across launches

The selected resolution preset and fullscreen state SHALL be persisted
in `localStorage` and restored on application start. On the first
launch with no persisted value, the default preset MUST be 1280×720
and fullscreen MUST be off.

#### Scenario: User picks a preset, restarts, sees the same window size

- **WHEN** the user selects 2560×1440 in Graphics Settings
- **AND** closes and relaunches the application
- **THEN** the application restores the window to 2560×1440 on launch
- **AND** the Graphics Settings page shows 2560×1440 as selected

#### Scenario: First launch has no persisted preset

- **WHEN** the application starts and `localStorage` contains no
  `desktop.graphicsPreset` entry
- **THEN** the application applies the 1280×720 preset
- **AND** fullscreen is off
- **AND** the preset is persisted to `localStorage`

### Requirement: Game UI renders inside a fixed 1920×1080 internal canvas

The desktop game UI SHALL be wrapped in a scaler component that always
renders a 1920×1080 inner box. The scaler MUST apply a uniform
`transform: scale(s)` where `s = min(window.innerWidth / 1920,
window.innerHeight / 1080)`. The inner box MUST be centered in the
window with its `transform-origin` at top-left.

#### Scenario: Window is 1920×1080

- **WHEN** the window is exactly 1920×1080
- **THEN** the scale factor is 1.0
- **AND** the canvas occupies the full window

#### Scenario: Window is 3840×2160 (4K)

- **WHEN** the window is 3840×2160
- **THEN** the scale factor is 2.0
- **AND** the 1920×1080 canvas renders at 3840×2160 with no margins

#### Scenario: Window is 1024×576

- **WHEN** the window is 1024×576
- **THEN** the scale factor is approximately 0.533
- **AND** the canvas is uniformly shrunk; no media-query-driven layout
  changes occur inside the canvas

### Requirement: Ultrawide windows are letterboxed

The canvas SHALL be letterboxed (fit by height, horizontally centered,
surrounded by a solid background color) whenever the window's aspect
ratio is wider than 16:9 — for example the 3440×1440 preset at 21.5:9.
The canvas MUST NOT stretch to fill window width. The letterbox area
MUST render as the application background color (typically black).

#### Scenario: Window is 3440×1440

- **WHEN** the window is 3440×1440
- **THEN** the scale factor is `min(3440/1920, 1440/1080) = 1.333…`
- **AND** the canvas renders at 2560×1440, centered in the 3440-wide
  window
- **AND** approximately 440px of background color appears on each side

### Requirement: Battle area always renders 14 slots in 2 rows of 7

The opponent's and player's battle areas SHALL each render exactly 14
slots arranged as 2 rows of 7 columns. The grid layout MUST NOT vary
with window size because the canvas size is fixed; the layout MUST be
expressed via `grid-template-columns: repeat(7, ...)` (or equivalent)
so the 14-slot count produces exactly two rows.

#### Scenario: Empty battle area at any preset

- **WHEN** the battle area is rendered with zero permanents at any
  resolution preset
- **THEN** the user sees 7 empty slot placeholders in row 1 and 7 in
  row 2

#### Scenario: Battle area with 8 permanents

- **WHEN** the battle area contains 8 permanents (engine
  `battle_area.len() == 8`)
- **THEN** slots 1–7 (row 1) are filled with permanents 0–6
- **AND** slot 8 (row 2, leftmost) is filled with permanent 7
- **AND** slots 9–14 (rest of row 2) render as empty placeholders

### Requirement: Permanent positions animate when middle cards are removed

The UI SHALL animate surviving permanents sliding to their new visual
positions whenever the engine removes a permanent from a non-terminal
`field_index` (causing later permanents to shift left). The animation
MUST complete within approximately 200–300ms using an ease-out curve.
Cards MUST NOT teleport between positions when only an engine-index
shift occurred without a re-key.

#### Scenario: Middle permanent is deleted

- **WHEN** the player has permanents at engine slots [0, 1, 2, 3, 4]
  (visual slots 1–5)
- **AND** the permanent at engine slot 2 is deleted (e.g., destroyed
  in combat)
- **AND** the engine's resulting state has permanents at engine slots
  [0, 1, 2, 3] (the former slot 3 is now at slot 2, etc.)
- **THEN** the cards previously at visual slots 4 and 5 animate
  sliding left to visual slots 3 and 4
- **AND** the animation completes within 300ms
- **AND** the card previously at visual slot 3 (the deleted one) does
  not animate (it's been removed)

#### Scenario: Last permanent is deleted

- **WHEN** the player has permanents at engine slots [0, 1, 2]
- **AND** the permanent at engine slot 2 (last) is deleted
- **THEN** no surviving permanent changes visual slot
- **AND** no slot-shift animation occurs

#### Scenario: New permanent is played

- **WHEN** a permanent enters the battle area at engine slot
  `battle_area.len() - 1` (the natural append position)
- **THEN** the existing card-play animation
  (`animate-card-play-in`) plays at the new slot
- **AND** no other slot animates

### Requirement: Window resize is disabled outside of preset selection

The desktop window SHALL NOT be user-resizable via window-edge dragging.
The Graphics Settings page is the only path through which the window
dimensions may change. Tauri configuration MUST set the window's
`resizable` flag to false, and the runtime MUST call
`appWindow.setResizable(false)` after applying each preset.

#### Scenario: User tries to drag the window edge

- **WHEN** the user attempts to drag the window's edge to resize
- **THEN** the window does not resize
- **AND** the cursor does not change to the resize affordance

#### Scenario: Fullscreen does not require setResizable(true)

- **WHEN** the user enables fullscreen
- **THEN** the application enters fullscreen even though the window is
  marked `resizable: false`
- **AND** exiting fullscreen returns to the previously selected
  preset's dimensions

### Requirement: Default startup window matches the 1280×720 preset

The application SHALL start at 1280×720 in windowed mode on a fresh
install with no persisted graphics preferences. The Tauri configuration's
default window size MUST match 1280×720 so the initial pre-React window
flash does not differ from the post-React applied preset.

#### Scenario: First-ever launch

- **WHEN** the user installs the application and launches it for the
  first time
- **THEN** the Tauri window opens at 1280×720
- **AND** the React app reads the absent localStorage preset
- **AND** the React app applies 1280×720 without resizing (no flicker)
- **AND** the React app persists 1280×720 as the selected preset
