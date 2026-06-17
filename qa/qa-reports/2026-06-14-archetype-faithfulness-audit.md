# Archetype Faithfulness Audit — 2026-06-14

Capstone re-verification of existing `tests/archetypes/` interaction suites. Each
named combo's asserted behavior was cross-examined against the printed card text,
`Digimon TCG resources/general_rule.pdf`, and the DCGO C# reference
(`$BASE_DCGO/.../<CARD_ID>.cs`). This audit does **not** edit engine code or
re-implement cards; confirmed divergences would route to the shared gap trackers
(`docs/RUST_ENGINE_GAPS.md` / `qa/archetype-qa/engine-gaps.md`).

The audit covers two batches authored on this date:

- **Batch 1 (Medusamon, Rocks)** — full combo-by-combo re-verification; both
  **FAITHFUL**.
- **Batch 2 (DNA Omnimon, Omnimon ACE)** — the two Omnimon-family DNA-digivolve
  archetypes; every executable combo is **faithful**, but each carries one or more
  combos **blocked** on already-tracked engine-primitive gaps (the DNA-spine
  `<Activated Digivolve>` execution route and the place-self-as-Delay play-path),
  so neither reaches a clean FAITHFUL — both are **INSUFFICIENT_COVERAGE** (no
  divergence; blocked/disabled combos remain).

## Summary

| Archetype | Verdict | Combos faithful | Divergent | Untested/Blocked | Findings filed | Suite |
|-----------|---------|:---------------:|:---------:|:----------------:|:--------------:|-------|
| Medusamon | **FAITHFUL** | 6 / 6 | 0 | 0 | 0 | 15/15 pass |
| Rocks | **FAITHFUL** | 5 / 5 | 0 | 0 | 0 | 11/11 pass (archetypes binary 26/26) |
| DNA Omnimon | **INSUFFICIENT_COVERAGE** | 4 / 5 | 0 | 1 (Combo E) | 0 | 8 run, 8 pass (1 `#[ignore]`d) |
| Omnimon ACE | **INSUFFICIENT_COVERAGE** | 4 / 6 | 0 | 2 (Combo 1, BT22-013 spine) | 0 | 10 run, 10 pass (2 `#[ignore]`d) |

Medusamon and Rocks are **FAITHFUL**: every audited combo reproduces the real TCG
behavior vs card text + `general_rule.pdf` + DCGO; no divergence; all tests pass.
DNA Omnimon and Omnimon ACE have **no confirmed divergence either** — every combo
whose pieces are executable resolves faithfully against the same three sources — but
each has combos that cannot be exercised on the real play path because of a tracked
engine gap (already logged; no new finding to file), so their verdict is
**INSUFFICIENT_COVERAGE** rather than FAITHFUL. All test issues recorded below are
**minor** test-quality / scaffolding observations — none invalidate a green status or
warrant a gap-tracker finding.

---

## Medusamon

- **Model doc:** `qa/archetype-qa/medusamon-model.md`
- **Test file:** `code/digimon-engine/tests/archetypes/medusamon.rs`
- **Verdict:** **FAITHFUL** (6/6 combos faithful, 0 findings, 0 untested)

Medusamon is a Petrification-token / security-trash control shell: it hands the
opponent `Petrification Tokens` that, on deletion, trash the opponent's top
security, which in turn feeds a `[On Opponent's Security Removed]` engine (extra
removal, inherited memory gain, and `<=5000`-DP free-plays). The audit confirmed the
whole chain resolves faithfully — including the token-controller / owner asymmetry on
`OnDeletion` and the additive per-buried-source memory payout.

### Combo-by-combo

| Combo | Status | Covering tests |
|-------|--------|----------------|
| Petrification security-trash loop [BT24-017, BT24-018] | **FAITHFUL** | `petrification_token_deletion_trashes_opponent_security`, `petrification_token_deletion_chains_into_styracomon_removal`, `petrification_token_deletion_no_security_does_not_chain` |
| Owen digivolve buff + extra attack [BT24-082] | **FAITHFUL** | `owen_digivolve_buffs_dragonkin_ally_and_grants_attack`, `owen_digivolve_does_not_buff_non_trait_ally` |
| Raid target-change -> Lamiamon security trash [BT24-011, BT21-025] | **FAITHFUL** | `raid_target_change_with_lamiamon_trashes_opp_security`, `raid_target_change_without_trait_permanent_trashes_nothing` |
| Security-removal feeds the memory engine (snowball) [BT24-008, BT24-012] | **FAITHFUL** | `real_stack_snowballs_two_memory_on_one_security_removal`, `petrification_token_deletion_feeds_inherited_memory` |
| Lamiamon inherited free-play (<=5000) on opp-security-removed [BT21-025] | **FAITHFUL** | `lamiamon_inherited_free_plays_small_dragonkin_on_security_removal`, `lamiamon_inherited_does_not_free_play_oversized_dragonkin` |
| EX11-012 token-shield (any-owner token) [EX11-012] | **FAITHFUL** | `ex11_012_survives_by_deleting_opponents_petrification_token` |

**Evidence highlights**

- **Petrification loop:** `petrification.rs` `OnDeletion` sets `owner = ctx.player`
  (the token's controller is the opponent) and calls `trash_top_security(owner)`;
  `trash.rs:165-205` pops the opponent's top security to trash and fires
  `fire_security_removed_observers` (`helpers.rs:63`). BT24-018's
  `on_opponent_security_removed / all_turns / once_per_turn / optional ->
  select_opponent_permanent -> delete_permanent` clause matches DCGO `BT24_018.cs`
  `OnLoseSecurity` (lines 200-255). The happy chain asserts opp security −1 and opp
  field −2 (token + Styracomon-deleted Digimon); the gate is declinable and targets
  `OppField`; the zero-security path correctly no-ops. Sources: DCGO `BT24_017.cs`
  (PlayPetrificationToken(2) on `card.Owner.Enemy`), `BT24_018.cs`;
  `general_rule.pdf` 17-1-3; CLAUDE.md rule 25 (OnDeletion post-trash).
- **Owen:** BT24-082 `on_digivolve` gated on `event_target_owner:you` +
  Reptile/Dragonkin -> `suspend{target:source}`, `add_dp_modifier +3000
  expiry:end_of_turn`, `may_attack_now` (optional) — matches DCGO `BT24_082.cs`
  (OnEnterFieldAnyone, trait gate, +3000, SelectAttackEffect). Unhappy path uses
  BT21-024 Cyberdramon (Cyborg, non-trait) -> no suspend, no buff.
- **Raid -> Lamiamon:** board outcomes are faithful (happy: Lamiamon + Cyclonemon,
  both Dragonkin, trashes opp top security; unhappy: only Cyberdramon (Cyborg),
  trashes nothing). See test issue MED-1 — the asserted *reason* mischaracterizes the
  gate (it gates on the attack-target-change **event source's** trait, not "a
  Dragonkin on your field") but the **board result is correct**, so the combo is
  faithful.
- **Snowball:** BT24-008 + BT24-012 both carry `scope:inherited /
  on_opponent_security_removed / once_per_turn / your_turn -> gain_memory:1`; the real
  stack [BT24-008, BT24-012, BT21-025] removing one security nets **+2** memory (both
  buried sources fire), while token-driven removal with one buried Elizamon nets **+1**
  — proving additive per-source payout reaches buried inherited sources.
- **Lamiamon free-play:** clause-3 (`scope:inherited, on_opponent_security_removed,
  optional`) `select_hand {kind:digimon, dp_lte:5000, Reptile/Dragonkin} ->
  play_from_hand_free`, matching DCGO `BT21_025.cs` inherited (CardDP<=5000, trait
  gate). Happy plays BT24-011 Cyclonemon (5000 DP) free; unhappy with only BT24-018
  Styracomon (14000 DP) plays nothing. Driven via a **real** token-deletion security
  removal (un-ignoring the per-card clause-3 `#[ignore]`).
- **EX11-012 token-shield:** `would-leave` replacement `select_any_permanent
  {kind:token, optional} -> delete_permanent -> cancel_replacement`, faithful to DCGO
  `EX11_012.cs` (`CanSelectPermanentCondition => permanent.IsToken`, any owner;
  `canNoSelect:true`). Scanning both battle areas is required and correct (gap
  `G-EX11-012-TOKEN-SHIELD-OWN-ONLY` resolved).

### Test issues (minor)

- **MED-1 — Mischaracterized gate + under-isolated unhappy path**
  (`raid_target_change_without_trait_permanent_trashes_nothing` + the Combo-3
  doc-comment lines 393-394/426-428 and `medusamon-model.md` lines 96-97). The
  doc-comment/model claim BT21-025's `[Your Turn]` clause is "gated on having a
  Reptile/Dragonkin on **your field**". The **real** gate (BT21-025.yaml
  `event_target_owner:you` + `event_target_trait_has Reptile/Dragonkin`; DCGO
  `BT21_025.cs` `PermanentCondition` via `CanTriggerOnPermanentAttackTargetSwitch`)
  gates on the **attack-target-change event source's** trait. The unhappy path removes
  both Lamiamon *and* any trait permanent, then fires from a non-trait Cyberdramon — so
  it conflates "Lamiamon absent" with "event source not a trait Digimon" and never
  isolates the actual gate. **Board outcome is still correct; only the asserted reason
  is wrong.** Suggested fix: keep Lamiamon on the field and fire the target-change from
  a non-trait attacker (assert nothing trashed), plus a second negative where the
  source IS a trait Digimon and Lamiamon is the only field card. Severity: **minor**.

- **MED-2 — Event-injection helpers (process note, not a divergence)**
  (`petrification_token_deletion_*` + `cheat_in` / `owen` / `lamiamon_security_swap`).
  Triggers are injected via low-level engine helpers (`delete_permanent_with_cause` for
  the token; `enqueue_triggered` for OnDigivolve / OnAttackTargetChange / WhenAttacking)
  rather than driven through the real in-deck deletion source / digivolve action /
  declared attack. This is acceptable under the faithfulness rules because each named
  card's own effect still fires through its **real trigger path** and the load-bearing
  board assertions prove it. Residual gap: BT24-017 Medusamon's real `[When
  Digivolving]` token-**minting** (delete lowest-DP, return 2 trash, play 2 tokens on
  opponent) is never exercised by the suite — the token is spawned directly via
  `ctx.play_token`. The model states minting is covered by the per-card `bt24_017`
  test, so it is not an interaction-suite obligation, but the Combo-1 narrative
  ("Medusamon hands the opponent tokens") is only half-exercised here. Severity:
  **minor** (process note).

### Findings filed: none
No engine-primitive or card-faithfulness divergence was confirmed; nothing routed to
`docs/RUST_ENGINE_GAPS.md` or `qa/archetype-qa/engine-gaps.md`. MED-1/MED-2 are
recorded here as test-quality notes only.

### New tests authored: none (re-verification / audit run)

### Deferred / blocked: none

---

## Rocks

- **Model doc:** `qa/archetype-qa/Rocks-model.md`
- **Test file:** `code/digimon-engine/tests/archetypes/rocks.rs`
- **Verdict:** **FAITHFUL** (5/5 combos faithful, 0 findings, 0 untested)

Rocks (Mineral/Rock) is a source-trash value engine: cards pay costs by trashing
their own Mineral/Rock digivolution sources, which both fuels active removal and fans
out inherited triggers on the trashed sources, then re-buries sources from trash to
recur. The audit confirmed the Magneticdramon double-delete fan-out, the Close-gated
Proganomon cheat-evolve, the Pyramidimon trash-3 / re-bury recursion, the Close
suspend-refuel, and the Gravel Hearts free-play + Delay (cross-turn, not-on-placing-turn)
all resolve faithfully.

### Combo-by-combo

| Combo | Status | Covering tests |
|-------|--------|----------------|
| C1 — Magneticdramon source-trash double removal (EX10-036 + EX8-048) | **FAITHFUL** | `c1_magneticdramon_source_trash_triggers_inherited_fanout_double_delete`, `c1_without_inherited_delete_sources_only_active_delete_fires` |
| C2 — Proganomon cheat-evolve, Close-gated (EX10-032 + EX8-067) | **FAITHFUL** | `c2_proganomon_cheat_evolve_with_close_places_landramon_and_digivolves`, `c2_proganomon_cheat_evolve_masked_without_close` |
| C3 — Pyramidimon trash-3 highest-cost delete + re-bury recursion (EX11-044 + EX8-005) | **FAITHFUL** | `c3_pyramidimon_trash_three_fanout_and_rebury_restores_sources` |
| C4 — Close suspend-refuel on Mineral/Rock digivolve (EX8-067) | **FAITHFUL** | `c4_close_suspends_to_refuel_sources_on_mineral_digivolve`, `c4_close_already_suspended_offers_no_refuel` |
| C5 — Gravel Hearts cheat-play + Delay cost-reduced digivolve (EX10-069 + EX8-067) | **FAITHFUL** | `c5_gravel_hearts_main_free_plays_sunarizamon_and_arms_delay`, `c5_gravel_hearts_delay_does_not_fire_on_placing_turn` |

**Evidence highlights**

- **C1:** EX10-036 Clause A = cost (trash exactly 3 Mineral/Rock sources) -> delete 1
  opp Digimon (**unfiltered** by cost) + trash opp top security (DCGO `EX10_036.cs`
  281-329: `if(trashedCount==3)` -> SelectPermanentEffect Destroy + IDestroySecurity
  fromTop). EX8-048 inherited (`on_digivolution_card_trashed`, host Mineral/Rock, opp
  cost<=4) deletes a **second** Digimon (DCGO `EX8_048.cs` 87-141). Trashing EX8-048
  (traits [Mineral, LIBERATOR]) as one of the 3 sources both pays the Mineral/Rock cost
  and fires its inherited delete. Happy asserts opp field −2 + security −1; unhappy (3
  plain Mineral fillers, no inherited source) asserts opp field −1. EX8-047 Sunarizamon
  ([Reptile, LIBERATOR]) correctly excluded.
- **C2:** EX10-032 `[Hand][Main]` `CanUseCondition` = on hand + own turn + Close on
  field + Sunarizamon on field + Landramon in trash (DCGO `EX10_032.cs` 28-35); process
  places Landramon from trash under Sunarizamon then
  `DigivolveIntoHandOrTrashCard(payCost:true, fixedCost:3, ignoreRequirements)`. Happy
  asserts HAND_EFFECT_START legal, trash −1, stack top = EX10-032, Landramon a source,
  memory −3. Unhappy (no Close) masks HAND_EFFECT_START illegal.
- **C3:** EX11-044 Clause A trash 3 Mineral/Rock -> delete opp `IsMaxCost` (highest
  play-cost Digimon **or** Tamer); Clause B (`on_digivolution_card_trashed`, gated to
  THIS permanent) re-buries up to 3 Mineral/Rock from trash as own bottom sources (DCGO
  `EX11_044.cs` 87-129 / 192-260). EX8-005 Tumblemon ([Rock, LIBERATOR], inherited +1
  memory) buried among the 3 -> +1 memory fan-out. Test asserts opp field −1 (the
  cost-11 target is the victim, not cost-3), memory >= before+1, net source-count
  unchanged (−3 trashed +3 re-buried); assertions robust to re-bury pick order.
- **C4:** EX8-067 `[Your Turn]` — on a Mineral/Rock digivolve on your turn,
  `CanActivateSuspendCostEffect` gate (unsuspended), suspend Close then place up to 2
  Mineral/Rock from trash as that Digimon's bottom sources (DCGO `EX8_067.cs` 18-130).
  Wired through the **real** EX8-047 -> EX8-048 ([Mineral]) digivolve. Happy: optional
  suspend-refuel prompt, accept -> Close suspended, >=1 source placed, trash drops by the
  placed count. Unhappy: Close pre-suspended -> no prompt, trash unchanged.
- **C5:** EX10-069 `[Main]` play 1 Sunarizamon/Close from hand OR trash without paying,
  then `PlaceDelayOptionCards` (DCGO `EX10_069.cs` 16-144). Test asserts hand −2,
  Sunarizamon on field, memory −3 (only Gravel Hearts' own cost — body is free), Gravel
  Hearts parked `OptionState::Delayed{OnEvent(OnSuspend)}`. Unhappy (`general_rule.pdf`
  §16-16-3): suspending Close the **same (placing) turn** does NOT fire the Delay — the
  cross-turn system fact.

### Test issues (minor)

- **RCK-1 — Synthetic Mineral/Rock fillers, last-resort justification not stated**
  (`make_mineral_carrier` C1 / `mk_filler` Mineral sources C1+C3 / `mk_trash` C3).
  Synthetic `make_test_card` stand-ins are used for the Mineral-trait carrier permanent
  (whose `host_permanent_trait_has: Mineral` gate the inherited triggers require) and for
  inert Mineral/Rock filler/trash sources. This is a **justified last resort** — there is
  no vanilla/effectless Mineral or Rock DSL Digimon in the embedded pack (the only Mineral
  Lv6 cards EX10-033/034, EX11-044, EX8-055 all carry effects that would perturb the
  asserted board diff; lower-level Mineral/Rock bodies carry inherited deletes). Allowed by
  the contract, but `make_mineral_carrier`'s doc-comment explains its trait purpose without
  explicitly stating "no real implemented DSL card can fill this role", which the
  faithfulness rule asks for. Severity: **minor**.

- **RCK-2 — C1 active clause under-specified**
  (`c1_magneticdramon_source_trash_triggers_inherited_fanout_double_delete`). All 3 opp
  Digimon are seeded at play cost <=4 (3, 4, 2). EX10-036 Clause A's active delete is
  **unfiltered** by cost (DSL `{kind: digimon}`), while only the EX8-048 inherited is
  cost<=4. Because every target is cost<=4, the test cannot distinguish "active delete is
  unfiltered" from "active delete is also cost<=4-filtered" — it under-specifies the active
  clause. A stronger test would seed a cost-5+ opponent that the active clause can delete
  but the inherited cannot. The −2 assertion is correct; this is an under-specification, not
  a wrong assertion. Severity: **minor**.

- **RCK-3 — C5 Delay payoff board-diff not asserted (gap-fill candidate)**
  (`c5_gravel_hearts_main_free_plays_sunarizamon_and_arms_delay` +
  `c5_gravel_hearts_delay_does_not_fire_on_placing_turn`). Neither C5 test asserts the
  Delay's actual **payoff** board diff (on a later turn, suspending a Close trashes Gravel
  Hearts and a Mineral/Rock Digimon digivolves into a Mineral+LIBERATOR hand card at −3
  cost). The tests cover the Main free-play and the §16-16-3 not-on-placing-turn timing gate
  (the cross-turn system fact), so the combo is faithfully but only **partially** exercised
  vs the model's full C5 outcome. Gap-fill candidate, not a divergence. (Latent out-of-scope
  edge: DCGO `CardCondition` uses `HasRockMineralTraits && Liberator` (Mineral OR Rock +
  LIBERATOR) for the hand target while the DSL filter requires Mineral AND LIBERATOR per
  printed text — no test exercises it, so it does not affect the C5 verdict.) Severity:
  **minor**.

### Findings filed: none
No engine-primitive or card-faithfulness divergence was confirmed. RCK-1/2/3 are recorded
here as test-quality / gap-fill notes only.

### New tests authored: none (re-verification / audit run)

### Deferred / blocked: none

---

## DNA Omnimon

- **Model doc:** `qa/archetype-qa/DNA Omnimon-model.md`
- **Test file:** `code/digimon-engine/tests/archetypes/dna_omnimon.rs`
- **Verdict:** **INSUFFICIENT_COVERAGE** (4/5 combos faithful, 0 divergent, 1 blocked
  Combo E, 0 findings filed)

DNA Omnimon is the Greymon/Garurumon two-tribe DNA-digivolution combo deck: it ramps
both colour lines to a pair of Lv.6 jogress materials and converts them into an
Omnimon-name Lv.7 that simultaneously wipes the board and pressures security
(`general_rule.pdf` §8-2 DNA digivolution; §8-2-2-1-6 the new DNA Digimon may attack
the same turn). The audit confirmed the four payoff/assembly combos (A–D) resolve
faithfully against card text + `general_rule.pdf` + DCGO; the fifth (E, the Nokia
cost-6 Lv.6 jump) is genuinely **blocked** on an engine-primitive gap and is correctly
`#[ignore]`d, not weakened.

### Combo-by-combo

| Combo | Status | Covering tests |
|-------|--------|----------------|
| A — DNA Omnimon Alter-S blowout (Blue6 + Red6 → EX9-021) | **FAITHFUL** | `combo_a_dna_blowout_deletes_all_highest_level_and_grants_immunity`, `combo_a_standard_digivolve_does_not_grant_immunity` |
| B — Miraculous Mega Knight Delay → reactive DNA Omnimon | **FAITHFUL** | `combo_b_delay_consumes_leaving_lv6_into_merged_omnimon`, `combo_b_delay_does_not_fire_for_opponent_lv6_leaving` |
| C — Blast DNA Omnimon off opponent's turn (BT17-078 Counter) | **FAITHFUL** | `combo_c_blast_dna_counter_bottom_decks_same_level_then_prompts_delete`, `combo_c_blast_dna_rejects_broad_greymon_garurumon_names` |
| D — Free cross-tribe Lv.6 assembly (both DNA materials in one turn) | **FAITHFUL** | `combo_d_free_cross_tribe_assembly_yields_both_lv6_materials`, `combo_d_no_gabumon_yields_no_second_lv6` |
| E — Nokia accel into cheap Lv.6 (BT22-013 cost-6 [Hand][Main] jump) | **BLOCKED** | — (`combo_e_nokia_cost6_lv6_jump` `#[ignore]`d) |

**Evidence highlights**

- **Combo A:** DCGO `EX9_021.cs` L129-189 installs the opponent-source-only effect
  immunity (`CanNotAffectedClass`, `SkillCondition EffectSourceCard.Owner==Enemy`)
  **inside** `if (IsJogress(_hashtable))`, while the delete (`Enemy.GetBattleAreaDigimons().Filter(IsMaxLevel)`)
  runs **outside** that block = unconditional. The happy test asserts both tied-Lv.7
  victims deleted, the Lv.5 survives, opp trash +2, and one-sided immunity (immune to
  opponent Digimon/Tamer/Option, **not** to its own controller) — matching DCGO
  exactly; the unhappy test fires a non-DNA digivolve and asserts no immunity.
  `general_rule.pdf` §8-2-2-1-6 confirms the DNA result may attack the same turn
  (`stacks_unsuspended:true`). `cards/ex9/EX9-021.yaml` matches: `dna_origin`-gated
  immunity clause + an unconditional `for_each` over the highest level.
- **Combo B:** DCGO `BT17_095.cs` Clause B (`WhenRemoveField`, L164-484) gates to the
  OWNER's Lv.6 [Greymon]/[Garurumon] leaving **outside battle** (`!IsByBattle`) then
  selects an Omnimon-name Lv.7 + field perm + Lv.6 hand partner and `SetJogress` = the
  DNA merge. The happy test drives the real Clause-B replacement
  (`effect_initiated_dna_digivolve_hand_partner`) and asserts the merged EX4-060 with
  the leaving WarGreymon as a **source** (stack ≥ 3, WarGreymon **not** in trash, hand
  emptied); `general_rule.pdf` §8-2-2-1-2 ("cards that become digivolution cards are
  considered new cards") confirms consumed-not-trashed. The unhappy test confirms an
  opponent's leaving Lv.6 installs no prompt and trashes normally.
- **Combo C:** DCGO `BT17_078.cs` L104-114 wires `OnCounterTiming` to a
  `BlastDNADigivolveEffect` with the **exact** `BlastDNACondition("WarGreymon")` +
  `("MetalGarurumon")` marker (distinct from the broad-name standard DNA at `None`
  timing). The When-Digivolving body bottom-decks the chosen Digimon + all same-level
  opp Digimon then runs a **mandatory** (`canNoSelect:false`) delete. The happy test
  seeds 2× Lv.5 + 1× Lv.6, chooses a Lv.5 anchor, and asserts deck +2, field-size 1,
  and a non-optional delete prompt with `selecting_player == 1`; the rejection test uses
  real ST1-07 Greymon + ST2-06 Garurumon (broad names) and asserts no Counter window
  opens. `general_rule.pdf` §16-30 confirms `<Blast DNA Digivolve>` and the
  exact-name material marker.
- **Combo D:** DCGO `BT17_015.cs` On-Play/When-Digivolving (L189-300) branch 2 = free
  Gabumon → MetalGarurumon via `DigivolveIntoHandOrTrashCard(payCost:false,
  ignoreDigivolutionRequirementFixedCost:0)`. The happy test fires BT17-015's real
  branch-1 (`effect_initiated_digivolve` cost 0, ignore requirements) and asserts both a
  Red Lv.6 WarGreymon and a Blue Lv.6 MetalGarurumon now sit on field — the DNA pair
  Combos A/C consume; the unhappy test (no Gabumon base) asserts MetalGarurumon stays in
  hand. `general_rule.pdf` §8-1-2-2 confirms ignore-requirements digivolution.
- **Combo E (BLOCKED):** BT22-013's [Hand][Main] cost-6 jump is a
  `CompiledAltPathKind::ActivatedDigivolve` alt-path; `dna_digivolve.rs` matches only
  `Digivolve`/`DnaDigivolve`/`BlastDnaDigivolve`, and `action/space.rs` + `action/mask.rs`
  offer **no action ID** for it — the jump cannot be played or behaviorally driven, so
  its named board diff cannot be produced faithfully. Tracked as
  **G-ACTIVATED-DIGIVOLVE-EXECUTION** (`qa/archetype-qa/engine-gaps.md` L586-593, OPEN
  for BT22-013/026 + BT16-027). The combo-presence static gate still PASSES (all named
  cards load) — the block is an engine-PRIMITIVE gap (no execution route), not a missing
  card. `combo_e_nokia_cost6_lv6_jump` is correctly `#[ignore]`d (body
  `unimplemented!()`) with the gap reason; the executable [When Digivolving] branches are
  covered by `tests/cards_behavioral/bt22/bt22_013.rs`. Secondary card-local follow-up:
  `BT22-013.yaml` does not populate `AltPathSpec.condition` to enforce the Nokia
  precondition. Both stand as un-actioned, already-tracked gaps.

### Test issues (minor)

- **DOM-1 — Combo B uses documented-gap scaffolding around the asserted behavior**
  (`combo_b_delay_consumes_leaving_lv6_into_merged_omnimon` /
  `combo_b_delay_does_not_fire_for_opponent_lv6_leaving`). BT17-095 is seated via the
  `seat_as_delay_option` helper (manually setting `OptionState::Delayed`) instead of the
  real play path (`play_option_from_hand` → `place_self_as_delay_option`), and the leave
  is triggered with `Game::delete_permanent_with_cause` rather than a real card effect.
  Both are necessitated by the real, documented engine gap
  **G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH** (`docs/RUST_ENGINE_GAPS.md:384` — the
  real [Main] play would no-op the place-self step and trash the Option). The asserted
  behavior (the reactive Clause-B DNA merge) **is** exercised through the real card
  effect, the doc-comment is transparent, and the gap is already tracked. Consequence:
  the happy-path Clause-A free-play + seat is **not** exercised by this combo (a per-card
  concern); the combo test is faithful only to its scoped reactive claim. Severity:
  **minor** — not a faithfulness divergence.
- **DOM-2 — Combo A initiates the DNA via the effect-initiated entry, not the player
  action** (`combo_a_dna_blowout_deletes_all_highest_level_and_grants_immunity`). The DNA
  digivolve is initiated via `effect_initiated_dna_digivolve(blue, red, hand_card, 0,
  true)` (the effect-initiated entry to `Game::dna_digivolve_inner`) rather than the
  player-facing `DNA_DIGIVOLVE_START` action. Both route through the **same**
  `dna_digivolve_inner` merge + `WhenDigivolving(dna_origin=true)` trigger sequence, so
  the assertion is faithful; flagged only because it is an effect-initiated stand-in for
  a player DNA play of EX9-021 from hand. Acceptable; no change required. Severity:
  **minor**.

### Findings filed: none
No engine-primitive or card-faithfulness divergence was confirmed. Combo E is blocked on
the **already-tracked** G-ACTIVATED-DIGIVOLVE-EXECUTION gap
(`qa/archetype-qa/engine-gaps.md`) — no new finding to file; the card-local
`BT22-013.yaml` Nokia-precondition follow-up is likewise pre-existing. DOM-1/DOM-2 are
recorded here as test-quality / scaffolding notes only.

### New tests authored: none (re-verification / audit run)

### Deferred / blocked
- **Combo E (Nokia cost-6 Lv.6 jump)** — deferred until the
  `<Activated Digivolve>` execution route (G-ACTIVATED-DIGIVOLVE-EXECUTION) lands.
  Un-ignore `combo_e_nokia_cost6_lv6_jump` when the route exists; also populate
  `BT22-013.yaml` `AltPathSpec.condition` for the Nokia gate at that time.

---

## Omnimon ACE

- **Model doc:** `qa/archetype-qa/Omnimon ACE-model.md`
- **Test file:** `code/digimon-engine/tests/archetypes/omnimon_ace.rs`
- **Verdict:** **INSUFFICIENT_COVERAGE** (4/6 combos faithful, 0 divergent, 2 blocked
  — Combo 1 and the BT22-013 DNA-spine — 0 findings filed)

Omnimon ACE is the Red+Blue Omnimon DNA midrange/combo shell built around **BT17-095
Miraculous Mega Knight** as a reusable value engine (cheap body recursion, a `<Delay>`
that re-buys an Omnimon off a leaving Lv.6, and an inherited [Security] tempo play) and
two Omnimon payoffs (BT17-078 cohort-wipe, BT20-102 X-Antibody source-gated mass
deletion). This run carried **no fresh per-combo audit input** for Omnimon ACE (the
orchestrator supplied only the suite result), so the combo statuses below are taken from
the durable model (`Omnimon ACE-model.md`, all combos GREEN/BLOCKED as recorded there)
and the green suite run. Combos 2–5 are faithful; Combo 1 is blocked on a tracked engine
gap and `#[ignore]`d; the carried-over BT22-013 DNA-spine combo is blocked on the same
gap as DNA Omnimon's Combo E.

### Combo-by-combo

| Combo | Status | Covering tests |
|-------|--------|----------------|
| 1 — Mega Knight [Main]: free [Agumon]/[Gabumon] recursion + arm the Delay | **BLOCKED** | — (`combo1_mega_knight_free_plays_agumon_from_trash_and_seats_as_delay`, `combo1_mega_knight_declining_recursion_still_seats_delay` — both `#[ignore]`d) |
| 2 — Mega Knight `<Delay>` leave-trigger → Omnimon DNA from hand | **FAITHFUL** | `combo2_mega_knight_delay_dna_digivolves_into_omnimon_from_hand`, `combo2_mega_knight_delay_does_not_fire_on_battle_leave` |
| 3 — Mega Knight inherited [Security]: off-turn [Tai]/[Matt] free-play + return self to hand | **FAITHFUL** | `combo3_mega_knight_security_free_plays_tamer_and_returns_self_to_hand`, `combo3_mega_knight_security_returns_self_to_hand_with_no_tamer` |
| 4 — Omnimon (BT17-078) DNA digivolve: same-level cohort bottom-deck + delete | **FAITHFUL** | `combo4_omnimon_dna_digivolve_bottom_decks_chosen_level_then_deletes`, `combo4_omnimon_non_dna_play_does_not_fire_level_wipe` |
| 5 — Omnimon (X Antibody) BT20-102: source-gated protect-1-per-player mass deletion + bottom-deck | **FAITHFUL** | `combo5_x_antibody_with_omnimon_source_wipes_to_single_survivor_and_bottoms`, `combo5_x_antibody_without_omnimon_source_does_not_fire_body` |
| 6 — DNA-spine execution gap: BT22-013 [Hand][Main] cost-6 Lv.6 jump (shared DNA Omnimon line) | **BLOCKED** | — (no test; shared with DNA Omnimon Combo E) |

**Evidence highlights** (per the model, GREEN unless noted)

- **Combo 2:** BT17-095 Clause B (`general_rule.pdf` §16 `<Delay>`; DCGO `BT17_095.cs`
  `WhenRemoveField` + `!IsByBattle` + `SetJogress`) — the leaving Lv.6
  [Greymon]/[Garurumon] is consumed as a DNA material under the merged [Omnimon] (not
  trashed; replacement Cancelled, hand −2). The battle-cause leave does NOT trigger the
  Delay (the "outside of a battle" gate). Engine primitive
  `effect_initiated_dna_digivolve_with_hand_partner` (G-DSL-DNA-FROM-HAND-PARTNER,
  resolved 2026-05-20).
- **Combo 3:** BT17-095 Clause C (DCGO `SecuritySkill` + `PlayPermanentCards(payCost:false)`
  + `AddThisCardToHand`) — driven through the real combat/security-check path
  (`attack_player`): free-plays a [Tai Kamiya]/[Matt Ishida] Tamer, then returns BT17-095
  itself to **hand**; with no eligible Tamer the mandatory add-to-hand tail still fires.
- **Combo 4:** BT17-078 [When Digivolving] (DNA path) bottom-decks the whole chosen
  same-level opp cohort then deletes 1 opp Digimon (DCGO `BT17_078.cs` `IsJogress`-gated);
  a non-DNA play does NOT get the body (DNA-origin gate). (Same card as DNA Omnimon's
  Combo C body, here on the standard DNA path rather than the Blast/Counter path.)
- **Combo 5:** BT20-102 [When Digivolving] gated on an [Omnimon]/[X Antibody] **source**
  (DCGO `BT20_102.cs` `IsOmniOrXAntiSource` — scans *sources only*, excludes the
  carrier's own top-card name): protect 1 Digimon per player, delete every other Digimon,
  bottom-deck 1 surviving opp Digimon. The test drives the own-protect pick
  deterministically to BT20-102 itself before asserting it survives its own wipe; a bare
  BT20-102 (no Omnimon/X-Antibody source) does NOT fire the body. Engine primitives
  `self_digivolution_sources_contain_name` + `for_each` exclude-binding
  (G-SELF-DIGIVOLUTION-CONTAINS-NAME-SOURCES-ONLY / G-FOR-EACH-EXCLUDE-BINDING, both
  resolved).
- **Combo 1 (BLOCKED):** BT17-095 is a *Standard* Option that seats itself via the DSL
  `place_self_as_delay_option` step inside its [Main] body. On the **real**
  `Game::play_option_from_hand` lifecycle the Option card is moved into the
  single-occupancy `pending_option` slot before the [Main] body runs, so the place-self
  step (which scans only hand/trash) finds nothing and **no-ops**; `dispose_option` then
  trashes the Standard Option. Net on the real path: BT17-095 goes to trash, not seated as
  a Delay. Tracked as **G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH**
  (`docs/RUST_ENGINE_GAPS.md`). Both Combo-1 tests are `#[ignore]`d pending the fix rather
  than weakened or routed through the `activate_hand_main` bypass; the per-card
  `bt17_095.rs` covers the [Main] free-play via the bypass.
- **Combo 6 (BLOCKED):** the shared BT22-013 [Hand][Main] cost-6 jump — same
  G-ACTIVATED-DIGIVOLVE-EXECUTION engine-primitive gap as DNA Omnimon's Combo E (no
  action ID / no execution route for `<Activated Digivolve>`); no faithful interaction
  test possible until the route lands.

### Test issues
None new from this run (no fresh per-combo audit input was supplied for Omnimon ACE; the
green suite was re-run). The Combo-1 scaffolding concern is the same engine gap noted for
DNA Omnimon's DOM-1 and is documented in the model doc.

### Findings filed: none
Both blocked combos rest on **already-tracked** engine-primitive gaps
(G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH and G-ACTIVATED-DIGIVOLVE-EXECUTION in
`docs/RUST_ENGINE_GAPS.md` / `qa/archetype-qa/engine-gaps.md`); no new finding to file.

### New tests authored: none (re-verification / audit run)

### Deferred / blocked
- **Combo 1 (Mega Knight [Main] free-play + Delay-seat)** — deferred until
  G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH is fixed; un-ignore both
  `combo1_mega_knight_*` tests at that time.
- **Combo 6 (BT22-013 DNA-spine cost-6 jump)** — deferred until
  G-ACTIVATED-DIGIVOLVE-EXECUTION lands (shared with DNA Omnimon Combo E).

---

## Suite run (this audit)

All audited tests pass (blocked combos `#[ignore]`d, not failing).

- `medusamon` — 15/15 pass (the 6 combos' covering tests + `cheat_in_chains_swap...`,
  `cheat_in_blocked_without_owen...`, `lamiamon_security_swap_feeds_inherited_memory`).
- `rocks` — 11 archetype combo/structural tests pass (the 5 combos' covering tests +
  `greymon_removal_with_koromon_deletes_mid_dp_target` /
  `greymon_removal_without_koromon_spares_mid_dp_target`); full `archetypes` binary 26
  passed, 0 failed.
- `dna_omnimon` — 8 tests run, 8 pass (Combos A–D happy+unhappy = 8 green);
  `combo_e_nokia_cost6_lv6_jump` `#[ignore]`d (BLOCKED, G-ACTIVATED-DIGIVOLVE-EXECUTION).
- `omnimon_ace` — 10 tests run, 10 pass (Combos 2–5 happy+unhappy = 8 green);
  `combo1_mega_knight_free_plays_agumon_from_trash_and_seats_as_delay` /
  `combo1_mega_knight_declining_recursion_still_seats_delay` `#[ignore]`d (BLOCKED,
  G-OPTION-PLACE-SELF-AS-DELAY-ON-PLAY-PATH).

Triage outcomes: none. No interaction test failed; the only non-green cases are the
`#[ignore]`d BLOCKED sentinels above, each resting on an already-tracked engine gap.
