## Context

Three engine gaps were surfaced (and proven) by the judge-quiz discovery wave. This change fixes them. Code sites and evidence are pinned below so implementation starts from verified ground, not re-discovery.

| Gap | Code site | Evidence |
|-----|-----------|----------|
| `G-NO-GENERAL-ZERO-DP-RULES-CHECK` | `game_actions.rs::run_rule_check_after_arts` (≈1701), sole call site ≈1607; `effect_context/mod.rs::add_modifier` (4886) has no delete; `effect_queue.rs::drain_effect_queue` (697) has no sweep | `tests/judge_quiz/b_deferred_rules_check.rs::zero_dp_probe_reduced_digimon_deleted_after_effect_resolves` fails (−1000-DP Digimon survives) |
| `G-RETURN-TRASH-DIGI-EGG-ROUTING` | `effect_context/mod.rs::return_trash_cards_to_deck_bottom` (5538) — `deck.insert(0, card)` for every card, no `CardKind::DigiEgg` branch; sibling `_to_deck_top` (5570) | `tests/judge_quiz/f_token_and_memory.rs::q22_digi_egg_returned_to_deck_bottom_routes_to_digitama_deck` fails (digitama empty) |
| `G-ON-TRASH-OBSERVER-SYNCHRONOUS` | `game_actions.rs::fire_digivolution_card_trashed` (3283) — `enqueue_triggered(...)` then immediate `drain_effect_queue()` (3308), intentional per inline comment (EX10-036) | `tests/judge_quiz/d_activation_site.rs::cluster_d_on_trash_observer_fires_synchronously_not_deferred` characterizes the synchronous fire |

Existing engine facts to respect: deletion runs through the batched flow (`Game::delete_permanents_batch`) with `OnDeletion` handlers firing post-trash (CLAUDE.md §25); the immunity machinery (`permanent_is_unaffected_by_effect`, `EffectControllerFilter`) and the `<Partition>` cause-filter are already correct.

## Goals / Non-Goals

**Goals**
- A general state-based ≤0-DP rules-check, invoked at the right resolution boundaries (NOT mid-effect), that reproduces the judge timing for Q6/Q8/Q13/Q14/Q24.
- Digi-Egg cards route to the digitama deck on every "return to deck" movement, while still counting toward dependent costs (Q22).
- Inherited on-trash triggered effects defer and re-check remain-in-trash, WITHOUT breaking EX10-036's synchronous intra-effect observer dependency (Q21/Q23).
- Each gap's judge-quiz test flips from `#[ignore]` to a real pass; no regression in `combat`, `option_flow`, `deletion_batching`, or the EX10-036 behavioral test.

**Non-Goals**
- The AD1-025 `[Assembly]` data-ingest fix (Q5) and `G-DSL-GRANT-TRIGGERED-EFFECT-TO-OPPONENT` (Q2/Q16/Q17) — separate changes (see proposal Non-Goals).
- Authoring the BLOCKED-CARD scenarios. Gap 1's test flips happen card-by-card as those cards land; the engine fix lands now and is proven by synthetic probes + Q22.
- A broader state-based-action engine (e.g. memory ≤ −10 loss, security-empty loss) — this change adds only the ≤0-DP sweep; other state-based actions stay where they are.

## Decisions

### D1 — One general state-based rules-check, invoked at top-level resolution boundaries only
Promote `run_rule_check_after_arts` to `Game::run_state_based_rules_check` (delete every battle-area Digimon with `effective_dp ≤ 0`, via the batched deletion flow). Invoke it at: (a) the end of `drain_effect_queue` once the queue is fully empty (the "effect/rule-action finished" boundary), (b) after combat DP changes resolve, and (c) at phase transitions. Crucially it runs only when the top-level resolution has finished, never between sub-steps of one effect — that is exactly the judge rule ("rule checks don't happen until an ongoing effect or rule action finishes") and gives Q6 (Pillomon at 0 DP not deleted until Flame Hellscythe resolves) and Q13/Q14 (ShoeShoemon survives until Nyabootmon's `[When Digivolving]` fully resolves) for free.

### D2 — Re-check to a fixpoint
After deleting ≤0-DP Digimon, their `OnDeletion` handlers (and expiring auras) can change other Digimon's DP, so the sweep loops until no battle-area Digimon is at ≤0 DP (bounded by battle-area size). DCGO runs state-based actions repeatedly until none apply. Guard against re-entrancy with the existing batched-deletion machinery.

### D3 — Q24 ordering falls out of D1, but verify the trigger-vs-check interleave
Q24: Tentomon is suspended → gets −4000 from Rapidmon (X Antibody)'s `[All Turns]`, and Kokomon's inherited `[Your Turn]` "+2000 when a Digimon is suspended" should NOT save it. The judge: the rules-check deletes Tentomon BEFORE Kokomon's trigger resolves. With D1, the `[All Turns]` DP change resolves, the queue drains, the rules-check runs and deletes Tentomon — and Kokomon's `[Your Turn]` trigger (a separate queued effect) finds no valid subject. Confirm the rules-check fires at the boundary between the DP-change resolution and the next queued trigger, not after all triggers. This is the one timing subtlety in D1 — pin it with a dedicated test when BT23-101/BT23-037/EX6-004/BT16-101 are authored; until then a synthetic analog.

### D4 — Digi-Egg routing via a single zone-routing helper
Add a private helper `move_card_to_deck(player, card, position)` that routes `CardKind::DigiEgg` to `digitama_deck` (bottom = index 0, top = push) and everything else to `deck`. Re-point `return_trash_cards_to_deck_bottom`, `return_trash_cards_to_deck_top`, and any other trash→deck / bounce→deck movers through it. The returned `moved` Vec is unchanged, so Medusamon's "return 2" cost stays satisfied (Q22's surface answer) AND the egg lands in the digitama deck (Q22's actual rule).

### D5 — Gap 3: separate "intra-effect observer" from "deferred inherited trigger" (calibration spike first)
The synchronous drain in `fire_digivolution_card_trashed` exists for EX10-036, whose SECONDARY CLAUSE (same resolving effect) must see just-trashed cards. Tumblemon's gain-memory is different: it is the trashed card's OWN inherited triggered effect, which per the rules goes to the pending queue and resolves after the current effect, re-checking that the card remains in trash. Decision: gate the immediate drain so that only same-effect observer consumption fires synchronously, while inherited triggered effects on the trashed card are enqueued and drained at the D1 resolution boundary with a remain-in-trash activation re-check. Because EX10-036 is the load-bearing constraint, **task block 4 starts with a calibration spike** that pins EX10-036's exact dependency (read the card + its behavioral test) before any dispatch change; if separating cleanly proves larger than the other two fixes, gap 3 may be split into its own follow-up change and Q23/Q21 stay BLOCKED on it.

### D6 — Land in priority order, each independently shippable
Order: D4 (Digi-Egg routing — smallest, isolated) → D1/D2/D3 (≤0-DP rules-check — highest impact, hot-path) → D5 (on-trash deferral — riskiest, spike-gated). Each is a self-contained task block with its own tests; a stall on D5 does not block D1/D4.

## Risks / Open Questions

- **D1 hot-path cost & double-deletion.** A sweep after every `drain_effect_queue` and combat step adds work to the hottest paths. Must not double-delete battle losers (combat already deletes them) — invoke the sweep AFTER combat's own deletions, and make it idempotent (a handle already gone is a no-op). Regression gate: full `combat` + `option_flow` suites + RL smoke.
- **D1 vs batched deletion / OnDeletion.** The sweep must route through `delete_permanents_batch` so OnDeletion handlers fire correctly (CLAUDE.md §25) and not reintroduce retired side-channels.
- **D5 EX10-036 dependency (open).** Does deferring inherited on-trash triggers break EX10-036's secondary-clause pickup? The spike answers this before any change. If the two needs can't be cleanly separated, D5 splits out.
- **D3 exact interleave (open).** Whether the rules-check fires between each queued trigger or only after the queue fully drains affects Q24. D1's "after drain" default may need refinement to "after each top-level effect in the queue." Pin with a synthetic test.
- **Other state-based actions.** This change adds only ≤0-DP. If the same invocation sites should also enforce other state-based actions, that is deliberately deferred (Non-Goal) to keep the hot-path change minimal and reviewable.
