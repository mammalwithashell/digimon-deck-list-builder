## Context

The Rust engine pivot is the active source-of-truth migration (`docs/RUST_PYTHON_PARITY.md`). The per-card faithfulness campaign exists because hand-written behavioral tests cannot reach the long tail of card interactions; bugs hide in cross-card timing, optional-trigger ordering, and resolution edge cases that only emerge in real games. The Python engine partially provided a coverage net (parallel implementations cross-check each other), but as we retire it, we lose that safety net unless we replace it with a different oracle.

DCGO is a fan-maintained Unity client (Unity 2021.3.45f2, Photon PUN) that already implements every released Digimon TCG card. It exposes three play modes — bot match, room match, random match — all routing decisions through a single `MainPhaseAction` packet chokepoint (`TurnStateMachine.QueueMainPhaseAction`) and a single selection chokepoint (`UserSelectionManager.Set{Int,Bool}ForPlayer`). The bot is a random-weighted heuristic baseline (50% mulligan, 85% hatch, 99% attack-if-able, picks first legal target); it is useless as a behavioral-cloning target but ideal as a high-throughput trajectory generator with `isAuto + IsAI` set (DCGO plays both sides unattended).

Our action space (`code/digimon-engine/src/action/space.rs`) is a single Rust module declaring 2192 discrete action IDs across well-documented ranges (play/attack/digivolve/effect/selection). Action IDs are intentionally reused across phases — same numeric ID means different things under `Main` vs `SelectReveal` vs `BlockTiming`, etc. The action layout is already the cross-language contract: Python, frontend, and PyO3 bindings all consume it.

Constraints driving this design:
- DCGO is **always BO1**; our BO3 `SelectPlayOrder` IDs (94/95) and `MatchEnv` wrapper do not apply.
- DCGO has a mulligan but no first-player pick (room creator's seat is fixed).
- In PvP, the client only sees its own deck order — opponent's deck is hidden, revealed incrementally through draws/security pops.
- DCGO's Assets folder is delivered out-of-band per its README; local build setup is non-trivial.
- DCGO runs on a community Photon AppId, not Bandai infrastructure. Local-only recording is benign; we are not modifying transmitted packets.

## Goals / Non-Goals

**Goals:**

- Capture every decision both players make in a DCGO game as a 2192-space action ID, sufficient to deterministically replay the game in the Rust engine and assert parity.
- Run thousands of DCGO bot-vs-bot games unattended, replay each through the Rust engine, and produce a per-card parity report (which card scripts cause divergence, frequency, first-failing step).
- Keep the action-space encoding in DCGO byte-identical to the Rust source of truth via codegen with a CI drift gate.
- Define the JSONL schema as the cross-tool contract: the parity replay binary and the BC dataset emitter both consume the same files.
- Introduce an "opaque opponent deck" engine mode that is also reusable by RL inference against unknown opponents — not a single-purpose feature.
- Stage the work so Phase 1 (bot fuzzer) ships independently and de-risks Phases 2–3.

**Non-Goals:**

- Building a stronger DCGO bot. The bot's data quality is irrelevant; only its action-stream throughput matters.
- Reverse-engineering DCGO's UI or network protocol beyond the documented C# chokepoints.
- Publishing recorded PvP games. The corpus stays local to the training infrastructure.
- Modifying DCGO's gameplay rules, packet format, or matchmaking. The mod is a passive listener with one-line call-site additions; no Photon traffic changes.
- Supporting historical DCGO commits. We pin to one upstream commit per the submodule pointer; rebasing the patch is on demand.
- BC pretraining the policy as part of this change. The dataset emitter is in scope; integration with `pilot_training.py` is a follow-up.
- Reconstructing opponent deck composition in PvP from observed reveals alone. Opaque-deck mode treats the opponent's deck as a black box with externally-supplied reveals — we do not infer.

## Decisions

### Encode-at-record-time, not at replay-time

The DCGO mod encodes each decision into a 2192-space action ID *before* writing the JSONL row. The recording format is `(actor, action_id)` — engine-clean, no DCGO-specific payloads in the wire format.

Considered: keep DCGO's native packet format in the JSONL and run an offline Rust/Python converter. Rejected because (a) the converter would need card-data alignment + phase context anyway, so the complexity doesn't reduce, just relocates; (b) recording-time encoding makes the mod a fuzzer for the action space itself — any legal-in-DCGO decision that has no representation surfaces as an encoder failure, which is information we want.

Consequence: action-mapping logic lives in C# (in `DCGO/Assets/Scripts/Script/Recording/ActionEncoder.cs`), driven by codegen so it cannot drift from Rust.

### Codegen the C# table from `space.rs`

A new Rust workspace member `code/tools/action-space-export/` imports `digimon_engine::action::space::*` and prints a JSON descriptor of all ranges, constants, formulas, and phase scopes. A small Python emitter consumes the JSON and writes `DCGO/Assets/Scripts/Script/Recording/ActionSpace.cs` (constants + formula helpers). CI runs the regeneration and diffs against committed output.

Considered: parse `space.rs` directly with regex or `syn`; hand-maintain the C# table. Rejected because the existing constants include arithmetic-on-prior-constants (e.g., `BREEDING_SOURCE_SELECT_END = SOURCE_SELECT_END + BREEDING_SOURCE_CARRIERS * SOURCES_PER_FIELD`) — runtime printing gives us resolved values without recomputing expressions. Hand-maintenance is rejected for the obvious reason (drift).

### Phase-aware encoder, with explicit phase context in JSONL

Because action IDs are reused across phases, the encoder needs to know what kind of decision is being asked for. It cannot infer phase from the action payload alone — the same payload (e.g., "value=3") might mean "select revealed card 3" or "select security 3" depending on the prompt.

The encoder takes its phase context from DCGO state: `GameContext.TurnPhase` plus the currently-pending `SelectXEffect` instance (if any). The JSONL emits a `phase` field on every action row — redundant for the replay harness (the engine knows its own phase), but invaluable for debugging mis-encodes.

Considered: omit `phase` from JSONL and trust the engine to know. Rejected; the cost of the field (a small string per row) is dwarfed by the cost of debugging a misencoded recording without it.

### One-sided replay; opponent acts as scripted-input source

The Rust replay harness consumes recordings from one player's perspective. The opponent's actions are not "replayed" in the sense of re-deriving them from a policy — they are fed into the engine as the recording dictates. The engine still has to *resolve* every action correctly (calculate cost reductions, fire triggers, apply modifiers); we only stipulate which decisions were taken.

Considered: two-sided recording (both clients modded, logs merged). Rejected for the PvP path because we cannot coordinate with random ladder opponents. Kept available for self-play sessions (you + a friend, both modded) as a debug aid but not a required mode.

### Opaque-opponent-deck as a first-class engine mode

For PvP replay, the engine cannot pre-shuffle the opponent's deck because we don't know its order. Instead, `Game::new_with_opaque_opponent(my_deck_in_order, opp_decklist_unordered)` initializes the opponent's deck as an *opaque* pile of size N with known composition. When the engine would draw, mill, or pop security from that pile, it calls into a `RevealSource` (a queue or callback) supplied by the harness, which returns the next recorded card.

Considered alternatives:
- *Defer opaque-deck to the harness*: have the harness intercept draws and inject card identities. Rejected — the engine's draw path is deep in coroutines and runs implicitly during effect resolution; intercepting at the harness layer means re-implementing draw logic outside the engine.
- *Don't replay PvP at all, only bot*: Phase 1 alone. Rejected as the long-term target; PvP recordings are the main BC-seed-data source. Phasing the work so Phase 1 ships first is sufficient mitigation, not a reason to drop Phase 3.

Side benefits: the opaque-deck path is also what `DigimonEnv` needs for RL inference against an unknown opponent (currently approximated by exposing the full deck to the agent — a known information leak). Sharing this code path is a non-trivial win.

### Parity oracle: legality-checked action acceptance + winner match

Each step of replay asserts two things:
1. The action ID is legal under the Rust engine's current mask at the actor whose turn it is.
2. After consuming the full action stream, the Rust engine's `winner()` matches the recorded `winner`.

The first catches DCGO-vs-Rust legality divergences (the most diagnostic failure mode). The second catches subtle resolution differences that don't surface until end-of-game.

Considered: per-step terminal-state diff (compare hand sizes, memory, field DP, etc., after every action). Rejected as default because (a) the JSONL doesn't carry that ground truth without significant log bloat, and (b) the engine's intermediate state representation may legitimately differ from DCGO's even when both are correct (animation timing, queue ordering). Per-step diff is a *debug verbose* mode the harness can enable when chasing a specific divergence.

### Mod intrusiveness: minimal call-site additions, recorder is a separate file

The mod adds **one new directory** `DCGO/Assets/Scripts/Script/Recording/` containing the recorder class, encoder, and (generated) `ActionSpace.cs`. It modifies existing files in two places only: `TurnStateMachine.cs` (inside `QueueMainPhaseAction` and the AI-driven decision branches) and `UserSelectionManager.cs` (inside the two `Set*ForPlayer` PunRPC bodies). Each modification is a single `GameRecorder.Instance?.Log(...)` call.

Considered: Unity event-bus subscription, partial class extensions, Harmony patching. Rejected for complexity. The DCGO patch is small enough (~10 lines of edits plus the new file tree) that maintaining it against upstream commits is straightforward.

### Three-phase rollout

```
   Phase 1 ── bot fuzzer
      Recorder mod (bot mode only — both decks visible)
      Codegen pipeline + ActionSpace.cs
      Rust replay harness (full-deck mode)
      Per-card parity report
      Ship value: faithfulness fuzzer for Rust engine

   Phase 2 ── opaque-deck engine
      Game::new_with_opaque_opponent + supply_reveal
      Rust replay harness extension (opaque mode)
      Tests against synthetic recordings
      Ship value: reusable engine capability

   Phase 3 ── PvP recording + BC emitter
      Recorder extension: log opponent reveals
      Replay harness wire-up to opaque mode
      BC dataset emitter (numpy shards)
      Ship value: human-game corpus + BC seed data
```

Each phase ships independently and is independently valuable. Phase 1 alone justifies the work even if 2–3 never happen.

## Risks / Trade-offs

- **[DCGO build setup is fragile]** Unity 2021.3.45f2 must be installed manually; assets are delivered out-of-band per the DCGO README; the upstream community Photon AppId may rotate. → Mitigation: pin DCGO submodule to a specific upstream commit known to build; document the asset-bundle URL in `docs/DCGO_BUILD.md`; the recorder writes to disk only, so a Photon disconnect mid-game just yields a truncated recording (the harness rejects it cleanly).
- **[Phase mapping bugs in the encoder]** A subtle bug where the encoder thinks DCGO is in phase X but it's actually phase Y will silently produce wrong action IDs. Single most likely source of "parity failures" that are actually recorder bugs, not engine bugs. → Mitigation: dedicated C# unit-test suite for the encoder covering a hand-built table of `(prompt_type, payload) → expected_action_id` for every prompt type. Recordings include a `phase` field for cross-check at replay time.
- **[DCGO's resolution ordering differs from Rust's effect queue]** Both engines may correctly resolve a set of simultaneous triggers but in different orders, leading to different optional-trigger surfaces and divergent recordings. → Mitigation: per-card parity reports surface this as a card-script issue, which it sort-of is; some divergences may require explicit timing-fix tasks in the Rust card scripts. Document tolerance policy: outcome-match is parity, intermediate-ordering-mismatch is investigation-worthy but not a hard failure.
- **[Card-ID format mismatch]** DCGO's internal card identifiers may not exactly match our `data/cards.json` IDs (suffix differences, alt-art variants). → Mitigation: ship a hand-maintained alias table for known discrepancies (`code/tools/dcgo-replay/card_id_aliases.json`); fuzz on Day 1 by listing every card DCGO uses vs. our pool and triaging.
- **[Action space evolves; old recordings become unreplayable]** Adding/removing action IDs breaks recordings made against the old space. → Mitigation: `recording_version` field in JSONL header, harness rejects mismatched versions with a clear error pointing at a regeneration step (re-record from DCGO, don't try to migrate the data).
- **[Recording-induced performance impact]** Unity client doing disk writes on a per-decision basis could lag the UI. → Mitigation: buffered writer flushing every N rows or on phase boundary; bench during dev. The volume is small (~hundreds of rows per game), so this is unlikely to matter.
- **[ToS / community policy]** DCGO is fan-maintained; modifying the client connecting to its Photon cloud may violate community norms even if it's benign. → Mitigation: Phase 1 (bot-vs-bot) does not touch Photon at all — the mod can be developed and validated entirely offline. Before turning on Phase 3 PvP recording, confirm with DCGO maintainers that local-write-only recording mods are acceptable. If not, Phase 3 reverts to "you + a friend in a private room" only.
- **[Opaque-deck mode increases engine surface area]** New constructor and reveal queue add code paths that need their own tests. → Mitigation: dedicated test module `tests/opaque_deck.rs`; reuse existing `add_card_to_hand`-style fixture patterns; opaque-mode replay against a known recording validates end-to-end.

## Migration Plan

This is additive. No rollout sequence affects existing engine, RL, or hosted-API users.

- **DCGO submodule**: pin to a specific upstream commit via `git submodule set-branch` or by tracking a sha in `.gitmodules`. Document the pinning in `docs/DCGO_BUILD.md`. Rebasing the patch onto a newer upstream is a manual operation when needed.
- **CI integration**: add a new workflow step `cargo run -p action-space-export | diff - DCGO/Assets/Scripts/Script/Recording/ActionSpace.cs` after the existing test suite. Initially non-blocking (warning only) until the codegen pipeline stabilizes, then promoted to required.
- **Phase 1 delivery**: lands behind no feature flag. The recorder mod is inert unless the DCGO submodule is checked out and built — the Rust engine and RL code paths are unaffected.
- **Phase 2 delivery**: opaque-deck mode lands disabled. Activated via the new constructor only; no existing call sites change.
- **Phase 3 delivery**: PvP recording defaults to off. Recordings live under `recordings/dcgo/` and are gitignored. BC emitter is opt-in via CLI.
- **Rollback**: revert the DCGO patch (single submodule pointer change); the Rust workspace members can stay in tree harmlessly. Opaque-deck mode rollback would mean removing the constructor and reveal queue — only required if a deep bug surfaces; otherwise leaving it in is safe (no caller, no impact).

## DCGO PvP Information Model (verified 2026-05-26)

Important fact verified during Group 8 implementation: **in PvP, each DCGO
client knows the opponent's full decklist composition, but not the
post-shuffle order**. This shape exactly matches our opaque-deck design.

### Where the data lives

Each player's full decklist is published to their Photon `CustomProperties`
at battle setup, under the key `ContinuousController.DeckDataPropertyKey`
(= the string `"BattleDeckData"`). The value is a deck-code string the
`DeckData(string)` constructor parses into a `List<CEntity_Base>`.

```
   ContinuousController.cs:1742 — SignUpBattleDeckData() (coroutine)
       PhotonNetwork.LocalPlayer.CustomProperties[
           "BattleDeckData"
       ] = ContinuousController.instance.BattleDeckData.GetThisDeckCode();
       PhotonNetwork.LocalPlayer.SetCustomProperties(hash);
       // Photon synchronizes CustomProperties across all clients in
       // the room. Each client reads the opponent's deck from
       // opponentPhotonPlayer.CustomProperties at game start.
```

At game start, both clients read the opponent's deck and shuffle their
local view independently:

```
   CardObjectController.cs:130-153 — DeckRecipie(Photon.Realtime.Player player)
       if (!GManager.instance.IsAI) {
           Hashtable hashtable = player.CustomProperties;
           if (hashtable.TryGetValue(
                   ContinuousController.DeckDataPropertyKey,
                   out object value)) {
               DeckData deckData = new DeckData((string)value);
               return RandomUtility.ShuffledDeckCards(deckData.DeckCards());
               //     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ local client shuffles
               //     its own view of the opponent's deck
           }
       }
```

### Implications for the recording schema

- The recorder MUST emit `opp_decklist_composition` in PvP `game_start`
  rows (see `recording.rs::GameStart`). The source is `Opponent.PhotonPlayer.CustomProperties["BattleDeckData"]` parsed via `new DeckData(...)`. This is the recorder-side
  work in task 7.3.
- The recorder MUST NOT emit `opp_deck_post_shuffle` for PvP — the local
  client's shuffle is divergent from the authoritative order (which is
  governed by Photon RPCs / MasterClient draws, not the local
  `RandomUtility` call). Setting this to `null` is correct.
- The replay harness consumes `opp_decklist_composition` (preferred) or
  falls back to deriving composition from the reveal stream. The
  preferred path is what Group 7 wires up.

### Implications for opaque-mode replay

The harness's `Game::new_with_opaque_opponent` constructor takes the
opponent's composition multiset, which matches DCGO's information model
exactly — the local client knows what cards the opponent has, just not
in what order. Reveals from the recording stream incrementally surface
the actual order as the game progresses.

This document is the single source of truth for "what does each client
know about the opponent's deck in DCGO PvP." Anyone picking up Group 7
should reference this section rather than re-investigating the DCGO
codebase.

### Lazy security reveal (resolved 2026-05-26)

**Status: implemented for the primary security-pop path.**

Initial Group 6 implementation consumed security reveals **eagerly** at
`setup_security_for_player`, requiring 5 `RevealKind::Security` reveals
in the queue at game-start time. This was wrong for real PvP recordings,
where security cards are revealed **lazily** only when flipped via
attack-triggered `SecurityCheck`.

The fix landed during the "tackle B" sweep:

```
   CardSource.is_opaque_placeholder: bool         (new field)
   CardSource::new_opaque_security_placeholder()   (constructor)
   OpaqueDeckState::reserve_placeholders(count)    (debit total without per-card)
   OpaqueDeckState::consume_per_card_only(id)      (debit per-card without total)
   Game::materialize_opaque_security_placeholder() (idempotent flip-time helper)

   In opaque mode:
     setup_security_for_player → pushes N placeholders; reserves count
                                  in the multiset (debits total_remaining
                                  but not per-card entries)
     combat::pop_and_start_security_check → before popping, if security[top]
                                  is a placeholder, materialize via reveal
                                  source (consume_per_card_only balances
                                  the multiset). Errors are logged but
                                  don't crash — engine continues with
                                  data_index=0 garbage, replay harness
                                  surfaces the divergence.
```

**Remaining residual (resolved 2026-05-26)**: previously the
materialization was only wired into the primary security-pop path. The
effect-driven security sweep landed in the same session — 11 additional
sites across `effect_context/mod.rs` and `game_actions.rs` are now
opaque-aware via the new convenience helper
`Game::ensure_security_materialized(pid, security_idx)`, which checks
`is_opaque_placeholder` and calls
`materialize_opaque_security_placeholder` if needed before the calling
effect reads or removes the security card.

Sites swept (each gets a one-line `ensure_security_materialized` before
the `security.remove(...)` or identity-dependent read):

- `effect_context::trash_top_security` — before WhenWouldBeTrashed lookup
- `effect_context::trash_bottom_security` — same
- `effect_context::trash_security_card` (by handle) — same
- `effect_context::add_to_hand_from_security` — before move to hand
- `effect_context::play_from_security_index` — before play
- `game_actions::add_to_hand_from_security` — same as effect_context's
- `game_actions::take_card_source_ref` (Security variant) — generic-zone take
- `game_actions::place_as_bottom_source_observed` (Security source)
- `game_actions::find_and_remove_card_anywhere` (security branch)
- `game_actions::place_in_security_observed` — redirect-to-trash branch
- `game_actions::place_in_security_observed` — fallthrough place branch

The convenience helper no-ops cleanly for non-opaque players,
already-materialized slots, and invalid pid/idx (defensive). 15 opaque
integration tests cover the helper directly, the lazy combat path, and
the construction-time placeholder semantics. Full opaque-mode security
flow is now end-to-end correct for both combat-driven and effect-driven
manipulation.

## Open Questions

- **DCGO card-ID format**: needs Day-1 audit. We have `data/cards.json` IDs (e.g., `BT15-104`); DCGO's `CardBaseEntity/[Set ID]/[Color]/[Type]` hierarchy may use the same strings or different ones. Compatibility list needs scanning.
- **DCGO redraw mechanics**: confirm mulligan is per-player and goes through observable RPCs (we expect yes from `SetRedraw` calls seen in `TurnStateMachine`); confirm DCGO does not implement the optional "vanilla post-mulligan ordering check" some online clients use.
- **Effect-resolution ordering tolerance**: when DCGO and Rust both correctly resolve simultaneous triggers but in different order, what's the parity verdict? Suggested default: "soft warning, not a failure" with the divergence card listed in the parity report. Open for refinement once we see actual data.
- **Opaque-deck reveal exhaustion**: what happens if the recording's reveal queue runs out before the game ends (e.g., truncated recording, or engine wants to draw more than the recording witnessed)? Suggested behavior: engine errors with a clear "reveal queue exhausted at step N" message; the harness reports it as a recording-corruption failure, not a parity failure.
- **BC emitter observation profile**: which tensor profile does the emitter snapshot — `StandardCompactV1` (1375), `StandardLiteV2` (8320, Rust default), or `StandardFullV2`? Probably the same profile used by `pilot_training` for BC consumption, but pin it explicitly to avoid downstream silent shape mismatches. To be locked in Phase 3 task list.
- **Run-time placement of the mod**: a Unity `MonoBehaviour` requires a hosting GameObject. Best mounted on the persistent `GManager` singleton, or its own DontDestroyOnLoad object created in `RuntimeInitializeOnLoadMethod`. Decide during Phase 1 implementation.
