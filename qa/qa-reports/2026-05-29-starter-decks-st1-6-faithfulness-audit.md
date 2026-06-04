# Starter Decks ST-1 … ST-6 — Card-Text Faithfulness Audit

Date: 2026-05-29
Pipeline: `/batch-implement-cards-rust-dsl` (AUDIT mode, report-only)
Scope: all 96 unique card slots across ST-1…ST-6 (full re-audit; ST-2/ST-4 re-audited despite prior IMPLEMENTED verdicts)
Method: 18 Opus auditor sub-agents (3 per deck), each diffing the shipping YAML against the per-card printed text (`code/digimon-engine/cards/<set>/<ID>.json`), the base-repo DCGO C# (`DCGO/Assets/Scripts/CardEffect/ST<n>/<Color>/`), the official `general_rule.pdf` / `glossary.pdf`, and existing behavioral tests. 24 vanilla cards verified directly by the orchestrator. No YAML or test files were modified.

## Summary

| Verdict | Count |
|---|---|
| AUDITED-OK | 88 |
| AUDITED-MISSING-TESTS | 2 |
| AUDITED-DRIFT (major) | 1 |
| AUDITED-DRIFT (minor) | 5 |
| BLOCKED | 0 |
| **Total** | **96** |

> **Correction (2026-05-29):** ST5-15 Laser Eye was initially counted as a 7th drift; it is a **false positive**. Its real text is "\<De-Digivolve 1\> **up to** 2 of your opponent's Digimon" (the per-card JSON dropped "up to"), so `optional_zero: true` was correct all along. Corrected counts above; details in the Resolution section.

**Headline:** implementations are overwhelmingly faithful. One real timing bug (ST2-14). The other five drifts are one recurring pattern — the `optional` flag set on a *mandatory* targeted selection, which surfaces an illegal PASS the card text doesn't grant (a no-approximations over-exposure that lets an RL agent decline a forced suspend/return/add). All six are DSL-expressible today — fixes are flag/value changes, no engine/DSL gaps.

## Confirmed drift (action items)

### ST2-14 Sorrow Blue — DRIFT (major)
- **[Security]** "...can't attack or block until the end of **your next turn**" is modeled with `expiry: end_of_turn`. Installed during the opponent's attacking turn, `end_of_turn` expires at the end of that same turn — a full turn too early.
- **Fix:** both `[Security]` `add_modifier` expiries → `end_of_your_next_turn`. The `[Main]` half (`end_of_opponents_turn`) is correct.
- **Source:** printed text + DCGO `ST2_14.cs` (`UntilOwnerTurnEnd` on the security half vs `UntilOpponentTurnEnd` on main). No test covers the security expiry, so the bug is silent.
- File: `code/digimon-engine/cards/st2/ST2-14.yaml`

### The `optional`-on-mandatory-selection pattern — DRIFT (minor ×5)
Each card's printed text is a mandatory directive (no "may", no "up to"), but the YAML marks the target selection `optional: true`. The engine then exposes a PASS at that pending selection (`PendingSelection.is_optional` admits PASS), letting the controller skip a forced effect. DCGO uses `canNoSelect: false` for all of these.

| Card | Clause | YAML | Fix |
|---|---|---|---|
| ST4-03 Tentomon | "If it's a green Digimon card, **add it** to your hand" | `select_reveal optional:true` | `optional:false` |
| ST4-10 Lillymon | "**Add 1** level 6+ Digimon among them to your hand" | `select_reveal optional:true` | `optional:false` |
| ST4-13 HerculesKabuterimon | "**Suspend 1** of your opponent's Digimon" (post Digi-Burst) | `select_opponent_permanent optional:true` | remove `optional:true` |
| ST4-15 Needle Spray | "**Suspend 1**…" (main + security) | `optional:true` ×2 | remove ×2 |
| ST4-16 Electro Shocker | "**Return 1**… suspended Digimon" (main + security) | `optional:true` ×2 | remove ×2 |

Note on `select_reveal` (ST4-03/ST4-10): the engine already auto-skips when no eligible card is revealed (`select_reveal` returns `false` on an empty candidate set — [selections.rs:886](../../code/digimon-engine/src/effect_context/selections.rs)), so `optional:true` is *not* needed for the no-match path; it only adds the illegal decline when a match exists.

**Correct contrast — NOT drift (`optional_zero: true` is faithful):** "up to N" effects let the player pick 0, so `optional_zero: true` is correct. This covers **ST5-15 Laser Eye** ("\<De-Digivolve 1\> **up to** 2 of your opponent's Digimon" — the per-card JSON dropped "up to"; DCGO's description and `canEndNotMax: true` confirm it), **ST1-15 Giga Destroyer** ("Delete **up to 2**"), and **ST5-12 / ST6-12** ("**Up to 2** of your Digimon gain \<keyword\>"). The official "up to N ⇒ 0 is legal" rule outranks DCGO's force-≥1 UI here. **Lesson:** before flagging `optional_zero` as drift, confirm the card really lacks "up to" — the API-ingested JSON is lossy; cross-check DCGO's description / `canEndNotMax` flag or the fandom wiki.

## Missing tests (faithful YAML, coverage gap)

- **ST1-08 Garudamon** — `[When Digivolving]` +3000 DP effect has no behavioral test (the card is only exercised as a digivolution source elsewhere).
- **ST1-12 Tai Kamiya** — `[Security]` free-play clause is untested; only the DP aura is exercised.

## Non-blocking observations (no fix required)

- **Card-JSON ingest errors (YAML is correct):** ST3-15 Holy Flame's `<Security A. -N>` parenthetical says "checks N *additional*" but the glossary + DCGO define it as N *fewer* — the YAML correctly encodes "fewer". Several In-Training/inherited cards (ST1-09, ST2-01, ST2-12, ST3-01, ST5-11, ST6-11, …) carry their inherited text inside `effect_description_eng` with an empty `inherited_effect_description_eng`; the YAML correctly scopes these as `inherited`. These are `data/cards.json`-ingest artifacts, not card bugs.
- **Option `[Security]` scope label (ST6-15, ST6-16):** modeled with `scope: inherited`; functionally harmless because `Game::enqueue_from_security_card` filters only on the `security` flag. Cosmetic inconsistency shared with many sibling Option cards across sets.
- **Expiry aliasing (ST4-12, ST5-09, ST5-12, ST5-13):** `end_of_opponents_turn` vs `end_of_opponents_next_turn` is behaviorally identical for `[When Digivolving]`/`[Main]` (own-turn) installs; faithful.
- **ST6-13 CresGarurumon:** the `[Main]` activation additionally gates on a valid recur target existing, whereas DCGO lets you pay Digi-Burst and whiff. More player-friendly; hides no choice; minor.
- **ST6 test style:** ST6 behavioral tests are mostly structural shape assertions on compiled IR rather than end-to-end DebugRunner play-throughs — a coverage-depth note across the deck (not a faithfulness defect).

## Per-deck results

- **ST-1 Gaia Red:** 16/16 faithful. ST1-08, ST1-12 → MISSING-TESTS. No drift.
- **ST-2 Cocytus Blue:** 15/16 faithful. **ST2-14 → DRIFT (major).**
- **ST-3 Heaven's Yellow:** 16/16 faithful. No drift.
- **ST-4 Giga Green:** 11/16 faithful. **ST4-03, ST4-10, ST4-13, ST4-15, ST4-16 → DRIFT (minor).**
- **ST-5 Machine Black:** 16/16 faithful. (ST5-15 was a false positive — see correction above.)
- **ST-6 Venomous Violet:** 16/16 faithful. No drift.

Per-card verdicts recorded in `qa/qa-reports/validated_cards_dsl.json` (status + `audit_note`).

## Resolution (2026-05-29) — all gaps fixed (TDD)

Every finding was fixed test-first: a behavioral test was written to fail against the drifted YAML, then the YAML was corrected to pass it. All 96 ST-1…ST-6 cards are now `AUDITED-OK`.

| Card | Fix | Pinning test |
|---|---|---|
| ST2-14 | `[Security]` modifiers `end_of_turn` → `end_of_your_next_turn` | `st2_14_security_restriction_survives_until_your_next_turn` |
| ST4-03 | `select_reveal` `optional: true` → `false` | `st4_03_reveal_add_is_mandatory_when_eligible` |
| ST4-10 | `select_reveal` `optional: true` → `false` | `st4_10_reveal_add_is_mandatory_when_eligible` |
| ST4-13 | dropped `optional: true` on Digi-Burst suspend | `st4_13_digi_burst_suspend_is_mandatory` |
| ST4-15 | dropped `optional: true` (main + security suspend) | `st4_15_main_suspend_is_mandatory` |
| ST4-16 | dropped `optional: true` (main + security return) | `st4_16_main_return_is_mandatory` |
| ST5-15 | **reverted** — false positive; `optional_zero: true` restored (card is "up to 2") | `st5_15_main_de_digivolve_targets_up_to_two_and_allows_zero` |
| ST1-08 | (no YAML change) coverage added | `st1_08_when_digivolving_buffs_one_own_digimon` |
| ST1-12 | (no YAML change) coverage added | `st1_12_security_plays_tamer_for_free` |

Notes from the fix work:
- `select_reveal` already auto-skips an empty candidate set (returns before building a `PendingSelection`), so `optional: false` is the faithful flag for "add it if it matches" reveals — `optional: true` only added an illegal decline.
- ST2-14's security-installed modifier is attributed `source_player = controller (P1)`; installed during the opponent's turn its `pending_skips = 0`, so `EndOfYourNextTurn` correctly expires at the controller's *first* upcoming turn-end (the printed "your next turn"). DCGO's `UntilOwnerTurnEnd` agrees.
- **ST5-15 was first "fixed" to min-1, then reverted** once the real "up to 2" text was confirmed (the per-card JSON had dropped "up to"). Net YAML change for ST5-15 = none; it gained a test pinning the "up to 2, may pick 0, De-Digivolve 1 each" semantics.
- The new tests pass; the full `cards_behavioral` suite shows no new failures (7 unrelated pre-existing failures in `bt21-072` / `ex7-030` / `p-134` / `p-197` confirmed to fail identically on `HEAD` before this change).
