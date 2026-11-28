## ADDED Requirements

### Requirement: Studio Board Staging Fidelity
The Rust engine (`Game::stage_*`, `Game::to_scenario()`) and PyO3 `RustDebugGame` / FastAPI `/debug` + `/studio` staging surface SHALL support authoring and lossless round-tripping of full board states for both Player 1 and Player 2, including:
- Battle-area permanents with bottom-to-top digivolution `stack`, `is_suspended`, `turn_played`, and optional `linked_cards`.
- Breeding-area digivolution stack (bottom-to-top).
- `hand`, `trash`, ordered `deck_top` (index 0 = next draw), ordered `egg_deck` (index 0 = next hatch), and ordered `security` (index 0 = top) with per-card `face_up` state.
- Scalar game state: `memory` (`-10..=10`), `turn` (`>= 1`), `first_player` / active turn player (`1` or `2`), and `phase`.
- Existing `qa/scenarios/*.json` files omitting `egg_deck`, `linked_cards`, or `face_up` security fields SHALL continue to load unchanged.

#### Scenario: Round-trip linked cards, egg deck, and face-up security
- **WHEN** a board is staged with a battle-area permanent carrying `linked_cards: ["BT21-050"]`, a custom `egg_deck`, and a face-up top security card, and `to_scenario()` is called
- **THEN** the captured snapshot preserves `linked_cards`, `egg_deck`, and the top security card's `face_up: true` flag, and re-staging from that snapshot produces an identical board state

#### Scenario: Legacy scenario fixtures remain compatible
- **WHEN** an existing `qa/scenarios/*.json` fixture with string-only `security` arrays and no `egg_deck` or `linked_cards` is loaded
- **THEN** staging succeeds with face-down security defaults and no schema errors

### Requirement: Client-Distribution-Ready Studio Transport and Feature Flag
The Studio frontend route (`/studio`) SHALL be gated by a build-time flag `VITE_ENABLE_STUDIO` (default `true` on web dev/admin builds and `false` on desktop builds so it remains tree-shaken out of desktop releases until explicitly enabled).

All Studio UI components and stores SHALL communicate with the backend exclusively through a typed `StudioTransport` interface (implemented by `HttpStudioTransport` in MVP), avoiding direct `fetch` calls or web-only assumptions inside React Studio components so a future desktop `TauriStudioTransport` can plug in without refactoring the Studio UI.

#### Scenario: Desktop build excludes Studio when flag is disabled
- **WHEN** the desktop bundle is built with `VITE_ENABLE_STUDIO` unset or `false`
- **THEN** `/studio` is not mounted in `DesktopRoutes` and Studio modules are tree-shaken out of the desktop bundle

#### Scenario: Studio UI relies only on StudioTransport abstraction
- **WHEN** Studio components trigger board staging, playout stepping, library operations, or replay loading
- **THEN** every call routes through the injected `StudioTransport` interface rather than calling raw HTTP endpoints directly

### Requirement: Deck Template Initialization and Card Search
The Studio SHALL allow starting a new composition from either:
- Existing archetypes / saved decks from `data/deck_library.json` or saved user decks for Player 1 and Player 2, OR
- Quick blank/filler decks that keep the Rust engine's draw/deck invariants valid.

The Studio SHALL provide an integrated Card Search drawer backed by `cards.json` that defaults to filtering to Rust-implemented cards (`load_implemented_card_ids()`), with a toggle to show all cards (marking unimplemented cards clearly) and filters for text/ID, color, level, and card kind.

#### Scenario: Start from archetype decks and filter implemented cards
- **WHEN** the user opens `/studio`, selects two archetype decks from the deck library template picker, and searches for `"Wingdramon"` in the Card Search drawer
- **THEN** a staged `RustDebugGame` session is created with those underlying decks and the search results display matching Rust-implemented cards first with an option to include unimplemented cards

#### Scenario: Start from blank board with filler decks
- **WHEN** the user chooses "Blank Board" in `/studio`
- **THEN** the Studio initializes an empty battle area, breeding area, hand, security, and trash on both sides backed by valid filler libraries so draws and effects do not crash

### Requirement: Direct On-Board Stack and Zone Composition
In Compose mode, the Studio SHALL render the live `GameBoard` and allow the user to add, reorder, edit, and remove cards directly on the board via drag-and-drop or click-to-place:
- Dropping or placing a card onto an empty battle slot creates a new permanent; dropping onto an existing permanent adds the card to its digivolution stack (top or bottom source) or as a linked card.
- Selecting any battle-area or breeding permanent opens a Stack Inspector allowing reordering/removing sources, toggling `is_suspended`, editing `turn_played` (summoning sickness), managing `linked_cards`, and deleting the permanent.
- Clicking Hand, Security, Trash, Deck Top, or Egg Deck opens a zone editor allowing ordered insertion, removal, reordering, and (for Security) toggling `face_up`.
- Scalar controls allow setting `memory`, `turn`, `phase`, and active turn player, and the backend validates the staged state via `stage_validate()` and surfaces any legality error non-destructively.

#### Scenario: Compose a multi-card digivolution stack on the board
- **WHEN** the user places `EX13-005` onto P1's battle area, drops `EX13-018` on top of it, and opens the Stack Inspector to toggle `suspended = true` and `turn_played = 1`
- **THEN** the backend updates the `RustDebugGame` permanent stack to `["EX13-005", "EX13-018"]` (suspended, `turn_played = 1`) and the live `GameBoard` immediately reflects the stack, top card, sources count, and rotated suspended visual

#### Scenario: Engine validation error surfaced on invalid staging
- **WHEN** the user stages a state that fails `stage_validate()`
- **THEN** the Studio displays the Rust validation diagnostic in-place without discarding the user's current composer edits
