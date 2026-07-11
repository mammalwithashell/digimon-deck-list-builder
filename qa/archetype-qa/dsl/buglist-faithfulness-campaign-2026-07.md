# Bug-List Faithfulness Campaign — 2026-07-09/10

Driven by the user's bug spreadsheet (patch 0.3.4/0.4.0 bug list) plus the two
decklists from the logged games (50 unique cards). Two phases:

- **Phase A — engine/systemic bugs:** all 12 sheet issues fixed (or verified
  already-fixed and regression-pinned), plus 2 follow-ups (phantom alt-path
  family, modifier re-key at security-placement sites). Commit `0e7af2323`.
- **Phase B — per-card faithfulness audit:** all 50 deck cards audited
  clause-by-clause against the official Bandai DB bundle + card image + DCGO,
  in 7 batches with a per-batch central test gate. Commits `16aeff127` (batches
  1–3) + the campaign-final commit (batches 4–7 + trackers).

## Sheet-bug resolutions (Phase A)

| # | Reported bug | Resolution |
|---|---|---|
| 1 | Rule to give types not working (EX8-019, EX7-016, EX7-023) | FIXED — 91 cards pool-wide missing `(Rule) Trait` grants reconciled from the official DB; reconcile_traits.py widened; permanent guards (`tests/dsl/official_rule_grants.rs`) |
| 2 | Delay trigger mandatory instead of optional (P-228) | FIXED — OnEvent `<Delay>` lowering now optional + outer prompt (P-228/P-229/BT22-098/BT24-089/EX10-069/BT23-096); turn-scheduled family logged as G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN |
| 3 | EX8-028 when-attacking breaks game | ALREADY-FIXED (0.4.0) + NEW latent mid-attack fizzle fixed (`shift_pending_attack_after_battle_area_remove` at all removal sites) + no-wedge regression test |
| 4 | Surrender can show a victory screen | FIXED — Tauri `rust_surrender` read frontend 1/2 IDs as Rust 0/1 (every desktop surrender crowned the surrenderer); now translates, routes `Game::concede`, drains events |
| 5 | Digivolution-card count displayed wrong | FIXED — `sourceCount` excluded top card on both DTO wires; badge shows source count |
| 6 | Timing effects trigger for a different Digimon (P-215) | FIXED — self-scope gate on `on_move`; 8 cards mis-authored (P-215, ST24-04, ST23-06, BT25-008, EX11-008, BT24-034, BT25-063, BT25-078); vocab gap logged |
| 7 | Turn resets / second breeding phase + second start-of-main | FIXED — `enter_main_phase` now parks behind pending selections (phantom-Breeding restore bug; EX11-008 Eliza repro); "breeding not working" was the same corruption |
| 8 | Digivolving to play a tamer free made the digivolution free (BT21-017) | FIXED — phantom cost-0 alt_path in BT21-015/017 YAML (mis-authored circle); family swept (BT25-057, ST23-09, EX10-020, BT20-056, BT23-054/058, BT25-025, P-123); permanent guard `alt_path_printed_cost_guard` |
| 9 | Special digivolution not working for names (BT24-018, EX8-019, EX11-014) | FIXED — breeding area now consults the alt-path registry (incl. rule-17 cost choice); BT24-018 condition re-authored; BT20-076 name typo |
| 10 | Breeding area not working (Eliza/Dimetro) | Same root cause as #7 — breeding digivolve machinery itself verified correct end-to-end |
| 11 | Blanket effects not applying to newly played Digimon (ST1-13) | FIXED — one-shot grant → live continuous mass modifier; 4 more suspect cards flagged (BT13-112, BT20-037, BT22-052, BT9-103) |
| 12 | Gaining extra security from inherited effects (ST1-07) | TWO causes: inherited Sec+1 double-count already fixed (cb80d46ad, pinned) AND stale materialized grants after source-removal fixed mechanism-wide this campaign |
| 13 | Can't use Options without targets (ST1-15) | FIXED — option playability no longer condition-gated at the mask (rules 1-3-11-1/15-1-5); 7 option YAMLs affected |
| 14 | Color requirements based on primary color (BT25-052, BT25-056) | ALREADY-FIXED (2026-07-02 official-DB evo-cost reconciliation); regression-pinned (`tests/digivolve_color_requirements.rs`); ST1-01 egg colors verified correct |
| 15 | Piercing not working | FIXED — 3 defects: slot-shift misread of the deleted defender, stale aura keyword state, bogus 0-security game-win; §16-6-4 ordering |
| 16 | Sec +1 lasting after inherited source trashed | FIXED — materialized inherited grants now re-tick on every source-removal path (trash/return/Partition) |
| 17 | Clicking the log moves the UI | FIXED — `scrollIntoView` scrolled the `overflow-hidden` board; scroll containment |
| 18 | Bot chose to do nothing, forced surrender | NOT-REPRODUCIBLE — no-progress tripwire added to the desktop agent loop (silent stalls now fail loudly with diagnostics) |

Plus engine bug found BY the audit itself: **[Once Per Turn] counters only
reset at the owner's turn start** — `[All Turns][OPT]` effects stayed locked
through the opponent's turn. Fixed (`Player::reset_effect_activations`, both
players cleared every turn).

## Per-card verdicts (Phase B) — all 50 AUDITED-OK after fixes

See `qa/qa-reports/validated_cards_dsl.json` (report
`buglist-faithfulness-campaign-2026-07`) for per-card notes. Summary:

- **Clean (no YAML change):** BT21-043, BT21-047, BT21-059, BT21-070, BT21-084,
  BT23-079, BT24-087, P-241, P-228, EX7-016, EX7-021, EX7-023, EX8-066,
  EX11-002, EX11-057 (15)
- **Drift fixed (YAML):** the other 35 — dominated by four systemic families:
  1. **Missing printed digivolve circles** (esp. off-primary halves of split
     circles; the lossy digimoncard.io ingest drops them): BT21-009, BT21-018,
     BT21-023, BT21-073, BT24-067, BT25-007, BT25-036, BT25-045, BT25-052,
     BT25-056, BT25-060, BT25-070, BT25-072, AD1-005, BT17-077, EX8-019,
     EX8-022, EX8-023, EX8-028, EX11-014, EX11-015, EX11-016, EX11-017
  2. **Missing "Link DP +N" linked auras** (BT25 set-wide): BT25-007, BT25-036,
     BT25-045, BT25-052, BT25-056, BT25-061, BT25-070, BT25-072
  3. **Dead grade gates** (`"Super App"`/`"Standard App"` → `"Sup."`/`"Stnd."`,
     phantom level gates): BT25-036, BT25-052, BT25-056, BT25-070, BT25-072,
     BT21-071
  4. **Wrong identity** (colors/attribute/form/kind/trait folds): BT21-005,
     BT21-101 (White/Red), BT25-004 (digi_egg), BT25-007, BT25-045, BT25-056
     (blue→yellow!), BT25-060 (+white), BT25-061 (purple→BLACK), BT25-070
     (+black), AD1-005 (+white), BT17-077 (+GREEN, tri-color), EX7-020, P-215
  Plus one-offs: BT21-097 Delay re-timed [Main]→[End of Your Turn]; BT25-060
  ungated `on_unsuspend`; ST22-12 + BT25-056 deck-bounce carried sources;
  P-217 over-broad link observer ([Creation]); stale App-Fusion BLOCKED notes
  (the `app_fusion` primitive shipped long ago).

## Gates

Every batch ended with a central `cards_behavioral` gate; failures were
triaged: 1 real engine bug (OPT turn reset), 3 test-sequencing bugs (all
validated FAITHFUL engine behavior: cross-turn expiry window, WD inline
resolution, rules-correct TriggerOrder prompt). Final wave 180/180 first try.

## Gap-tracker entries filed

- `qa/dsl-vocab-gaps.md`: G-DSL-TOKEN-HOST-EXCLUSION,
  G-DSL-CANDIDATE-LINKABILITY, G-DSL-MODIFIER-CAUSE-SCOPE,
  G-DSL-PREDICATE-UNKNOWN-FIELDS (serious — silent no-op gates), plus the
  earlier-filed `on_move` self-scope footgun.
- `docs/RUST_ENGINE_GAPS.md`: G-ENGINE-DELAY-BODY-BEFORE-TRASH, plus the
  earlier-filed G-ENGINE-SCHEDULED-DELAY-MANDATORY-SCAN.

## Follow-ups (out of campaign scope, flagged)

- BT20-056 play cost 0 vs printed 12 (play-cost reconciliation family).
- BT25-002/006 authored `kind: digimon` for Lv.2 eggs.
- ST1-13-style one-shot "all your Digimon" grants on BT13-112, BT20-037,
  BT22-052, BT9-103 (BT25-028 ambiguous).
- `color_includes` casualties BT24-017, EX11-012, EX11-074 (blocked on
  G-DSL-PREDICATE-UNKNOWN-FIELDS fix).
- P-241 absent from cards.json/official mirror/bundles — needs ingest.
