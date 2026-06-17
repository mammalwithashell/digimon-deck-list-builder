## ADDED Requirements

### Requirement: Proportional vertical band layout

The in-game board SHALL arrange its vertical regions — opponent field, memory
gauge, player field, and hand — using a flow-based layout (flex or grid column)
in which the regions share the board container's available height. The layout
MUST NOT rely on hardcoded pixel offsets that assume the board fills the full
1920×1080 design canvas (e.g. the previous `top:78px` / `bottom:154px` band
offsets).

#### Scenario: Player half is not compressed at 1920×1080

- **WHEN** a game is rendered at the 1920×1080 preset (CanvasScaler scale = 1.0)
- **THEN** the player field band receives vertical space equivalent to the
  opponent field band (within a small tolerance), so neither half appears
  smushed under the memory gauge

#### Scenario: Bands absorb container-height loss proportionally

- **WHEN** the board container is shorter than the design canvas because the
  NavBar and footer bars (BotSpeedControl, Seed readout, ActionBar) consume
  vertical space
- **THEN** the reduction is distributed across the regions proportionally
  rather than landing entirely on the bottom (player) band

### Requirement: Action controls remain within the viewport

The action bar and required gameplay controls SHALL remain fully visible within
the rendered canvas at every supported resolution preset and when the desktop
window is maximized, and MUST NOT be clipped below the viewport or the OS
taskbar. (At minimum: Pass / phase actions / Surrender.)

#### Scenario: Maximized window keeps the action bar reachable

- **WHEN** the desktop window is maximized on a 1920×1080 display
- **THEN** the action bar is fully visible and interactive without scrolling

#### Scenario: Smallest preset keeps the action bar reachable

- **WHEN** a game is rendered at the smallest supported resolution preset
- **THEN** the action bar is fully visible and interactive

### Requirement: Memory gauge does not overlap field rows

The memory gauge SHALL occupy its own vertical band between the opponent and
player fields and MUST NOT visually overlap either field's card rows.

#### Scenario: Gauge sits between the two fields

- **WHEN** the board is rendered at any supported preset
- **THEN** the memory gauge renders clear of both the opponent and player card
  rows, with no overlap

### Requirement: Geometry-dependent behavior is preserved

The layout change SHALL preserve all behavior that depends on the live rendered
geometry of board elements, including attack-target arrows, FLIP card-move
transitions, full-screen overlays, drag-and-drop, and the CanvasScaler design
canvas with its uniform scale and pointer-delta compensation.

#### Scenario: Attack arrow connects the correct elements

- **WHEN** an attack is declared and the attack arrow is drawn
- **THEN** the arrow connects the rendered attacker and target (or security
  area) correctly, because positions are measured from live element rects

#### Scenario: Drag-and-drop drop targets remain accurate

- **WHEN** the player drags a card onto a field slot at any preset scale
- **THEN** the drop registers on the correct slot, with pointer deltas
  compensated for the canvas scale

### Requirement: Cross-build rendering

The board layout SHALL render correctly in both the desktop build
(`VITE_BUILD_TARGET=desktop`, inside the CanvasScaler fixed canvas) and the web
build (which shares the same `GameBoard`), with no regression to either.

#### Scenario: Web build renders the board without regression

- **WHEN** the game board is rendered in the web build (no CanvasScaler)
- **THEN** the regions lay out correctly and no element is clipped or overlapped
