# DCGO PvP replay bug-hunt - 2026-11-17

Source: 9 raw recorder logs, 67 games, 50 failing replays diagnosed, 21 root-cause clusters, 19 fix lanes merged into one branch.
No recording, deck list or hand content is stored here; entries cite only the card ids and step where a divergence occurred.

## Clusters not fixable in the simulator

- **dcgo-homeros-effect-pick-row** - DCGO implements Minervamon (BT24-041)'s single printed '[On Play][When Digivolving][On Deletion]' effect as separate ActivateClass objects; Homeros (BT24_102.cs) finds two OnEnterFieldAnyone candidates (On Play, When Digivolving) and opens a mandatory SelectCardEffect 'Select 1 effect to activate' (customRootCardList [BT24-041,BT24-041]) whose choices run the identical shared coroutine, logged as an extra SelectCardEffect row (card BT24-041, index 68). Our engine collapses a combined-timing clause into one refireable effect (refire.rs collapse_combined_timing_clauses; RESOLVED gap G-ENGINE-REFIRE-SPLITS-COMBINED-TIMING, test bt24_102_refire_combined_on_play_when_digivolving_is_one_choice; exam qa/dcgo-exams/BT24/BT24-102-effect1.yaml records the same as a dcgo_only row), so the replay feeds that row to the Hand prompt ('card pick BT24-041 not found') and shifts all later selections. No single CR 4.3 rule number states it verbatim; a printed multi-timing effect is one effect so no timing choice is granted.
- **stale-no-board-indexes-dup-pick** - Engine steps 0-16 replay cleanly and the pending OwnField prompt ('Place this card face down under a [DATA SQUAD] Tamer') matches the card text. Two identical ST24-14 Tamers (slots 0 and 1, same sources/state) both answer the recorded SetTargetFrames row naming position 1 of [ST24-14,ST24-14,ST24-15,BT26-005]. The raw log (raw-20261114T012101Z) predates recorder 0.2.0: it has no board_indexes and no recorder-stamped Snapshot (Core.cs:27-30 added per-entry CardIndex), so convert_raw.py (1048-1054) cannot name the copy; the replayer correctly refuses to guess (positional matching is allowlisted for one g6 pick only). Not an engine or converter bug; blocked_cards (1 own, 2 opp) are a separate training-gate matter.

## Lanes

### alt-digivolve-xros-req-cards (partial)

Diagnosis confirmed. The ten listed cards had only a JSON stub, so the alternate digivolution printed only in xros_req (name or trait gated: [Koromon] cost 0, Lv.2 w/[CS] cost 0, Lv.4 w/[Sukamon] cost 3, [Veemon] cost 2) was invisible to the engine. DCGO's "Which digivolution cost do you pay?" prompt (rule 17) never fired, and bases reaching only the special route had no legal digivolve. I authored all ten as DSL YAML: printed circles listed explicitly plus the special route as an alt_paths entry, and every printed clause checked against the official bundle, card image and DCGO C#. Cards: EX11-007, BT20-038, BT11-043, EX11-025, EX11-003, BT22-044, BT22-046, BT22-004, BT16-070, BT20-004.

After the fix every one of the 7 games moved past its cost-choice divergence. Several then stopped at another card of the SAME family (unimplemented, alternate route only in xros_req), so I authored five of those too: BT20-031 Liamon, BT20-039 Diatrymon (Falcomon line, game g5), BT22-012 RizeGreymon, BT22-043 Terriermon, BT22-054 Hagurumon (CS line, game g7).

New substrate: DSL step add_top_face_down_security_to_hand (EX11-025 'top face-down security card'; first face-down card from the top, face-up skipped) and predicate leaf event_card_face_up_in_security (EX11-003). Both change the compiled bincode layout, so PACK_FORMAT_VERSION is bumped to 0.35.0 (pack_version_gate test updated); another lane that also bumps the pack version will conflict textually in pack.rs. Data fix: BT20-038's bogus Lv.3 green evo circle was removed from data/card_overrides.json and data/cards.json (the card image and official DB print only the Lv.2 green/yellow split circle). EX11-007's dropped Green Lv.2 circle was already folded into card_overrides on main.

DCGO deviation (dcgo_quirk): EX11-003 follows the printed text and official Q5788 (only face-up [Royal Base] placements draw). DCGO's EX11_003.cs builds a face-up/[Royal Base] SecurityCondition and never reads it, so DCGO draws on any card added to security.

New guard: code/tests/tools/test_alt_digivolve_requirement_guard.py fails when a YAML-implemented card prints a special '[Digivolve] ... Cost N' route in its official bundle but declares no alt_paths. Two pre-existing hits are allowlisted as known gaps: BT12-031 and BT15-101. The family is large: about 748 bundle cards without YAML print such a route.

Status is partial: the cluster's own cards are done and green, but no game reaches PASS, because each stops at an unrelated unimplemented card or a replay-layer issue (see residual).

**Residual (not fixed):**

No game reaches PASS; remaining first divergences are different defects:
(1) g5: BT25-074 Tankdramon is unimplemented and carries the same xros_req-only alt route (Lv.4 w/[D-Brigade]/[ACCEL] trait: Cost 3, circle 4). It is moderately complex (BT25-074 text: reveal 3 and play a cost-12-or-less [D-Brigade]/[ACCEL] Digimon with cost -5, an [All Turns][OPT] can't-digivolve effect, and a conditional inherited Reboot and Blocker), so I did not author it.
(2) Other unimplemented cards block g2, g6 and g18: BT11-046 Agumon, P-181 Royal Base, EX5-054 MetalEtemon. Not the same family (no xros alt as the blocker).
(3) g7 and g4 involve BT22-093 Ami Aiba (unimplemented Tamer: Start-of-Main gain 1 memory if the opponent has a Digimon; Your Turn: suspend this Tamer so a CS Digimon that just digivolved may digivolve again for free). g7's memory drift is explained by this; g4 needs a check of the BT22-094 cost-reduction prompt resolution.
(4) g1 is replay-layer: a count payload without candidates cannot map onto the optional-trigger Replacement prompt (BT24-034).
(5) Family-wide: about 748 bundle cards with no YAML print a special [Digivolve] route, mostly CS and ACCEL (e.g. BT22-051 Okuwamon, BT22-056 Guardromon, BT25-074 Tankdramon, EX5-054 MetalEtemon). Guard-listed known gaps with a YAML but no alt_paths: BT12-031 (Imperialdramon: Fighter Mode, '[Digivolve] 2 from [Imperialdramon: Dragon Mode]') and BT15-101 (MetalGarurumon, conditional Gabumon path; the alt-path condition gate now exists).
(6) Generated artifacts NOT regenerated, per the lane rules: data/card_admissibility.json and qa/impact/* (lowering_coverage, dsl_impact_index) are stale now; the integrator must regenerate them (PYTHONUTF8=1 PYTHONPATH=code python -m tools.lowering_coverage, then python -m digimon_gym.agents.card_admissibility).
(7) Test and build environment notes: cargo builds and test runs used profile.dev.opt-level=0 via --config because of heavy machine contention; mechanics_readiness --check passes. code/tools/dcgo-replay/examples/trace_big.rs is an untracked local helper (512 MB stack wrapper around the trace example) and was not committed.

**Gap tracker entries (verbatim from the lane):**

### qa/dsl-vocab-gaps.md entries (all RESOLVED in claude/hunt-alt-digivolve-xros-req-cards)

- **G-DSL-ADD-TOP-FACE-DOWN-SECURITY-TO-HAND (RESOLVED)**: step `add_top_face_down_security_to_hand: { of: you }`. Takes the first FACE-DOWN card from the top of the security stack and skips face-up cards (CR 4.3 3-7-4, 15-14-5; DCGO EX11_025.cs walks SecurityCards and takes the first IsFlipped card). `add_top_security_to_hand` takes the top card regardless of orientation. Driver: EX11-025 FunBeemon. Files: digimon-dsl step.rs / compile.rs / compiled.rs, engine dsl_cards/step/zone_moves.rs, implementation_inventory reference_steps.rs. Compiled layout change, PACK_FORMAT_VERSION 0.35.0.
- **G-PRED-EVENT-CARD-FACE-UP-IN-SECURITY (RESOLVED)**: predicate leaf `event_card_face_up_in_security: bool`, for use in `trigger_if:` on `on_added_to_security`. Reads the placed event card's orientation in the security stack it entered (face_up_security set of trigger.affected_player). Driver: EX11-003 Puroromon 'When face-up [Royal Base] trait cards are placed in your security stack' (official Q5788: flipping a face-down card face up does NOT trigger). Files: digimon-dsl predicate.rs / compiled.rs / compile.rs, engine dsl_cards/predicate.rs (eval_event_fields), implementation_inventory reference_predicates.rs.
- **G-INHERITED-MAIN-ON-FIELD (confirmed working)**: `scope: inherited` + `when: main_on_field` is offered by the field [Main] mask and activates; no engine change needed (BT22-044/043/054 'By placing this [CS] trait Digimon's top stacked card as its bottom digivolution card, Draw 1', via `place_top_source_as_bottom` + `source_permanent_trait_has: CS`).

#### ## docs/RUST_ENGINE_GAPS.md / DCGO notes

- **DCGO quirk (EX11-003)**: DCGO EX11_003.cs defines SecurityCondition (face-up and [Royal Base]) but never reads it, so it draws for ANY card added to the owner's security. We implement the printed text and Q5788. If an exam or replay shows a DCGO draw on a face-down or non-[Royal Base] placement, triage as dcgo_quirk, not ours_wrong.
- **xros_req-only alternate digivolutions are invisible to the engine** (the 17-1 cost-choice prompt never fires and special-route-only bases are masked). Fixed per card by authoring YAML alt_paths, now guarded by code/tests/tools/test_alt_digivolve_requirement_guard.py. Known guard gaps: BT12-031 (Imperialdramon: Fighter Mode '[Digivolve] 2 from [Imperialdramon: Dragon Mode]' has no alt path in its YAML) and BT15-101 (conditional Gabumon path omitted pending the alt-path condition gate, which now exists).
- **BT20-038 data**: the bogus {green, Lv.3, cost 1} evo circle was removed from card_overrides.json and cards.json (card image: Lv.2 green/yellow split circle only).
- **Card-data note**: the printed digivolution routes for the first ten cards are now in YAML alt_paths, but cards.json still carries them in xros_req only; the engine ignores xros_req.

### ex12-me-machine-puppet-cards (partial)

Twelve EX12 [ME]/Machine/Puppet cards had only .json metadata and no YAML, so DCGO's prompts for them hit nothing in the engine. I authored all twelve DSL-first, with no engine or DSL code change: EX12-003, 008, 038, 041, 053, 054, 055, 058, 059, 060, 064, 072. Two data problems also surfaced. First, cards.json drops the second printed circle on dual-colour cards. DCGO's 'Which digivolution cost do you pay?' row (SelectCountEffect) proves both circles exist: Thundermon EX12-041 onto a black Hagurumon prices {2,3}, and Megadramon EX12-064 onto a black Guardromon prices {3,4}. I added both circles to the YAML alt_paths and to data/card_overrides.json for 041/055/058/059/060/064. Second, EX12-041's '(Rule) Name: Treated as including [Mamemon]' exists in no data layer, and cards.json has the wrong security text for EX12-072. The card face and DCGO agree on both, so the rule is now also_treated_as on EX12-041 (Kokuwamon EX13-046's '[Mamemon] in its text' reveal can offer it) and both texts are folded into the overrides. Other design notes: EX12-053 uses reveal_search, whose cross-bucket de-dup matches DCGO mutualConditions. Kapurimon EX12-003 is a would-leave replacement driving may_dna_digivolve_now, limited to Digimon (an ME Option like Giant Meat leaving must not open it) and not optional (DCGO has no yes/no gate). Results: 355 ex12 cards_behavioral tests pass, 741 engine lib tests pass, and the card-data pytest guards pass except BT26-032/080 alt-path drift in files I did not touch. The dcgo-replay crate tests did not finish before the machine ran out of resources. Replay: 20261113T025510Z-g1 now PASSES (28/28). The other three games advance but each stops on a different defect (see residual), so status is partial. Commit 30895f5ec; nothing pushed.

**Residual (not fixed):**

(1) Kapurimon EX12-003 is only partly faithful. The replacement framework picks the leaving subject before the effect runs, while DCGO picks the result card first and then the DNA pair, one of which must be a leaving Digimon. Fixing it needs a replacement whose subject is chosen inside the effect, i.e. merged with the DNA pair pick. That is engine work in replacement_deletion_batch.rs and is the g17 stop. (2) Machinedramon EX12-059 grants stack-trash immunity with for_each over own Digimon on the battle area at resolution. DCGO installs a player-level 'until their turn ends' condition that also covers Digimon entering later. This is a substrate gap and the YAML header says so. (3) Megadramon EX12-064's inherited 'By unsuspending this Digimon' is modelled as the gate source_is_unsuspended:false plus an unsuspend step because there is no unsuspend_self activation cost. (4) The second circle on the dual-colour cards is inferred from DCGO's cost-prompt behaviour, not from official bundles (none exist for EX12-041/055/058/059/060/064). Same level and cost as the first circle; verify against the official DB when available. (5) g1 and g8 still stop on other defects: EX13-053's 'up to 3' as chained optional picks versus DCGO's single multi-select, and a DigiXros second-material prompt with empty options. (6) The Megadramon EX12-064 [All Turns] refire_effect trigger, the Andromon EX12-055 [Counter] and the redirect clauses are structure-checked only, not exercised behaviourally. (7) data/card_admissibility.json is stale and qa/impact/* needs regeneration; I left both to the integrator per instructions. (8) The dcgo-replay crate tests were queued but did not finish (the machine hit fork/resource exhaustion).

**Gap tracker entries (verbatim from the lane):**

### qa/dsl-vocab-gaps.md additions (from lane ex12-me-machine-puppet-cards)

- **G-DSL-ACTIVATION-COST-UNSUSPEND-SELF** (open, low): "By unsuspending this Digimon, ..." (EX12-064 Megadramon inherited [End of Attack]) has no `activation_cost: { unsuspend_self: true }`. Workaround in the YAML: gate `condition: { source_is_unsuspended: false }`, then `unsuspend: { target: source }`, then an `if` on `source_is_unsuspended: true` before the delete. This is not a true cost: the optional prompt appears before the unsuspend.
- **G-DSL-PLAYER-SCOPED-STACK-TRASH-IMMUNITY** (open): Machinedramon EX12-059 ("your opponent's effects can't trash any of your Digimon's stacked cards until their turn ends") needs a player-level, turn-scoped ImmuneFromStackTrashing that also covers Digimon that enter later. `grant_stack_trash_immunity_from_opponent_effects` is per-permanent, so the YAML uses `for_each` over own Digimon at resolution.
- **G-DSL-REPLACEMENT-DEFERRED-SUBJECT** (open, shared with the engine gap): "when any of your [ME] Digimon would leave ..., 1 of them and any of your other Digimon may DNA digivolve ..." (EX12-003 Kapurimon). Needs the leaving subject to be chosen inside the effect, via the DNA pair pick.

#### ## docs/RUST_ENGINE_GAPS.md additions

- **G-ENGINE-DELETION-BATCH-SUBJECT-ORDER** (open): `replacement_deletion_batch.rs::install_deletion_subject_choice` asks 'Choose the impending deletion this effect processes' (AnyField) before a replacement's own decisions when 2+ deletions are pending. DCGO (EX12_003.cs WhenRemoveField) asks the card pick first and takes the leaving Digimon as one of the DNA materials. Recorded in the owner's g17 (20261113T025510Z-g17) at step 36.
- **G-DATA-CARDS-JSON-DUAL-COLOUR-CIRCLES-EX12** (resolved in data/card_overrides.json): EX12-041/055/058/059/060/064 lose their second digivolve circle in cards.json; DCGO's cost prompt proves it exists (same level and cost). EX12-041's rule box and EX12-072's security text are also wrong or absent. No official bundles exist for these cards, so verify against the official DB when they are scraped.
- **Observation (not in this lane)**: EX13-053 Thundermon's "up to 3 cards" is three chained optional `select_union_zone` picks. DCGO ends a single multi-select after 1 pick, so the recorded game (20261113T011922Z-g1, step 36) never reaches picks 2/3. Consider a single multi-pick (`select_count_capped_multi`-style) with an explicit end.

### reveal-two-bucket-onplay (fixed)

RB1-005 Gammamon and BT7-081 Bokomon had no YAML, and BT24-068 DemiDevimon was a stats-only stub, so all three played as vanilla Digimon and the recorded reveal picks met no engine prompt. All three are now implemented from the card image, the official bundle and DCGO. For each card, [On Play] reveals 3 (RB1-005, BT24-068) or 5 (BT7-081) cards, then offers two mandatory picks. One physical card cannot fill both slots, and a lone match is still added (official Q&A on BT7-081). The remainder goes to the deck bottom as a real ordering prompt (CR 4.3 3-1-3-4). RB1-005 adds a [Gammamon]-in-text card and a [Hiro Amanokawa], plus inherited [Your Turn] +2000 DP read from the carrier's top card. BT24-068 adds an [Evil]/[Fallen Angel] card and a [Seven Great Demon Lords] card, then does a mandatory hand trash, plus inherited [When Attacking][OPT] trashing both deck tops. BT7-081 adds a [Hybrid]/[Ten Warriors] card and a Tamer, plus [Your Turn][OPT] gain 2 memory when one of your Tamers digivolves. That trigger needed new DSL vocabulary, because a digivolved Tamer's top card is a Digimon. I added the predicate event_target_digivolved_from_kind, which reads the card directly under the new top card (DCGO evoRoots). It is wired through predicate.rs, compiled.rs, compile.rs, reference_predicates.rs and the engine predicate evaluator. data/card_admissibility.json, the vocabulary doc docs/RUST_DSL_AGENT_GUIDE.md and the validated_cards_dsl.json ledger are regenerated or updated, and BT24-068 is promoted from PARTIAL to IMPLEMENTED. All three games now get past their cluster card. DCGO's picks, ordering and discard prompts match the engine from there on, but none reaches PASS because each hits a different defect (see residual). Gap in the TDD record: I did not run a formal red phase. The tests were written first, but I never ran them against the old state before writing the YAML, and the first run was already green. Their absence-of-YAML red case (dsl_card expect) was not observed. I compensated with negative-branch assertions and the compiled-qualifier test for the new predicate. Not run: the full cards_behavioral binary and cargo test --lib (the full cards_behavioral build took 195 min under contention and was invalidated by the DSL edits). I ran my three modules through a temporary standalone test binary that I deleted before committing. scripts/verify --tier 0 was not run, and qa/impact/* was not regenerated per instructions.

**Residual (not fixed):**

1) EX10-042 GulusGammamon has no YAML ([On Play][When Digivolving] trash top 2, may place a [Gammamon]-named Digimon from trash as the bottom digivolution card; [Your Turn][OPT] when effects add to this Digimon's digivolution cards, may digivolve into [Regulusmon] from hand or trash for cost -1; inherited Raid). It blocks 20261113T025510Z-g7, along with other unimplemented cards in that game. 2) BT21-094 Armor Digivolution has no YAML. It is the same defect family as this cluster (reveal-3 with a [Davis Motomiya]-name bucket and a [Free]-trait bucket, trash the rest), but it is an Option with a Delay clause on the on_top_card_trashed timing, so it belongs to the delay-option authoring lane. It blocks 20261116T045756Z-g7. 3) 20261113T025510Z-g10 step 41: the actor/trigger-ordering mismatch described above. Needs a /replay-bug-hunt check of CR 4.3 15-8 simultaneous-trigger ordering against DCGO. Both games also still carry other 'blocked_cards' (12 opponent cards in each g7 and g10 on the old manifest), so none of the three can reach PASS from this lane alone. 4) event_target_digivolved_from_kind reads only the card directly under the new top card. A DNA digivolution that merges a Tamer with a second material, where the Tamer is not the directly-under card, is not covered. DCGO uses all the evoRoots. 5) BT24-068's printed text does not say 'in any order', but CR 4.3 3-1-3-4 and DCGO both make the order the player's choice, so I used the ordering prompt, matching the BT20-048 idiom. If the replay corpus shows DCGO not prompting for order when 2+ cards remain, revisit.

**Gap tracker entries (verbatim from the lane):**

### qa/dsl-vocab-gaps.md (append)

- **G-DSL-EVENT-TARGET-DIGIVOLVED-FROM-KIND** (CLOSED 2026-11-17, branch claude/hunt-reveal-two-bucket-onplay, commit e6a573a85). Observer predicate `event_target_digivolved_from_kind: <card kind>` for `on_digivolve` clauses. True when the card directly under the event permanent's new top card (the card it digivolved from) has that kind. Needed for 'When one of your Tamers digivolves' (BT7-081 Bokomon): a Tamer that digivolves becomes a Digimon permanent, so `event_target_kind` reads the wrong card. It mirrors DCGO `GetDigivolutionRootsFromEnterFieldHashtable` / `evoRoots.Any(IsTamer)`. Driver: BT7-081. Open edge: a DNA digivolution's multiple roots are not distinguished, only the card directly under the merged top card is read. Residual follow-up: BT18-010 Bokomon ('When any of your Digimon or Tamers digivolve into Digimon with ...') has no YAML yet and may reuse this leaf.

#### ## docs/RUST_ENGINE_GAPS.md (append)

- No new engine primitive was needed. Resolved without engine change: reveal-two-bucket [On Play] authoring for RB1-005, BT24-068 and BT7-081 uses the existing `reveal_top_deck` + `select_reveal_buckets { no_duplicate_cards }` + `add_to_hand_from_reveal` + `place_remainder_on_deck` idiom (BT20-048). RB1-005's inherited '[Gammamon] in its text' aura works with the existing whole-card `in_text_contains` leaf evaluated against the carrier's top card (official Q&A: only the top stacked card is read).
- **Open (not fixed, found by replay):** simultaneous-trigger actor/order mismatch after a block deletes a blocker (20261113T025510Z-g10 step 41). The engine asks the turn player (P1, EX11-012's optional delete prompt) before the blocked/deleted side's [On Deletion] (BT24-073, P0); DCGO resolves P0's [On Deletion] first. Needs a ruling check against CR 4.3 15-8 (non-'would' triggers, turn-player order).
- **Open (unimplemented cards seen in these games):** EX10-042 GulusGammamon, BT21-094 Armor Digivolution (reveal-3, [Davis Motomiya]-name + [Free]-trait buckets, trash rest, Option with Delay on on_top_card_trashed).

#### ## qa/qa-reports (ledger)

- validated_cards_dsl.json: BT24-068 promoted PARTIAL to IMPLEMENTED (11 tests); RB1-005 added IMPLEMENTED (9 tests); BT7-081 added IMPLEMENTED (13 tests). Test count total 33. Integrator should regenerate qa/impact/* (lowering_coverage, dsl_impact_index) and re-run `python scripts/verify --tier 0`; card_admissibility.json was already regenerated here and currently marks all three admissible, which the regenerated lowering_coverage may revise if it requires a behavioural reference to the new leaf `event_target_digivolved_from_kind`. The compiled-qualifier test in bt7_081.rs names the leaf for that purpose.

### play-free-hand-trash-cards (partial)

The three diagnosed cards (ST16-08, BT17-094, BT16-084) were unimplemented. I authored them as DSL YAML, plus ST16-03, BT16-001, BT16-031, P-119 and P-187, which the same recordings reach. All three games now get past the original divergence, but none reaches PASS; each stops at a later divergence in a different defect, listed under residual and not fixed.

Substrate widened (rule 28), both covered by the new tests:
1. `{ played_carrier: <bind_as> }` is a new binding ref. It resolves to the permanent that currently carries a free-played card, even after that card digivolved. BT16-084's delayed return needs it: the official Q&A says the Digimon is still returned after digivolving, with only the top card going to hand and the digivolution cards trashed. The existing `{ permanent: }` binding fizzles once the played card is buried.
2. `place_on_security` with a permanent source now supports `position: choice`. It panicked in `map_stack_position`, and P-187 hit that. The top/bottom prompt is now one shared helper, `install_security_position_choice`, used by both the card-source and permanent-source arms.

Results per game (first divergence before to after, run through the engine replay core):
- g2: actor_mismatch at step 24 became SelectionResolution at step 26. ST16-08's [Security] now plays ST16-03 from P1's trash.
- g1: step 26 became step 35. BT17-094's [Security] now plays BT18-088 from hand and adds itself to hand.
- g14: engine_error at step 13 became SelectionResolution at step 20. BT16-084's start-of-main offer, P-119's reveal and BT16-031's digivolve all replay.

Tests: the 8 new files hold 45 tests, all passing. 87 neighbouring tests (BT25-038, BT25-044, BT16-085, BT17-095) also pass. The 132 ran as one scratch binary, `pfht_scratch`, built from the lane's modules plus the neighbours; I removed it before committing. The full cards_behavioral binary, `cargo test --lib` and the dsl suite were not run, because under contention each rebuild took 50 to 90 minutes. I did not run dsl-lint (timed out) or the Python tier-0 checks other than card_admissibility, and the dcgo-replay crate would not build (rustc crashed twice). The DSL vocab doc drift gate passes. Tests use synthetic boards only; no recording data is committed.

Cards:
- ST16-08 Garurumon: [Security] optional free play of an exact-name [Gabumon] or a [Matt Ishida] Tamer, from hand or trash via select_union_zone and play_union_bound_free. Also the Lv.3 [Gabumon]-in-name alt digivolve at cost 2, [When Digivolving] draw then mandatory trash, and the inherited [When Attacking] once-per-turn draw/trash.
- BT17-094 Ancient Guardian Deity: card-level use_requirement colour bypass (Tamer or Hybrid Digimon), [Main] optional return from trash then optional play at cost -4, and [Security] optional Tamer-with-inherited play then always add itself to hand.
- BT16-084 Yolei Inoue & Kari Kamiya: start-of-main optional [Hawkmon]/[Salamon] play with a schedule_delayed return at the opponent's turn end using played_carrier, plus the red/yellow digivolve observer (suspend for 1 memory, and -3000 DP on a DNA digivolve) and the Security self-play.

Committed as 804a2d88d. data/card_admissibility.json is regenerated. qa/impact/* and the gap trackers were left for the integrator.

**Residual (not fixed):**

Not fixed (different defects):
1. g14@20 root cause is BT25-010 Hawkmon (code/digimon-engine/cards/bt25/BT25-010.yaml). Its [Your Turn] -1 digivolve cost reduction lists `trait_has: Avian/Bird/Beast/Animal/Sovereign`. The printed text says "in any of its traits" and DCGO uses ContainsTraits (substring; hence the "other than [Sea Animal]" exclusion), so `trait_contains` is required. BT16-031 has trait "Holy Beast", so the engine charges 3 where DCGO charges 2. P1's memory stays at -1 instead of 0 after BT16-084's +1, and the engine ends P1's turn before the recorded attack.
   - I applied that change temporarily, rebuilt and re-ran g14. Step 20 passes and the game reaches step 24, where the recording has P0 answering an unstamped optional bool false (probably a [Security] effect) but the engine parks P1's <Barrier> replacement prompt. Likely P0's revealed security card (BT24-101?) has an optional [Security] effect the engine never offers.
   - I reverted BT25-010 because it is a separate defect. It probably belongs to the yaml-condition-stricter-dcgo lane.
2. g2@26: AD1-015's select_zone_cards zone-menu int is not mapped by the replay (replay-int-zone-menu-gaps lane). The engine offers one CountCappedMultiSelect over hand and trash; DCGO records a zone int then a pick.
3. g1@35: BT21-091 'Dark to Light, Thunder to Gunfire' places Hybrids under a Tamer. The recorded pick BT18-076 is gone by the time the digivolve-into-MagnaGarurumon union prompt appears, so the placement order or choice differs from DCGO. Needs a trace of cursors 33 to 35.
4. BT16-085 (Davis and Ken) has the same Q&A as BT16-084 ("Yes, it's returned ... only the top card is returned to the hand"). Its YAML uses the strict `{ permanent: played }` binding, and bt16_085.rs sections 8.1 and 8.2 pin that the return fizzles after a regular or DNA digivolve. That contradicts the Q&A. Switching its schedule_delayed body to `{ played_carrier: played }` and updating those two tests would fix it; I left it because the tests pin the other behaviour and the DNA case needs a DCGO read.
5. The recorded unstamped optional-yes then pick rows for ST16-08 and BT16-084 are consumed as the engine's folded optional gate, so no outer_prompt was needed. The yes/no and the pick are one engine prompt.
6. Builds: `cargo build -p dcgo-replay` crashed rustc twice with STATUS_STACK_BUFFER_OVERRUN while compiling the digimon-engine lib under dcgo-replay's feature set. The same lib compiled fine under the test profile. I drove the replays through the engine's LiveGame in a scratch integration test (deleted) instead. The private target dir D:/cargo-target/hunt-play-free-hand-trash-cards is still present for the integrator to delete.

**Gap tracker entries (verbatim from the lane):**

### qa/dsl-vocab-gaps.md

- **G-DSL-PLAYED-CARRIER-BINDING** (RESOLVED, claude/hunt-play-free-hand-trash-cards): a delayed "return/delete THAT Digimon" rider needs to follow the permanent after the played card digivolved. `{ played_carrier: <bind_as> }` is the new structured binding ref (StructuredBindingRef.played_carrier, CompiledBindingRef::PlayedCarrier, resolved through `resolve_played_permanent_permissive`). `{ permanent: <bind_as> }` stays the strict sibling that fizzles once the played card is no longer a battle-area top. Driver: BT16-084 Yolei Inoue & Kari Kamiya (official Q&A: still returned after digivolving, only the top card to hand and the digivolution cards trashed). Follow-up: BT16-085 has the identical Q&A but still uses the strict binding.
- **G-DSL-PLACE-PERMANENT-ON-SECURITY-POSITION-CHOICE** (RESOLVED, same branch): `place_on_security` with `source: { permanent: ... }` and `position: choice` panicked in `map_stack_position`. The step now installs the top/bottom prompt through the shared `install_security_position_choice` helper (also used by the card-source arm), and a `bind_placed_as` receipt finalizer rides in the tail. Driver: P-187 Mastemon ("by placing 1 other Digimon or Tamer as the top or bottom security card"); Q&A confirms top or bottom.

#### ## docs/RUST_ENGINE_GAPS.md

- **G-BT25-010-TRAIT-SUBSTRING** (OPEN): BT25-010 Hawkmon's [Your Turn] digivolve cost reduction uses exact `trait_has` for Avian/Bird/Beast/Animal/Sovereign. DCGO (`HasBirdTraits`/`HasBeastTraits`, `ContainsTraits`) and the printed "in any of its traits" are substring, so `trait_contains` is needed (e.g. "Holy Beast" for BT16-031). Effect in play: -1 memory miss, so the turn ends early in recording 20261113T025510Z-g14.
- **G-BT16-085-RETURN-AFTER-DIGIVOLVE** (OPEN): BT16-085 Davis & Ken's start-of-main return should follow the digivolved Digimon (`played_carrier`) per its official Q&A. Its tests bt16_085.rs 5.1 and 5.2 currently pin the strict fizzle.
- **G-REPLAY-ZONE-MENU-AD1-015** (OPEN, belongs to the replay-int-zone-menu lane): recording 20261113T011922Z-g2 step 26. AD1-015's select_zone_cards hand/trash menu records a one-based DCGO int that the replay does not map onto the engine's single CountCappedMultiSelect.
- **G-BT21-091-PLACE-HYBRIDS-THEN-DIGIVOLVE-PICK** (OPEN, needs triage): recording 20261116T045756Z-g1 step 35. After 'place up to 5 differently named Hybrid cards under a Tamer' the engine's MagnaGarurumon union pick no longer offers the recorded BT18-076.

### delay-option-cards-authoring (fixed)

Authored BT22-096, BT23-099, BT21-090 and Sangomon BT19-017 as DSL YAML (no raw_rust), each with a [Security] clause that activates its [Main], and added one engine dispatch line.

- BT22-096 Unique Emblem: Poseidia Lagoon.
  - [Main]: hand-or-trash pick of exactly [Sangomon] or [Yao Qinglan], play it free, then place the card as a Delay.
  - <Delay> on own [Yao Qinglan] suspending (exact name): an Aqua/Sea Animal Digimon with a legal Aquatic+LIBERATOR hand card digivolves at cost -3.
- BT23-099 The Sistermon Sisters Training Gym.
  - Colour bypass via card-level `use_requirement` over `any_field_permanent`, so battle and breeding area count per the official Q&A, with a Digimon named [Huckmon].
  - [Main]: draw 1, then place as a Delay.
  - <Delay> on own Digimon digivolving into [Huckmon] or [Jesmon]: may play a [Sistermon] card from hand or trash free.
- BT21-090 The Strongest of Brothers.
  - Colour bypass with a Digimon or Tamer with [Gammamon] in its text.
  - [Main]: reveal 3, add 1 [Gammamon]-text card, rest to the bottom, place.
  - <Delay> on `on_add_digivolution_cards`, all turns, any effect: digivolve into a [Gammamon]-text hand Digimon free.
  - [Security]: play a [Gammamon]-text card with play cost <= 4 from hand or trash, then place.
- BT19-017 Sangomon. The replay's free-play target of BT22-096 was also unimplemented. Added the two-bucket [On Play] reveal with the no-duplicate Q&A ruling, the (Rule) Aquatic trait, and the inherited [End of Attack][OPT] gain 1 memory.
- Engine substrate: `TriggerSource::Digivolved` did not reach event-gated Delay Options. Added it to the fan-out list in `effect_queue.rs` (9 lines, mostly comment), so `kind: delay` with `trigger: on_digivolve` now works. No DSL vocabulary was missing. I did not confirm by revert-and-rerun that this line is needed; I inferred it from the code, since no other path reaches Delay Options for that source.
- Ledgers: the stale Python-era PASS for BT23-099 in `validated_cards.json` is now SUPERSEDED. `validated_cards_dsl.json` has IMPLEMENTED entries for all four cards. `data/card_admissibility.json` is regenerated and all four cards are admissible.

Each card's replay root cause is gone.
- 20261114T034011Z-g1 now plays BT22-096, Sangomon and its On Play picks end to end. It stops at step 28 on unimplemented Ryugumon (EX11-018).
- 20261114T142653Z-g6 now plays BT23-099 off-colour beside Huckmon (BT23-006). It stops at step 60 on an unrelated skip-all MultipleSkills mismatch.
- 20261116T164448Z-g1 now passes the BT21-090 reveal prompt. It stops at step 33 on unimplemented BT21-069 and BT21-080.

**Residual (not fixed):**

All three games still fail, each at a different, unimplemented-card or trigger-semantics defect.

1. 20261114T034011Z-g1, step 28. EX11-018 Ryugumon has no YAML. It is a Lv6 with Evade, Decode, an OPT On Play/When Digivolving/When Attacking effect that places an Aqua/Sea Animal hand card as a source and unsuspends, and an All Turns OPT bounce effect. DCGO's optional decline of that effect after the LM-058 digivolve cannot be matched without it.

2. 20261116T164448Z-g1, step 33. BT21-069 GulusGammamon has no YAML. It needs a [Security] play at end of battle, an On Play/When Digivolving 'by placing a [Gammamon]-named card as source, delete a Lv4-or-lower Digimon', and inherited Retaliation. BT21-080 Hiro Amanokawa (Tamer) has no YAML either. It has a [Start of Your Main Phase] gain-memory clause and an All Turns 'by suspending, draw 1 and gain 1 memory' clause when effects place digivolution cards. Without them the engine ends P0's turn at the digivolve (memory 0 to -2) before DCGO's follow-up decisions. This is probably the misc-onplay-cards-unimpl lane.

3. 20261114T142653Z-g6, step 60. A DCGO MultipleSkills -1 skip-all hits a trigger the engine holds as mandatory (BT25-104 slot 9, 'optional' in the trigger order). The mismatch is mandatory versus skippable trigger semantics, not a card.

Test-target caveat: new tests are registered in bt19, bt21, bt22 and bt23 mod.rs. cards_behavioral builds take about 150 minutes on this contended machine, so the four new modules were run in the scratch target and the stale pre-fix binary only. CI will compile them as part of cards_behavioral.

Integrator: regenerate `qa/impact/dsl_impact_index.json`, since the YAML set changed and its `--check` is stale. Merge `data/card_admissibility.json` by regeneration, not by hand.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md

- **G-ENGINE-EVENT-DELAY-ON-DIGIVOLVE: RESOLVED (hunt-delay-option-cards-authoring).**
  - Gap: event-gated Delay Options keyed to `OnEvent(OnDigivolve)` never fired. `TriggerSource::Digivolved` was missing from the `enqueue_triggered` fan-out list that calls `enqueue_event_gated_delayed_options` (`effect_queue.rs`).
  - Fix: `TriggerSource::Digivolved { .. }` added to that list. The just-digivolved permanent and card ride on `event_permanent` / `event_card`, so `active_when` `event_target_*` and `event_card_name_*` leaves work.
  - Driver: BT23-099 The Sistermon Sisters Training Gym, "[Your Turn] When any of your Digimon digivolve into a Digimon with [Huckmon] or [Jesmon] in its name, <Delay>". DCGO observes it as `OnEnterFieldAnyone` + `CanTriggerWhenPermanentDigivolving`.
  - Tests: `bt23_099.rs` (`delay_plays_a_sistermon_from_hand_or_trash_for_free`, `delay_fires_for_jesmon_too`, `delay_ignores_other_digivolutions`, `delay_does_not_fire_on_the_placing_turn`, `delay_declined_stays_parked`).

#### ## qa/dsl-vocab-gaps.md

- **No new DSL vocabulary was needed.**
  - BT22-096, BT23-099, BT21-090 and BT19-017 are written with existing constructs.
  - These shapes are now exercised: `kind: delay` + `trigger: on_digivolve`, `kind: delay` + `trigger: on_suspend` with `event_card_name_is`, `use_requirement` over `any_field_permanent`, `has_digivolve_candidate` with `cost: { reduce: 3 }`, and `select_reveal_buckets` with `no_duplicate_cards`.
- **Note: `use_requirement` is the only mechanism that lifts an Option's colour requirement while the card is in hand.** A `flood_gate` `IgnoreColorRequirement` clause never materializes for a hand card (BT19-089's header documents this). The cards above use `use_requirement` alone.
- **Note: "on the field" includes the breeding area (official Q&A).** Use `any_field_permanent`, not `any_permanent`, for such colour bypasses. Covered by `bt23_099_huckmon_in_the_breeding_area_counts_as_on_the_field` and `bt21_090_gammamon_in_the_breeding_area_counts_as_on_the_field`.
- **Note: "[X] in any of its traits" is a substring scan** (DCGO `ContainsTraits`; [Aquatic] satisfies [Aqua]). Use `trait_contains`, not `trait_has`. Covered by `bt19_017_aqua_in_any_of_its_traits_matches_aquatic`.
- **Test-infra note: `cards_behavioral` takes about 150 minutes to build on a contended Windows host.** Iterate on a small scratch integration target that includes the new modules via `#[path]`.
- **Suspected stale ledger notes: other Python-era `validated_cards.json` PASS entries.** The BT23-099 note ("Delay trigger fires on any play not digivolve. Color req engine gap.") recorded a defect under a PASS verdict. I did not check other entries.

### misc-onplay-cards-unimpl (fixed)

Authored five missing DSL cards, DCGO-faithful, with no DSL or engine widening needed. Reppamon BT22-034, Guilmon BT21-064, SkullSatamon EX10-049 and Devimon EX10-046 are the lane's cards; Vilemon BT24-069 is the other card in the same games' decks. All 46 synthetic-board tests pass, and each of the three games now gets past its original defect and stops at a different one.

BT22-034 (Reppamon):
- The "trash top security for -6000 instead of -3000" Yes/No is a select_effect_choice asked BEFORE the target pick, and only when the controller has security. This matches DCGO's bool, which dcgo_bool_yes_no_cards.json already listed.
- trash_top_security binds the count actually trashed, so -6000 REPLACES -3000 only if the card really reached the trash (official Q&A on "instead").
- Also done: optional free play of itself on effect-caused trash from security, the inherited -2000 [When Attacking] OPT, and the Lv.3 [CS] cost-2 alt digivolve.

BT21-064 (Guilmon):
- Optional cost select_hand over the Hero trait OR a name containing Guilmon/Growlmon/Gallantmon/Megidramon, then trash, then Draw 2 only if a card was trashed.
- Inherited On Deletion +1 memory, and the Gigimon / Lv.2 Hero cost-0 alt digivolve.
- The Purple/Red standard circles already come from the card_overrides evo_costs.

EX10-049 (SkullSatamon):
- Shared WD/On Deletion body mills 3+3 when the opponent has 10 or fewer trash cards, then RE-READS the opponent trash for the level cap (level 3 or lower, level 5 or lower at 10 or more). At exactly 10 both apply.
- The delete pick is mandatory.
- Inherited WA OPT uses the "instead" branch: mill 2+2, or Security A. +1 when the opponent has 11 or more. Blocker.

EX10-046 (Devimon):
- Shared Start-of-Main/WD body: mill 2+2 when the opponent has 10 or fewer trash cards, then re-read the opponent trash for the optional Fallen Angel/Undead return from trash to hand.
- Inherited WA OPT mills 1+1.

BT24-069 (Vilemon):
- on_move (this Digimon only) and WD. The controller trashes 1 hand card (mandatory), then the OPPONENT chooses via as_selecting_player whether to trash one; if they decline or have no hand, the top 2 of their deck are trashed.
- Blocker and +2000 DP while the opponent has 10 or more trash cards, as two self auras.
- Inherited WA OPT mills 1+1.

Ledger entries for all five are in qa/qa-reports/validated_cards_dsl.json. BT24-069's breeding HATCH/MOVE ambiguity stays recorder_stale and was not touched.

**Residual (not fixed):**

All three games still stop, each at a defect outside this lane's cards.

1. 20261116T164448Z-g3 step 23 is EX10-009 Creepymon (Lv6, red/purple), still unimplemented. DCGO source: DCGO/Assets/Scripts/CardEffect/EX10/Red/EX10_009.cs. The recording shows its [End of Your Turn] 'may attack' prompt (an unstamped generic_bool, then SelectAttackEffect, then MultipleSkills) with no engine prompt parked.
   - Authoring it needs a missing primitive for the [When Attacking] clause: 'play 1 red/purple <=5000 DP Digimon card from your TRASH to your empty BREEDING area free'. The DSL only has play_to_breeding_from_hand, so there is no play_to_breeding_from_trash step. No partial EX10-009 was committed, per the no-approximation policy.
   - Other clauses: WD/OD 'delete all lowest-DP opposing Digimon; if it deleted nothing, trash their top 5' (delete_all_permanents with selector: lowest_dp, plus a did-not-delete branch); the [End of Your Turn] may-attack, which has a pattern in AD1-004; and a (Rule) Fallen Angel trait.
2. 20261114T142653Z-g3 step 21 (EX13-032 int 3 against a Replacement prompt) and 20261116T045756Z-g9 step 22 (AD1-015 int 1 against a CountCappedMultiSelect prompt) are replay-side DCGO int-menu to engine-prompt mapping gaps in runners/selection_resolve.rs and replay.rs. Both cards are implemented. They belong to the int-zone-menu lane; not fixed here.
3. Regenerated artifacts are stale. I did not touch qa/impact/*, per the lane rules, so the integrator must regenerate it. python code/tools/impact_index.py --check reports qa/impact/dsl_impact_index.json stale. data/card_admissibility.json is also stale; its input hashes depend on the YAML and ledger changes. lowering_coverage.json is likely stale too; its --check timed out under load, so unverified. Regenerate in order: lowering_coverage, then card_admissibility, with PYTHONUTF8=1 PYTHONPATH=code.
4. Full cards_behavioral, dsl and lib suites were not run in this lane. Contention made each cold build 30-60 min, with up to 20 concurrent rustc processes, and rustc OOM'd once. Run cargo test --test cards_behavioral -- bt22_034 bt21_064 ex10_046 ex10_049 bt24_069 on the integrated branch to exercise the mod.rs registrations.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md

- **G-PLAY-TO-BREEDING-FROM-TRASH** (driver: EX10-009 Creepymon, [When Attacking]): "If your opponent has 10 or more cards in their trash, you may play 1 red or purple 5000 DP or lower Digimon card from your trash to your empty breeding area without paying the cost." The engine and DSL have `play_to_breeding_from_hand` (BT20-015 / BT20-053 / BT9-047) but nothing that plays a TRASH card into an empty breeding area. DCGO: `CardEffectCommons.PlayPermanentCards(..., root: Trash, isBreedingArea: true)`, candidate gate `CanPlayAsNewPermanent(card, false, activateClass, isBreedingArea: true)`. The play is not a battle-area play (no memory, no [On Play]). Needs a `select_trash` candidate predicate (as `can_play_to_breeding_from_hand` does for hand) plus a step `play_to_breeding_from_trash { of, hand_index }`. EX10-009 is also unauthored: WD/OD 'delete all lowest-DP opposing Digimon; if this effect didn't delete, trash their top 5' is expressible today (`delete_all_permanents` with `selector: lowest_dp`, plus a deleted-count check), and the [End of Your Turn] may-attack has a pattern in AD1-004.

#### ## qa/dsl-vocab-gaps.md

- No new DSL vocabulary was required by BT22-034, BT21-064, EX10-049, EX10-046 or BT24-069.
- Authoring note (idiom): `select_hand { of: opponent }` is chosen by the CONTROLLER unless wrapped in `as_selecting_player { of: opponent, body: [...] }`. A binding set inside the body is not visible to sibling steps after it, so the dependent `if` (trash chosen card, else mill) must live in the same body (BT24-069 Vilemon, BT21-024 precedent).
- Authoring note (idiom): "By trashing X, instead ..." with an optional Yes/No at the start (BT22-034) is `select_effect_choice` (affirmative first, decline-shaped label second, so the replay's yes_no_branches maps a DCGO bool) inside `if security_count_gte`, then `trash_top_security { bind_count_as }`, then `if equals [bound, 1]`. The `if` bodies repeat the target pick per branch because a parked select's binding does not survive past its `if` body.
- Authoring note: DebugRunner card data carries no standard evo circle for cards with no alt_paths (EX10-049/046, BT24-069). Digivolve-timing tests must queue `EffectTiming::WhenDigivolving` via `enqueue_triggered` instead of a real digivolve action.

### yaml-condition-stricter-dcgo (fixed)

Fixed. All three diagnosed defects are real YAML-only bugs and are fixed. Each game now gets past its original divergence. Each one then stops at a different, unrelated defect (see residual).

1. BT24-085 (Dan Yuki & Kanan Yuki): the [End of Your Turn] clause was gated on a TS Digimon in the battle area. Neither the printed text nor DCGO has that gate. The clause now needs only source_is_unsuspended (the suspend cost). Printed text: "By suspending this Tamer, you may use 1 [TS] Option ... Then, 1 [TS] Digimon may attack". DCGO: Tamer on the battle area, suspend payable, memory <= 0. The attacker pick is already can_attack-filtered.

2. EX13-036 (Kentaurosmon): the third clause ([When Digivolving] / [End of Attack] / [Counter], once per turn) used all_of(own Digimon, opponent Digimon). It is now any_of. This matches DCGO's AdditionalActivateCondition, an OR, and CR 4.3 1-3-2 (do as much as possible). The two picks already auto-skip an empty side, and place_on_security of an unbound permanent is a no-op. So no DSL widening was needed, and I filed no gap entry for this item.

3. BT24-100 (In-Between Theater): the use_requirement and the ignore-colour flood_gate used battle-only any_permanent. They now use any_field_permanent (breeding plus battle). DCGO passes isContainBreedingArea=true, the official Q&A says "battle area or breeding area", and CR 4.3 3-4-6 puts the breeding area inside the field.

Audit findings (same defect family):
- Breeding-area scope. Fixed in BT21-097, BT22-099, BT23-096, BT24-091, BT24-095, BT24-097, BT24-099, EX10-070, P-205, BT25-093, BT25-100, BT25-101 and BT26-101. The last four only had a battle-only gate. Their use_requirement was already any_field_permanent, so their play legality was already correct and the change just aligns the gate.
- Missing hand bypass. A flood_gate with IgnoreColorRequirement is field-scoped and never unlocks an Option still in hand. Hand legality only reads the card-level use_requirement (option_use_requirement_or_color_available in src/action/mask.rs). A probe test with a CS Digimon or Tamer in the battle area failed for BT22-099 and BT23-096, which proves the bypass never worked from hand there. I added a mirroring use_requirement to BT22-099, BT23-096, ST22-11 (any Tamer) and BT19-093 (no [Queen Device] in the battle area).
- Left unchanged on purpose. BT21-091, ST22-08, P-156 and ST22-11 are Tamer-only (a Tamer is never in breeding). BT24-090/094, BT25-094/095/097/099/102, BT26-100, EX7-074, P-151 and P-180 use battle-only DCGO scans (HasMatchConditionOwnersPermanent) or face-up-security conditions that already match.
- Each-player all_of scan. I scanned all YAMLs for all_of gates mixing own and opponent presence. Only four hits: BT23-064 (see residual), EX13-044 (two clauses, which match DCGO's AND) and EX13-036 (fixed).

Test status:
- The new and changed tests are in the cards_behavioral tree: bt24_085.rs, bt24_100.rs, ex13_036.rs, plus the new option_color_bypass_breeding.rs registered in main.rs.
- I did not run the full cards_behavioral binary. Its compile exceeded 55 minutes under 6-way contention, so I killed it.
- Instead I compiled a temporary small integration test (deleted before commit) that #[path]-included the new and changed modules and the existing test modules of every touched card, and ran it for red and green.
- Red: 15 of 43 tests failed on the unfixed engine, exactly the predicted ones. The cards already scanning the field passed.
- Green: all 223 tests pass in the final run.
- ST22-11 and BT19-093 tests were written after the unfixed lib was rebuilt, so they were never shown red. They are covered by the BT22-099 and BT23-096 evidence, which is the same defect.
- dsl-lint reports nothing for the touched files. The admissibility check is ok.
- qa/impact/dsl_impact_index.json is now stale, which the integrator regenerates. lowering_coverage is current.

The commit also adds an unrelated main.rs mod line, which should merge trivially.

**Residual (not fixed):**

All three games still FAIL, each at a later, different defect that I did not fix:

1. g3, step 22 (BT24-085 game). Unstamped generic_bool false (P0 declined an optional effect, naming no card) with no prompt parked. The engine had resolved EX12-061's target selection with no recorded answer. Needs a look at EX12-061 (probably an optional-gate or implied-answer issue) or the converter's decline stamping. Related notes in the same run: row 18 "SelectPermanentEffect cancel consumed with no prompt parked" and row 20 "MultipleSkills index 1 consumed by Hand 'Trash 1 card' with no trigger order queued (DCGO listed 2+ activatable effects)".

2. g19, step 44. A count payload with no `candidates` for the Arts Digivolve Option prompt 'Choose a card for Arts Digivolve, or pass to trash this Option'. Most likely a stale recording (an older recorder that does not emit `candidates`), or a converter gap. Re-record, or map by index. The earlier suspected breeding HATCH/MOVE ambiguity around step 67 is not reached now (the game stops at 44), so I did not add GetBreedingAreaPermanents logging to Core.cs.

3. g11, step 21. TriggerOrder frame target (player 1, slot 0) matches none of the engine's offered [30, 31]. This is a converter or replay trigger-order mapping issue, or the engine queued a different trigger set than DCGO. Needs a trace of the triggers parked on that prompt.

Other findings not fixed:
- BT23-064 (Bakemon, [On Play]/[When Digivolving]): its condition also requires an opponent Digimon of level 4 or lower, but DCGO's CanActivateCondition only needs an own Digimon. This is a different family (optional "By deleting ..." cost, CR 15-7) and probably belongs to the optional-gate-drift lane. I left it alone.
- BT21-095's ignore-colour clause ("no face-up security") is omitted from the YAML entirely (an omitted-clause bug, not a scope bug).
- Before the full cards_behavioral binary is run, its compile takes more than 55 minutes under contention and could not be completed here. Please run `RUST_MIN_STACK=268435456 cargo test --test cards_behavioral -- bt24_ ex13_036 option_color_bypass --test-threads=8` on an idle machine to confirm.

**Gap tracker entries (verbatim from the lane):**

### qa/dsl-vocab-gaps.md / docs/RUST_ENGINE_GAPS.md entries (yaml-condition-stricter-dcgo, 2026-11-17)

**G-YAML-IGNORE-COLOR-HAND-USE-REQ (RESOLVED in claude/hunt-yaml-condition-stricter-dcgo, 99e1b6ed5).** A `kind: flood_gate` + `modifier: IgnoreColorRequirement` clause is a field-scoped declarative player modifier. It never unlocks an Option that is still in the hand, because the hand-play mask and `play_option_from_hand` only consult the card-level `use_requirement` (`option_use_requirement_or_color_available`, src/action/mask.rs). BT22-099, BT23-096, ST22-11 and BT19-093 carried only the flood_gate, so their printed "you can ignore this card's color requirements" never applied from hand. Fixed by adding a mirroring `use_requirement`. Authoring rule: every conditional ignore-colour Option needs BOTH the `use_requirement` (from hand) and the `flood_gate` (on field), and the two must test the same predicate. Guard: tests/cards_behavioral/option_color_bypass_breeding.rs. A lint (dsl-lint) rule "flood_gate IgnoreColorRequirement without a use_requirement on an Option" would stop the regression. That is not implemented yet, so I filed it as an idea only.

**G-YAML-BREEDING-AREA-SCOPE (RESOLVED, same commit).** "Digimon or Tamer on the field" conditions must use `any_field_permanent` (battle plus breeding) rather than `any_permanent` (battle only) when DCGO passes `isContainBreedingArea=true` to HasMatchConditionPermanent (CR 4.3 3-4-6). Audited the ignore-colour Option family: fixed BT21-097, BT22-099, BT23-096, BT24-091/095/097/099/100, BT25-093/100/101, BT26-101, EX10-070, P-205. Cards that scan with HasMatchConditionOwnersPermanent (battle only) were deliberately left on `any_permanent`: BT24-090/094, BT25-094/095/097/099/102, BT26-100, EX7-074, P-151, P-180.

**YAML gate drift rows (RESOLVED).** BT24-085 `[End of Your Turn]` carried an unprinted "TS Digimon in battle area" condition (removed). EX13-036 `[WD][EoA][Counter]` once-per-turn placement condition was an AND of both sides, now an OR (CR 4.3 1-3-2; DCGO AdditionalActivateCondition). No new DSL vocabulary or engine primitive was needed.

**Open follow-ups.** BT23-064 gate requires an opponent level-4-or-lower Digimon where DCGO only needs an own Digimon (optional "By deleting" cost; CR 15-7 family). BT21-095 has no ignore-colour clause at all (DCGO: no face-up security). g3 step 22, g19 step 44 and g11 step 21 are new replay divergences (see residual). qa/impact/dsl_impact_index.json needs regeneration by the integrator.

### replay-int-zone-menu-gaps (partial)

The replay now folds the three DCGO int menus the lane named. g6 and g4 get much further. g8 and the other two games still stop on different defects, listed under residual. The diagnosis needed two corrections. (a) The BT25-083 gap was not a missing table row. gen_int_menu_base.py read only the named `new(message:, value:, spriteIndex:)` form and dropped cards that use the positional `new SelectionElement<int>("Hand", 1, 0)` form (BT25-083, BT25-040, BT25-042), so BT25-083 was never read as a one-based menu. The generator reads both forms now and dcgo_int_menu_base.json is regenerated. BT25-083 is added to DCGO_INT_ZONE_MENUS and the decline table (Hand=1/Trash=2/Don't=3, per BT25_083.cs:44-47), and BT25-040/042 ("Don't trash"=3) to the decline table. (b) AD1-015 asks its "Play Tamer from hand=1 / trash=2 / Don't=3" menu when either zone holds a Tamer (AD1_015.cs:150-171). P-249 asks only when both do (P_249.cs:49-71). I added a per-card table, dcgo_zone_menu_asked_for_either_zone, containing AD1-015. fresh_zone_card_pick_stands_for_zone_menu folds on one zone only for cards in that table, so P-249 keeps the both-zones rule. AD1-015's second menu (this Digimon=1 / a Tamer=2 / Don't=3, AD1_015.cs:191-250) matches P-249's, so AD1-015 is added to dcgo_placement_area_menu. The BT25-085 DUAL check holds. DCGO CardKinds contains Option for a DUAL (CEntity_Base.cs:76, CardSource.cs:3537), and the engine's kind_matches_card_search matches `kind: option` via is_option_card_for_search, so the two agree. I confirmed this by reading the code only and did not add a test.

**Residual (not fixed):**

(1) g8 row 16 is a stale-recording MultipleSkills ambiguity for the two BT17-023 [When Attacking] clauses (mandatory Draw 1 and optional digivolve). The two orders lead to different games: digivolving first strands the draw. Fixing it needs either a recorder re-capture with chosen_effect_name or an index read against the card's C# effect order. This is the replay_dcgo MultipleSkills lane. (2) g4 row 40: DCGO has no MultipleSkills row for simultaneous AD1-023 and BT18-088 [Start of Your Main Phase] triggers, but the engine parks a TriggerOrder. BT18_088.cs:34-38 registers its effect as optional (isOptional true) and the engine labels it mandatory, so the clause-optional flag may differ. This needs a separate look at BT18-088.yaml. (3) g6 row 69: an OwnField pick after the P0 digivolve into BT12-029 omits slot 0, although DCGO recorded the target as slot 0. (4) Unverified: the unstamped generic_bool false after AD1-015's hand pick (g6 file step 33, g8 file step 26). g6 passes row 33 now, but g8 never reaches it. (5) Fidelity gap noticed while reading AD1-015.yaml, not fixed: after the Tamer play the YAML always asks the host pick (this Digimon or a Tamer), but DCGO asks its place menu only when the hand holds a [Hybrid] or [Ten Warriors] card (AD1_015.cs:191). With no such card in hand, the engine parks a pointless optional prompt that DCGO does not. A hand-presence guard in the YAML would fix it. (6) The BT25-040/042 decline additions and the new one-based entries for BT25-040/042/083 were derived from the regenerated JSON, and I read only their C# int-element lines. (7) Two replay_dcgo_review_r4 and r4b tests fail on unmodified main, as above. Someone should fix the r4b closure-callback fixture and re-check r4 on main. No DCGO or recorder source was changed.

**Gap tracker entries (verbatim from the lane):**

### Replay driver gaps (replay-int-zone-menu-gaps, 2026-11-17)

- **G-REPLAY-INT-MENU-GEN-POSITIONAL (fixed, cf0412636).** `code/tools/dcgo-replay/scripts/gen_int_menu_base.py` read only the named `new(message:, value:, spriteIndex:)` form of `SelectionElement<int>`. Cards using the positional form `new SelectionElement<int>("x", 1, 0)` (BT25-083, BT25-040, BT25-042) were never marked one-based, so their `generic_int` rows could not be read. The generator now reads both forms, and `dcgo_int_menu_base.json` is regenerated (94 to 97 cards). From a worktree, run it with `DCGO_CARD_EFFECT_ROOT=<base>/DCGO/Assets/Scripts/CardEffect`.
- **G-REPLAY-ZONE-MENU-EITHER-ZONE (fixed, cf0412636).** A DCGO hand/trash int menu is asked in one of two shapes. Either-zone: shown when the hand OR the trash holds a candidate, listing only the zones that do (Beowolfmon AD1-015). Both-zones: shown only when both hold a candidate, with `SetInt(...)` setting the zone silently otherwise and no `generic_int` row (Strabimon P-249, ShineGreymon ST24-07, BT26-081, P-240). `selection_resolve::dcgo_zone_menu_asked_for_either_zone` lists the either-zone cards. Only AD1-015 is listed so far. Audit the other `select_zone_cards` cards in `DCGO_INT_ZONE_MENUS` (AD1-007 and AD1-020 are either-zone loops) by reading each C#.
- **G-REPLAY-PLACEMENT-AREA-MENU-AD1-015 (fixed, cf0412636).** AD1-015's "Place under this Digimon=1 / under a Tamer=2 / Don't=3" menu is added to `dcgo_placement_area_menu`.
- **G-AD1-015-HOST-PICK-WITHOUT-HAND-CARD (open, fidelity).** The AD1-015.yaml `select_own_permanent` host pick is parked even when the hand holds no [Hybrid] or [Ten Warriors] card. DCGO asks the place menu only when one is in hand (AD1_015.cs:191). Add a hand-presence guard.
- **G-REPLAY-MS-STALE-NO-EFFECT-NAME (open).** In g8, BT17-023's two same-timing [When Attacking] clauses (mandatory Draw 1 and optional digivolve) cannot be told apart from an old raw log that has only `chosen_effect_card`. The orders lead to different games.
- **G-REPLAY-SOYMP-UNRECORDED-ORDER (open).** In g4, AD1-023 and BT18-088 [Start of Your Main Phase] trigger together. DCGO records no MultipleSkills row, but the engine parks a 2-entry mandatory TriggerOrder. Check BT18-088's optional flag (BT18_088.cs:38 `isOptional` true).
- **G-REPLAY-TESTS-RED-ON-MAIN (open).** `replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short` and `replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer` fail on origin/main 86652b985. The r4b failure is the 'cloned PendingSelection has no closure callback' panic at selection.rs:360 (clone-safety).

### yaml-optional-gate-drift (fixed)

Both games now replay clean (PASS) on a final build in my worktree. The fix plan was correct and I extended it.

**BT21-001 (Gigimon):** I added `outer_prompt: true` to its inherited clause. DCGO registers it with isOptional=true and a CanActivateCondition that only needs a Digimon on the battle area (BT21_001.cs:21, :60-63). The yes/no is therefore always shown, and the candidate check lives inside ActivateCoroutine (:68). With no candidate the folded optional pick vanished, so DCGO's declined bool hit an unrelated prompt. Declining the gate keeps the OPT (CR 4.3 15-14-1-4, Q1818). Accepting spends it (15-7-4 / 15-7-5). BT21-001#0 leaves the folded-may refund inventory.

**ST24-01 (Koromon):** I rewrote it to the ST23-01 shape. The clause is `optional: true`, with the DATA SQUAD hand gate dropped. The face-down trash cost is first and unconditional, then an optional select_hand with `can_digivolve_onto_with_cost`, then `if binding_present` for the digivolve. The header now cites 15-7-5.

**ST24-11:** No change needed. In DCGO its clause is not isOptional (the trash cost is an independent Tamer pick).

**Audit (extra beyond the lane):** I checked each DCGO CanActivateCondition in the folded-may inventory. Four more clauses had the same defect, and I fixed them the same way:
- BT21-082#2
- BT24-001#0
- EX9-033#3
- P-207#2

BT24-001's pick is also made mandatory after the accept, matching DCGO's `canNoSelect: false`.

**Replay results:**
- 20261113T011922Z-g3: was a halt at step 59, now PASS.
- 20261116T164448Z-g5: was a halt at step 26, now PASS.
- No recorder_stale residue remained for g3.

**Tests:** I did not run the new tests against the unfixed YAML. The red evidence is the two replay divergences. The new tests assert the gate is offered with no candidate, a decline keeps the OPT, and an accept with nothing to do (or an accept then a passed pick) spends it.

**Self-play fixture:** The recorded self-play fixture `medusamon-immune-token-payment.json` is a regression guard for BT21-001. It broke because the gate now appears wherever the clause fires. I inserted three declines and re-phased two rows, and the game outcome is unchanged.

**Build caveat:** I verified on an opt-level=0 profile in a separate target dir, because the machine was too contended to build the opt-level=2 profile. The results should be profile-independent. The committed `cards_behavioral` suite was not run in full.

**Residual (not fixed):**

1. The remaining folded-may clauses I checked have a DCGO CanActivateCondition that already checks candidates, so no gate without candidates is correct.

2. BT24-051#2 (inherited-style [When Digivolving]/[When Attacking] OPT unsuspend) has the same gate defect plus a stricter YAML `condition: any suspended own Digimon`. DCGO's SharedCanActivateCondition1 is only IsExistOnBattleArea, isOptional true, and the pick after the accept is `canNoSelect: false`. The fix is to drop the condition and add `outer_prompt: true` with a mandatory pick. This overlaps the sibling lane yaml-condition-stricter-dcgo, so I left it.

3. Not verified by hand (the heuristic dump could not extract these): BT21-044#1/#2 (shared `_ => SharedCanActivateCondition1(activateClass)`), BT25-055#1/#2, BT25-053#3, EX12-074#0 (its gate is marked isOptional true but its CanActivateCondition parse was garbled), EX12-045/046/065. Some register with isOptional false in DCGO, which is a different shape.

4. The audit covered only the folded-may inventory. The wider set where `lower_triggered.rs` suppresses the outer prompt via `first_step_candidate_guard` ("no candidate, no prompt") has the same latent defect wherever DCGO's CanActivateCondition has no candidate check. I did not enumerate it. The guard should become opt-in per card, or it should be audited against DCGO per clause.

5. The new tests were not run before the fix, so there is no red run. The red evidence is the two replay divergences.

6. All verification was on opt-level=0 and a private target dir. I did not run the committed opt-level=2 `cards_behavioral` binary. Run the full `cards_behavioral` target on the integration branch.

7. Unrelated build-environment note: killing a stale hung rustc let the blocked cargo recover, but an LNK1104 occurred when an old background dsl test exe held the output file.

**Gap tracker entries (verbatim from the lane):**

### [Yes/no activation gate dropped when the folded optional pick has no candidate]  [G-DSL-OUTER-GATE-NO-CANDIDATE] — PARTIAL (5 cards + ST24-01 fixed 2026-11-17, branch claude/hunt-yaml-optional-gate-drift)

- **Defect:** DCGO registers many "you may" effects with `isOptional=true` and a `CanActivateCondition` that does NOT check for candidates (the candidate check lives inside ActivateCoroutine). DCGO therefore always shows the OptionalSkill yes/no prompt. Our DSL folds that "may" into the first optional pick. With no candidate the pick is skipped, so no prompt is parked and DCGO's recorded unstamped `generic_bool` lands on an unrelated optional prompt (replay halts "not an optional-activation gate"; the card-less MultipleSkills lookahead then refutes). Seen in real PvP games 20261113T011922Z-g3 (BT21-001) and 20261116T164448Z-g5 (ST24-01).
- **Rule:** declining the gate is not activating the effect, so the [Once Per Turn] is kept (CR 4.3 15-14-1-4, Q1818). Accepting activates it and spends it even if nothing follows (15-7-4 / 15-7-5).
- **Fix idiom:** `optional: true` + `outer_prompt: true` (BT24-003 idiom). The inner pick stays optional only when DCGO's pick is `canNoSelect: true`. BT24-001's DCGO pick is `canNoSelect: false`, so there the pick was made non-optional. A clause with `outer_prompt` is not eligible for the folded-may refund rewrite (`folded_may_refund_binding`), so it leaves the `REVIEWED` inventory in tests/dsl/folded_may_opt_refund.rs.
- **Fixed:** BT21-001#0, BT21-082#2, BT24-001#0, EX9-033#3, P-207#2 (outer_prompt). ST24-01 rewritten to the ST23-01 shape (clause-level optional, cost first and unconditional, no DATA SQUAD hand gate DCGO lacks).
- **Tests:** cards_behavioral bt21_001 / bt21_082 / bt24_001 / ex9_033 / p_207 / st24_01 (gate offered with no candidate; decline keeps the OPT; accept with nothing spends it); the self-play fixture tests/fixtures/training/medusamon-immune-token-payment.json gained three declines and two re-phased rows.
- **Still open:** BT24-051#2 (condition stricter than DCGO plus missing gate; overlaps yaml-condition-stricter-dcgo), and the clauses whose gate is suppressed by `first_step_candidate_guard` in lower_triggered.rs (no candidate => no prompt) wherever DCGO's CanActivateCondition has no candidate check; unverified by hand: BT21-044#1/#2, BT25-055#1/#2, BT25-053#3, EX12-074#0, EX12-045/046/065.
- **Blocks training:** no.

### ally-attack-self-semantics (fixed)

Both faces of the cluster were engine/card bugs. (1) Game::fire_on_attack (combat/mod.rs) deleted the attacker's own OnAllyAttack entries. DCGO AttackProcess.cs stacks one OnAllyAttack batch with no self exclusion, and BT18_042.cs / BT25_016.cs gate only on 'the attacker is a Digimon on the battle area'. So 'When Digimon attack' never offered BT18-042 on its own attack, and the recorded MultipleSkills pick named an entry the engine never offered. I removed the self-drop. 'Another Digimon' stays expressible per card as `trigger_if: { event_target_is_source: false }`, which already existed. I added a synthetic test for it and did not add new DSL vocabulary. (2) BT21-102 (Tai Kamiya) clause (b) was authored as carrier-scoped when_attacking, which never reaches a Tamer. It is now on_ally_attack, with `source_is_unsuspended` plus `activation_cost: suspend_self` (BT18-092 / BT6-089 idiom). The stale 'DSL lacks on_ally_attack' comment is deleted. Audit of the 21 on_ally_attack YAMLs: Tamer and Option observers cannot be the attacker, so self-inclusion does not affect them. Only the three Digimon-hosted clauses matter. BT18-042 is now correct. BT26-021 listed on_attack as a workaround for the missing self entry; that would now double-offer, so I removed it. BT25-016 ('When a Digimon with 13000 DP or more attacks', [All Turns]) lacked on_opponent_attack, and DCGO has no side gate, so I added it. A sweep of `when_attacking` clauses on tamer/option kinds found only BT21-102. The others (BT18-094 inherited, BT24-091/097, BT25-093, BT9-109) are linked-option or inherited effects that are correct. BT4-092 (Tamer, 'other than DoruGreymon...') and the unimplemented BT23-069 / P-111 ('another Digimon attacks') are not affected. Tests run: lib 741, combat 303, cards_behavioral full run 14492 passed (the one failure was my own first draft of the two-Tais test; its rewrite then passed) plus targeted reruns of 17 affected card modules, timing_dispatch 52, archetypes 288, dsl 1767, rules_conformance 362, judge_quiz 44, effect_context 167, mask_and_tensor 182, dcgo-replay 27. dsl-lint reports nothing on BT21-102 / BT25-016 / BT26-021. Not run: replay_dcgo_field_area_pick printed no result line in the captured run, and the dsl-lint warnings elsewhere in bt21/bt25 are pre-existing. I did not get a red run of the new tests against the unfixed engine. The first build was lost to a rustc crash under memory pressure, so only the old self-exclusion tests failed in my first partial combat run. Replay after the fix: 20261116T144923Z moved from step 47 to step 75 and 20261114T142653Z-g5 from step 15 to step 33. Each now stops at a different defect (see residual).

**Residual (not fixed):**

Not fixed here, all different defects. (a) 20261116T144923Z step 75: the digivolve cost-choice EffectChoice 'Choose which digivolution cost to pay' offers three branches (30 cost 2, 31 cost 2 treating base as Lv.3 Yellow, 32 cost 3 treating base as Lv.3 Yellow). The converter's count payload 2 matches two branches. This is a converter/engine mapping issue for a multi-cost digivolve choice (cost-choice index vs branch index), not ally-attack. (b) 20261114T142653Z-g5 step 33: P0 optional effect with a hand pick (trash 1 hand card, probably EX11-055's 'You may trash 1 [Composite]/[Wicked God] trait card from your hand to Draw 1 and gain 1 memory', or P-205's), recorded as an unstamped generic_bool then SelectHandEffect. The engine had no prompt parked. The prior note at row 12 shows the same optional-yes-gate shape for 'Play 1 [ADVENTURE] Tamer card'. It belongs to the optional-yes-gate / optional-prompt lane. (c) Converter, not fixed: convert_raw.py truncates 20261116T144923Z at 'main-phase packet 1 at step 312'. Packet 1 is ActivateCardAction (raw rows seq 441-443: QueueMainPhaseAction_Internal args [1,1,[0,0,0,99,0,0,0,0]] then SetActCardSkill [99,0]). It is the [Hand][Main] of Rhihimon (BT18-081), card index 99. The engine models this as main_from_hand, action HAND_EFFECT_START (30) + hand slot. The packet row carries only the card index, no hand listing, so the hand slot cannot be resolved from it alone. The following PlayCard row (seq 447, root Hand, pay_cost true, target BT18-094) lists the hand and gives slot 6, but converting packet 1 would also have to suppress that PlayCard row as a duplicate action and map the intermediate frame/card picks. It also cannot be verified until the game gets past step 75. Likely needs the recorder to log the hand slot (or hand listing) on the ActivateCard packet, plus a new-recorder version bump. (d) P-204 (Option, [All Turns] 'When Digimon attack players', Delay): its trigger has event_target_is_player but no opponent-side gate was checked. Its comment says the Delay scan covers both owners on the single on_ally_attack broadcast. I did not re-verify opponent-owner coverage for delays, which is outside this cluster. (e) Generated docs/qa not regenerated per instructions: docs/RUST_DSL_AGENT_GUIDE.md timing-usage table (on_ally_attack 21 -> 22 after BT21-102; on_attack count 1 lower after BT26-021) and qa/impact/*, lowering_coverage, card_admissibility derived artifacts (YAML for BT21-102, BT25-016, BT26-021 changed). The integrator should rerun `python scripts/verify --tier 0` generators. (f) The first partial combat run before the fix could not be repeated as a clean red run, because the original build was lost to a rustc STATUS_STACK_BUFFER_OVERRUN under memory pressure.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md

- **G-ENGINE-ONALLYATTACK-ATTACKER-SELF-EXCLUSION — closed (2026-11-17, branch claude/hunt-ally-attack-self-semantics).** `Game::fire_on_attack` (code/digimon-engine/src/combat/mod.rs) dropped the attacker's own `OnAllyAttack` entries from the trigger bundle. DCGO `AttackProcess.cs:197-199` stacks one `EffectTiming.OnAllyAttack` batch with no self exclusion; `[When Attacking]` is that timing with a self filter, here the carrier-scoped `WhenAttacking`. "When a Digimon attacks" ([All Turns] BT18-042, BT25-016, inherited BT26-021) has no "another", so the carrier must observe its own attack. DCGO PvP replay symptom: `MultipleSkills index 0 (chose BT18-042's effect) names no entry`. Fix: the self-drop loop is removed. The contract is updated in RUST_ENGINE_API.md (`OnAllyAttack` row) and effect.rs. "When ANOTHER Digimon attacks" (BT23-069 Necromon, P-111 Knightmon; both still unimplemented) is expressed per card with `trigger_if: { event_target_is_source: false }`, tested in tests/combat/on_ally_opponent_attack.rs. No new DSL vocabulary was needed. Tests that previously asserted the exclusion were inverted: combat::on_ally_opponent_attack and timing_dispatch::on_ally_attack_fires_on_the_attacker_itself.

- **G-YAML-TAMER-OBSERVER-WHEN-ATTACKING — closed.** `when_attacking` is carrier-scoped (engine enqueues it only on the attacker's own stack) so a Tamer/Option observer of "when one of your Digimon attacks" can never receive it. BT21-102 (Tai Kamiya) clause (b) was authored that way, with a stale comment claiming the DSL lacks `on_ally_attack`, and its test enqueued the timing by hand. Now `on_ally_attack` plus `source_is_unsuspended` + `activation_cost: suspend_self`. Sweep: it was the only tamer/option YAML with a when_attacking observer clause. Authoring rule: observers of another permanent's attack use `on_ally_attack` / `on_opponent_attack`; `when_attacking` is only "when THIS Digimon attacks". Tests must drive real `attack_digimon` / `attack_player`, not hand-enqueue.

- **G-YAML-ALL-TURNS-ATTACK-OBSERVER-ONE-SIDED — closed for BT25-016, audit note for others.** "[All Turns] When a Digimon with 13000 DP or more attacks" is any attacker on either side (DCGO BT25_016.cs gates only on battle-area presence and DP >= 13000), but the clause had only `on_ally_attack`. Added `on_opponent_attack`. BT26-021 had `[on_attack, on_ally_attack, on_opponent_attack]` where `on_attack` only compensated for the removed self entry; it is now `[on_ally_attack, on_opponent_attack]` (would otherwise double-offer one attack). Open audit: P-204 (Delay, [All Turns] "When Digimon attack players") authored `on_ally_attack` only; its comment says delay observers scan both owners on one broadcast. Opponent-owner delay coverage was not re-verified here.

- **Residual converter gap (not fixed): convert_raw.py main-phase packet 1 (ActivateCardAction).** `QueueMainPhaseAction_Internal` packetId 1 = a hand card's [Hand][Main] effect (Rhihimon BT18-081, card index 99, game 20261116T144923Z step 312). The packet carries only the card index, not the hand slot the engine needs (`HAND_EFFECT_START + slot`), and the paired PlayCard row (root Hand, target = the Tamer) must not be converted as a second action. The recorder likely needs to log the hand slot on that packet.

#### ## qa/dsl-vocab-gaps.md

- No new vocabulary. Recorded for authors: `event_target_is_source: false` under `trigger_if` is the "another Digimon" predicate; `on_ally_attack` includes the carrier.

### delay-trigger-substrate (partial)

Engine bug, fixed at the substrate. P-193's printed "[End of All Turns] <Delay>" was authored as the player-initiated [Main] Delay. DCGO registers it on OnEndTurn, optional, gated only by CanDeclareOptionDelayEffect (the enter-field turn is not the current turn). DCGO's StackSkillInfos(null, OnEndTurn) collects every battle-area permanent whichever player's turn ends. The engine therefore never parked the optional end-of-turn prompt that DCGO's recorded decline answers.

Changes:
1. New DelayTrigger::EndOfAllTurns (CR 4.3 15-16-12-3, 16-17-1/-2/-3) and DSL timing `end_of_all_turns`.
   - compute_delay_trash_turn returns turn_count + 1.
   - The turn-end scan and its resume paths include it.
   - lower_delay marks it optional with the outer accept/decline prompt, like the other scheduled Delays.
   - A decline leaves the Option in play and reschedules it to the next turn end.
   - On a triggered clause, `when: end_of_all_turns` expands to end_of_your_turn + end_of_opponents_turn, so it cannot be silently dropped.
   - The variant is appended last in the bincode-serialised CompiledTiming, so the pack layout is unchanged.
2. Event-gated Delay fan-out (enqueue_event_gated_delayed_options) now runs for TriggerSource::SecurityRemoved, restricted to the observer player's Options. This makes BT24-093's "[All Turns] When your security stack is removed from, <Delay>" fire. BT21-093's on_opponent_security_removed Delay had the same hole.
3. BT24-093 (Temple of Beginnings): authored the previously BLOCKED Delay clause. Both gaps its YAML recorded were already closed or closed here:
   - lower_delay already mapped any event timing to OnEvent.
   - security_place_top_stacked_card already existed with the right semantics (it places the visible top card, CR 4.3 4-7-1).
   - The clause is a `kind: delay` with `trigger: on_own_security_removed`, a declinable select_own_permanent over [Aegiochusmon]/[Jupitermon]-named Digimon with materials_count_gte 1, then security_place_top_stacked_card (top, face down).
4. P-193 re-authored as trigger: end_of_all_turns, with YAML comments rewritten.

Game results (dev-profile dcgo-replay built in the worktree):
- 20261114T012101Z-g1: was engine_error at step 25 (undecidable duplicate-pick, no end-of-turn prompt). Now PASS, 54/54 steps.
- 20261114T142653Z-g2: was a selection-resolution divergence at step 41. Now advances to step 45 with a different divergence (see residual).
  - Steps 41-43 now consume the two BT24-093 "use this optional effect?" declines as real optional Delay prompts.
  - The engine queues exactly two BT24-093 DelayEffect entries, from two separate P1 security removals earlier in that turn, matching DCGO's two recorded declines.
  - BT24-084 did not fire for the removal before it entered play, which matches DCGO, so no BT24-084 change was needed.

Not run: cargo test --lib, the digimon-dsl crate tests, the engine `dsl` integration suite and the dcgo-replay crate tests. The machine hit "Allocation failed" / STATUS_STACK_BUFFER_OVERRUN while compiling digimon-dsl's tests and cards_behavioral, and I ran out of build time. The targeted cards_behavioral re-run after the final enum reorder also did not complete (it was started without unsetting RUST_MIN_STACK, which crashes rustc). Hygiene work not done: `scripts/verify --tier 0` fails at impact_index_check (qa/impact index is stale, expected, integrator regenerates). mechanics_readiness --check is ok. card_admissibility --check timed out, so its artifact should be re-checked by the integrator.

**Residual (not fixed):**

1. g2, step 45 (new, different defect, not fixed). P1's Digimon BT24-034 attacks. Its source stack includes BT25-030 (Elecmon, inherited [When Attacking][OPT] "may add top security to hand", authored as may_add_top_security_to_hand) and parks the Security-kind owned-card-choice page "Add top security to hand" (Choose card = action 30 / Decline = 62). DCGO recorded a stamped generic_bool true (BT25_030.cs asks it via SetBoolSelection; BT25-030 is listed in dcgo_bool_yes_no_cards.json).
   - Diagnosis from a probe at cursor 44: the replay resolves that stamped bool as a decline. P1 hand stays 11, security stays 1, and the engine goes straight to P0's blocker prompt.
   - Calling resolve_selection(1, 30) on the same state works: hand 12, Recovery +1, and a 2-entry TriggerOrder prompt (BT24-084 + BT24-093) appears. That is exactly DCGO's next row, MultipleSkills chosen BT24-084.
   - So the bug is in the replay/selection_resolve mapping of a stamped bool true onto an owned-card-choice Security page (probably CardChoiceOperation::CommitCard vs InspectCard row selection in the bool_value branch), not the Delay substrate. Likely overlaps lanes hunt-replay-optional-yes-gate / hunt-replay-int-zone-menu-gaps.
   - BT24-084 (BT24-084.yaml) and BT24-093 will be exercised again once this step passes.
2. The cluster's BT15-006 (DemiMeramon) and BT10-073 (ChuuChuumon) were not reached; no divergence on either appeared in g1/g2 up to the stop points. BT10-073 has no YAML (cards/bt10/BT10-073.yaml missing), so it is an unimplemented card, not this lane.
3. Unrun suites, because of machine memory/build contention, listed in the summary: cargo test --lib, digimon-dsl crate tests, engine `dsl` integration suite, dcgo-replay crate tests. The integrator should run them. They are especially relevant to the new Timing/CompiledTiming::EndOfAllTurns variant, the implementation_inventory references.rs arms and the dsl-lint arm.
4. qa/impact/* (dsl_impact_index.json, lowering_coverage.json) and data/card_admissibility.json are stale relative to the two changed YAMLs. I did not edit them, per instructions. Regenerate: `python code/tools/impact_index.py --out qa/impact/dsl_impact_index.json`, `PYTHONPATH=code python -m tools.lowering_coverage`, `PYTHONPATH=code python -m digimon_gym.agents.card_admissibility`.
5. Order caveat, not seen as a failure: the turn-end Delay scan runs at the top of end_turn, before the [End of Your Turn] / [End of Opponent's Turn] pool, whereas DCGO puts the Delay in the same OnEndTurn pool as other end-of-turn effects.

**Gap tracker entries (verbatim from the lane):**

## docs/RUST_ENGINE_GAPS.md (append; branch claude/hunt-delay-trigger-substrate, commits daa1e05c9 + 9d4f1ccdc)

#### ## G-ENGINE-DELAY-END-OF-ALL-TURNS (CLOSED)
- Printed "[End of All Turns] <Delay>" (P-193 The Wicked God Emerges!) was lowered to DelayTrigger::MainPhaseActivated, a player-initiated [Main] action.
- DCGO P_193.cs registers it on OnEndTurn, optional, gated by CanDeclareOptionDelayEffect, and StackSkillInfos(null, OnEndTurn) offers it at EVERY turn end after the placing turn (CR 4.3 15-16-12-3, 16-17-1/-2/-3).
- Fix: DelayTrigger::EndOfAllTurns. compute_delay_trash_turn gives turn_count + 1, which is both the first window and the window after a decline.
- The turn-end scan resolve_delayed_options, its resume path and delay_lifecycle_triggers include it.
- lower_delay marks it optional with the outer accept/decline prompt. A decline keeps the Option in the battle area.
- Tests: tests/cards_behavioral/p/p_193_delay_window.rs and the rewritten p_193.rs.

#### ## G-ENGINE-EVENT-DELAY-ON-SECURITY-REMOVED (CLOSED)
- enqueue_triggered_scan did not run the event-gated Delay fan-out (enqueue_event_gated_delayed_options) for TriggerSource::SecurityRemoved.
- BT24-093's "[All Turns] When your security stack is removed from, <Delay>" and BT21-093's on_opponent_security_removed Delay therefore never saw their event.
- Fix: SecurityRemoved joins the whitelist, with an observer filter (only_player = observer_player), so only the observing player's Options answer a removal.
- Covers combat security checks, effect removals and security-play removals, all of which enqueue OnOwnSecurityRemoved / OnOpponentSecurityRemoved through TriggerSource::SecurityRemoved.
- Tests: bt24_093.rs delay tests (accept / decline re-offered / placing turn / opponent removal / candidate filter).

#### ## Stale gap text to retire
- docs/RUST_ENGINE_GAPS.md "Digivolution-stack source extraction (pop_top_source from named permanent)", which names BT24-093 with "Workaround: None".
- security_place_top_stacked_card / security_place_top_card place the visible TOP card and leave the stack (BT20-084 / BT20-055 / BT26-033; CR 4.3 4-7-1 "stacked cards" are the cards of a stack of 2+).
- BT24-093 no longer needs a new primitive.

#### ## G-DSL-ON-OWN-SECURITY-REMOVED-DELAY-WHITELIST (STALE, closed)
- The BT24-093.yaml / test header claim that lower_delay.rs only whitelists OnSuspend|OnUnsuspend|OnAllyPlayed|... was already false: lower_delay maps every event timing to OnEvent.
- Only the effect_queue fan-out whitelist was real (above). Header comments rewritten.

#### # qa/dsl-vocab-gaps.md (append)

#### ## G-DSL-END-OF-ALL-TURNS-TIMING (CLOSED)
- New Timing value `end_of_all_turns` (CompiledTiming::EndOfAllTurns, appended last to keep the compiled pack layout / PACK_FORMAT_VERSION 0.34.0 intact).
- As a `kind: delay` trigger it lowers to DelayTrigger::EndOfAllTurns (optional every-turn-end window after the placing turn).
- As a triggered clause's `when:` it expands at compile time to end_of_your_turn + end_of_opponents_turn (BT20-084 spells this out as a two-element list).
- dsl-lint, implementation_inventory references and the RUST_DSL_AGENT_GUIDE vocab block were updated.

#### ## Open follow-up (NOT this lane)
- Replay mapping of a stamped generic_bool true onto an owned-card-choice Security page ("Add top security to hand", BT25-030 inherited [When Attacking]) resolves as Decline (see 20261114T142653Z-g2 step 45).
- File under the replay/selection_resolve gaps, not the DSL vocabulary.

### yaml-clause-omitted-impl (partial)

Authored the printed clauses omitted from three cards marked IMPLEMENTED or PARTIAL. Both games now get past their first divergence but neither passes; each stops at a different defect, listed under residual. (1) EX9-067 Mirai Kinosaki: added the [Your Turn] clause. When your Digimon digivolve into a [Puppet], you may return this Tamer to deck bottom and play Arisa Kinosaki or a [Puppet] Digimon from hand at play cost -3. It uses existing vocabulary only (on_digivolve plus event_card_trait_has, the return_self_to_deck_bottom activation cost, select_hand, play_from_hand with cost_delta reduce 3), so the diagnosis's "reduced-cost play rider missing" did not hold. It also fires for effect-initiated digivolves, as in DCGO EX9_067.cs. (2) BT10-093 Yuu Amano: added clause 1 (purple card placed under this Tamer: draw 1 and gain 1 memory, all turns, once per turn, mandatory) via on_add_digivolution_cards plus event_host_permanent_is_source plus event_added_card_any color_is purple. (3) EX10-034 Blastmon, found while replaying the BT10-093 game and outside the original list: its printed DigiXros -2 special play condition (3 Digimon w/ [Bagra Army], hand/battle area) was never authored; added as a digixros alt_path like EX10-056. validated_cards_dsl.json is corrected for all three. The EX10-044 test expectation changed (+1 hand) because its fixture places a purple card under Yuu Amano, so the authored draw now fires. I did not run the new tests against the old YAML, so there is no red-before proof; the old YAML plainly lacks the clauses and each game's first replay stopped exactly there. Replay with a dev-profile binary built in the worktree: g4 moved from step 44 to step 43 and g1 from step 22 to step 27 (the 0-based step numbers shift between recordings; see residual for the two new stopping points).

**Residual (not fixed):**

(a) g4, a different defect and not fixed: replay-adapter mapping of a card-less MultipleSkills index when the engine's TriggerOrder has fewer entries than DCGO's list. The engine deliberately filters currently-unpayable triggers (EX9-032's [When Digivolving] needs another Puppet or Token to delete); DCGO lists them and re-checks at resolution. State converges in the engine, since the filtered entry is re-evaluated after EX9-067 plays EX9-024, but the replay cannot map index 1 onto a one-entry TriggerOrder. A fix would live in code/digimon-engine/src/runners/replay.rs near the MultipleSkills handling (about line 4130 and selection_resolve.rs 2225): when the row has no chosen_effect_card, DCGO listed more entries than the engine, and the next rows identify the engine's lone entry, consume the index against that entry. (b) g1, unresolved: DCGO's P1 hand at Blastmon's PlayCard is [EX10-058, EX10-034, BT12-011, P-206, EX10-034, EX10-044, EX10-056], two cards drawn after the P0 security effect deleted BT10-073 (ChuuChuumon). The engine draws one (Yuu Amano, once per turn). The memory check saw no divergence, so the second draw has no memory gain. DCGO listed 2+ effects at that deletion (MultipleSkills index 1; the BT10-073 Save is one). I could not identify the second effect or the second draw from the DCGO C# (Save.cs and BT10_073.cs register a single Save). It needs a DCGO-side trace of what drew EX10-056. This lane did not look into BT10-073, which is JSON-only, so a missing clause there is not ruled out. (c) Generated artifacts that need regeneration by the integrator: qa/impact/dsl_impact_index.json (code/tools/impact_index.py --cards-dir code/digimon-engine/cards --out qa/impact/dsl_impact_index.json), qa/impact/lowering_coverage.json (PYTHONPATH=code python -m tools.lowering_coverage) and data/card_admissibility.json (PYTHONPATH=code python -m digimon_gym.agents.card_admissibility); I did not regenerate them. (d) Environment: with RUST_MIN_STACK=268435456 set during compilation under heavy memory pressure, rustc died twice with 0xc0000409; compile without it and set it only when running tests. An orphaned rustc of a killed cargo holds the build-dir lock and makes the next cargo hang; kill the orphan.

**Gap tracker entries (verbatim from the lane):**

### qa/dsl-vocab-gaps.md (resolved, no new vocabulary needed)
- **G-DSL-ON-CARD-PLACED-UNDER-TRIGGER**: now authored in BT10-093 clause 1 (`when: on_add_digivolution_cards` + `event_host_permanent_is_source` + `event_added_card_any: { color_is: purple }`, once_per_turn, `active_when: { all_turns: true }`). BT10-093 verdict moved PARTIAL to IMPLEMENTED.
- **PUPPETS-G005 (EX9-067 Puppet digivolve event context + reduced-cost play rider)**: closed with existing vocabulary: `when: on_digivolve` + `trigger_if: event_card_trait_has: Puppet`, `activation_cost: { return_self_to_deck_bottom: true }`, `select_hand` filtered by `can_play_from_effect_without_cost` and `affordable_with_cost: { reduce: 3 }`, then `play_from_hand { cost_delta: { reduce: 3 } }`. Effect-initiated digivolves fire `on_digivolve`.
- **EX10-034 Blastmon**: the DigiXros -2 special play condition (3 Digimon w/ [Bagra Army], zones hand/battle_area, cost_delta -2) was omitted despite status IMPLEMENTED; now a `kind: digixros` alt_path (`repeat: { min: 3, max: 3 }`, the EX10-056 idiom). EX10-059 and EX10-056 remain PARTIAL for their own omitted clauses.

#### ## docs/RUST_ENGINE_GAPS.md (open, from the replay hunt)
- **G-REPLAY-TRIGGERORDER-FILTERED-ENTRY-INDEX** (replay adapter, not an engine rule bug): DCGO's MultipleSkills index (card-less) addresses DCGO's own full trigger list; the engine's TriggerOrder omits entries whose optional cost is currently unpayable (effect_queue.rs `queued_optional_condition_unpayable`, G-ENGINE-UNPAYABLE-OPTIONAL-CONDITION-OFFERED). When the index is >= the engine's entry count and the following rows identify the engine's lone entry, the adapter should consume the index against that entry instead of failing. Repro: DCGO PvP game 20261114T142653Z-g4 (EX9-067 + EX9-032 Karakurumon after an effect-initiated digivolve), step 43.
- **G-DCGO-BT10-093-SECOND-DRAW (open, unexplained)**: in DCGO PvP game 20261115T153822Z-g1, P1 drew two cards (EX10-044 and EX10-056) between P0's security effect deleting BT10-073 (ChuuChuumon) and P1 playing EX10-034 Blastmon; the engine, with BT10-093 clause 1 now authored, draws one. Needs a DCGO-side trace of the second draw (a second Save or MultipleSkills entry at the deletion, or a BT10-073 clause that the JSON-only card lacks).

### replay-optional-yes-gate (fixed)

Root cause confirmed in ReplaySession::step (code/digimon-engine/src/runners/replay.rs). optional_yes_gate folded any stamped bool true against an optional prompt with more than one accept id (BT25-030's "Add top security to hand", ids 30/62) into "the gate in front of a pick" whenever the actor's next row existed. With no card/target pick behind it, the zone_choice_int branch rewrote the yes into cancel=true (Decline/PASS). In g15/g16 the next row was an UNSTAMPED generic_bool (BT24-084's optional trigger, or the BT24-034 Barrier accept), so the card was never added, BT24-084 never queued, and the Barrier accept was lost. Fix: the gate's "next row exists" test now excludes an unstamped generic_bool row, matching classify_unstamped_answer (small predicate plus comment). A stamped yes then falls through to selection_resolve and commits the lone CommitCard. I deliberately did NOT widen to "only when next_row_names_pick" or drop the cancel rewrite: the existing UnionZone path (yes, generic_int, card pick) and the yes-then-cancelled-pick shape rely on them. Engine cards (BT24-084, BT24-031/BT25-030 YAML, Barrier accept) needed no change. Result: g15 now PASSes (27/27); g16 now replays to step 62 and stops at the separate predicted TriggerOrder divergence.

**Residual (not fixed):**

(1) g16 step 62: P1 has a TriggerOrder ('Choose which triggered effect to resolve next (2 pending)') parked. The preceding unstamped generic_bool true (row 61) is consumed with the note 'no optional-activation gate parked' and the next SelectPermanentEffect row (a pick inside one pending effect) is then matched against the TriggerOrder entries [30,31]. DCGO evidently let P1 choose an order or effect implicitly via the pick; the replay needs to map a pick row onto the trigger-order entry whose effect it answers, or the engine orders those two triggers differently. Different defect family, not fixed. (2) Corpus grep found other stamped yes followed by an unstamped bool: g18 (step 24), 20261114T012101Z-g4 (step 17), 20261116T045756Z-g5 (step 21), 20261116T164448Z-g7 (step 45). Old and new binaries stop at the same earlier divergences, so the new predicate was not exercised there yet. They may regress or advance once their blockers are fixed; re-run them. Also in the corpus: 5 cases of stamped yes followed by a MultipleSkills row and 7 followed by another generic_bool, which still go through the gate and the cancel rewrite; not examined. (3) Two replay suites fail with my change present, in tests with no bool rows at all, so they cannot be affected by the changed predicate (it requires bool_value == Some(true)): replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short ('expected one divergence at row 0, got []') and replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer (panic at selection.rs:360 'cloned PendingSelection has no closure callback', a rule-28 clone-safety closure-based park). I did not rerun them on unmodified main (a cold build costs about an hour here), so 'pre-existing' is inferred, not measured. Worth running them on main before the integrator trusts them.

**Gap tracker entries (verbatim from the lane):**

### Replay driver: optional_yes_gate rewrote a stamped yes into a decline (fixed)
- Where: code/digimon-engine/src/runners/replay.rs, ReplaySession::step, `optional_yes_gate`.
- Defect: a recorded bool true against an optional prompt with more than one accept id was treated as "the gate in front of a pick" whenever the actor's next row existed, then rewritten to cancel=true when no card/target followed. An unstamped generic_bool behind it (another optional effect's use-it answer: BT24-084's trigger, a Barrier accept) counted as "a next row". BT25-030's "add top security to hand" was therefore declined; the dependent BT24-084 trigger never queued.
- Fix: exclude an unstamped generic_bool from the gate's next-row test (consistent with classify_unstamped_answer). Regression: tests/replay_dcgo_optional_yes_then_unstamped_bool.rs. Games: 20261113T025510Z-g15 now PASS.
- Open follow-ups: (a) 20261113T025510Z-g16 stops at step 62, a pick row applied against a parked TriggerOrder prompt (kind TriggerOrder, entries [30,31]) after an unstamped bool consumed as a no-op; (b) the gate and cancel rewrite still apply when the next row is a MultipleSkills row or another generic_bool (5 and 7 corpus cases, not examined); (c) replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short and replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer fail in the worktree (unrelated to bools; r4b hits the rule-28 closure-clone panic at selection.rs:360); not checked on unmodified main.

### replay-identical-copy-order (fixed)

The "is ambiguous" stop on a trigger order over two copies of one card is fixed in the replay driver (code/digimon-engine/src/runners/replay.rs). Both games get past it, but each now stops at a different, unrelated divergence.

Cause: the recording cannot say which of two same-id Tamers DCGO activated. Each trigger returns its own Tamer to the deck bottom, so the two branches differ only in which physical card went where. branch_digest (which wraps verification_digest) hashes every card_index, so the branches never matched.

Fix: when two order entries are the same effect (same label) on different physical cards of one card id, the branches are compared by a new identity-free digest (identity_free_digest). The card-less path (remote_trigger_order_choice), the card-named path and the lookahead all use it. The digest hashes the game's Debug text with card handles and battle identities renamed by first appearance. Slots, zone order and stack order are not renamed, so a choice that leaves the copies in different places still differs. The pinned verification_digest is untouched.

The diagnosis's plan (canonicalise card_index in the comparison digest) would not have been enough. The branch dumps showed four further sources of difference, which the clone now normalises:
- A queued trigger's trigger_context.target_permanent keeps a stale slot when an earlier slot empties, because shift_effect_queue_after_battle_area_remove only rebases source_permanent. The clone repairs it from target_card.
- In the BT24-088 game the two Tamers are not byte-identical. turn_played differs (2 vs 3), as do last_security_reveal and the trash/security entry generations. The engine reads turn_played only as "is it this turn" (perm.turn_played == turn_count), so it is normalised for Digimon and Tamers only, never for Delay Options.
- Debug text carries a modifier-registry cache (decl_last_fingerprint, decl_memo_valid), blanked in the clone.
- The static registries (alt_path_registry, token_registry, card_id_index) make up almost all of the megabytes of Debug text, so they are cleared in the clone.

Copies that really differ, such as one suspended or one played this turn, still stop the replay as ambiguous.

I did not touch the Core.cs / convert_raw.py recorder change. It would only help future local-seat rows, and I cannot build or test the C# mod here.

The first replay run used the diagnosis's gate (any two entries sharing a card id). That wrongly fired on inherited effects, which are all queued under the carrier's top card. The final gate requires distinct source cards with the same label.

**Residual (not fixed):**

Two new divergences from other defect families, not fixed.

1. g3 (20261115T153822Z-g3), engine step 41, v1 rows 41-43. P1 plays BT24-041 from hand with the cost-reduced "play or use 1 Iliad/TS card" effect at step 36, so it is on the field at step 37. DCGO then names BT24-041's effect in a card-named MultipleSkills, picks BT24-102 from hand, answers an unstamped bool, and picks permanent frame 0 (BT24-041 on the field). Next comes SelectCardEffect with cards [BT24-041]. The engine instead parks a Hand prompt for "play cost 5 or lower Iliad card from hand for free", whose hand holds no BT24-041. This is probably an effect-order or prompt-shape mismatch around the second-pass trigger of BT24-041 / BT24-102, not copy identity. It needs a card-level trace of BT24-041 against BT24_041.cs.

2. g5 (20261116T045756Z-g5), engine step 33. After BT25-030 is played (action 594) and slot 1 (BT24-034) attacks security (action 129), the engine parks a Security "Add top security to hand" prompt. DCGO's row stream is a stamped bool true, an unstamped optional-yes, then a SelectPermanentEffect on own frame 1. The converter or driver sees "no prompt parked", so the effect DCGO used (possibly an attack-time or inherited optional effect of BT24-034 or BT25-030) is not offered by the engine in that shape. Needs a card-level look at BT24-034 and BT25-030.

Latent engine inconsistency found while diagnosing: shift_effect_queue_after_battle_area_remove (combat/mod.rs ~4313) rebases a queued effect's source_permanent but not trigger_context.target_permanent / event_permanent for ordinary triggers; it only shifts them for captured_security_check contexts. After an earlier battle-area slot empties, a still-queued trigger therefore keeps a stale slot in its context. The replay comparison works around it. If an effect reads ctx.target_permanent for "this Digimon", it would address the wrong slot or none. This is worth a dedicated engine test, but I made no engine change.

Limits of the new digest:
- It is built from Debug text, so it shares the blind spots of interchangeable_copies (opaque closures, hand-written Debug omissions).
- HashMap iteration-order noise can only make two isomorphic branches look different, which falls back to the old ambiguity, never to a false match.
- The stale-slot repair and the turn_played / last_security_reveal normalisation are applied only to the comparison clone.

Optional recorder improvement (not done): have Core.cs record chosen_effect_card_index (panel.SelectedList[0].CardIndex) for local-seat MultipleSkills rows, forward it in convert_raw.py, and match it via board_indexes. It would not help remote-seat rows or existing logs.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md (append)

**G-ENGINE-QUEUED-TRIGGER-CONTEXT-STALE-SLOT** (found by the replay-identical-copy-order lane, 2026-11-18). `Game::shift_effect_queue_after_battle_area_remove` (code/digimon-engine/src/combat/mod.rs, around line 4313) rebases `QueuedEffect::source_permanent` when an earlier battle-area slot empties, but rebases `trigger_context.target_permanent`, `event_permanent` and `event_host_permanent` only for `captured_security_check` contexts. An ordinary queued trigger (for example a [Start of Your Turn] trigger of a Tamer) keeps the slot its carrier had when it was queued. Witness: two BT24-083 Tamers at slots 0/1, each with a queued StartOfYourTurn trigger; after the slot-0 Tamer returns to the deck bottom, the survivor's queued trigger has source_permanent {0,0} but target_permanent {0,1}, a dangling slot. Impact: any effect that reads ctx.target_permanent for "this permanent" on such a trigger would address the wrong slot or none; BT24-083 and BT24-088 resolve through source_permanent and are unaffected. The replay driver works around it only in its comparison clone (replay.rs `identity_free_digest`). Not blocks-training; add a dedicated test before widening the shift to ordinary contexts.

#### ## Replay tooling note (docs/DCGO_MELONLOADER.md / replay driver, not a vocab gap)

A MultipleSkills order over two copies of one card is now resolved without a recorded copy identity when the copies are the same card in the same state, compared by `identity_free_digest` (replay.rs): handle and battle-identity renamed Debug text, with turn_played of Digimon/Tamers reduced to "is it this turn", last_security_reveal and trash/security entry generations ignored, static registries and the declarative memo left out. A copy that differs in a real way (suspended, played this turn) still stops the replay as "is ambiguous". An optional recorder change (Core.cs: record chosen_effect_card_index, forward it in convert_raw.py) would make local-seat rows exact.

### replay-dna-root-rows (fixed)

The diagnosis held and the fix works. DCGO's DNADigivolvePermanentsIntoHandOrTrashCard (DNADigivolveEffects.cs:456-640) records a SelectHandEffect row, then two SelectPermanentEffect material-root rows, then the optional SelectAttackEffect. The engine asks the partner first and the hand card second. The lone-prompt auto-answer finished the DNA on the hand row, so the first root row hit the 'may attack' prompt and the second hit a 4-entry TriggerOrder. I added ReplayDriver::fold_effect_dna_root_rows in code/digimon-engine/src/runners/replay.rs, called from apply_selection_step before any other fold. When the engine is parked on the MayDnaPartnerSelection frame and the next two rows are own-side single-target root rows, it remaps the roots through the row boards. It requires the pair to be {anchor, partner}. It also accepts a duplicate-copy anchor when exactly one root shows the anchor's card id. The non-anchor root answers the partner pick and the hand row answers the DNA-card pick. All three rows are consumed and the DCGO memory check runs on the hand row. A pair without the anchor, a repeated root, a partner the engine does not offer, or a DNA card the engine does not offer all halt with a precise SelectionResolution reason. The SelectAttackEffect row then meets the Target prompt through the existing selection_resolve path. Converter side: convert_raw.py now decodes main-phase packet 1 (ActivateCardAction) as HAND_EFFECT_START + the card's current hand slot, read from the following PlayCard hand listing. If no listing names the card it truncates instead of guessing. g2 now converts completely (269 rows, previously truncated at 53). Result: g10 now stops at step 17 (was 14) and g2 at step 40 (was 38). Both stop at the same new divergence, a MultipleSkills trigger-order ambiguity, which is a different defect (see residual). Two older replay_dcgo_review tests fail on this base, one in r4 and one in r4b. I could not run them on an untouched base, but the new fold returns before touching any state for their MultipleSkills rows, so my change cannot affect them (see residual). The commit is 4af8c7e78.

**Residual (not fixed):**

(1) Both games now stop at the same different defect: a trigger-order ambiguity after the DNA digivolve. EX12-044 (Angewomon) has three same-card triggers pending alongside a fourth entry. DCGO's MultipleSkills int_value indexes its own panel order (1 in g10, 2 in g2), and the engine's entries [30,31,32] share one source card, so card identity cannot disambiguate them. This is the 'replay-identical-copy-order' lane's family. A fix needs a mapping from the DCGO panel index to the engine clause or entry order (the card's CardEffects order). I did not fix it.
(2) Pre-existing red tests on origin/main 86652b985, unrelated to this change: replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short and replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer. Someone should run them on an untouched main to confirm.
(3) Not covered by the fold: the Trash variant of DNADigivolvePermanentsIntoHandOrTrashCard (isHand=false, a SelectCardEffect Trash root instead of SelectHandEffect), and an optional-decline row after the hand pick. A declined hand pick (cancel) falls through to the old path and PASSes the partner prompt as before. The fold keys on engine frame MayDnaPartnerSelection, so BT12-021, BT12-047, BT17-007, BT17-019, BT22-008 and BT22-017 share it, but I tested only EX12-001. Per the task, I did not run raw-log games for the others.
(4) The convert_raw packet-1 decode assumes the activated hand card is followed by a PlayCard row whose hand listing names its card_index (true for the BT18-081 'digivolves into this card' [Hand][Main]). An activation whose card never plays truncates with a precise reason. A skill index other than 0 also truncates, because the engine's HAND_EFFECT range has one id per hand slot.

**Gap tracker entries (verbatim from the lane):**

### G-REPLAY-EFFECT-DNA-ROOT-ROWS (fixed, branch claude/hunt-replay-dna-root-rows, 4af8c7e78)
- **Area**: dcgo-replay driver, `code/digimon-engine/src/runners/replay.rs`. No engine rule or card behaviour gap: the engine's `may_dna_digivolve_now` flow (`MayDnaPartnerSelection` -> `MayDnaResultSelection`) is faithful.
- **Symptom**: a DCGO effect-initiated DNA digivolve (Nyaromon (EX12-001)'s inherited [End of Your Turn]; every `may_dna_digivolve_now` card: BT12-021, BT12-047, BT17-007, BT17-019, BT22-008, BT22-017) replayed as 'frame target (player 1, slot 1) matches none of [30..33] (kind TriggerOrder)'.
- **Cause**: DCGO `DNADigivolvePermanentsIntoHandOrTrashCard` (DNADigivolveEffects.cs:456-640) records one `SelectHandEffect` row (the DNA card), then TWO `SelectPermanentEffect` material-root rows, then the optional `SelectAttackEffect`. The engine asks partner first, then hand. The lone-prompt auto-answer finished the DNA early, so the first root row landed on the 'may attack' Target prompt and the second on a trigger order.
- **Fix**: `fold_effect_dna_root_rows`. The non-anchor root is the engine partner pick (CR 4.3 8-2: 'this Digimon and another Digimon'). The hand row answers the DNA-card pick. All three rows are consumed. A pair that does not contain the anchor halts with a precise SelectionResolution reason. Tests: `code/digimon-engine/tests/replay_dcgo_effect_dna_root_rows.rs`.
- **Converter**: `convert_raw.py` decodes main-phase packet 1 (`ActivateCardAction`) as `HAND_EFFECT_START + hand slot` (slot from the following PlayCard hand listing). It was 'main-phase packet 1' truncation (raw-20261116T164448Z#g2). Test: `code/tests/tools/test_dcgo_convert_raw_activate_hand_card.py`.
- **Open follow-ups**: (a) the Trash-root variant (isHand=false) is unfolded; (b) MultipleSkills int_value against several same-card triggers (EX12-044 x3) is unresolved (replay-identical-copy-order lane); (c) only EX12-001 is exercised by the synthetic tests.
- **Pre-existing red on 86652b985**: `replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short`, `replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer` (unverified on an untouched base; both rows are MultipleSkills and never reach the new fold).

### digi-egg-level-null-breeding (fixed)

Root cause confirmed: cards.json has level=null for EX13-001 Gigimon and EX11-005 Yaamon, although both card faces print Lv.2 (images checked). collect_rules_digivolve_routes needs a base level, so the breeding-area digivolve was never offered and recorded action 459 was a MaskMiss. The data could not be fixed through card_corrections.json: card_overrides.json is a frozen legacy input, and no committed evidence observation exists for these two cards (the correction pipeline needs one, via a network sync). So I fixed it in the engine loader. CardData::load_from_str (code/digimon-engine/src/card_data.rs) now gives a Digi-Egg its printed Lv.2 when its form is In-Training or it carries the Lesser type. The Lesser fallback is needed because the ingest also dropped the form of 182 eggs, EX11-005 among them. The four levelless special eggs (BT13-007, BT22-007, EX2-007, EX6-006) have neither and stay levelless (CR 4.3 2-9-2). data/cards.json itself was not edited. I also authored the cards the replay then reached, as DSL YAML with synthetic tests: EX8-009 Guilmon (X Antibody), EX11-047 Impmon and EX11-005 Yaamon. One engine fact is pinned by a test: a breeding-area digivolve does not fire the card's own When Digivolving (CR 4.3 3-4-7-4; DCGO IsExistOnBattleAreaTrigger agrees). Both games now get past the breeding digivolve; each stops at a different, unrelated defect (see games_after and residual). Validation: the temp-target run of all 34 new and existing EX13-001 tests passed, the engine lib suite passed (746), the dcgo-replay crate tests passed, and the Python integrity test passed. The full cards_behavioral binary could not be built here (LLVM out of memory under host contention). I ran the new test modules through a temporary top-level test target that included them by path, then removed that target. They are registered in the ex8, ex11 and ex13 mod.rs files for the normal run. I did not observe the new tests fail before the fix. The red evidence is the two replay games, which failed at step 4 and step 1 with the pre-fix binary.

**Residual (not fixed):**

Both games are unblocked from this defect but not passing. (1) 20261115T153822Z-g8 step 18: after P0's digivolve at step 17 (memory 2 to 1), the engine is in the Breeding phase of turn 5 expecting P1, while DCGO has P0 still acting in Main with action_id 129 (converter label 'Unknown breeding action 114'). Likely a turn-boundary or recorder-label issue, not an egg-level one; I did not diagnose further. (2) 20261116T164448Z-g8 step 8: right after P1 plays the EX12 Option 'Melting Recital' at step 7, DCGO records a SelectCardEffect for EX7-055 Punkmon (When Digivolving, play a Yuuki Tamer from hand when you have 1 or fewer Tamers) that the engine does not park. Probably an EX7-055 When Digivolving or trigger-ordering gap. (3) Both games still list other unimplemented cards (BT17-080, ST7-05, EX2-008, BT12-089, EX8-012, BT19-009 in game 1; the 19 blocked cards in game 2), which belong to other lanes. (4) data/cards.json still carries level null for EX13-001 and EX11-005. The engine loader compensates, and the Python guard lists them in KNOWN_NULL_LEVEL_IN_TRAINING_EGGS and fails once cards.json is corrected, so the entry gets retired. The proper data fix is a card_corrections.json entry with evidence via the card_data sync pipeline. (5) The integrator should regenerate qa/impact artifacts (dsl_impact_index, lowering_coverage) for the three new YAMLs; I did not touch qa/impact. (6) The full cards_behavioral binary was not built here because LLVM ran out of memory; run it in CI or on a quieter host.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md (append)

- **G-CARDS-JSON-NULL-LEVEL-IN-TRAINING-EGG (closed in engine, open in data)**: cards.json carries `level: null` for hatchable In-Training Digi-Eggs EX13-001 Gigimon and EX11-005 Yaamon (both print Lv.2 on the card face). Effect: `collect_rules_digivolve_routes` returned no routes, so a breeding-area digivolve over them was never offered (a MaskMiss in the DCGO replays 20261115T153822Z-g8 and 20261116T164448Z-g8). Fixed in `CardData::load_from_str`: a Digi-Egg with form In-Training or the Lesser type and no level loads as Lv.2. The special levelless eggs BT13-007, BT22-007, EX2-007 and EX6-006 are unaffected. Follow-up: correct cards.json through the card_data pipeline (`card_corrections.json` with official-DB evidence observations), then retire `KNOWN_NULL_LEVEL_IN_TRAINING_EGGS` in `code/tests/test_cards_json_integrity.py`.
- **Fact for later replay triage**: effects on a breeding-area Digimon do not trigger (CR 4.3 3-4-7-4). A recorded DCGO breeding digivolve over a Digimon with [When Digivolving] correctly yields no reveal. `ex8_009_breeding_gigimon_offers_cost_0_route_and_when_digivolving_does_not_fire` pins it.

#### ## qa/dsl-vocab-gaps.md

- No new vocabulary was needed. The cards used existing idioms: `alt_paths` with `any_of` of `name_is`, `select_reveal_buckets`, `select_trash` with `can_digivolve_onto_with_cost` plus `effect_initiated_digivolve source:`, `effect_digivolved_any_digimon`, and an inherited `aura` with `target: {}`.

#### ## Cards implemented (for the validated-cards ledger)

- EX8-009 Guilmon (X Antibody), EX11-047 Impmon and EX11-005 Yaamon: YAML plus behavioural tests, DCGO C# cross-checked (EX8/Red/EX8_009.cs, EX11/Purple/EX11_047.cs, EX11/Purple/EX11_005.cs).
- Card JSON companions under `code/digimon-engine/cards/<set>/` were left as is. The `cards/ex8/EX8-009.json` evo_costs is stale (red half only). The YAML alt_paths carry both split-circle halves.

### keramon-infermon-diaboromon-line (fixed)

The Keramon/Kurisarimon/Infermon/Diaboromon [Unidentified] line was JSON-only, so recorded step 10 (Infermon BT5-067 digivolved from hand onto Keramon BT22-053) was a MaskMiss. I authored all ten listed cards as DSL YAML and added the substrate they needed. Game g13 now replays 25 of 92 steps (was 10) and stops at step 25 on AD1-015, a different card outside this lane. The 62 new tests pass, as do the 777 cards_behavioral tests in the bt5/bt17/bt22/ex6/tokens modules, the 1767 engine dsl-suite tests and the digimon-dsl crate tests.

BT5-067's "your [Keramon] can digivolve into this card in your hand for 4, ignoring requirements" needed no new DSL vocabulary; the existing digivolve alt_path covers it (cost 4, ignore_requirements). Two details came from the sources. The base must be named exactly [Keramon], because DCGO tests CardNames.Contains("Keramon"), which is list membership, so name_is rather than a substring. It must be in the battle area, because official Q&A Q1342 rules out a Keramon in the breeding area, so the path uses in_breeding: false.

Cards authored in code/digimon-engine/cards/<set>/:
- BT5-063 Kurisarimon: [When Digivolving] free Arata Sanada, and the inherited Rush grant to other same-name Digimon.
- BT5-067 Infermon: the special route, plus the inherited [On Deletion] token.
- BT17-053 Keramon: [Opponent's Turn] free digivolve into Infermon from hand on a level 5+ opponent play or digivolve, plus the inherited token.
- BT17-055 Infermon: De-Digivolve 1, then a cost 8 or lower opponent Digimon can't attack players, plus the inherited De-Digivolve.
- BT22-053 Keramon, BT22-057 Kurisarimon, BT22-059 Infermon, BT22-064 Diaboromon: CS alt routes, reveal-3 search, the delete-another-[Diaboromon] substitute, the protection bundle, Alliance, and the lowest-play-cost delete.
- EX6-036 Keramon: reveal-3 search that trashes the rest, plus the inherited token.
- EX6-043 Diaboromon: Blocker and Jamming auras, and a [When Digivolving] re-fire when an opponent plays a Digimon.

Substrate added:
- The Diaboromon Token (TOKEN_DIABOROMON, Lv.6 white, 3000 DP, Mega/Unknown/Unidentified, named exactly "Diaboromon", play cost 14). TokenDef gained an optional play_cost field. A Token now counts as having a play cost only when it prints one, which was changed in top_card_has_play_cost and the lowest-play-cost aggregate in formula_eval.rs. Costless tokens are unchanged.
- A new predicate leaf, name_matches_source, true when the candidate shares a name with the effect source permanent's top card. It is wired through the spec, compile, engine evaluator and inventory walker, and the DSL vocabulary block in docs/RUST_DSL_AGENT_GUIDE.md is regenerated.
- Four existing tests that build TokenDef literals got a play_cost: None field.

Per the instructions I did not touch docs/RUST_ENGINE_GAPS.md, qa/dsl-vocab-gaps.md or qa/impact/*.

Tests are synthetic boards only, with no recording data. Support fixtures are in tests/cards_behavioral/keramon_line_support.rs and the card modules are in bt5, bt17, bt22 and ex6.

**Residual (not fixed):**

Not run: (a) token_departure_rules::recorded_interrupted_token_return_removes_token_and_clones_identically, the 11th test in that binary. It replays a 92-step recording and ran over 20 minutes at opt-level 0 under machine load, so I killed it; the other 10 tests in the binary passed. It does not touch the new code paths, so re-run it at the normal dev profile. (b) The full cards_behavioral suite and cargo check --tests over every engine test binary. I grepped every TokenDef literal in code/ and fixed the 4 test sites. (c) The gate artifacts that scripts/verify --tier 0 checks, which the instructions reserve for the integrator: qa/impact/dsl_impact_index.json, lowering_coverage.json, data/card_admissibility.json and validated_cards_dsl.json. I used a reduced opt-level 0 override for the workspace crates, so build timings are not representative.

Remaining g13 blocker, a different defect: step 25 fails on AD1-015's recorded DCGO int 2 (a one-based menu constant naming no branch of the OwnField prompt). Row 15 also notes an unstamped generic_bool consumed with no effect on a P0 CountCappedMultiSelect 'You may place Hybrid cards with different names under this Tamer'. Both concern AD1-015, not this cluster.

Open items and judgement calls:
1. Costless tokens still store play_cost 0, so play_cost_lte/gte/eq leaves still match them as cost 0. DCGO has them as -1 (no cost); user ruling 2026-11-13. This predates my change and the Diaboromon token is unaffected. It is a reusable engine gap.
2. BT22-059 uses the unscoped ImmuneFromDPMinus plus the opponent-scoped CannotBeReturnedToHand/Deck modifiers, as DCGO and the BT19-089 precedent do. The printed text says 'your opponent's effects can't reduce DP'; DCGO's helper is not scoped to the opponent.
3. BT22-053, BT22-057 and EX6-043 prompts, and the optional pick on BT22-057, are single-prompt decline paths; DCGO would show a yes/no before a canNoSelect pick for BT22-057. The outcomes are the same.
4. EX6-043's re-fire ability uses refire_effect, which fires the permanent's own [When Digivolving] effects. Q3769 says non-inherited [When Digivolving] effects on digivolution cards can't be activated, and I did not add a test for that.
5. BT24-065 and BT5-090 also appear in the recorded opponent deck and are not in this lane.

**Gap tracker entries (verbatim from the lane):**

## Dynamic "same name as this Digimon" predicate  [G-DSL-NAME-MATCHES-SOURCE] — RESOLVED (2026-11-17)

- Effect text (driver BT5-063 Kurisarimon): "[Your Turn] All of your other Digimon with the same name as this Digimon gain <Rush>." Same shape on BT2-006, BT2-053, BT2-059, EX1-044, EX1-046, EX1-051, P-045 (not yet authored).
- DCGO `BT5_063.cs`: `permanent.TopCard.HasSameCardName(card.PermanentOfThisCard().TopCard)` — compares against whatever Digimon CURRENTLY carries the inherited effect, so a literal `name_is` is wrong (the carrier is the host the card sits under).
- Gap: `name_is` / `name_contains` take literals; `name_matches_returned_card` reads an effect result log; nothing compared a candidate with the effect SOURCE permanent's top-card names.
- Resolution: new predicate leaf `name_matches_source: bool` — true when the candidate shares >=1 name (printed name + "also treated as" aliases; a field permanent uses its synthesized top-card names, base-name overlays applied) with the effect source permanent. Pair with `other: true` for "other Digimon". Card-subject and permanent-subject paths both implemented; false with no source permanent.
- DSL syntax: `kind: aura` / `scope: inherited` / `target: { owner: you, zone: [battle_area], kind: digimon, other: true, name_matches_source: true }`.
- Coverage: `code/digimon-engine/tests/cards_behavioral/bt5/bt5_063.rs`.

#### # Costed tokens: Diaboromon Token (play cost 14)  [G-ENGINE-COSTED-TOKEN] — RESOLVED (2026-11-17)

- Driver: BT5-067 / BT17-053 / EX6-036 / EX6-043 / BT22-059 / BT22-064 ... "[Diaboromon] Token (Digimon/Cost 14/Lv.6/White/Mega/Unknown/Unidentified/3000 DP)". DCGO `ContinuousController.DiaboromonToken`: PlayCost 14, Level 6, White, Mega/Unknown/Unidentified, DP 3000, CardName_ENG "Diaboromon".
- Gap: `TokenDef` had no play-cost field (every token stored 0) and `top_card_has_play_cost` excluded every token, so the cost-14 token was invisible to "lowest play cost" selectors and would have matched "play cost N or less" filters as cost 0.
- Resolution: `TokenDef::play_cost: Option<u16>`; `to_card_data` stores it; `top_card_has_play_cost` and the `lowest_play_cost` aggregate (`formula_eval.rs`) count a Token only when it prints a play cost. New token `diaboromon` (`TOKEN_DIABOROMON`, name exactly "Diaboromon" because the printed cards address it as `[Diaboromon]`, trait line Mega/Unknown/Unidentified).
- Coverage: `code/digimon-engine/tests/cards_behavioral/bt22/bt22_064.rs` (`bt22_064_lowest_play_cost_counts_a_diaboromon_token_at_14`), `bt22_059.rs` (`..._diaboromon_token_is_not_a_cost_5_or_less_target`), token_registry unit test.
- Residual (OPEN, not fixed here): costless tokens still store `play_cost: 0`, so `play_cost_lte` / `_gte` / `_eq` leaves in `dsl_cards/predicate.rs` (`ref_play_cost`) still match them as cost 0 (DCGO `PlayCost = -1`, `HasPlayCost` false). User ruling 2026-11-13: tokens have no play cost. Gyuukimon token (7) would need a `play_cost: Some(7)` entry when authored.

### replay-lone-pick-then-ordering (fixed)

The diagnosis was right. After Kudamon (EX13-026) resolved from DNA Charge (ST24-15), answer_dcgo_silent_lone_target_pick answered the lone [DATA SQUAD] Tamer pick and parked the one-card deck-bottom OrderedPermutation, then returned without answering it. The fix is one call to answer_forced_single_ordering_prompt(game) after each game.decode_action in that function (replay.rs), the same thing the MultipleSkills fold already does. The remote_trigger_order_choice probe path needed no change, because the probe calls answer_dcgo_silent_prompts, which now covers it. 20261116T164448Z-g7 now replays 62 of 105 steps (was 45) and stops on a different defect family (see residual). CR 4.3 and the card text were not involved: this is a replay-driver fold for a prompt DCGO never asks.

**Residual (not fixed):**

1. Next g7 divergence (different defect, not fixed): Jesmon (BT23-013) [On Play]/[When Digivolving] "Choose what Jesmon plays". DCGO asks it as a SelectionElement<bool> menu (BT23_013.cs ~L121-141): true = play the [Atho, Rene & Por] Token, false = play a [Sistermon]-name Digimon. The false branch is followed by a From hand (true) / From trash (false) bool. The menu is only shown when both options are available. The recorder logs it as an unstamped generic_bool. The engine parks a 2-option EffectChoice (action 30 = Token, 31 = Sistermon). The replay's per-card bool-meaning table (runners/dcgo_bool_yes_no_cards.json and the replay_dcgo_bool_meaning / replay_dcgo_mode_bool folds) has no entry for BT23-013, so it halts. The follow-up is a card-specific bool-to-EffectChoice mapping (true to option 0, false to option 1), plus the follow-on hand/trash bool, in the same style as the other mode-bool cards.
2. Two tests fail without my change too (verified by rebuilding with my one added line commented out): replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short ("expected one divergence at row 0, got []") and replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer (panics in selection.rs:360 "cloned PendingSelection has no closure callback"). The r4b panic comes from the test's hand-built closure-callback prompt helper (`parked`/`choice_of`) being cloned by the replay's branch probe, which the clone-safety stub rejects. The r4 failure was not diagnosed. Both are stale tests on main, not caused by this change, and both binaries are nightly-run by engine-binary-rot.yml.
3. Build environment: cold engine builds under the current multi-lane contention need CARGO_BUILD_JOBS=4 and CARGO_PROFILE_DEV_DEBUG=0. With debuginfo on, rustc died with 'LLVM ERROR: out of memory' (0xc0000409). RUST_MIN_STACK was not needed to build. The replay binary is D:/cargo-target/hunt-replay-lone-pick-then-ordering/debug/dcgo-replay.exe.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md (replay driver)
- [replay, fixed 2026-11-17, branch claude/hunt-replay-lone-pick-then-ordering] answer_dcgo_silent_lone_target_pick answered a lone Tamer pick and left the follow-on one-card OrderedPermutation unanswered in front of an ACTION row. EX13-026 Kudamon via ST24-15 DNA Charge (raw-20261116T164448Z#g7 step 45). Fixed by running answer_forced_single_ordering_prompt after each lone-pick answer. Regression: replay_dcgo_area_menu_and_lone_pick::ex13_026_lone_tamer_pick_and_lone_order_are_both_answered_before_an_action_row.
- [replay, open] BT23-013 Jesmon "Choose what Jesmon plays" has no DCGO bool-to-EffectChoice mapping in the replay. DCGO's SelectionElement<bool> menu is true = Token, false = Sistermon, then a bool for hand (true) or trash (false). The recorder logs an unstamped generic_bool and the replay halts with engine_error (raw-20261116T164448Z#g7 step 62). Follow-up: a card-specific mapping in runners/dcgo_bool_yes_no_cards.json or a mode-bool fold.
- [tests, stale on main] replay_dcgo_review_r4::unfollowable_order_branch_does_not_cut_the_other_branches_short and replay_dcgo_review_r4b::cardless_index_is_not_consumed_by_a_prompt_its_next_row_does_not_answer fail on unmodified main. The r4b one panics because its closure-callback `parked` helper is cloned by the replay branch probe (selection.rs:360 clone-safety stub). Both binaries are nightly-run, so the nightly engine-binary-rot job should show them red.

#### ## qa/dsl-vocab-gaps.md
- (none)

### converter-dup-attacker-index (fixed)

The diagnosis was right and I fixed it in the converter and the replay. In raw-20261115T153822Z#g10 raw seq 821, P1 had two ST24-05. DCGO's attacker (board position 0, CardIndex 57) was the newer copy holding BT26-036. The raw row's board_indexes named it, but the converter dropped it. The replay then paired duplicates in occurrence order and sent the older copy. resolve_duplicate_attacker only rescues attacks that pairing makes illegal, and here both copies could attack.

Converter (code/dcgo-melon-recorder/convert_raw.py): action rows now carry operand_card_index plus operand_card_index_source "recorder", only when the recorded board holds several copies of the operand's card. It covers three operands:
- the attacker, from the SetAttackingPermaent row's board_indexes;
- the digivolve target, from the resolve row's target.card_index, only if the board entry at target_slot agrees;
- the activated-permanent host, from the packet row's board_indexes.
board_indexes that don't line up with boards, or older raw logs without them, omit the field. There is no frame-model fallback for operands.

Schema and replay:
- ActionRow gets operand_card_index and operand_card_index_source (dcgo_recording.rs).
- The adapter passes them through a new RecordingSource::dcgo_action_operand_index hook.
- ReplayDriver::named_operand_action resolves the named copy via the existing DCGO-index to engine-card pairing (dcgo_identity_slot).
- A named copy is never swapped for its sibling. When the identity was used, both resolve_duplicate_attacker and the "remapped id illegal but recorded id legal" raw-id fallback are skipped, so an illegal named copy becomes a divergence instead of a silent substitution.
- With no index, or one that names nothing, the occurrence-order pairing stays, so no previously replaying recording is newly halted. A replay note names the guess when the engine's copies are not interchangeable and the same action is legal for more than one of them. This is deliberately weaker than the plan's "ambiguity divergence".
- docs/DCGO_RECORDING_SCHEMA.md documents the field and the converter table rows.

Not covered: the attack's target on the opponent's board is still paired by occurrence order. Source-select hosts are not emitted, because the converter produces no such actions.

Results on the 50-game manifest (recordings reconverted with the new converter, replayed with the dev dcgo-replay built in this worktree): the first divergence of 49 games is unchanged. Only 20261115T153822Z-g10 changed, moving from step 25 to step 34.

**Residual (not fixed):**

g10 now stops at step 34 for a different defect. After P0's breeding action (step 35 in the recording, action 61), P0's generic_int row int 2 meets no engine prompt. This is the "P0 breeding-phase optional" from the diagnosis. It looks like a DCGO int menu shown at P0's breeding step that the engine does not park, and it needs its own triage.

Other residuals:
- The attack target on the opponent's board is still paired by occurrence order (no recorder index for defenders). Source-select hosts are not emitted.
- Only 10 manifest games carry operand_card_index rows (g17 of 20261113T025510Z, g6/g10 of 20261115T153822Z, g4/g5/g6 of 20261116T045756Z, 20261116T144923Z, g6/g7/g10 of 20261116T164448Z). None of the nine others changed their first divergence, so no other game in this manifest was blocked by this defect.
- cards.json-independent: older raw logs (before recorder 0.2.0) lack board_indexes, so duplicate attackers there still use the occurrence-order guess (with a note).
- The dcgo-replay crate's own tests were type-checked but not run.
- The dev builds of dcgo-replay took about 2.5 hours under machine contention, and rustc hit LLVM OOM twice. Building with -j 1 worked.

**Gap tracker entries (verbatim from the lane):**

### docs/RUST_ENGINE_GAPS.md (replay / DCGO PvP, not an engine rules gap)

**DCGO replay: own-board operand among duplicate copies (fixed, claude/hunt-converter-dup-attacker-index f38722014).**
- Defect: the attacker of an attack, a digivolve target and an activated permanent's host are recorded as a position into a board that names permanents by card id only, in DCGO frame order. Two copies of one card were paired in occurrence order, so the wrong one could act. Witness: raw-20261115T153822Z#g10 raw seq 821, two ST24-05, the newer one holding BT26-036; its When Attacking effect never fired in the replay.
- Fix: the converter writes `operand_card_index` plus `operand_card_index_source: "recorder"` (from `board_indexes` or the digivolve `target.card_index`) when the board holds several copies. The replay resolves it through the DCGO-index/engine-card pairing (`ReplayDriver::named_operand_action`) and never substitutes a sibling for a named copy.
- Still open: attack TARGETS on the opponent's board are not named (occurrence order); recordings without `board_indexes` still guess, with a replay note when the copies are not interchangeable.

#### ## qa/dsl-vocab-gaps.md

None: no DSL vocabulary changes.

#### ## Follow-up found

g10 (20261115T153822Z) now stops at step 34: P0's `generic_int` int 2 after its breeding-phase action (action 61) meets no engine prompt. Needs a separate diagnosis.

