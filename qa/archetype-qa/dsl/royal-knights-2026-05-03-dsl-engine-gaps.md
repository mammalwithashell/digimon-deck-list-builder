# Royal Knights Rust DSL/Engine Gap Rollup

> **Royal Knights COMPLETION — 2026-06-15:** After closing the engine gaps,
> all 16 remaining PARTIAL/BLOCKED cards were authored in a parallel
> isolated-worktree wave, integrated, and faithfulness-verified against their
> card images (a read-only re-audit caught + fixed 3 digivolve-path bugs:
> BT20-045 missing standard DNA path, BT22-009 missing "Stnd." digivolve circle,
> BT20-060 name-gated-instead-of-level/color DNA route). **Royal Knights is now
> 69 IMPLEMENTED / 3 PARTIAL / 0 BLOCKED of 72.** The only 3 remaining residual
> gaps (one clause each): `G-BREEDING-DIGIVOLVE-UNION-ZONES` (BT20-056),
> `G-SUSPEND-SELF-COST-ON-OPPONENTS-TURN` (BT13-102), `G-OPTION-PERSIST-AS-FIELD-CARRIER`
> + `G-OPTION-SELF-TRASH-TRIGGER` (BT19-093) — all filed in `qa/dsl-vocab-gaps.md`
> ("OPEN 2026-06-15 — Royal Knights final-3 residual gaps"). Of the original
> ~30 "blockers", only 7 were real engine gaps (all closed); the other ~21 were
> authorable with shipped vocabulary. Report: `royal-knights-author-remaining-2026-06-15`.
>
> **Royal Knights engine-gap closure pass — 2026-06-15:** Adversarial scoping
> of the ~30 RK-"blocking" gaps (one scout each) found that **14 were NOT real
> gaps** — composable from shipped vocabulary today, needing only card authoring
> — and **6 genuine small/medium gaps were closed via TDD**, each making its
> consumer card fully faithful: `event_card_text_contains` (AD1-018, `19be5a16`),
> `return_selected_sources_to_deck` (BT13-075, `a83d2827`), `color_matches_returned_card`
> (EX10-068, `78c84132`), Delay free-reducer auto-apply (ST12-15, `b414917f`),
> deleted-DP effect payload + `effect_deleted_opponent_digimon_dp_gte` (EX4-065,
> `ba9afcee`), and `select_union_zone` material/breeding-source union (BT13-019,
> `59eb5994`). RK verdicts now **IMPLEMENTED 55 / PARTIAL 14 / BLOCKED 3**.
> Closures + the 14 reclassifications are itemized in `qa/dsl-vocab-gaps.md`
> ("RESOLVED / RECLASSIFIED 2026-06-15"). Still genuinely open (deferred):
> the large frameworks — G-OPTION-BATTLE-AREA-CARRIER (BT19-093),
> G-ONDECLINE-CALLBACK + G-WAS-PLAYED-BY-EFFECT-OBSERVER (BT13-102),
> G-BREEDING-DIGIVOLVE-UNION-ZONES (BT20-056), G-UNION-HAND-SOURCE-PLAY
> (EX11-053), G-OPPONENT-PLAYED-DIGIMON-LEVEL-BRANCH (RB1-035),
> G-SECURITY-END-OF-BATTLE-PLAY (BT22-009), G-OWN-SECURITY-ADDED-OBSERVER
> (BT8-090). Report: `royal-knights-engine-gaps-2026-06-15`.
>
> **Royal Knights re-audit + implementation pass — 2026-06-14:** A full
> 72-card re-audit (vs printed text + YAML + tests, 8 parallel auditors) found
> the verdict tracker `validated_cards_dsl.json` was **badly stale** — most
> "BLOCKED"/ignored-test gap markers cited substrate that had already shipped,
> and several cards (BT13-112, BT20-017, BT23-072, BT20-083, BT13-110, BT23-013,
> BT23-054, …) were fully FAITHFUL but still labelled BLOCKED/PARTIAL. The pass:
> (1) **implemented ~17 authorable-now clauses** via TDD — headline fix:
> **BT13-007 King Drasil's Royal-Knight cost reduction was a silent no-op in
> production** (its `amount_fn` was registered only in a test file → `formula_eval`
> returned 0); replaced with a native `base_per_delta(4 + material_count)` formula.
> Also closed: AD1-004, AD1-017 (was `effects:[]`), AD1-018, BT22-009 (dropped
> inherited delete), BT23-057 (was `effects:[]`), BT23-035, BT23-047, BT13-095,
> BT21-086, BT22-041, BT23-058, BT22-025, BT15-084, BT20-021 (Blast Digivolve),
> BT13-030. (2) **Reconciled all 72 verdicts** → IMPLEMENTED 49 / PARTIAL 19 /
> BLOCKED 4 (was reported 25/36/11). (3) **Filed 6 new gaps** in
> `qa/dsl-vocab-gaps.md` (G-DSL-EVENT-CARD-TEXT-CONTAINS,
> G-RETURN-SELECTED-SOURCE-TO-DECK-BOTTOM, G-PLAY-COST-GTE-MODIFIER-AURA,
> G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD, G-FOR-EACH-COUNTED-FIELD-OBJECTS,
> G-SOURCE-COUNT-SECURITY-TRASH). BT13-075 remains the only fully-BLOCKED RK
> body (needs the latter two-ish gaps); BT13-102/BT15-092/BT19-093 stay BLOCKED
> on engine-level gaps (on-decline callback, discard-from-security trigger,
> Option-battle-area carrier). Report: `royal-knights-audit-2026-06-14`.
>
> **Royal Knights reconciliation — 2026-05-22:** The current
> `qa/archetype-qa/royal-knights/deck_pool.json` resolves to **72** cards;
> all 72 have Rust YAML under `code/digimon-engine/cards/` and a behavioral
> test file under `code/digimon-engine/tests/cards_behavioral/`. Current
> source reconciliation before card-body migration classifies the active
> Royal Knights gap markers as follows:
>
> | Area / Cards | Current classification | Code-verified result |
> |---|---|---|
> | Optional own breeding permanent selection (`BT13-110`, `BT20-083`) | **Closed substrate / migrated consumers** | `select_own_breeding_permanent optional: true` now exposes PASS/decline and leaves mandatory/no-candidate behavior distinct. BT13-110 and BT20-083 consume it in active behavioral tests. |
> | Filtered own breeding permanent target (`BT13-093`, related King Drasil placement cards) | **Closed substrate / migrated current consumers** | RK-G001 filter support exists in the DSL compiled predicate and pre-check path; BT13-110, BT20-083, BT23-072, and related source-play paths now use the closed selection/source primitives where their printed clauses fit. |
> | Breeding material selection and play (`BT13-112`, `BT13-110`, `BT20-083`, `BT23-072`) | **Closed substrate / migrated current consumers** | `select_materials` and `play_from_materials` can select from the `BREEDING_TARGET` carrier, enforce name uniqueness, bind picked source cards, and suppress On Play. BT13-112, BT13-110, BT20-083, and BT23-072 now have focused production coverage for their expressible clauses. |
> | DP-budget opponent selection (`BT17-018`) | **Closed substrate / migrated** | `select_opponent_dp_budget` and engine running-budget selection exist. BT17-018 now uses native DSL for the 15000 DP budgeted delete instead of the stale raw-rust approximation. |
> | Event-bound may-attack / keyword grants (`BT20-017`, `BT23-072`) | **Closed or composable substrate / migrated** | Event-target binding plus `may_attack_now` / keyword grant support are present and consumed in BT20-017, BT23-013, and BT23-072 behavioral coverage. |
> | `BT23-013` | **Implemented for current closed substrate** | Rush, Alliance, Atho/Rene/Por token branch, Sistermon hand/trash name-excluded play, and other-Digimon-played may-attack observer are migrated. |
> | `BT13-019` | **Open substrate** | Needs a reusable union over trash plus breeding sources with name exclusions, not just hand/trash union. |
> | `EX11-053` | **Partial / residual open source-play shape** | On Play Royal Knight hand-to-fielded-King-Drasil source placement is migrated. On Deletion Omnimon X union hand/source play plus attach-self remains open. |
> | `BT20-021` | **Open substrate** | Union hand/trash source-as-cost, source-DP compare, unsuspend, and source-count security trash remain outside the closed Royal Knights substrate. |
> | `BT23-057` | **Open substrate plus card-authoring backlog** | Multi-card trash-to-deck placement/cost-reduction and dynamic play-cost delete-all remain true blockers; Hinukamuy token registration alone is closed. |
> | `BT23-058` | **Open substrate / out of this change's narrow closure** | Self-scoped on-suspend and aggregate lowest play-cost delete-all remain current blockers. |
>
> Raw-rust occurrences in the current Royal Knights pool are concentrated in
> `BT13-007`, `BT13-040`, `BT20-102`, `BT23-014`, and `EX8-074`;
> BT17-018's stale DP-budget raw-rust bridge has been removed.
> This change targets only the stale/raw Royal Knights gaps called out above;
> unrelated raw-rust bodies remain separate tracker debt unless a task names
> their primitive directly.

> **Closeout update — 2026-05-22:** `close-royal-knights-substrate-gaps`
> migrated the now-unblocked Royal Knights clauses for BT17-018, BT13-112,
> BT13-110, BT20-083, BT20-017, BT23-072, BT23-013, and EX11-053. Remaining
> active ignored tests now point to current tracker entries only:
> `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY` (BT13-019),
> `G-UNION-HAND-TRASH-SOURCE-COST` / source-count security trash
> (BT20-021), BT23-057 trash-return cost reduction plus dynamic play-cost
> delete, BT23-058 self-scoped on-suspend plus aggregate lowest play-cost
> delete-all, and EX11-053 On Deletion hand/source play plus attach-self.

> **Implementation slice — 2026-05-22,
> `complete-royal-knights-event-union-gaps`:** Printed-text reconciliation
> for the next change splits the target set into two reusable primitive
> families. Event-context coverage owns the security and event-payload clauses:
> BT15-084 (security trashed/removed), BT20-056 and BT20-060 (security stack
> removed), BT23-035 and BT23-047 (own/opponent security removed), BT8-090
> (security added), BT9-092 (same-level X Antibody digivolution), and RB1-035
> (opponent-played level branch). Union/source coverage owns BT13-019
> (Sistermon-from-trash OR Royal-Knight-from-breeding-sources play with fixed
> Gankoomon/Omnimon exclusion), EX11-053 On Deletion (Omnimon X from hand OR
> King Drasil source, then attach this card), and BT20-021 (Royal Knight
> hand/trash source-placement cost plus follow-up delete; source-count
> security trash remains the paired formula/security blocker). EX11-069 remains
> union-adjacent only if its trash digivolve and suspend-cost recursion can use
> the same source-binding machinery; otherwise it stays a separate
> effect-digivolve-from-trash/end-all-turns suspend-cost gap. Out-of-scope for
> this slice: BT23-057 trash-return cost reduction, BT23-058 lowest play-cost
> aggregate delete-all, and unrelated raw-rust card bodies.

> **Phase 2 Track J PR 1 — 2026-05-17:** Substrate enabler PR landed.
> Closures: **RK-G001** (filter on `select_own_breeding_permanent` +
> `BreedingPermanentRef` surfaced as a `Permanent(BREEDING_TARGET)`
> handle, unblocking hand-Main bottom-source placement under King Drasil).
> **Atho, René & Por** registered in `token_registry` with printed stats
> (White, 6000 DP, Reboot/Blocker/Decoy(Red/Black)). The token-registration
> half of `bt20_017_token_and_other_played_attack_observer` and
> `bt23_013_token_sistermon_union_play_and_attack_observer` is now closed.
> The two independent substrate halves are now both RESOLVED:
> `G-ALLY-PLAYED-MAY-ATTACK` (Task S2.1, already-composable) and
> `G-UNION-HAND-TRASH-NAME-EXCLUSION` (Task S2.2 — `select_union_zone`
> lowering now applies its `filter`, plus the new
> `name_not_shared_by_field_digimon` predicate leaf). Step 0 against
> printed text corrected the substrate plan: only BT23-013 has the
> hand+trash name-excluded play; BT20-017 has no union play, BT13-019
> plays from trash-or-breeding-sources (separate
> `G-UNION-TRASH-OR-BREEDING-SOURCES-PLAY`), BT20-021 *places* a source
> as a cost (separate `G-UNION-HAND-TRASH-SOURCE-COST`). Both resolved
> gaps are detailed in [qa/resolved-gaps.md](../../resolved-gaps.md).
> **RK-G003** audit confirmed
> closed for current Track B consumers (BT23-054, BT20-091, BT20-100) —
> the remaining BT23-058 ignored test stays parked on
> `G-SELF-ON-SUSPEND + G-PLAY-COST-AGGREGATE`, which are out of Track J
> scope. **BT17-077** already fully implemented (20 passing tests) —
> bulk trash-to-deck, returned-card binding (via pre-check workaround
> equivalent to "all-of-chosen-trash"), and by-cost return-opp-Digimon
> +unsuspend all working through existing primitives. Card authoring
> for BT13-093, BT13-110, BT20-083, EX11-053, EX11-071, BT20-017,
> BT23-013, and the Examon trio lands in Track J PR 2 / PR 3.

> **Tracker hygiene sweep — 2026-05-10:** Cross-referenced against PRs
> #449–#458. Track E DSL verbs landed (PR #454) so `raw_rust` carve-outs
> for the ten zone-movement verbs in `qa/dsl-vocab-gaps.md` are now
> expressible in YAML. Track C deferred modifier variants landed (PR
> #455) with typed `ModifierPayload`; identity overlays / DigiXros
> aliases / Security Attack / EndTurn min memory / Link cost+max are
> wired but a structured DSL payload schema is still pending. Track G
> keyword library closed (PR #457) — Evade printed-semantics fix,
> Decoy color-filter via `Keyword::Decoy(u8)`, Progress card-shape
> backfill. `Expiry::UntilCondition` runtime controller landed (PR
> #458). For the canonical engine-side closures consult
> [docs/RUST_ENGINE_GAPS.md](../../../docs/RUST_ENGINE_GAPS.md);
> per-archetype `raw_rust` carve-out audit lives in
> [qa/dsl-vocab-gaps.md](../../dsl-vocab-gaps.md). See
> `.claude/plans/pre-scaling-cleanup-batch.md` §2 for the closure-
> index narrative.


Date: 2026-05-03

Assessment source: `.codex/skills/assess-rust-engine-archetype/`

Target: `data/deck_library.json` archetype `Royal Knights`, prioritizing the current competitive lists and recurring core cards. This is a Rust DSL/engine gap document, not the legacy Python-lane faithfulness report in `qa/archetype-qa/royal-knights.md`.

## Purpose

This document distills the Royal Knights audit into reusable DSL and engine gaps that should feed a future cross-archetype implementation spec. It intentionally separates:

- reusable engine or DSL primitives that unblock many archetypes;
- Royal Knights card-YAML/test work that should not become a generic gap;
- reusable gaps already closed by recent groups so future specs do not reopen them.

## Verdict

`blocked`

Royal Knights is not faithfully implementable as executable Rust YAML DSL yet. The archetype depends on King Drasil in breeding, delayed Royal Knight options, stack-source selection/play, leave-field replacement prevention, and immediate attack effects. Some reusable primitives are already closed, but several remaining blockers affect action masks or pending-selection fidelity and therefore cannot be papered over with hidden auto-selection.

## High-Frequency Card Pool

The current `Royal Knights` deck-library entry has 51 decklists. These cards recur often enough to treat as core or near-core for cross-archetype gap planning:

| Card | Role | Current Rust DSL status | Notes |
|---|---|---|---|
| `BT13-007` King Drasil_7D6 | Core breeding engine | example YAML only | Start-main breeding trigger still needs breeding-source dispatch. |
| `BT20-100` The Last Guardian | Core Delay/prevention option | production YAML implemented for Track B slice | Search, option placement, Security hand/trash branch, and Delay Omnimon-name leave-prevention implemented. |
| `BT20-091` Cool Boy | Core tamer draw/memory and Omekamon play | production YAML implemented for Track B slice | RK play/digivolve suspend-cost draw+memory, opponent-turn non-cancelling Omekamon would-leave response, and Security play implemented. |
| `BT20-102` Omnimon (X Antibody) | Core finisher | production YAML partial | Boardwipe uses raw Rust; end-turn attack without suspending now routes through `force_attack` + `without_suspending`. |
| `BT20-083` Omekamon | Core bridge/blocker | production YAML partial | Blocker and low-security free Omnimon X digivolve implemented; breeding-source and material-play behavior still blocked. |
| `BT23-054` Magnamon | Core RK body | production YAML partial | Blocker, draw, selected return protection, and Armor Purge replacement implemented; broadened non-Track-B coverage still pending. |
| `BT13-112` Omnimon | Core payoff | production YAML gap stub | Plays one each different-name RK from King Drasil sources and suppresses On Play. |
| `BT20-017` Jesmon | Core body/token pressure | production YAML gap stub | Needs token creation plus immediate may-attack/action flow. |
| `BT20-060` Alphamon: Ouryuken | Core/near-core ACE | production YAML partial | ACE metadata, routes, DP reduction, Counter Blast DNA field+hand material flow, and DNA-origin security trash + Recovery rider implemented; global security-removed memory observer remains blocked. |
| `BT13-110` Royal Knights of the Purge | Core Delay option | production YAML partial | Draw and option placement implemented; King Drasil source placement/play blocked by `RK-G001`. |
| `BT23-035` Dynasmon | Common RK body | production YAML partial | Barrier and security-trash -6000 DP slice implemented; security-removed timing payload is wired, but the recovery branch still needs card-local authoring/tests. |
| `BT23-072` King Drasil_7D6 | Common support | production YAML gap stub | Hand main bottom-source placement; grants Rush/Raid/Reboot/Blocker. |
| `BT19-072` LordKnightmon | Common RK body | production YAML implemented slice | Trash play of level 4 or lower Digimon implemented; opponent-turn Royal Knight attack retarget implemented 2026-05-08. |

## 2026-05-05 Batch 1 Implementation Notes

Resolver input: `python code/tools/resolve_deck.py "Royal Knights"` wrote `qa/archetype-qa/royal-knights/deck_pool.json` with 51 decklists and 72 unique cards.

Implemented / audited in this batch:

| Card | Status | Implemented slice | Stubbed gap |
|---|---|---|---|
| `BT6-082` Sistermon Blanc | `IMPLEMENTED` | [All Turns] filtered Sistermon Blocker aura while own Huckmon/Royal Knight is in play; [On Play] Draw 1. | none |
| `BT13-093` Omekamon | `PARTIAL` | [On Play] Draw 1. | `RK-G001`: filtered King Drasil breeding target before placing a Royal Knight hand card as source. |
| `BT20-083` Omekamon | `PARTIAL` | Face-up Blocker; low-security optional free digivolve into hand [Omnimon (X Antibody)]. | `RK-G001` for On Deletion King Drasil target; `G-BREEDING-TRIGGER-DISPATCH` plus source/material play for inherited breeding trigger. |
| `EX11-071` Cool Boy | `PARTIAL` | [On Play] dual-bucket reveal top 3, add Omekamon/Omnimon X plus Royal Knight/LIBERATOR, bottom remainder. | `RK-G002`: return-this-Tamer activation cost feeding a reduced-cost hand play. |

Regression files:

- `code/digimon-engine/tests/cards_behavioral/bt6/bt6_082.rs`
- `code/digimon-engine/tests/cards_behavioral/bt13/bt13_093.rs`
- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_083.rs`
- `code/digimon-engine/tests/cards_behavioral/ex11/ex11_071.rs`

## 2026-05-05 Batch 2-3 Implementation Notes

Implemented / audited in these batches:

| Card | Status | Implemented slice | Stubbed gap |
|---|---|---|---|
| `BT20-100` The Last Guardian | `IMPLEMENTED` for current DSL surface | [Main] dual-bucket reveal/search, bottom remainder, place self as Delay option; [All Turns] Delay trash-self prevention for own Omnimon-name would-leave; [Security] explicit hand/trash branch for optional Omekamon/Cool Boy play, then place self. | Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_100_delay`. |
| `BT23-054` Magnamon | `PARTIAL` | Blocker; standard and CS/Veemon digivolve routes; Draw 1 plus selected Royal Knight/CS return-to-hand/deck protection; Armor Purge top-source replacement. | Remaining follow-ups are card-specific protection breadth/tests, not `RK-G003`. |
| `BT23-058` Craniamon | `PARTIAL` | Reboot, Blocker, standard/CS digivolve routes, optional suspend-self replacement preventing one own Digimon/Tamer from leaving by opponent effects. | Self-scoped `on_suspend` predicate plus aggregate lowest play-cost delete-all. |
| `BT13-110` Royal Knights of the Purge | `PARTIAL` | [Main] Draw 1 then place self in battle area; [Security] place self in battle area. | `RK-G001`: hand-to-King-Drasil source placement and Delay play from breeding sources with On Play suppression/Rush. |
| `BT20-091` Cool Boy | `IMPLEMENTED` for current DSL surface | Your-turn Royal Knight play/digivolve observer with suspend self, Draw 1, gain 1 memory; opponent-turn Royal Knight would-leave response optionally plays Omekamon and proceeds with the leave; [Security] play self. | Covered by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave bt20_091_decline_would_leave_response_proceeds_without_playing_omekamon bt20_091_no_omekamon_in_hand_does_not_offer_response`. |
| `BT19-072` LordKnightmon | `PARTIAL` | [On Play]/[When Digivolving] optional level 4 or lower Digimon play from trash. | `G-ATTACK-RETARGET`: opponent-turn attack target switch to own Royal Knight. |
| `BT20-060` Alphamon: Ouryuken | `PARTIAL` | Printed metadata, ACE Overflow -5, standard Black Lv.6 route, Black + Yellow/Red Lv.6 DNA route, [On Play]/[When Digivolving] selected -15000 DP modifier, Counter Blast DNA field+hand material flow, and DNA-origin security trash/recovery tail. | Global security-removed memory observer. |
| `BT19-072` LordKnightmon | `IMPLEMENTED` | [On Play]/[When Digivolving] optional level 4 or lower Digimon play from trash; [Opponent's Turn][OPT] attack target switch to own Royal Knight via `on_opponent_attack` + `redirect_attack_target`. | none for Track D retarget slice. |

Regression files:

- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_060.rs`
- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_100.rs`
- `code/digimon-engine/tests/cards_behavioral/bt23/bt23_054.rs`
- `code/digimon-engine/tests/cards_behavioral/bt23/bt23_058.rs`
- `code/digimon-engine/tests/cards_behavioral/bt13/bt13_110.rs`
- `code/digimon-engine/tests/cards_behavioral/bt20/bt20_091.rs`
- `code/digimon-engine/tests/cards_behavioral/bt19/bt19_072.rs` (`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt19_072_opponents_turn_switches_attack_target_to_royal_knight`)
- `code/digimon-engine/tests/cards_behavioral/bt19/bt19_072.rs`

## 2026-05-05 Batch 4-15 Implementation Notes

Pool coverage after the full batched pass: `qa/archetype-qa/royal-knights/deck_pool.json` resolves to 72 unique cards, and all 72 now have Rust DSL YAML entries under `code/digimon-engine/cards/`. The final 25 cards in this pass each have an active embedded-pack load test plus an ignored gap test for unsupported printed clauses.

Implemented / audited in these batches:

| Card | Status | Implemented slice | Stubbed gap |
|---|---|---|---|
| `AD1-004` Examon | `PARTIAL` | Raid and Piercing. | Token play from sources / printed multi-part action flow not covered in this RK pass. |
| `AD1-017` Dynasmon | `BLOCKED` | Load-only gap stub. | Top-or-bottom security-trash cost plus board-wide debuff. |
| `AD1-018` Gallantmon | `PARTIAL` | [On Play]/[When Digivolving] play-cost 3 or lower delete. | Inherited security-removed / retaliation shape. |
| `BT13-019` Gankoomon | `PARTIAL` | Blocker. | Union play from trash or breeding sources with name exclusions. |
| `BT13-030` UlforceVeedramon | `BLOCKED` | Load-only gap stub. | Counted source trash by Royal Knight/blue Tamer count; sourceless-opponent aura. |
| `BT13-040` Kentaurosmon | `PARTIAL` | Blocker, Recovery +1, and non-cancelling would-leave draw plus optional Veemon play from hand/materials. | Security-search/placement and attack-prevention tail. |
| `BT13-075` Alphamon | `BLOCKED` | Load-only gap stub. | Source-placement cost tied to play-cost 10+ attack-player restriction; security-trash leave replacement. |
| `BT13-087` Dynasmon | `PARTIAL` | Reveal 4; add up to two Lucemon/Royal Knight cards; trash rest. | Another matching Digimon played observer and delete-all level 4 or lower. |
| `BT13-102` Keenan Crier | `PARTIAL` | [Security] play self. | Opponent hidden-hand choice; opponent-turn effect-play memory observer. |
| `BT13-111` Gallantmon | `PARTIAL` | Rush. | Combined-trash play-cost reduction; delete-result fallback. |
| `BT13-112` Omnimon | `IMPLEMENTED` for current closed substrate | Modal delete and different-name Royal Knight source play from breeding, King Drasil trash, Rush grant, and On Play suppression. | none for this change's migrated slice. |
| `BT15-092` Revelation of Light | `BLOCKED` | Load-only gap stub. | Security-trash self-dispatch; security search/play; self-to-security top; security-Digimon debuff. |
| `BT17-077` Imperialdramon: Paladin Mode | `PARTIAL` | ACE metadata; trash all sources of all opponent Digimon. | Blast Digivolve; bulk trash-to-deck; returned-card memory binding; sourceless bottom-deck cost. |
| `BT19-093` Queen Device | `BLOCKED` | Load-only gap stub. | Option battle-area carrier lifecycle; negative color-bypass predicate; two-target security modifier. |
| `BT20-017` Jesmon | `IMPLEMENTED` for current closed substrate | Atho/Rene/Por token play; other-Digimon-played delete and may-attack observer. | none for this change's migrated slice. |
| `BT20-021` Jesmon GX | `BLOCKED` | ACE metadata and standard route. | Union hand/trash source cost; source-DP compare; unsuspend; source-count security trash. |
| `BT20-045` Examon | `PARTIAL` | ACE metadata; Raid, Piercing, Blocker, Evade; Counter Blast DNA using Breakdramon + Slayerdramon. | DNA-gated highest-DP bottom-deck sweep; any-Digimon-suspend observer. |
| `BT20-056` Alphamon | `PARTIAL` | Barrier; [On Play]/[When Digivolving] Recovery +1. | During-attack breeding digivolve; security-removed observer; inherited replacement. |
| `BT22-025` UlforceVeedramon | `PARTIAL` | ACE metadata; [When Attacking][OPT] unsuspend self. | Blast Digivolve; modal lowest-level bottom-deck or blue Tamer play. |
| `BT22-041` Kentaurosmon | `PARTIAL` | Blocker, Barrier, optional yellow hand-to-top-security. | Total-security play-cost reduction; self-suspend security-trash unsuspend cost. |
| `BT22-052` Leopardmon | `PARTIAL` | ACE metadata; optional 5000 DP-or-lower hand play; own level 3+ Blocker grant; Blast Digivolve marker and other-Digimon would-leave memory observer. | Remaining gaps are outside the Track B replacement/Counter marker slice. |
| `BT23-013` Jesmon | `IMPLEMENTED` for current closed substrate | Rush, Alliance, Atho/Rene/Por token choice, Sistermon hand/trash name-excluded play, and other-Digimon-played may-attack observer. | none for this change's migrated slice. |
| `BT23-035` Dynasmon | `PARTIAL` | Barrier; top-security cost into -6000 DP board debuff. | Security-removed Security A. +1 / recovery tail. |
| `BT23-047` Examon | `PARTIAL` | Piercing, Security A. +1, and declared green Lv.5 + blue Lv.5 Partition source requirement. | Five-target suspend; next-unsuspend lock; may attack; security-removed tail. |
| `BT23-057` Gankoomon | `BLOCKED` | Load-only gap stub. Hinukamuy token registered in `token_registry.rs` (Track J S2.3 — White/6000 DP/`<Alliance> <Reboot> <Blocker>`). | Multi-card trash-to-deck cost reduction; dynamic play-cost delete. |
| `BT23-072` King Drasil_7D6 | `IMPLEMENTED` for current closed substrate | Hand-main source placement; played-Digimon Rush/Raid/Reboot/Blocker grant; inherited breeding-source play. | none for this change's migrated slice. |
| `EX8-073` Gallantmon (X Antibody) | `BLOCKED` | Load-only gap stub. | Source-gated DP swings; delete-or-security fallback; memory aura immunity. |
| `EX10-068` Digimon Emperor | `PARTIAL` | [On Play] delete play cost 5 or lower; [Security] play self. | Opponent distinct-color count; returned-card color binding into same-color hand/trash play. |
| `EX11-053` Omekamon | `PARTIAL` | On Play Royal Knight hand-to-fielded-King-Drasil source placement. | On Deletion Omnimon X union hand/source play and attach self. |

Regression files were added under:

- `code/digimon-engine/tests/cards_behavioral/ad1/`
- `code/digimon-engine/tests/cards_behavioral/bt13/`
- `code/digimon-engine/tests/cards_behavioral/bt15/`
- `code/digimon-engine/tests/cards_behavioral/bt17/`
- `code/digimon-engine/tests/cards_behavioral/bt19/`
- `code/digimon-engine/tests/cards_behavioral/bt20/`
- `code/digimon-engine/tests/cards_behavioral/bt22/`
- `code/digimon-engine/tests/cards_behavioral/bt23/`
- `code/digimon-engine/tests/cards_behavioral/ex8/`
- `code/digimon-engine/tests/cards_behavioral/ex10/`
- `code/digimon-engine/tests/cards_behavioral/ex11/`

## Reusable Open Gaps

### Breeding-Area Trigger Fan-Out

- **Gap:** Effects whose source permanent remains in breeding are not generally enqueued at turn/event timings.
- **Type:** `engine-gap`
- **Tracker:** `qa/archetype-qa/engine-gaps.md` (`G-BREEDING-TRIGGER-DISPATCH`)
- **Blocks:** `BT13-007` start-main source tuck; breeding-source observer shapes under King Drasil.
- **Why it matters:** King Drasil is the archetype's central source stack. If it must be moved to battle or silently skipped, the legal game state and action mask diverge from printed text.
- **Evidence:** `code/digimon-engine/cards/_examples/BT13-007.yaml` authors `when: start_of_your_main_phase` with `active_when: { in_breeding: true }`; `qa/archetype-qa/engine-gaps.md` records that start-main dispatch scans battle-area observers, not breeding sources.
- **First test:** Put `BT13-007` in player 0 breeding, a Royal Knight in player 0 battle area, and at least one card in the Digi-Egg deck. Enter main phase and assert the digitama plus the battle-area Royal Knight move under King Drasil.
- **Implementation hint:** Add a breeding trigger source or an `include_breeding` fan-out mode for timings like `StartOfYourMainPhase`, while preserving source-card/controller attribution and once-per-turn accounting.

### Global Security-Removed Observer Timing

- **Gap:** Security-stack removal must fan out to battle and relevant inherited/breeding observers with correct controller and event context.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`Global OnOpponentSecurityRemoved observer timing`)
- **Blocks:** `BT20-083` inherited Omekamon, `BT20-060` Alphamon: Ouryuken, `BT23-035` Dynasmon, `BT20-056` Alphamon.
- **Why it matters:** Royal Knights repeatedly reacts to security being removed. Missing or partial observer fan-out either removes legal optional effects from the action mask or lets mandatory effects fail to trigger.
- **Evidence:** `docs/RUST_ENGINE_GAPS.md` lists global `OnOpponentSecurityRemoved` as a blocking reusable timing gap; `BT20-083` example YAML has inherited `when: on_opponent_security_removed`.
- **First test:** Place `BT20-083` as a source under King Drasil in breeding, remove the controller's security on the opponent's turn, and assert the optional Omekamon play-from-materials prompt appears.
- **Implementation hint:** Treat security removal as a first-class event with trigger context fields for affected player, source effect controller, removed count/card if available, and observer fan-out over battle and breeding/inherited sources.
- **Updated 2026-05-06:** Battle-area and battle/effect security-removal payloads are now wired for both `OnOpponentSecurityRemoved` and `OnOwnSecurityRemoved` and proved by BT24-001 plus BT4-097 fixtures. This RK gap is narrowed to breeding-resident/inherited King Drasil fan-out and card-local follow-up selections.
- **Updated 2026-05-08:** Breeding-resident/inherited fan-out for `OnOpponentSecurityRemoved` / `OnOwnSecurityRemoved` is now wired through `TriggerSource::SecurityRemoved` plus `enqueue_from_breeding_permanent`. `BT20-083` has a card-shaped fan-out proof that the inherited breeding source fires exactly once and retains the security-removal payload / `BREEDING_TARGET` carrier. Remaining BT20-083 work is the printed body: suspend the breeding carrier as a cost and play an [Omekamon] from that breeding stack's materials without paying the cost. Evidence: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test timing_dispatch -- on_opponent_security_removed_fans_out_to_breeding_inherited_once_with_payload`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_083_inherited_breeding_security_removed_fans_out_once_with_payload`.

### Immediate May-Attack / Attack Without Suspending

- **Gap:** Scripts cannot install an immediate in-effect attack, including the "without suspending" variant.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`Force-follow-up-attack / "may attack without suspending" script helpers`)
- **Blocks:** `BT20-102` end-of-turn clause; `BT20-017` Jesmon tail; `BT13-112` and `BT13-110` Rush payoff patterns after playing Royal Knights.
- **Why it matters:** These effects grant a player-visible attack decision. Granting only Rush or auto-attacking would change the action surface.
- **Evidence:** `code/digimon-engine/cards/bt20/BT20-102.yaml` now lowers the printed Rush + attack-without-suspending clause to `force_attack` with `without_suspending: true`, matching DCGO's optional trigger followed by `SelectAttackEffect.SetCanNotSelectNotAttack() + SetWithoutTap()`.
- **Status update (2026-05-08):** Partially closed for `BT20-102`. DSL has `force_attack` / `may_attack_now` immediate attack steps with `without_suspending`, and the BT20-102 card-shaped behavior now proves the selected Digimon opens a mandatory attack prompt and remains unsuspended after attacking. Remaining Royal Knights follow-up attack cards still need their own YAML/test migration. Proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt20_102`.
- **First test:** Resolve `BT20-102` end-of-turn effect, choose a Digimon, and assert the next pending action allows that Digimon to attack without suspending, with pass/decline behavior matching printed optionality.
- **Implementation hint:** Add a script primitive such as `force_follow_up_attack` or `may_attack_now` plus an attack flag for `without_suspending`, reusing existing pending attack/action-mask machinery.

### Leave-Field Replacement and Prevention Effects

- **Gap:** Remaining Royal Knights prevention work is now card-specific follow-up coverage rather than a missing Track B replacement substrate.
- **Type:** `engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`WhenWouldBeDeleted / leave-field replacement-effect framework`, `<Armor Purge>`, `<Barrier>`)
- **Blocks:** No longer blocks `BT23-054`, `BT23-035`, `BT23-058`, `BT20-056`, or `BT20-100` on the core replacement framework. Residual card rows track their non-replacement clauses.
- **Why it matters:** The archetype survives by preventing departures from battle. These are optional or costed replacement decisions, so they must surface as choices before the zone move resolves.
- **Evidence:** `BT23-054` Armor Purge, `BT23-035`/`BT20-056` Barrier, `BT23-058` optional opponent-effect prevention, `BT20-100` Delay prevention, `BT13-040` non-cancelling would-leave, and `BT20-091` non-cancelling Omekamon response all use the same replacement/pending-selection layer.
- **Proof:** `cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements`; `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_054_armor_purge ex11_019_inherited_barrier bt13_040_when_leaving bt20_091_opponent_turn_may_play_omekamon_when_royal_knight_would_leave`.
- **Implementation hint:** Treat future issues here as card-specific tests or missing follow-up primitives unless a new printed prevention shape cannot be expressed through `kind: replacement`.

### Stack-Source Multi-Selection and Play From King Drasil

- **Gap:** The archetype needs robust selection and extraction from a breeding permanent's digivolution cards, including "one each with different names", On Play suppression, and Rush grants.
- **Partial closure 2026-05-19 (Phase 2 Track J Task S1.1):** The **On Play suppression** sub-primitive is resolved — `play_from_materials` (and the other `play_from_*` steps) accept `suppress_on_play: true`, which skips only the just-played permanent's [On Play] enqueue. See [`qa/resolved-gaps.md`](../../resolved-gaps.md#engine--dsl-gap-effect-play-with-played-digimon-on-play-suppression--resolved-2026-05-19-phase-2-track-j-task-s11-puppets-g030) (PUPPETS-G030, proven by BT5-106).
- **Partial closure 2026-05-19 (Phase 2 Track J Task S1.2):** The **count-capped / name-unique source multi-select + batch play** sub-primitive is resolved for battle-area carriers. A new `select_materials` DSL step picks up to N digivolution sources of a carrier permanent in ONE count-capped multi-pick with `uniqueness: name` ("1 of each different name"); it lowers to `EffectContext::select_count_capped_multi` + `CountCappedZone::Material` + `DistinctByMode`. `play_from_materials` now consumes the bound `CardList` as a batch, each picked source becoming a fresh permanent, composing with `suppress_on_play`. Proven by `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials`.
- **Source-select substrate CLOSED 2026-05-20 (Phase 2 Track J Task S1.3):** the breeding-carrier source-select residual is now closed. Task S1.3 appended a 24-slot `BREEDING_SOURCE_SELECT` action sub-range (`2168..2192`, keyed by carrier owner), raising `ACTION_SPACE_SIZE` 2168→2192 — a deliberate action-space version bump (existing trained RL models must be retrained). `select_material` / `select_materials` against a `BREEDING_TARGET`-sentinel carrier now install a real `pending_selection` with breeding-source action IDs; `material_zone_geometry` is the single battle-vs-breeding branch point. Proven by `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials::select_materials_breeding_carrier` and `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_carrier`. **Remaining open work for this gap is now purely card-authoring:** the Rush grant on the played Digimon, and authoring the production YAML for each card below.
- **Type:** `engine-gap` / `dsl-gap` — engine/DSL substrate CLOSED; remaining work is card-authoring.
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` and `qa/dsl-vocab-gaps.md` under source/material selection and play-from-material helpers
- **Blocks:** `BT13-112`, `BT13-110`, `BT13-019`, `EX11-053`, `BT20-083`, `BT23-072` — the breeding-carrier source-select encoding is no longer a blocker; remaining per-card work is the Rush grant and production YAML authoring.
- **Why it matters:** Auto-playing the first matching source or ignoring name uniqueness hides a major Royal Knights decision. The selected cards must leave the source stack and become fresh permanents with correct On Play suppression when printed.
- **Evidence:** `BT20-083` example YAML uses `select_material` and `play_from_materials`. The `suppress_on_play` flag closure is proven by `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt5_106`; the count-capped / name-unique multi-pick + batch play (battle and breeding carriers) is proven by `cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test dsl -- select_materials`.
- **First test:** Give King Drasil multiple Royal Knight sources with duplicate and distinct names, resolve `BT13-112`, and assert the player can choose at most one per name, selected cards enter battle, On Play effects are suppressed, King Drasil is trashed, and all played Digimon gain Rush. *(S1.2 + S1.3 prove the count-cap + name-uniqueness + batch-play + suppress + breeding-carrier source-select slices in `tests/dsl/select_materials.rs`; the full BT13-112 card remains card-authoring work — the Rush grant plus production YAML.)*
- **Implementation hint:** A generic count-capped source selection with uniqueness predicates exists (`select_materials`) and works for both battle-area AND breeding-area carriers as of S1.3. Remaining per-card work is the Rush grant and authoring the production YAML.

### Raid Target-Switch Timing

- **Gap:** Closed for the reusable mid-attack Raid primitive; remaining work is card authoring where Raid is granted dynamically.
- **Type:** `card-yaml/test-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` (`Raid target-switch interrupt`)
- **Blocks:** `BT20-102`, `BT23-072` granted Raid, and any other Royal Knight tech with printed Raid.
- **Why it matters:** Mask-time targeting is not equivalent to "when this Digimon attacks, you may switch the target". The player must be able to attack security first and then decide whether to Raid.
- **Evidence:** `RaidOpen` now sits before Alliance/Counter/Block, offers an optional switch when the opponent has unsuspended highest-DP Digimon, and dispatches `OnAttackTargetChange` with `reason = Raid`. Proof: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat -- raid_retarget`.
- **Next card test:** Add card-shaped coverage when a Royal Knight card grants Raid dynamically, such as BT23-072's played-Digimon keyword package after its played-Digimon observer gap closes.

### Aggregate-Sum Multi-Select

- **Status:** Resolved on 2026-05-22 for `BT17-018` via native `select_opponent_dp_budget`.
- **Former gap:** Select any number of targets constrained by a running aggregate sum, such as total DP.
- **Type:** `resolved-engine-gap`
- **Tracker:** `docs/RUST_ENGINE_GAPS.md` / `qa/archetype-qa/engine-gaps.md` (`G-DP-BUDGET-MULTI-SELECT`)
- **Unblocked:** `BT17-018` Gallantmon: Crimson Mode, a common Royal Knights tech.
- **Why it matters:** A single-pick fallback is not faithful when the player may choose any number of opposing Digimon whose total DP is at most 15000.
- **Evidence:** `code/digimon-engine/cards/bt17/BT17-018.yaml` now uses `select_opponent_dp_budget` with `dp_budget: 15000`; `bt17_018_delete_clause_uses_native_dp_budget_selection` and the active 7000/8000/9000 DP behavioral tests cover the running-sum mask.

## Resolved Reusable Gaps To Avoid Reopening

The following reusable primitives surfaced in Royal Knights or adjacent audits and are now closed enough that a future cross-archetype spec should treat remaining work as card migration/test work unless new evidence appears:

| Primitive | Status | Remaining Royal Knights work |
|---|---|---|
| Delay option lifecycle and placement-turn gating | resolved 2026-05-02 | Author/migrate `BT20-100` and `BT13-110` YAML/tests. |
| `OnOptionPlaced` timing and breeding inherited fan-out for placed options | resolved 2026-05-02 | Use native timing in King Drasil inherited effect; add card-level regression. |
| Breeding permanent selection and bottom-source placement to real breeding slot | resolved 2026-04-29 / 2026-05-02 | Use explicit breeding selection or sugar in Omekamon/King Drasil scripts. |
| `self_digivolution_contains_name` predicate | resolved 2026-05-02 | Migrate `BT20-102` away from raw Rust only after other boardwipe/attack blockers are handled. |
| `not_in_binding` for excluding saved permanents | resolved 2026-05-01 | Use for BT20-102 boardwipe once source-stack predicate and flow are native. |
| `card_count_in_zone` formula filters | resolved 2026-05-02 | `BT8-097` already uses filtered count in YAML. |
| `dp_lte` / `dp_gte` permanent predicates | resolved 2026-05-02 | Update stale card comments/tests that still cite the old predicate gap. |
| Ace Overflow metadata and covered stack-leave paths | resolved 2026-05-02 | Add targeted coverage for exotic ACE movements before relying on them. |
| Dynamic cost reduction and triggered pay-cost selections | resolved Group 3 | Use for King Drasil cost reduction and similar play-cost hooks. |

## Card-Local YAML/Test Backlog

These items should not be promoted to generic gaps until a failing test proves a reusable primitive is missing:

| Card | Needed next step |
|---|---|
| `BT20-100` The Last Guardian | Delay leave-prevention implemented; remaining follow-up is King Drasil inherited memory from option placement when breeding fan-out is reliable. |
| `BT20-091` Cool Boy | Opponent-turn optional Omekamon hand play when an RK would leave implemented. |
| `BT23-054` Magnamon | Armor Purge is implemented; broaden behavioral runtime tests for return-to-hand/deck protection. |
| `BT13-112` Omnimon | Fill payoff stub after source multi-select/play support; test On Play suppression and Rush grant. |
| `BT20-017` Jesmon | Fill token/observer stub after token registration and may-attack flow; test token creation, other-Digimon-play observer, delete target, and may-attack tail. |
| `BT20-060` Alphamon: Ouryuken | Add security-removed memory observer after the global security-removed fan-out gap closes. Counter Blast DNA and DNA-gated security trash/recovery are now covered by `bt20_060_hand_counter_blast_dna_uses_alphamon_and_ouryumon` and `bt20_060_dna_origin_trashes_security_and_recovers`. |
| `BT23-035` Dynasmon | Add security-removed recovery branch after observer gap; broaden runtime test for security-trash cost debuff. |
| `BT23-072` King Drasil_7D6 | Fill support stub after hand-main source placement and breeding/inherited fan-out support; test granted keyword package. |
| `BT13-110` Royal Knights of the Purge | Add source selection from King Drasil, On Play suppression, and Rush grant after `RK-G001`. |
| `EX11-053` Omekamon | Fill stub after hand-to-fielded-source and union hand/source play support; test placing RK source under King Drasil and low-security Omnimon play from hand/materials. |

## Suggested Spec Grouping

For a cross-archetype gap spec, group remaining work by reusable capability rather than by Royal Knights card:

1. Breeding-source event fan-out: start-main, security-removed, and inherited observer attribution.
2. Immediate attack action flow: may-attack, force-follow-up attack, and without-suspending variants.
3. Replacement/prevention framework: would-leave, Armor Purge, Barrier, and Delay-as-replacement.
4. Source-stack selection/play: breeding carrier sources, uniqueness constraints, play-from-materials, On Play suppression.
5. Combat interrupt timing: Raid and attack-target-change events.
6. Aggregate pending selections: DP-budget multi-select.
7. Card migration pass: Royal Knights YAML/tests using only primitives closed above.

## Spec Input Checklist

A future spec should require each reusable gap to include:

- one failing Rust behavioral test under `code/digimon-engine/tests/`;
- one DSL lowering/compiler test when YAML vocabulary changes;
- action-mask or `PendingSelection` assertions for every player-visible choice;
- no `ACTION_SPACE_SIZE` or tensor contract expansion unless the action/tensor specs are updated in the same change;
- tracker updates in `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, and this file when a reusable gap closes or is split.
