# Rocks Rust DSL Batch Log

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


Date: 2026-05-04

Deck resolver input: `Rocks`

Resolved pool artifact: `qa/archetype-qa/rocks/deck_pool.json`

## Batch 1

Cards: `BT14-009`, `BT18-064`, `EX8-051`, `ST13-08`

Status: `IMPLEMENTED`

Verification:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt14_009 bt18_064 ex8_051 st13_08 --nocapture
```

Result: 9 passed.

Notes:

- `BT14-009` moved from example-only YAML to production set YAML and covers bilateral `CannotPlayDigimonByEffect`.
- `BT18-064` covers hand/deck return immunity plus inherited opponent-turn DP contribution.
- `EX8-051` covers printed keywords and exact-source-trash inherited De-Digivolve 1; this required reusable queue support for the trashed source card's own inherited `OnDigivolutionCardTrashed` effect.
- `ST13-08` covers bilateral play-cost-reduction lock.

## Batches 2-9

Status: pool pass complete.

Implemented or partial YAML/test passes:

- Batch 2: `EX8-005`, `BT21-055`, `EX10-025`, `EX8-047` - source-trash inherited memory/delete clauses.
- Batch 3: `EX8-046`, `EX11-038`, `EX10-028`, `EX10-032` - source-trash draw/delete/De-Digivolve plus `EX8-046` Blocker slice.
- Batch 4: `BT4-072`, `EX8-050`, `EX10-034`, `P-215` - static inherited/face-up keyword and DP slices.
- Batch 5: `EX8-048`, `P-167`, `P-186`, `EX8-055` - source-trash delete/De-Digivolve, Rush/Blocker, Fragment(3).
- Batch 6: `BT23-059`, `EX10-033`, `EX10-036`, `EX11-044` - Blocker/Reboot/Fragment(3) slices.
- Batch 7: `EX8-067`, `P-039`, `P-107`, `P-169` - memory setters, Memory Boost/Training reveal and Delay slices.
- Batch 8: `EX10-063`, `EX7-049`, `LM-031`, `LM-032` - source-trash Tamer memory, De-Digivolve, and Scramble main digivolve slices.
- Batch 9: `BT23-096`, `BT8-094`, `EX10-069`, `ST22-11` - security/main De-Digivolve, security play, and Unique Emblem hand/trash play slices.

Blocked after pass (Phase 2 Track E 2026-05-17 update — BT9-103 advanced to IMPLEMENTED):

- `BT20-055`: face-up security lifecycle and security end-of-opponent-turn play timing.
- `BT21-021`: conditional inherited keyword, Save, and Xros Heart play routing.
- ~~`BT9-103`~~: **IMPLEMENTED** 2026-05-17 via Phase 2 Track E. Authored with
  `add_player_modifier` (CannotAddSecurityByEffect) + `for_each` +
  `add_modifier` (CannotAttackPlayer, `play_cost_lte: 7`, expiry
  `end_of_opponents_turn`). See `code/digimon-engine/cards/bt9/BT9-103.yaml`
  + `code/digimon-engine/tests/cards_behavioral/bt9/bt9_103.rs`.
- `EX10-003`: attack cancellation by trashing three Mineral/Rock sources.
- `EX11-065`: hand-or-source costs plus source placement from hand/trash.
- `EX8-070`: source-cost selection, temporary protection, and lowest-play-cost security delete.
- `P-130`: effect move-from-breeding and on-move suspend-memory trigger.

2026-05-22/23 EX8-050 clause completion:

- `EX8-050` Gogmamon: upgraded from Blocker-only to full clause coverage. Clauses now authored: `<Blocker>`, `[On Deletion]` reveal-top-3 + optional bucket-select + `play_from_revealed_free` + trash rest, inherited `[Opponent's Turn][OPT]` redirect-attack-to-self. YAML uses `select_reveal_buckets` (not `select_reveal`) so the trash tail always runs via the callback even when the player picks nothing. 2026-05-23 follow-up closed `G-PLAY-FROM-REVEALED-FREE`; focused coverage is 17 passed / 0 ignored for `ex8_050`.

Phase 2 Track E (2026-05-17) PARTIAL → IMPLEMENTED advancements:

- `P-167`: `[Start of Your Main Phase][When Digivolving]` reveal/source-trash/search/source-placement clause now authored using the new `choose_from_reveal` + `order_remainder` DSL verbs. Inherited source-trash De-Digivolve clause unchanged.
- `EX8-047`: `[On Play]` reveal/two-pick clause (Mineral/Rock + LIBERATOR → hand, remainder to deck bottom) authored using the same new verbs. Inherited source-trash delete unchanged.

Phase 2 Track E (2026-05-17) DSL modernization (no status change):

- `P-206`: inherited security clause's `raw_rust { fn: p_206_add_self_to_hand }` replaced with native `add_this_option_to_hand: {}` (identical engine call). `G-ADD-OPTION-SELF-TO-HAND` closed.

Pulled-main update:

- `P-123` now has production YAML/tests on main and is no longer counted in the Rocks blocked remainder.

Verification slices:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_005 bt21_055 ex10_025 ex8_047 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_046 ex11_038 ex10_028 ex10_032 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt4_072 ex8_050 ex10_034 p_215 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_048 ex8_055 p_167 p_186 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_059 ex10_033 ex10_036 ex11_044 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex8_067 p_039 p_107 p_169 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- ex10_063 ex7_049 lm_031 lm_032 --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt23_096 bt8_094 ex10_069 st22_11 --nocapture
```

---

## Batch 10 (2026-05-11) — formerly-blocked IMPLEMENT pass

Date: 2026-05-11
Pipeline: `/batch-implement-cards-rust-dsl Rocks`
Scope: 4 cards previously listed as "Blocked after pass" — re-attempted given engine progress since 2026-05-04.

| Card ID  | Name | Verdict | Tests | Notes |
|---|---|---|---|---|
| BT21-021 | OmniShoutmon | IMPLEMENTED | 24 (1 ign) | digixros_aliases ["Shoutmon"] + 3 alt-paths + [End of Attack] cost-5 play & self-delete + [On Deletion] hand/trash place under Tamer + auto-Save keyword + inherited [Your Turn] Rush aura. Ignored test: `G-DSL-AURA-TARGET-SOURCE-PERMANENT` for carrier-trait condition on inherited aura. |
| EX11-065 | Close | IMPLEMENTED (supersedes Batch 10 PARTIAL) | 18 | 2026-05-23: Clause 0 [Start of Your Main Phase] now uses the hand-or-digivolution-source union-zone cost selector. All Turns observer and Security remain implemented. |
| EX8-070  | Zofr Kabus | IMPLEMENTED | 22 | Main: select Mineral/Rock Digimon + select source to trash + grant Collision/Piercing/Reboot/CannotBeReturnedToHand/CannotBeReturnedToDeck/+3000DP all with `expiry: end_of_opponents_turn`. Security: lowest-cost opponent delete via `raw_rust` workaround for `G-PLAY-COST-AGGREGATE` (same pattern as BT9-112). |
| P-130    | Lui Ohwada | IMPLEMENTED (supersedes Batch 10 PARTIAL) | 14 | 2026-05-23: [On Play] move-from-breeding now uses the optional level-filtered `move_from_breeding` DSL step. [Your Turn] on_move trigger and Security remain implemented. |

Verification:
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt21_021 ex11_065 ex8_070 p_130
# 70 passed; 0 failed; 8 ignored
```

Full-suite regression (no per-binary failures):
```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
# All binaries pass; cards_behavioral: 2251 passed, 499 ignored.
```

### Blocked / partial residual after Batch 10
- `BT9-103` — play-cost-filtered player attack restriction + opponent security-add lock (carried over).
- `EX10-003` — attack cancellation by trashing three Mineral/Rock sources (carried over).
- `EX11-065` Clause 0 — RESOLVED 2026-05-23: hand-or-digivolution-source union-zone cost selector.
- `P-130` [On Play] — RESOLVED 2026-05-23: `move_from_breeding` DSL step with optional level-filtered prompt.
- `BT20-055` — face-up security lifecycle and security end-of-opponent-turn play timing (carried over).
- All other Rocks AUDIT-mode cards re-verified green under cargo (36 of 36 build + tests pass, prior PARTIAL/BLOCKED verdicts unchanged — no engine-gap closures since 2026-05-04 that lift these).

### New DSL-vocab gaps filed
- `G-DSL-AURA-TARGET-SOURCE-PERMANENT` — carrier-trait condition on inherited aura target filter (BT21-021 [Your Turn] Rush).
- `G-MOVE-BREEDING-DSL` — RESOLVED 2026-05-23; archived in `qa/resolved-gaps.md`.
- `G-DSL-SELECT-OWN-SOURCES-FILTER` / hand-or-source union cost — RESOLVED 2026-05-23; archived in `qa/resolved-gaps.md`.

### New pattern worth documenting in RUST_DSL_TEST_API.md
- **Single-trigger optional auto-fire vs declinable cost-gating.** When a clause is `optional: true` and the body has a synchronous step BEFORE any `PendingSelection`, that step (e.g. `suspend`) runs unconditionally — the optional flag does not gate it. To make a "by suspending this Tamer, you may …" body actually declinable, the FIRST step must install a selection with `optional: true`, and cost-paying steps must live in that selection's callback. EX11-065's Batch 10 fix adopted this pattern (`select_own_permanent: { optional: true }` first → suspend + place in the accept callback).

---

## Phase A completion pass (2026-05-22) — OpenSpec change `complete-rocks-archetype`

Calibration spike found the verdict tracker badly stale: the 47-card pool ran
`cargo test` at 239 passed / 0 failed / 9 ignored, while the tracker claimed
2 BLOCKED + 30 PARTIAL. Phase A re-audited the 26 PARTIAL cards in 5 batches of
~6 parallel agents and authored every omitted clause whose substrate had landed.

Result: Rocks pool **15/30/2 → 37 IMPLEMENTED / 9 PARTIAL / 1 BLOCKED**.
Full `cards_behavioral`: 3104 passed / 0 failed / 192 ignored. All 39 engine
test binaries green.

Newly IMPLEMENTED (23): BT21-021, BT21-055, EX10-025, EX10-028, EX10-032,
EX10-033, EX10-036, EX10-063, EX10-069, EX7-049, EX8-048, EX8-055, EX8-067,
LM-031, LM-032, P-039, P-107, P-169, P-186, P-215, ST22-11, BT4-072, P-206.
Reclassified already-done (5): EX10-003, EX7-074, BT9-103, EX8-047, P-167.

### Incident note — parallel-agent git corruption

Phase A agents ran without worktree isolation in a shared tree; several invoked
`git stash` to probe pre-existing failures and buried other agents' uncommitted
work in 3 overlapping stashes. Test files for EX10-025/028/032/063 and P-186
were lost and re-authored from the (recovered) YAML. Lesson: card-authoring
agents must run worktree-isolated, or be explicitly forbidden from any `git`
command. The 8 re-dispatch agents in the recovery wave were run with an explicit
no-git rule — no further loss.

### Remaining 10 cards — genuine substrate gaps (Phase B follow-up)

| Card | Gap | Slice |
|---|---|---|
| P-130 | RESOLVED 2026-05-23: `move_from_breeding` DSL step with optional level-filtered prompt | B2 |
| EX11-065 | RESOLVED 2026-05-23: hand∪source union-zone cost selector | B3 |
| EX11-038 | RESOLVED 2026-05-23: hand∪source union cost Draw clause | B3 |
| BT20-055 | RESOLVED 2026-05-23: face-up security lifecycle (flip + checks-face-up observer) | B4 |
| BT23-096 | RESOLVED 2026-05-22, verified 2026-05-23: `G-DSL-DELAY-ON-ATTACK-EVENT` | B5 |
| BT8-094 | RESOLVED 2026-05-23: `event_target_level_eq/lte/gte` predicates + cross-player OnMove observer | NEW |
| BT23-059 | RESOLVED 2026-05-23: `when: on_option_trashed` + unsuspend/immunity Clause B | NEW |
| EX10-034 | RESOLVED 2026-05-23: `grant_triggered_effect.target` accepts a selected binding; Clause A authored and verified | NEW |
| EX11-044 | RESOLVED 2026-05-23: `highest_play_cost` selector + `event_host_permanent_is_source` predicate; Clause A and Clause B authored | NEW |
| EX8-050 | RESOLVED 2026-05-23: `play_from_revealed_free` sub-step; On Deletion now plays selected revealed Mineral/Rock cost<=5 Digimon free and trashes rest | NEW |

Phase A surfaced 5 new small DSL gaps not in the original B1–B5 scope. B1
(carrier-trait predicate) collapsed — `source_permanent_trait_has` already
existed, so BT21-021 was completed as a pure authoring fix.

## Final Rocks Verdict Table (2026-05-23)

All 47 resolved Rocks pool cards are verified `IMPLEMENTED` in
`qa/qa-reports/validated_cards_dsl.json`.

| Card | Name | Verdict | Tests | Test file |
|---|---|---:|---:|---|
| BT14-009 | Gotsumon | IMPLEMENTED | 2 | `code/digimon-engine/tests/cards_behavioral/bt14/bt14_009.rs` |
| BT16-082 | Ukkomon | IMPLEMENTED | 19 | `code/digimon-engine/tests/cards_behavioral/bt16/bt16_082.rs` |
| BT18-064 | Mercurymon | IMPLEMENTED | 3 | `code/digimon-engine/tests/cards_behavioral/bt18/bt18_064.rs` |
| BT20-055 | Invisimon | IMPLEMENTED | 3 | `code/digimon-engine/tests/cards_behavioral/bt20/bt20_055.rs` |
| BT21-021 | OmniShoutmon | IMPLEMENTED | 25 | `code/digimon-engine/tests/cards_behavioral/bt21/bt21_021.rs` |
| BT21-055 | Bombermon | IMPLEMENTED | 6 | `code/digimon-engine/tests/cards_behavioral/bt21/bt21_055.rs` |
| BT23-059 | Justimon: Blitz Arm | IMPLEMENTED | 13 | `code/digimon-engine/tests/cards_behavioral/bt23/bt23_059.rs` |
| BT23-096 | Comet Hammer | IMPLEMENTED | 14 | `code/digimon-engine/tests/cards_behavioral/bt23/bt23_096.rs` |
| BT4-072 | Gogmamon | IMPLEMENTED | 5 | `code/digimon-engine/tests/cards_behavioral/bt4/bt4_072.rs` |
| BT8-094 | Digimon Emperor | IMPLEMENTED | 10 | `code/digimon-engine/tests/cards_behavioral/bt8/bt8_094.rs` |
| BT9-103 | Kongou | IMPLEMENTED | 3 | `code/digimon-engine/tests/cards_behavioral/bt9/bt9_103.rs` |
| EX10-003 | Tumblemon | IMPLEMENTED | 1 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_003.rs` |
| EX10-025 | KoDokugumon | IMPLEMENTED | 20 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_025.rs` |
| EX10-028 | Golemon | IMPLEMENTED | 27 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_028.rs` |
| EX10-032 | Proganomon | IMPLEMENTED | 27 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_032.rs` |
| EX10-033 | Pyramidimon | IMPLEMENTED | 12 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_033.rs` |
| EX10-034 | Blastmon | IMPLEMENTED | 14 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_034.rs` |
| EX10-036 | Magneticdramon | IMPLEMENTED | 19 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_036.rs` |
| EX10-063 | Close | IMPLEMENTED | 14 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_063.rs` |
| EX10-069 | Unique Emblem: Gravel Hearts | IMPLEMENTED | 9 | `code/digimon-engine/tests/cards_behavioral/ex10/ex10_069.rs` |
| EX11-038 | Sunarizamon | IMPLEMENTED | 6 | `code/digimon-engine/tests/cards_behavioral/ex11/ex11_038.rs` |
| EX11-044 | Pyramidimon | IMPLEMENTED | 11 | `code/digimon-engine/tests/cards_behavioral/ex11/ex11_044.rs` |
| EX11-065 | Close | IMPLEMENTED | 18 | `code/digimon-engine/tests/cards_behavioral/ex11/ex11_065.rs` |
| EX7-049 | Metallicdramon | IMPLEMENTED | 16 | `code/digimon-engine/tests/cards_behavioral/ex7/ex7_049.rs` |
| EX7-074 | Vortex Resonance | IMPLEMENTED | 27 | `code/digimon-engine/tests/cards_behavioral/ex7/ex7_074.rs` |
| EX8-005 | Sakuttomon | IMPLEMENTED | 2 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_005.rs` |
| EX8-046 | Golemon | IMPLEMENTED | 3 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_046.rs` |
| EX8-047 | Sunarizamon | IMPLEMENTED | 4 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_047.rs` |
| EX8-048 | Landramon | IMPLEMENTED | 15 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_048.rs` |
| EX8-050 | Gogmamon | IMPLEMENTED | 17 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_050.rs` |
| EX8-051 | Proganomon | IMPLEMENTED | 2 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_051.rs` |
| EX8-055 | Pyramidimon | IMPLEMENTED | 19 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_055.rs` |
| EX8-067 | Close | IMPLEMENTED | 19 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_067.rs` |
| EX8-070 | Zofr Kabus | IMPLEMENTED | 22 | `code/digimon-engine/tests/cards_behavioral/ex8/ex8_070.rs` |
| LM-031 | Black Scramble | IMPLEMENTED | 16 | `code/digimon-engine/tests/cards_behavioral/lm/lm_031.rs` |
| LM-032 | Purple Scramble | IMPLEMENTED | 21 | `code/digimon-engine/tests/cards_behavioral/lm/lm_032.rs` |
| P-039 | Black Memory Boost | IMPLEMENTED | 6 | `code/digimon-engine/tests/cards_behavioral/p/p_039.rs` |
| P-107 | Defense Training | IMPLEMENTED | 12 | `code/digimon-engine/tests/cards_behavioral/p/p_107.rs` |
| P-123 | Ukkomon | IMPLEMENTED | 15 | `code/digimon-engine/tests/cards_behavioral/p/p_123.rs` |
| P-130 | Lui Ohwada | IMPLEMENTED | 14 | `code/digimon-engine/tests/cards_behavioral/p/p_130.rs` |
| P-167 | Landramon | IMPLEMENTED | 4 | `code/digimon-engine/tests/cards_behavioral/p/p_167.rs` |
| P-169 | Close | IMPLEMENTED | 15 | `code/digimon-engine/tests/cards_behavioral/p/p_169.rs` |
| P-186 | Gallantmon | IMPLEMENTED | 26 | `code/digimon-engine/tests/cards_behavioral/p/p_186.rs` |
| P-206 | Digital Gate Open | IMPLEMENTED | 32 | `code/digimon-engine/tests/cards_behavioral/p/p_206.rs` |
| P-215 | Icemon | IMPLEMENTED | 18 | `code/digimon-engine/tests/cards_behavioral/p/p_215.rs` |
| ST13-08 | Chikurimon | IMPLEMENTED | 2 | `code/digimon-engine/tests/cards_behavioral/st13/st13_08.rs` |
| ST22-11 | Defense Plug-In F | IMPLEMENTED | 14 | `code/digimon-engine/tests/cards_behavioral/st22/st22_11.rs` |
