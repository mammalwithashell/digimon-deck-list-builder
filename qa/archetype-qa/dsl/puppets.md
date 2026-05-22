# Archetype DSL Implementation: Puppets
Date: 2026-05-21
Total cards in pool: 61
Processed this run: 18 (all prior-PARTIAL cards; 43 already IMPLEMENTED, skipped)
Pipeline: batch-implement-cards-rust-dsl (completion mode — extend PARTIAL YAML now that blocking gaps are closed)

## Summary
- IMPLEMENTED: 18
- PARTIAL: 0
- AUDITED-OK: 0
- BLOCKED: 0
- SKIPPED (prior IMPLEMENTED verdict): 43

All 18 prior-PARTIAL cards re-attempted in completion mode and all 18 are now
IMPLEMENTED — every blocking engine/DSL gap closed, no approximations shipped.
P-229's Mirai-played event-gated Delay (the last open gap) was closed in a
follow-up pass. Every reviewer batch APPROVED; 3 review NEEDS-FIX items
(EX7-024 negative test, BT20-084 security-face, EX11-060 suspend-cost gate)
applied.

Post-run gap closures:
- `G-DSL-MODIFIER-PENDING-SKIPS` closed (engine auto-computes `pending_skips`
  at modifier-install time) → EX4-074 advanced PARTIAL → IMPLEMENTED.
- `G-ZONE-SELECTED-TRASH-TO-DECK-TOP` closed (new `return_trash_list_to_deck_top`
  DSL verb) → LM-029 advanced PARTIAL → IMPLEMENTED.
- `PUPPETS-G004` closed (hybrid — `lower_delay.rs` now maps `on_ally_played` →
  `DelayTrigger::OnEvent(OnAllyPlayed)`, and `effect_queue.rs` fans `EnteredField`
  dispatches out to `enqueue_event_gated_delayed_options`) → P-229 advanced
  PARTIAL → IMPLEMENTED.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| EX7-024 | Shoemon | COMPLETE | IMPLEMENTED | APPROVED (after fix) | 9 | Source-scoped digivolve-into-Puppet cost reduction + inherited opp-security -3000 DP aura; both deferred gaps closed (Track H/I) |
| EX7-025 | ShoeShoemon | COMPLETE | IMPLEMENTED | APPROVED | 7 | Inherited opp-security -3000 DP aura added (PUPPETS-G008 closed Track I) |
| EX7-027 | Chaperomon | COMPLETE | IMPLEMENTED | APPROVED | 8 | Inherited [All Turns][OPT] leave-prevention replacement confirmed faithful; tests extended 4→8 |
| ST19-08 | ShoeShoemon | COMPLETE | IMPLEMENTED | APPROVED | 9 | Inherited opp-security aura clause was missing from YAML; added + test rewritten (old test was false-positive trap) |
| BT20-084 | Sistermon Ciel (Awakened) | COMPLETE | IMPLEMENTED | APPROVED (after fix) | 12 | Trash observer + end-of-turn top-stacked→security confirmed; review fixed security placement face up→down |
| BT23-077 | Sistermon Ciel | COMPLETE | IMPLEMENTED | APPROVED | 10 | Self-suspend De-Digivolve 1 observer confirmed faithful (PUPPETS-G029 closed); tests extended |
| EX11-021 | Kokeshimon | COMPLETE | IMPLEMENTED | APPROVED | 14 | Inherited attack-end clause rewritten — fixed wrong-delete-target + free-end-attack defects |
| EX11-060 | Arisa Kinosaki | COMPLETE | IMPLEMENTED | APPROVED (after fix) | 12 | Deletion observer + Overclock rider confirmed; review fixed suspend-cost gate for multi-copy correctness |
| BT22-040 | Cendrillmon | COMPLETE | IMPLEMENTED | APPROVED | 10 | [All Turns][OPT] refire-When-Digivolving-on-other-deletion confirmed faithful; tests extended incl. OPT reset |
| BT22-042 | Nyabootmon | COMPLETE | IMPLEMENTED | APPROVED | 13 | Conditional Chaperomon cost-6 alt-path added (G-ALT-PATH-CONDITION closed); When-Digivolving compound + refire confirmed |
| EX4-074 | ShineGreymon: Ruin Mode | COMPLETE | IMPLEMENTED | APPROVED | 6 | End-of-Attack chain + −5000 DP clause; gap G-DSL-MODIFIER-PENDING-SKIPS closed post-run |
| EX6-011 | RagnaLoardmon | COMPLETE | IMPLEMENTED | APPROVED | 8 | [Hand][Counter] Blast DNA Digivolve activation confirmed faithful (PUPPETS-G032 closed Track D) |
| BT22-098 | Unique Emblem: Fable Waltz | COMPLETE | IMPLEMENTED | APPROVED | 14 | Hand-or-trash union play (Main+Security) + Delay [Main] + Arisa-suspend Delay all closed (PUPPETS-G014/G009/G033) |
| EX7-074 | Vortex Resonance | COMPLETE | IMPLEMENTED | APPROVED | 28 | All 14 stale-ignored tests un-ignored (harness matured); fixed card color green/blue→green/yellow |
| LM-029 | Yellow Scramble | COMPLETE | IMPLEMENTED | APPROVED | 16 | Main + Security + [Start of Your Turn] Delay; gap G-ZONE-SELECTED-TRASH-TO-DECK-TOP closed post-run |
| P-156 | Future Potential! | COMPLETE | IMPLEMENTED | APPROVED | 14 | [Security] optional Tamer play before mandatory add-to-hand tail (PUPPETS-G017 closed) |
| EX9-024 | Hanimon | COMPLETE | IMPLEMENTED | APPROVED | 13 | Audit fixed 2 defects in inherited attack-end clause (wrong delete target + free attack-end); On Play recursion confirmed |
| P-229 | Unique Emblem: Narrative Ronde | COMPLETE | IMPLEMENTED | APPROVED | 13 | Main/Security reveal-search + option battle-area placement + Mirai-played event-gated Delay (level≤6 LIBERATOR digivolve cost-3) all complete; hybrid gap PUPPETS-G004 closed post-run |

## Engine-Gap Blocked Cards
(none — all closed)

### P-229 Unique Emblem: Narrative Ronde — RESOLVED 2026-05-21 (engine half of PUPPETS-G004)
- `enqueue_triggered` in `effect_queue.rs` now fans `TriggerSource::EnteredField`
  dispatches out to `enqueue_event_gated_delayed_options` (previously only
  `EventObserved` / `AttackTargetChanged` reached it). The candidate scan only
  matches Options whose `OnEvent(event_timing)` equals the dispatch timing, so
  dispatching for both the `OnEnterFieldAnyone` and `OnAllyPlayed` play
  broadcasts is harmless. A placed Delay-Option keyed to `on_ally_played` now
  fires when a matching card is played after the placing turn.

## DSL-Vocab-Gap Blocked Cards
### EX4-074 ShineGreymon: Ruin Mode — RESOLVED 2026-05-21
- `G-DSL-MODIFIER-PENDING-SKIPS` closed: `EffectContext::add_modifier` now auto-computes `ModifierEntry.pending_skips` via `modifiers::pending_skips_for_install`, so `expiry: end_of_opponents_next_turn` is faithful for mid-opponent-turn installs. EX4-074's −5000 DP clause is implemented; card is IMPLEMENTED.

### LM-029 Yellow Scramble — RESOLVED 2026-05-21
- `G-ZONE-SELECTED-TRASH-TO-DECK-TOP` closed: added the `return_trash_list_to_deck_top` DSL verb + `EffectContext::return_trash_cards_to_deck_top` engine method (mirror of the deck-bottom verb, appending to the deck end). LM-029's [Start of Your Turn] Delay clause is implemented; card is IMPLEMENTED. (LM-027 / LM-030 Scramble Delay clauses share this gap and are now unblockable — not implemented here.)

### P-229 Unique Emblem: Narrative Ronde — RESOLVED 2026-05-21 (DSL half of PUPPETS-G004)
- `code/digimon-engine/src/dsl_cards/lower_delay.rs` now maps
  `CompiledTiming::OnAllyPlayed` → `DelayTrigger::OnEvent(EffectTiming::OnAllyPlayed)`
  alongside the existing `on_suspend` / `on_unsuspend` arm. P-229's
  `kind: delay` clause (`trigger: on_ally_played`, reduced-cost level≤6
  LIBERATOR digivolve) lowers faithfully — the digivolve / level-filter /
  cost-reduce primitives already existed (proven by BT22-098). The Option
  parks indefinitely (`OnEvent(_)` → `compute_delay_trash_turn` = `u16::MAX`),
  so the [Main]/[Security] `place_self`/auto-place tail is no longer an
  approximation. P-229 is IMPLEMENTED.

## New Patterns Discovered
- `EVENT-GATED-DELAY` (`on_ally_played`): an Option placed as a `kind: delay`
  clause keyed to a card-play event, firing the body when a name-matched card
  is played after the placing turn. First card: P-229 (sibling of BT22-098's
  `on_suspend` event-gated Delay).
