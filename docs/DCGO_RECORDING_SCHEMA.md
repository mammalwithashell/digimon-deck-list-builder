# DCGO Recording JSONL Schema

This document specifies the JSONL recording format produced by the modded
DCGO client (`DCGO/Assets/Scripts/Script/Recording/GameRecorder.cs`) and
consumed by the Rust replay harness (`code/tools/dcgo-replay/`) and the
behavioral-cloning dataset emitter (planned).

The schema is the contract between the C# producer and the Rust consumers.
It's defined in code at:

- **Producer**: `DCGO/.../Recording/GameRecorder.cs`
- **Consumer types**: `code/tools/dcgo-replay/src/recording.rs`
- **Schema version constant**: `digimon_engine::action::space::SCHEMA_VERSION`
- **Spec (normative)**: `openspec/changes/add-dcgo-recording-parity-harness/specs/dcgo-parity-harness/spec.md`

This document is descriptive — when the spec and this doc disagree, the
spec wins.

## Wire format

One JSON object per line, UTF-8. Lines are read in order; ordering of rows
within a recording is significant (the replay harness consumes them
left-to-right; the engine consumes reveals in stream order via a FIFO
`RevealQueue`).

Each row carries a `"type"` field — the discriminator the parser uses to
deserialize into the right variant.

Recognized types: `game_start`, `action`, `encoder_failure`, `reveal`,
`game_end`. Unknown types are tolerated (Rust harness reads them as
`Row::Unknown` and skips) for forward compatibility.

A well-formed recording starts with exactly one `game_start`, ends with
exactly one `game_end`, and has any mix of `action` / `encoder_failure` /
`reveal` rows in between.

## Header row: `game_start`

Emitted exactly once, as the first line.

```jsonc
{
  "v": 1,                                  // SCHEMA_VERSION
  "type": "game_start",
  "game_id": "abc123def456",               // stable per-game uuid
  "timestamp": "2026-05-26T14:00:00Z",     // UTC ISO-8601
  "my_player_id": 0,                       // 0 or 1 — local client's seat
  "first_player": 1,                       // 0 or 1 — who takes turn 1 (absent in pre-2026-08-16 recordings)
  "is_ai": true,                           // true for Bot Match, false for PvP

  // Local player's deck, post-shuffle order. Drawn from index 0 first.
  "my_deck_post_shuffle": ["BT1-001", "BT1-001", "BT1-010", ...],

  // Bot Match: opponent's deck in post-shuffle order (both decks observable).
  // PvP: null — the local client doesn't see the authoritative shuffle.
  "opp_deck_post_shuffle": ["BT1-001", ...] | null,

  // OPTIONAL. Required for PvP (when opp_deck_post_shuffle is null).
  // The opponent's decklist as an unordered multiset — composition without
  // post-shuffle order. DCGO reads this from the opponent's Photon
  // CustomProperties under the "BattleDeckData" key (both players publish
  // their decklists there during room setup).
  // The replay harness uses this to construct an opaque-mode game via
  // Game::new_with_opaque_opponent.
  "opp_decklist_composition": ["BT1-001", "BT1-001", "BT1-010", ...],

  // OPTIONAL (absent in pre-2026-08-16 recordings). Local player's
  // digitama (egg) deck, post-shuffle order — index 0 is hatched first.
  "my_egg_deck": ["ST1-01", "ST1-01", ...],

  // OPTIONAL. Bot Match only: opponent's digitama deck, post-shuffle
  // order. Absent for PvP (the opponent's digitama order is not
  // observable, same as their main deck) and in older recordings.
  "opp_egg_deck": ["ST1-01", ...]
}
```

### Field reference

| Field | Type | Required | Notes |
|---|---|---|---|
| `v` | u32 | yes | Schema version (`SCHEMA_VERSION`, currently 1). Harness rejects unknown versions. |
| `type` | "game_start" | yes | Discriminator. |
| `game_id` | string | yes | Stable per-game UUID; appears in parity reports. |
| `timestamp` | string | yes | UTC ISO-8601. |
| `my_player_id` | u8 | yes | 0 or 1 — the local client's player ID. |
| `first_player` | u8 | no | 0 or 1 — who takes turn 1 (DCGO's `NonTurnPlayer` at `StartGame`). Absent in recordings made before 2026-08-16; the replay adapter then infers it from the first mulligan actor (DCGO's first player mulligans first). |
| `is_ai` | bool | yes | true for Bot Match (against DCGO's heuristic bot); false for PvP. |
| `my_deck_post_shuffle` | string[] | yes | Local player's deck (post-shuffle order). |
| `opp_deck_post_shuffle` | string[] \| null | yes | Bot Match: ordered. PvP: null. |
| `opp_decklist_composition` | string[] | no\* | \*Required for PvP. The opponent's decklist as an unordered multiset (composition only). |
| `my_egg_deck` | string[] | no | Local player's digitama (egg) deck, post-shuffle order (index 0 hatched first). Absent in pre-2026-08-16 recordings; the replay adapter then uses an empty egg deck. |
| `opp_egg_deck` | string[] | no | Opponent's digitama deck, post-shuffle order. Bot Match only — absent for PvP and in older recordings. The replay adapter appends both egg lists to the ordered deck lists; `Game`'s card-kind routing places `DigiEgg` cards into the digitama deck without re-shuffling. |

## Decision row: `action`

Emitted once per encoded decision. Both players' decisions interleave in
turn-of-decision order.

```jsonc
{
  "type": "action",
  "step": 0,                      // monotonic step counter (action + encoder_failure + reveal combined)
  "actor": 0,                     // 0 or 1 — the player making the decision
  "action_id": 0,                 // 0..2191 — the 2192-space action ID
  "phase": "Mulligan",            // DCGO phase name at encoding time (debugging breadcrumb)
  "source": "mulligan"            // emitter-side categorical (mulligan|main_phase|selection_int|selection_bool|play_card_extra)
}
```

The `action_id` is encoded by `ActionEncoder.cs` (DCGO-side) using
`ActionSpace.cs` (codegen'd from `digimon_engine::action::space`). The
`phase` field is a string version of the DCGO `GameContext.TurnPhase`
enum — useful for debugging mis-encodes but ignored by the replay
harness (which infers phase from engine state).

The `source` field tells you which emitter site produced the row:

| source | Emitter site |
|---|---|
| `mulligan` | `TurnStateMachine.SetRedraw` |
| `main_phase` | `TurnStateMachine.QueueMainPhaseAction` (any of the 6 MainPhaseAction subclasses) |
| `selection_int` | `UserSelectionManager.SetIntForPlayer` |
| `selection_bool` | `UserSelectionManager.SetBoolForPlayer` |
| `play_card_extra` | A digivolution-source pick decomposed out of a `PlayCardAction`'s baked-in jogress / burst / app-fusion fields |

### Board-position index space

Every board reference in a recording — `action_id` operands for attacks,
digivolves and field effects, plus the `targets` of a `selection` row —
is a **compact battle-area index**: the position of the permanent within
the player's packed list of occupied slots, counting from 0.

This matters because DCGO itself has two index spaces and they are easy
to confuse:

| DCGO representation | Shape | Matches our action space? |
|---|---|---|
| `Player.FieldPermanents[]` indexed by `FieldCardFrame.FrameID` | Sparse — empty frames are `null` holes | **No** |
| `Player.GetFieldPermanents()` | Compact, ascending frame order | **Yes** |

The gameplay packets carry the compact form already
(`TurnStateMachine.SetActSkill` and `SetAttackingPermaent` both index
`GetFieldPermanents()` directly), so the encoder passes them through
after a bounds check (`ActionEncoder.ValidateFieldSlot`). The one packet
that carries a *sparse* frame id is `PlayCardAction.TargetFrameID`, which
`ActionEncoder.FrameIdToFieldSlot` converts.

#### Why board operands are remapped at replay time

Compact index alone is still not enough. DCGO compacts by ascending frame
id, and permanents **migrate between frames at runtime**
(`CardController` repositions them toward `PreferredFrame`), while the
engine's `battle_area` is in play order. The two orderings disagree
routinely, not exceptionally — a recorded `attack_0` can mean the
engine's slot 1.

So every `action` and `selection` row carries a snapshot of both players'
battle areas:

```jsonc
board_p0: [EX10-010, BT16-082],   // DCGO compact order, card IDs
board_p1: [EX12-035]
```

At replay time `remap_board_slots` (and `remap_selection_targets`) match
these against the engine's board by card identity and rewrite the board
operands of attack / digivolve / field-effect / source-select action IDs
and of selection `targets`. Duplicates are paired in occurrence order,
which is exact whenever both sides hold the same multiset.

A slot whose card the engine does not have maps to nothing and the
operand is left alone, so a genuine board desync surfaces as a divergence
instead of being silently rewritten onto the wrong permanent. Recordings
without these fields (pre-0.4) replay unchanged, on the assumption that
the orders agree.

## Selection row: `selection`

Emitted for each response to a `Select*Effect` prompt — the semantic
counterpart to an `action` row. Where an `action` row carries a resolved
2192-space `action_id`, a `selection` row carries the *payload* of the
choice and lets the replay harness resolve it against whatever
`PendingSelection` the engine has installed at that point. That
indirection is deliberate: the same DCGO prompt maps to different engine
action IDs depending on which selection is pending.

```jsonc
{
  "type": "selection",
  "step": 11,                        // monotonic step counter (shared with action rows)
  "actor": 0,                        // the player answering the prompt
  "prompt": "SelectPermanentEffect", // which Select*Effect asked
  "phase": "Main",                   // DCGO phase name at capture time
  "targets": [{"player": 0, "frame": 3}]  // payload — shape varies by prompt
}
```

Exactly one payload field is present, keyed by prompt kind:

| `prompt` | Payload field | Meaning |
|---|---|---|
| `SelectPermanentEffect` | `targets` | Battle-area picks: `player` is the absolute player ID, `frame` the compact battle-area index (see above) |
| `SelectAttackEffect` | `targets` | Attack target; `frame: -1` means the player/security rather than a Digimon |
| `SelectHandEffect` | `card_ids` | Hand picks, by card ID |
| `SelectCardEffect` | `card_ids` | Generic card picks (deck/trash/security), by card ID |
| `SelectCountEffect` | `count` | A numeric "how many" answer |
| `SelectDigiXrosClass` | `int_value` | Chosen DigiXros recipe index |
| `MultipleSkills` | `int_value` | Which of several simultaneous effects to process first |
| `OptionalSkill` | `bool_value` | Yes/no on a "you may" trigger |
| `generic_bool` | `bool_value` | `UserSelectionManager` fallback channel |

A `cancel: true` field replaces the payload when the player backed out of
the prompt.

Resolution happens in `runners/selection_resolve.rs`: the harness reads
the engine's live `PendingSelection`, matches the recorded payload
against the offered targets (by identity for card picks, by index for
board picks and effect choices), and emits the corresponding action ID —
or `PASS` for a decline.

## Sentinel row: `encoder_failure`

Emitted when the DCGO recorder cannot map a decision to a 2192-space ID.
Carries diagnostic context for offline analysis. The replay harness halts
cleanly with `ReplayOutcome::PartialPass` when it hits one of these — the
engine is still waiting for the unencoded decision, so subsequent action
rows would be rejected as out-of-sequence.

```jsonc
{
  "type": "encoder_failure",
  "step": 5,
  "actor": 0,
  "phase": "Main",
  "source": "selection_int",
  "reason": "selection_prompt_kind_unknown",   // structured failure reason
  "raw_value": "int_value=3 phase=Main"        // human-readable debug info
}
```

Common reasons:

| `reason` | Meaning |
|---|---|
| `selection_prompt_kind_unknown` | A `SetIntForPlayer` / `SetBoolForPlayer` call fired but the recorder doesn't yet know which `Select*Effect` is prompting (Phase 1 fallback; task 3.5 follow-up plumbs prompt identity). |
| `play_card_hand_index_out_of_range` | `PlayCardAction.CardIndex` outside [0, 30). |
| `attack_attacker_frame_lookup_failed` | The attacker's compact index was out of range for the actor's battle area. |
| `activate_card_nonzero_skill_index` | Activating a hand effect with skill_idx > 0 — the action space's `HAND_EFFECT` range is single-skill-per-slot. |
| `activate_permanent_nonzero_skill_index` | Activating the 2nd+ `[Main]` ability of a permanent — the action space reserves one `[Main]` sub-slot per permanent. |
| `digivolve_frame_beyond_engine_slots` | The digivolve target resolved past `MAX_FIELD_SLOTS` (more than 14 occupied battle slots), or onto an empty frame. |
| `cheat_action_unsupported` | Debug-mode `CheatAction` (never appears in real recordings). |

## Reveal row: `reveal`

Emitted only in PvP, every time an opaque-opponent card becomes visible
to the local client (draws, security pops, mill effects). Loaded into the
engine's `RevealQueue` at replay-construction time and consumed in stream
order as the engine performs opaque-pile reads.

```jsonc
{
  "type": "reveal",
  "step": 5,             // monotonic step counter (shared with actions/encoder_failures)
  "actor": 1,            // the opaque opponent's player ID
  "card_id": "BT15-104", // the revealed card's identity
  "source": "draw"       // one of: "draw" | "security" | "mill" | "effect"
}
```

The `source` field tags the engine's `RevealKind`:

| `source` | `RevealKind` | When emitted |
|---|---|---|
| `draw` | `Draw` | `DrawClass.Draw()` consumes a card from the opponent's deck into hand |
| `security` | `Security` | `IBreakSecurity.SecurityCheck()` flips a face-down security card face-up |
| `mill` | `Mill` | `IAddTrashCardsFromLibraryTop` trashes a card from the opponent's deck top |
| `effect` | `Effect` | Effect peeks at top of opponent's deck (catch-all) |

**Note on security ordering**: security reveals appear in the JSONL in
flip-order (= position order, since security is popped top-to-bottom).
The replay harness preloads them into the RevealQueue; the engine
consumes them lazily when `SecurityCheck` flips the top of an opaque
opponent's security stack (see `Game::materialize_opaque_security_placeholder`).

## Terminal row: `game_end`

Emitted exactly once, as the last line.

```jsonc
{
  "type": "game_end",
  "winner": 1,                 // 0 or 1 for the winning player; -1 for draw/disconnect
  "reason": "concede",         // free-text reason
  "total_steps": 14            // cumulative step counter (same scale as `step` field)
}
```

### Reason categories

The DCGO recorder writes:

| `reason` | Meaning |
|---|---|
| `concede` | One player conceded (`Surrendered=true`); the conceder's opponent wins. |
| `disconnect` | Photon room lost both players or terminated without a winner. Often pairs with `winner: -1`. |
| `effect:<name>` | A specific card effect ended the game (e.g., security-zero attacks, an alt-win condition). The `<name>` part is the DCGO effect-source name. |
| `win` | Generic natural win (typically security-zero attack with no specific effect name). |

## Recording file naming

Files are written under `Application.persistentDataPath/dcgo_recordings/`:

- Windows: `%APPDATA%\..\LocalLow\<company>\<product>\dcgo_recordings\`
- macOS: `~/Library/Application Support/<company>/<product>/dcgo_recordings/`
- Linux: `~/.config/unity3d/<company>/<product>/dcgo_recordings/`

Naming convention: `<utc_timestamp>_<game_id>.jsonl`
(e.g. `20260526T140000Z_abc123def456.jsonl`).

One file per game. The recorder appends `game_end` and flushes when
`TurnStateMachine.EndGame` runs; if the application quits mid-game, the
recorder writes a fallback `game_end` row with `reason: "app_quit"` and
`winner: -1`.

## Versioning policy

The `v` field is bumped by `digimon_engine::action::space::SCHEMA_VERSION`
in lockstep with any of:

- A change to the 2192-action-space layout (action IDs shift)
- A change to a JSONL row shape (fields added/removed/renamed in a
  non-backward-compatible way)
- A change to the reveal model (new kinds, ordering semantics)

Adding new **row types** (e.g. a hypothetical `phase_marker` row) does
NOT require a version bump — older harnesses fall through to
`Row::Unknown` and skip them. Adding new **fields** to existing rows
also does not require a bump if the field is `#[serde(default)]`-able
(unrecognized fields are tolerated by serde).

The replay harness validates `v == SUPPORTED_SCHEMA_VERSION` at parse
time and rejects mismatched recordings with a clear error pointing at
the regeneration path.

## Example: minimal bot game

```
{"v":1,"type":"game_start","game_id":"g1","timestamp":"2026-05-26T14:00:00Z","my_player_id":0,"is_ai":true,"my_deck_post_shuffle":["BT1-001",...],"opp_deck_post_shuffle":["BT1-001",...]}
{"type":"action","step":0,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}
{"type":"action","step":1,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}
{"type":"action","step":2,"actor":0,"action_id":62,"phase":"Main","source":"main_phase"}
{"type":"action","step":3,"actor":1,"action_id":60,"phase":"Breeding","source":"main_phase"}
{"type":"action","step":4,"actor":1,"action_id":62,"phase":"Main","source":"main_phase"}
{"type":"action","step":5,"actor":0,"action_id":93,"phase":"Main","source":"main_phase"}
{"type":"game_end","winner":1,"reason":"concede","total_steps":6}
```

## Example: minimal PvP game

```
{"v":1,"type":"game_start","game_id":"pvp1","timestamp":"2026-05-26T14:05:00Z","my_player_id":0,"is_ai":false,"my_deck_post_shuffle":["BT1-001",...],"opp_deck_post_shuffle":null,"opp_decklist_composition":["BT1-001","BT1-010",...]}
{"type":"reveal","step":0,"actor":1,"card_id":"BT1-010","source":"draw"}
{"type":"reveal","step":1,"actor":1,"card_id":"BT1-010","source":"draw"}
{"type":"reveal","step":2,"actor":1,"card_id":"BT1-010","source":"draw"}
{"type":"reveal","step":3,"actor":1,"card_id":"BT1-010","source":"draw"}
{"type":"reveal","step":4,"actor":1,"card_id":"BT1-010","source":"draw"}
{"type":"action","step":5,"actor":0,"action_id":0,"phase":"Mulligan","source":"mulligan"}
{"type":"action","step":6,"actor":1,"action_id":0,"phase":"Mulligan","source":"mulligan"}
{"type":"action","step":7,"actor":0,"action_id":93,"phase":"Main","source":"main_phase"}
{"type":"game_end","winner":1,"reason":"concede","total_steps":8}
```

## See also

- `docs/DCGO_BUILD.md` — building the DCGO mod from source (Unity install, asset bundle, submodule pinning)
- `docs/RUST_ENGINE_API.md` §"Opaque opponent deck mode" — how the engine consumes reveal streams
- `openspec/changes/add-dcgo-recording-parity-harness/` — the original OpenSpec change covering this work
- `code/tools/dcgo-replay/` — the Rust harness that consumes these recordings
- `code/tools/action-space-export/` — the codegen pipeline that keeps `ActionSpace.cs` aligned with `space.rs`
