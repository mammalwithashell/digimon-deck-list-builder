# Phase 2 Track C — OPT-Slot Enforcement on Triggered Effects + Attack-Cycle Reset

You are closing two coupled OPT-slot (Once Per Turn) substrate edges in `code/digimon-engine/src/effect_queue.rs`. Both touch the same activation-count keying machinery, so they ship as one PR — but the work is two distinct fixes you should sequence.

This track is independent of Track A (DSL eval-arm sweep) and Track B (`activation_cost(...)` builder). The only file-level overlap is `effect_queue.rs` if Track B's cost-failure short-circuit lands first — those two changes are at different call sites in the same function (`run_queued_effect_inner`), so merge conflicts should be mechanical.

## Why this matters

The substrate-reality-check audit (2026-05-14) flagged G-OPT-TRIGGERED (139 refs at the time) and G-INHERITED-DISPATCH (107 refs) as the two highest-volume substrate edges. PRs since then have cleared most of those refs but the residue — **26 G-OPT-TRIGGERED + 27 G-INHERITED-DISPATCH refs in ignored tests** — still gates ~50 tests, almost entirely in Medusamon and DNA Omnimon pilot archetypes.

The OPT-trigger residue is the single largest test-count payoff available. Medusamon alone has 22 pending G-OPT-TRIGGERED refs across its ignore set. The clauses involved are common multi-timing observers: "[On Play] [When Digivolving] [Once Per Turn] …", "[All Turns] [Once Per Turn] When your Digimon are deleted …", and similar shapes.

G-OPT-RESET-VIA-ATTACK-CYCLE (BT16-040 Wormmon, 3 BLOCKED refs) is the same family — "[When Attacking] [Once Per Turn]" carriers whose OPT slot doesn't reliably reset after a full player-end → opponent-end → player-attacks-again cycle.

## What's actually broken

Read these in order. The fix isn't "add OPT enforcement" — that already exists. The fix is *making it work*:

1. `code/digimon-engine/src/effect_queue.rs:1733-1785` (`run_queued_effect_inner`). The `max_per_turn` gate at line 1772 reads `source_permanent_activation_count(...)` and compares to `effect.max_per_turn`. The gate exists. Yet the BLOCKED message in the ignored tests reads `"BLOCKED: G-OPT-TRIGGERED — max_per_turn is not enforced in run_queued_effect_inner"`. Both can be true: the lookup may be missing data, or the activation-count is never being incremented after the body runs, or the slot key (`(card_handle, effect_slot)`) doesn't match the inherited-source key.
2. Specifically search for `record_activation` / `activation_count` consumers — confirm `run_queued_effect_inner` actually calls `record_activation` after a successful body run (i.e., the post-body bookkeeping that ought to mirror the gate's pre-body read).
3. `code/digimon-engine/src/effect_queue.rs:1246` (`enqueue_from_permanent`). Per the BLOCKED test reasons, this function "only scans top card + linked_cards + Training" — it never walks the digivolution stack for inherited triggered effects. That's the G-INHERITED-DISPATCH residue. Note: this is Track D's territory, not yours — but the OPT-slot key for inherited triggers needs to be stable across the dispatch boundary, so confirm the slot-key shape is well-defined before Track D wires the digivolution-stack walk.
4. `code/digimon-engine/src/permanent.rs` — `record_activation` / `activation_count` methods. Note the slot identity: is it keyed by `(card_handle, effect_slot)`, by `(permanent_handle, effect_slot)`, or by source-card-identity? G-OPT-RESET-VIA-ATTACK-CYCLE's BLOCKED comment in `qa/archetype-qa/engine-gaps.md` § "OPT Reset via Attack Cycle" notes: *"OPT key may persist across turn boundaries when carrier permanent's source identity differs from trigger source."* So the bug is likely that the slot key uses one identity in the read path and a different one in the reset path.

## Tags to close

| Tag | Refs | Where it's broken |
|---|---:|---|
| **G-OPT-TRIGGERED** | ~26 (BLOCKED + pending across tests) | `run_queued_effect_inner` reads activation count but post-body increment is missing or mis-keyed |
| **G-OPT-RESET-VIA-ATTACK-CYCLE** | 3 BLOCKED | `Game::end_turn` (or `rotate_turn_player`) doesn't reset OPT slot for inherited [When Attacking] carriers when the carrier's source identity differs from trigger source |

Expected unblock: **~40 tests stop being `#[ignore]`'d** (some may surface secondary failures that re-tag with different gaps — that's fine, document in PR).

## Read these first (in order)

1. `CLAUDE.md` — Working Rules §17 (no-approximations: OPT lockout must be visible to RL agent — but the lockout is enforced at queue time, not via the action mask, so this is about runtime correctness not action surface).
2. `docs/RUST_ENGINE_GAPS.md` — confirm there's no separate "OPT enforcement" entry. The Phase 2 spec lists G-OPT-TRIGGERED as a substrate edge (item 13).
3. `qa/archetype-qa/engine-gaps.md` § "OPT Reset via Attack Cycle [G-OPT-RESET-VIA-ATTACK-CYCLE]" — confirms the suspected root cause about slot-key carrier-vs-source identity.
4. `code/digimon-engine/src/effect_queue.rs:1733-1785` (`run_queued_effect_inner`) — the gate. Read the surrounding 200 lines for context.
5. `code/digimon-engine/src/effect_queue.rs:1246` (`enqueue_from_permanent`) — the dispatch. Just confirm slot-key shape; Track D owns this function's body.
6. `code/digimon-engine/src/permanent.rs::record_activation` / `activation_count` — the storage. **This is the most likely site of the bug.**
7. `code/digimon-engine/src/game.rs` — search for `end_turn` and confirm where per-permanent activation counts get reset.
8. Pick three ignored tests as your failing-test corpus:
   - One Medusamon "[On Play] [When Digivolving] [Once Per Turn]" card (e.g., `bt24_018.rs` or similar — grep `code/digimon-engine/tests/cards_behavioral/` for `G-OPT-TRIGGERED`).
   - `code/digimon-engine/tests/cards_behavioral/bt16/bt16_040.rs::bt16_040_opt_resets_after_turn_cycle` — the canonical reset test.
   - One DNA Omnimon shape (grep for tag).
9. DCGO reference for tiebreaker only: `DCGO/Assets/Scripts/CardEffect/` — search for `OncePerTurn` and `IsOPTReset`. DCGO resets OPT at the start of each player's *turn* keyed by carrier-instance, not by source-card.

## Work to be done

### Phase 1 — Diagnose

Before writing any fix, write a paragraph in the PR description that pins down exactly:

- What is the slot key used by `record_activation`?
- What is the slot key used by `activation_count`?
- Is `record_activation` called from `run_queued_effect_inner` after a successful body invocation? Where? If not, where else does it get called, and is that call site reachable for queued triggered effects?
- When does the activation count get reset? At `end_turn`? At `begin_turn`? At `rotate_turn_player`?
- Does the reset key match the read/write key?

This diagnosis IS the design. Don't skip it.

### Phase 2 — Fix G-OPT-TRIGGERED

Whatever the diagnosis surfaces, the fix is most likely one of:

- Adding a `record_activation(...)` call inside `run_queued_effect_inner` after the body runs successfully (post-body, not post-condition-check — a failed `condition` or `optional` decline shouldn't consume the slot; a successful body should).
- Aligning the slot key between the gate's read (line 1774) and the post-body write so they reference the same `(carrier_card_handle, effect_slot)` pair.
- If Track B has landed: ensure the activation-cost-failure path ALSO consumes the slot (per the no-retry-after-cost-fail rule from Track B's plan).

### Phase 3 — Fix G-OPT-RESET-VIA-ATTACK-CYCLE

Once the slot key is unified (Phase 2), the reset becomes straightforward:

- At the start of each player's turn (or the end of the previous, depending on existing convention), iterate every battle-area permanent and every inherited source card, reset activation counts where `effect.max_per_turn > 0`. Mirror DCGO's per-carrier-instance keying.
- Specifically test the BT16-040 sequence: player 0 attacks (uses [When Attacking] OPT), turn passes to player 1 and back, player 0 attacks again — the OPT must fire a SECOND time.

### Phase 4 — Un-ignore tests

For every `#[ignore = "BLOCKED: G-OPT-TRIGGERED..."]` or `"BLOCKED: G-OPT-RESET-VIA-ATTACK-CYCLE..."` annotation, remove it and confirm the test passes. Any test that fails post-unignore for a *different* reason should be left ignored with the new tag and called out in the PR.

Note: many tests have combined tags like `"BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED"`. Those tests will still fail after this track lands — they need Track D too. Leave them `#[ignore]`'d but update the reason to drop the G-OPT-TRIGGERED half: `"BLOCKED: G-INHERITED-DISPATCH"`.

## Acceptance gates

- The three failing tests you pinned in step 8 all pass without test-body edits.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_040_opt_resets_after_turn_cycle` passes.
- Net `#[ignore]` count across `code/digimon-engine/tests/` drops by at least 25.
- No regression in `effect_queue` / `timing_dispatch` test suites.
- The new PR description includes the diagnosis paragraph (Phase 1) — Track D will need it.

## Constraints

- No-approximations: OPT lockout failure must not surface a hidden retry path. A locked-out trigger drops silently — it does NOT prompt the player to "wait, try again".
- Do NOT change `ACTION_SPACE_SIZE`, tensor profiles, PyO3 exports.
- Do NOT modify `enqueue_from_permanent`'s scan scope — that's Track D's surface. You may add helper functions that Track D consumes, but the inherited-stack walk is not your work.
- Do NOT collapse the OPT slot key for top-card vs. inherited-source effects without explicit reason — these are conceptually distinct activation slots, and a unified key may cause cross-effect interference.
- Source priority: printed text + Rules Manual say OPT resets at the start of the *controller's* turn. DCGO confirms per-carrier-instance keying. Do NOT use first-firing-player as the reset trigger.

## Verification

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt16_040
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_queue
cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral
cargo test --manifest-path code/digimon-engine/Cargo.toml
git grep -c '#\[ignore' code/digimon-engine/tests | awk -F: '{s+=$2} END {print s}'
```

Pre-PR baseline: 596 ignored. Post-PR target: ≤ 570.

## Tracker discipline

- `qa/archetype-qa/engine-gaps.md` — close G-OPT-RESET-VIA-ATTACK-CYCLE entry. Move to `qa/resolved-gaps.md` with "Phase 2 Track C closure — 2026-05-XX".
- `qa/dsl-vocab-gaps.md` — search for G-OPT-TRIGGERED references; close any DSL-side entries.
- `docs/RUST_ENGINE_GAPS.md` — no canonical entry exists; add a "Phase 2 Track C — OPT enforcement closure" line to the closures section at the top.
- `qa/qa-reports/validated_cards_dsl.json` — BT16-040 can advance from PARTIAL/BLOCKED if all printed text is now covered. Audit a few Medusamon cards too.

## Out of scope

- Track D (inherited-stack walk in `enqueue_from_permanent`).
- New OPT-related selection kinds or action surface.
- Card YAML re-authoring.
- BeforePayCost / `.pay_cost_fn` cost-slot semantics — unrelated.

## Discovery rider

If diagnosis (Phase 1) reveals that the slot-key issue is more architectural than expected (e.g., requires a new `ActivationSlotId` type), STOP and write a design note. This is the kind of finding that should defer Phase 3/4 of this plan until reviewed.
