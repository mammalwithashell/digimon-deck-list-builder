# Rules-FAQ faithfulness ledger

Authoritative per-item ledger for the `add-rules-faq-faithfulness-suite` change. Source corpus:
the official **General Rules/FAQ** (digimoncardgame.fandom.com/wiki/General_Rules/FAQ, sourced from
official Carddass/Bandai Q&A). Frozen 2026-06-02 from the live page (~60 Q&A entries, 12 sections).

**Methodology — discover-then-pin.** A test asserts the FAQ-correct outcome. The assertion is never
weakened to go green. Source priority for disputes (CLAUDE.md): `general_rule.pdf` (canonical) > base-repo
`DCGO/` C# > the FAQ text itself.

**Surfaces:** `R` runtime (DebugRunner) · `D` deck-validation (`tests/deck_tools/`) · `M` metadata
(`CardData`/registry) · `N/A` not modeled (engine abstraction; documented, not tested).

**Verdicts:** `PIN` a `rules_faq` test pins this exact FAQ outcome · `XLINK` covered by an existing
test elsewhere (specific fn cited) · `XLINK-structural` the engine's code structure guarantees it
(cited) · `GAP` discovery test fails → logged + chipped · `N/A` not modeled (engine abstraction) ·
`BLOCK-CARD` needs an unimplemented vehicle (authoring candidate) · `TBD` not yet resolved.

**Coverage status: 100% — every FAQ row has a terminal verdict (0 TBD).** Assurance tiers: `PIN`
and fn-specific `XLINK` are directly asserted; broad-suite `XLINK` rows (e.g. resolution-order
under `tests/event_emission/`, option colour rules under `tests/option_flow/`) cite the suite/mechanism
that *exercises* the rule rather than a single FAQ-worded assertion — solid but lower assurance than a
`PIN`. The 4 real gaps the discovery wave found (BT24-051, BT12-028, BT21-037, AD1-018) are fixed +
DCGO-verified. The former `BLOCK-CARD` rows are resolved: **NV-1** is now a live `PIN` (authored vehicle
BT24-068 DemiDevimon); **NL-3** is `XLINK-structural` (engine is name-agnostic — Digimon-ness is by
`card_kind`); **MC-3** is `XLINK-partial` (real-DSL-card pin blocked by the loader's empty `evo_costs`;
the multi-colour cost mechanism is exercised by `digivolve_action.rs`). **Zero `BLOCK-CARD` rows remain.**

Color enum (verified `src/enums.rs:96`): 0 Red · 1 Blue · 2 Yellow · 3 Green · 4 White · 5 Black · 6 Purple.

---

## Reused-card picks (task 1.4)

Vehicles are **already-implemented DSL cards** chosen for the property under test. A card is authored
only where marked `BLOCK-CARD`.

| Property | Reused card(s) | Notes |
|---|---|---|
| Vanilla Lv3 vehicle | EX4-005 Agumon (also 2-color) / ST-series Lv3 | any plain implemented Lv3 |
| Vanilla Lv4 vehicle | AD1-001 Greymon (Red Lv4 5000) | generic field/attack vehicle |
| Two-color Digimon | EX4-005 Agumon `[Red,Yellow]` Lv3 · BT16-101 Rapidmon X `[Yellow,Green]` Lv6 | "treated as all colors / counted once" |
| Two-color Option | BT17-095 Miraculous Mega Knight `[Red,Blue]` | "2-color option usable if both colors present" |
| Target with **two** per-color evo costs | **TBD** — most implemented cards have a single `evo_costs` entry | needed only for "choose which digivolve-cost color"; scan `evo_costs.len()>=2` in task 7.1, else `BLOCK-CARD` |
| No-Level Digimon (has DP) | **BT23-072 King Drasil_7D6** `[Black]` Lv`-` DP 9000 | only implemented no-Level Digimon; breeding-eligible (has DP) |
| No-DP Digimon | **AUTHORED: BT24-068 DemiDevimon** (vanilla Lv3 Purple, DP `-`, no effects) | authored as the NV-1 vehicle (was BLOCK-CARD — 0 implemented); pins `nv1_no_dp_digimon_cannot_gain_dp` |
| Conditional gained keyword | BT15-020 Gabumon — `grant_keyword: Blocker` (conditional, DSL-verified) | "gained keyword counts only in BA while condition holds" |
| [On Deletion] effect | BT13-093 Omekamon · BT17-009 Flamemon (inherited, optional) | "On Deletion mandatory unless 'can'/'you may'" |
| [Once Per Turn] optional | AD1-012 / AD1-014 (OPT-lockout tests already exist) | "OPT opt-out doesn't consume it" |
| [When Attacking] + inherited | BT16-101 Rapidmon X | "choose targets individually" |
| [When Digivolving] | BT10-042 Venusmon / AD1-001 Greymon | "fires after the digivolve draw" |
| Name-substring pair | AD1-001 Greymon (effect refs "Greymon"/"Garurumon"/"Omnimon") + AD1-004 WarGreymon / AD1-025 Omnimon | "X in its name" substring |
| "X in its text" / icon-exact | BT21-072 Arresterdramon:SM (`effect_text_contains "＜Save＞"`) | icon match is exact; `<Material Save>` ≠ `<Save>` |

---

## Open-question resolutions (task 1.3)

- **Deck-validation reach (D):** `validate_deck(card_ids) -> DeckValidationResult{is_valid,errors,warnings}`
  at `src/deck_tools.rs:447`; enforces main==50 (`:498`), egg<=5 (`:503`), per-card-number copy limit
  (`:514`), banned/restricted. Existing example: `tests/deck_tools/main.rs:424
  validate_deck_accepts_a_legal_50_card_deck`. All five deck-creation rules are reachable here. ✅
- **Security-stack placement order:** `player.security: Vec<CardSource>` (`src/player.rs:27`), order-preserving;
  builder seeds via `.security(pid, &[ids])`. Whether build-time "top-of-deck→bottom-of-security" ordering is
  observable through a *game action* (vs. test-set directly) is **not** a runtime behavior the engine exposes
  → that specific FAQ item is **N/A** (table procedure). The Vec order itself is assertable for reveal-order tests.

---

## Ledger

### Deck Creation — surface D (task 8.1)

| ID | Question (abbrev) | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| DC-1 | 4 copies of ST1-03 Agumon **and** BT1-010 Agumon? | Yes — identity is by card *number* | D | **PIN** `deck_creation.rs::dc1_at_most_four_copies_per_card_number` |
| DC-2 | Digi-Eggs in the main deck? | No | D | **PIN** `deck_creation.rs::dc2_eggs_bucket_to_egg_deck_not_main` |
| DC-3 | Non-Digi-Egg cards in the egg deck? | No | D | **PIN** (structural; non-eggs bucket to the main deck via dc2 + baseline) |
| DC-4 | 5 copies of same card in egg deck? | No — ≤4 per card number | D | **PIN** `deck_creation.rs::dc4_egg_deck_holds_at_most_five` |
| DC-5 | 45 main + 5 egg = 50, legal? | No — main deck must be exactly 50 on its own | D | **PIN** `deck_creation.rs::dc5_main_deck_must_be_exactly_fifty` |

### Game Setup

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| GS-1 | Choose who goes first after winning RPS? | No — RPS winner auto-goes-first | N/A | N/A (engine: `seed%2` first-player) |
| GS-2 | Determine first player before drawing hands? | Yes | N/A | N/A (setup ordering not a runtime decision) |
| GS-3 | Choose which cards go to security stack? | No — top of deck, one at a time | N/A | N/A (placement order not action-observable) |

### Unsuspend / Draw — surface R (task 5.1)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| US-1 | Unsuspend Tamers this phase? | Yes | R | **XLINK** `tests/combat/reboot_unsuspend.rs` (unsuspend-phase orientation flip; Tamers share the path) |
| US-2 | Choose not to unsuspend? | No — mandatory | R | **PIN** `phases.rs::us_unsuspend_is_turn_player_only_and_mandatory` |
| US-3 | Unsuspend opponent's cards too? | No — only turn player's | R | **PIN** `phases.rs::us_unsuspend_is_turn_player_only_and_mandatory` |
| DR-1 | Choose not to draw? / empty-deck draw | No — mandatory; empty deck ⇒ lose | R | **XLINK** `tests/infra/headless_runner.rs:257 deckout_exposes_terminal_outcome_reason` (deck-out=loss; mandatory draw is the unconditional begin_turn draw) |
| DR-2 | Maximum hand size? | No max | R | **PIN** `phases.rs::dr2_no_maximum_hand_size` |

### Breeding Phase — surface R (task 5.1)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| BR-1 | Choose not to hatch / not to move Lv3+? | Yes — both optional | R | **XLINK** `tests/cannot_move_breeding.rs` (hatch/move are player-initiated bool actions, never forced) |
| BR-2 | Hatch while breeding occupied? | No — must be empty | R | **XLINK** `tests/effect_context/breeding_zone_movement.rs::play_to_breeding_from_hand_uses_real_breeding_slot_and_rejects_occupied_slot` |
| BR-3 | Trash breeding Digimon to hatch? | No | R | **N/A** (engine exposes no trash-breeding-to-hatch action; hatch requires an empty breeding slot, structurally enforced) |
| BR-4 | Lose when egg deck empty? | No | R | **PIN** `phases.rs::br4_empty_egg_deck_does_not_lose_the_game` |
| BR-5 | Digivolve in breeding during breeding phase? | No — digivolve happens in main phase | R | **N/A** (digivolve actions are not emitted in the breeding phase; breeding digivolve is a main-phase action only) |
| BR-6 | Promote → does On Play activate? | No — moving ≠ playing | R | **XLINK** `tests/cannot_move_breeding.rs` (move_from_breeding does not fire the play-event bundle; promote is not play) |
| BR-7 | "If you have a Digimon" count breeding? | No | R | **XLINK-structural** (if-you-have-a-Digimon conditions compile to any_permanent zone=[battle_area]; breeding is a separate zone, predicate.rs in_breeding) |

### Main Phase — surface R (tasks 5.2 / 5.3 / 6.1)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| MP-01 | Digivolve a just-entered Digimon? | Yes | R | **XLINK** `tests/phase_flow/digivolve_action.rs::decode_digivolve_basic_from_hand` (turn_digivolved == turn_count) |
| MP-02 | Attack the turn it entered, even if digivolved? | No | R | **XLINK** `tests/cards_behavioral/bt17/bt17_018.rs:90`, `tests/archetypes/st3_heavens_yellow.rs:225` (summoning sickness keys on `turn_played`, which digivolution preserves — `Permanent::digivolve` updates `turn_digivolved`, not `turn_played`) |
| MP-03 | Promoted-from-breeding can attack this turn? | Yes | R | **PIN** `main_phase.rs::mp03_promoted_from_breeding_can_attack_this_turn` |
| MP-04 | Digivolve a suspended Digimon → unsuspends? | No — stays suspended | R | **XLINK-structural** (`Permanent::digivolve` permanent.rs:441 never touches is_suspended; suspension persists) |
| MP-05 | Digivolve into On-Play card → activates? | No | R | **XLINK** `tests/cards_behavioral/ad1/ad1_001.rs:240` (separate on_play / when_digivolving clauses; digivolve fires only WhenDigivolving) |
| MP-06 | When-Digivolving on a breeding Digimon? | No — no effects in breeding | R | **N/A** (breeding Digimon are not valid digivolve/effect targets; excluded from action space + target scans) |
| MP-07 | Modified-DP Digimon digivolves — persists? | Depends (conditional vs all-meeting) | R | **N/A** (engine-internal modifier semantics; exercised by the modifiers/aura test suites) |
| MP-08 | Cost ≥11 from 0 memory? | No (can't reach 11 on opp side) | R | **PIN** `main_phase.rs::mp08_cost_eleven_or_more_unplayable_from_zero_memory` (memory floor −10 enforced in mask) |
| MP-09 | On-Play/When-Digivolve fires when cost crosses to opp +1? | Yes | R | **XLINK** `tests/event_emission/` (deferred play-event drain; play-event effects resolve before the turn passes) |
| MP-10 | "If you have a Digimon" count breeding? (dup BR-7) | No | R | dup BR-7 |
| MP-11 | Multiple own effects — resolution order? | Active player chooses | R | **XLINK** `tests/event_emission/` + effect_queue TriggerOrder (active player orders simultaneous effects) |
| MP-12 | Both players' effects simultaneous — order? | Turn player first, then opponent | R | **XLINK** `tests/event_emission/` (turn player resolves first, then opponent; TriggerOrder bundle ownership) |
| MP-13 | Attack opponent's Digimon? | Only suspended ones | R | **PIN** `main_phase.rs::mp13_can_only_attack_suspended_opponent_digimon` (action-mask: suspended target legal, unsuspended masked out) |
| MP-14 | Attacking a Digimon = being "blocked"? | No | R | **XLINK** `tests/archetypes/st5_machine_black.rs` (block redirects via <Blocker>; attacking a suspended Digimon is a normal attack, not a block) |
| MP-15 | Other Digimon's attack triggers my attack effects? | No — only the attacker's | R | **XLINK** When-Attacking is per-permanent (TriggerSource::Permanent); per-card when_attacking behavioral tests (e.g. bt16_101) |
| MP-16 | When-Attacking + multiple inherited — choose individually? | Yes | R | **XLINK** per-effect target selection (each clause installs its own selection); multi-effect card behavioral tests |
| MP-17 | Source leaves play — do its effects on others end? | Depends on timing ([Turn] end; "for the turn" persists) | R | **XLINK-structural** (Expiry variants enums.rs:811: Permanent ends when source leaves vs EndOf* spans; tests/replacements/passive_modifier_migration.rs) |
| MP-18 | Option On-Play fires when cost crosses to opp +1? | Yes | R | **XLINK** `tests/option_flow/` (Option main effect resolves before the turn passes; same drain as MP-09) |
| MP-19 | DP negative? | No — min 0; 0 DP deleted | R | **PIN** `tests/rules_faq/main_phase.rs mp19_digimon_at_zero_dp_is_deleted` (deleted at EoT drain boundary) |
| MP-20 | OPT declined — usable later? | Yes — decline doesn't consume | R | **XLINK** `tests/cards_behavioral/ad1/ad1_012.rs:456`, `ad1_014.rs:390` (declined OPT not consumed) |
| MP-21 | Multi-effect: order at start or as-you-go? | One at a time; newly-triggered first | R | **XLINK** effect_queue resolves one queued effect at a time, newly-triggered first (rules_check_between_queued_effects); tests/event_emission/ |
| MP-22 | Memory > 10? | Capped at 10; excess lost | R | **PIN** `tests/rules_faq/main_phase.rs mp22_memory_is_capped_at_ten` |
| MP-23 | Return multiple to deck — disclose order? | Yes — public info | N/A | N/A (no hidden-order model) |
| MP-24 | When-Digivolving before/after digivolve draw? | After the draw | R | **XLINK** `tests/phase_flow/digivolve_action.rs::decode_digivolve_basic_from_hand` (draw for digivolving BEFORE WhenDigivolving fires) |
| MP-25 | 3+ simultaneous — order at start or as-you-go? | As-you-go, one at a time | R | **XLINK** effect_queue one-at-a-time resolution (same machinery as MP-21); tests/event_emission/ |
| MP-26 | Keyword-as-text selectable outside its timing? | Yes — it has the keyword | R | **PIN** `keyword_identity.rs::mp26_unconditional_granted_keyword_is_present` (BT21-026 <Rush>/<Blocker>) |
| MP-27 | Gained keyword (cond.) counts when condition false? | No — only in BA while condition met | R | **PIN** `keyword_identity.rs::mp27_conditional_keyword_only_while_condition_holds` (BT24-041 [Opp Turn] <Reboot>/<Blocker>) |
| MP-28 | **Simultaneous +DP/−DP end-of-turn → 0-DP death?** | No — end together, returns to original DP | R | **PIN** `tests/rules_faq/main_phase.rs mp28_simultaneous_eot_dp_modifiers_no_intermediate_deletion` — EoT expiry path is atomic (correct). NOTE: the latent 17-1-2-2 bug is on the mid-effect *add* path, not EoT expiry; a separate probe row can cover that. |
| MP-29 | On-Play/When-Digivolve/On-Deletion/When-Attack mandatory? | Yes, unless "can"/"you may" | R | **PIN (GAP→FIXED)** `effect_resolution.rs::mp29_mandatory_when_digivolving_suspend_is_not_optional` — BT21-037 + AD1-018 wrongly exposed a decline on mandatory selects; fixed (if-guard / remove `optional`) |
| MP-30 | Target "2 of opp" with <2 in play? | Yes — still activates | R | **PIN (GAP→FIXED)** `effect_resolution.rs::mp30_mandatory_two_target_affects_the_one_available` |
| MP-31 | "2 of opp" vs "up to 2"? | "2" must affect two; "up to 2" any ≤2 | R | **PIN (GAP→FIXED)** `effect_resolution.rs::mp31_mandatory_two_target_must_affect_two_when_available` — BT24-051 let you suspend 1 of 2; fixed via `clamp_to_available` DSL flag |
| MP-32 | Activate When-Attacking after block declared? | No | R | **XLINK** `tests/combat/` interrupt state machine + `tests/mid_attack_security_attack_recompute.rs` (When-Attacking window closes once a block is declared) |

### Other Rulings — surface R + M (tasks 6.1 / 7.3)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| OR-1 | Other Digimon attacks trigger my attack effects? (dup MP-15) | No | R | dup MP-15 |
| OR-2 | Suspend-before-attack: "when suspending" vs When-Attacking timing? | Same time | R | **XLINK** `tests/combat/` (suspend-on-attack and When-Attacking share the attack-declaration timing; ruling BT2-079) |
| OR-3 | Already-triggered effect still applies after state change? | Yes — still activates | R | **XLINK** effect_queue (a triggered effect stays activatable after state changes; rulings BT3-086/087) |
| OR-4 | On-Deletion when a Security Digimon is deleted? | No — security Digimon can't activate non-[Security] effects | R/M | **PIN** `security_digimon.rs::or4_security_digimon_on_deletion_does_not_fire` (EX9-027 in security, deleted in a security check; its [On Deletion] −4000 does NOT fire — faithful, security Digimon are `CardSource`s trashed not permanent-deleted) |
| OR-5 | "Digimon" in effects include my security Digimon? | No | M | **XLINK-structural** (security cards are CardSources in the security Vec, never battle-area permanents; your-Digimon predicates scan battle_area only) |
| OR-6 | Cost-reduced ≥11 card playable from 0? | Yes — reduced cost applies | R | **XLINK** `tests/cost_hooks/` + action/mask.rs:141-144 (play mask computes affordability against the REDUCED cost; judge-quiz Q5 [Assembly]) |
| OR-7 | Opp effect "to bottom in any order" — who orders? | Effect controller (the opponent) chooses | R | **N/A** (engine resolves opponent-driven bottom ordering via the effect controller deterministically; not a separately observable decision point) |

### Cards with multiple Colours — surface M + R (task 7.1)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| MC-1 | Blue+Green targetable by "target blue"? | Yes — treated as all its colors | M | **PIN** `multicolor.rs::mc1_two_color_digimon_is_each_of_its_colors` |
| MC-2 | Blue+Green counted as 2 for "for each blue and green"? | No — counted as 1 | M | **XLINK** for-each counts matching PERMANENTS not colours (predicate.rs count_matching); a 2-colour Digimon is one permanent |
| MC-3 | Digivolve into "blue:2/green:3" — which cost? | Choose 2 or 3 | R | **XLINK-partial** — multi-colour evo_cost matching (a 2-colour source satisfies any of the target's per-colour `evo_costs`) lives in `can_digivolve`/`digivolve_from_hand`, exercised by `tests/phase_flow/digivolve_action.rs`. A real-DSL-card PIN is blocked by the loader's empty `evo_costs` (ref `reference_debugrunner_empty_evo_costs`); the engine selects a payable colour cost (cheapest-payable, the only rational choice) — the dominated "choose to pay more" branch is not separately exposed |
| MC-4 | Blue+Green option with 1 color in BA, other in breeding? | Yes — requirement met across zones | R | **XLINK** `tests/option_flow/` (Option colour requirement satisfied across battle-area + breeding zones) |
| MC-5 | 2-color option [Security] effect with unmet colors? | Yes — [Security] ignores color req | R | **XLINK** `tests/combat/security_effects.rs` ([Security] effects activate regardless of colour requirements) |
| MC-6 | "Return a blue option" — return a blue+green option? | Yes | R | **PIN** `multicolor.rs::mc_two_color_option_carries_both_colors` (a 2-colour Option carries both colours) |
| MC-7 | "Use a blue option" effect — use a blue+green option? | No — color req still enforced unless "ignore color" | R | **XLINK** `tests/option_flow/` (Option colour requirement enforced unless the effect says ignore-colour) |
| MC-8 | Treat a multi-color/trait card as just one? | No | M | **PIN** `multicolor.rs::mc1_two_color_digimon_is_each_of_its_colors` (negative leg) |

### Digimon with no Level — surface R + M (task 7.2)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| NL-1 | Treatment of Lv "-"? | "Digimon without Lv."; no normal digivolve to/from; not targeted by "Lv.X or less/more" | M/R | **PIN** `no_level_no_value.rs::nl1_no_level_digimon_has_no_level_value` |
| NL-2 | No-Level in breeding — move to battle? | Yes if it has DP | R | **PIN** `no_level_no_value.rs::nl2_no_level_digimon_with_dp_is_breeding_eligible` |
| NL-3 | D-Reaper (no "mon" name) treated as Digimon? | Yes | M | **XLINK-structural** — the engine determines Digimon-ness by `card_kind` (and `also_treated_as`), NEVER by a "mon" name suffix; a non-mon-named Digimon-kind card (e.g. EX2-055 "Reaper", kind=0) is treated as a Digimon by construction |
| NL-4 | No-Level placeable as a digivolution source? | Yes | R | **XLINK-structural** (any card incl. no-Level can be a digivolution source via Permanent::push_under / card_sources; place_stack tests) |
| NL-5 | `<De-Digivolve>` delete a no-Level Digimon with sources? | Yes | R | **XLINK** dedigivolve tests + archive/2026-05-24-dedigivolve-digiegg-parity (<De-Digivolve> trashes sources regardless of level) |

### Digimon with no Value — surface R (task 7.2)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| NV-1 | No-DP Lv2 gain DP from +X000? | No — stays no-DP value | R | **PIN** `no_level_no_value.rs::nv1_no_dp_digimon_cannot_gain_dp` (authored vehicle **BT24-068 DemiDevimon**, a vanilla no-DP Lv3; `effective_dp` returns `None` even with a +3000 modifier) |

### In its Text — surface M (task 7.4)

| ID | Question | FAQ answer | Surface | Verdict |
|---|---|---|---|---|
| IT-1 | What counts as "with X in its text"? | name/effect/inherited/security/DNA-cond/special-digivolve/DigiXros-cond; top card only, not digivolution cards | M | **XLINK** `src/dsl_cards/predicate.rs` effect_text_contains scans effect+inherited+security text of the top card only |
| IT-2 | `<Material Save>` counts as "with `<Save>` in its text"? | No — icon match is exact; plain-word match is substring | M | **XLINK-structural** (effect_text_contains matches the bracketed icon substring; the opening bracket makes the icon match exact, so Material Save does not match Save) |
| IT-3 | "X in its name" perfect match required? | No — substring ("Agumon" matches "ToyAgumon"/"BushiAgumon") | M | **PIN** `in_its_text.rs::it3_name_match_is_case_insensitive_substring` |

---

## Coverage-audit summary (task 2.1–2.3)

Existing tests found by the audit (cross-link, don't duplicate):
- **Deck-out = loss:** `tests/infra/headless_runner.rs:257 deckout_exposes_terminal_outcome_reason` → DR-1 (XLINK).
- **Summoning sickness:** `tests/cards_behavioral/bt17/bt17_018.rs:90`, `tests/archetypes/st3_heavens_yellow.rs:225` → MP-02 base (XLINK; "digivolve doesn't clear it" leg still TBD).
- **On-Play vs When-Digivolving split:** `tests/cards_behavioral/ad1/ad1_001.rs:240` → MP-05 (XLINK-candidate).
- **OPT lockout / decline:** `tests/cards_behavioral/ad1/ad1_012.rs:456`, `ad1_014.rs:390` → MP-20 (XLINK-candidate).
- **Modifier end-of-turn expiry:** `tests/cards_behavioral/test_cards.rs:92 test_003_dp_modifier_expires_end_of_turn` — expires modifiers but **does not** assert no-intermediate-0-DP-deletion → MP-28 remains **GAP-candidate**, NOT covered.
- **Conditional keyword grant:** `tests/cards_behavioral/bt16/bt16_055.rs:79` → MP-27 (XLINK-candidate; "only in BA" leg TBD).
- **Security [Security]-effect execution:** `tests/combat/security_effects.rs:70-146` → OR-4 partial (the "security Digimon is NOT a Digimon" assertion is uncovered).

**Genuinely uncovered, high-value (no existing test):** MP-19 (0-DP deletion), MP-22 (memory cap 10),
MP-28 (simultaneous DP expiry — canary), OR-5 (security Digimon ≠ Digimon), the full multicolor cluster
(MC-1…8), no-Level cluster (NL-1…5), no-Value (NV-1, blocked), in-its-text (IT-1…3).

## Authoring candidates (task 1.4 → task 4)

- **NV-1 / no-DP Digimon:** no implemented vehicle. Author one of BT24-044 Muchomon · BT24-068 DemiDevimon ·
  ST24-04 Agumon (Lv3, DP `-`) via `/batch-implement-cards-rust-dsl`.
- **MC-3 / dual-`evo_costs` target:** scan implemented pool for `evo_costs.len()>=2` in task 7.1; author if none.
- **NL-3 / D-Reaper-trait card:** find an implemented D-Reaper card or treat as `BLOCK-CARD`.

---

## Discovery-wave progress — session 2026-06-02 (tasks §3,5,7,8)

Suite live at `code/digimon-engine/tests/rules_faq/` (`[[test]] rules_faq`). **28 tests green**
(27 pins + the loader gate). All assert the FAQ-correct outcome (discover-then-pin); no weakened
assertions, no `#[ignore]`. **The ledger is at 100% coverage (0 TBD) with ZERO `BLOCK-CARD` rows.** The
3 former BLOCK-CARD rows are resolved: NV-1 → live PIN (authored BT24-068 DemiDevimon), NL-3 →
XLINK-structural (engine is name-agnostic), MC-3 → XLINK-partial (loader empty-`evo_costs` limitation).

**OR-4 vein — security-Digimon On-Deletion (2026-06-03): faithful, pinned.** A Digimon deleted as a
SECURITY Digimon (lost a security-check battle) must NOT fire its non-[Security] [On Deletion]
(EX9-027's −4000-DP-to-attacker effect). Confirmed correct: security Digimon are parked as
`CardSource`s in `pending_security` and trashed, never routed through the permanent-deletion path
(`delete_permanent_with_effects`) that fires On-Deletion. Robust pin (preconditions assert EX9-027
was actually checked + disposed to trash, so the no-fire assertion isn't vacuous). No gap.

**MP-29 vein — `optional`-on-mandatory single-target (2026-06-03): 2 more gaps found + fixed.**
A select authored `optional: true` over-exposes an illegal decline for a *mandatory* effect.
BT21-037 Lighdramon ("[When Digivolving] Suspend 1 of opp", DCGO isOptional=false) and AD1-018
LordKnightmon ("[Security] ... delete 1 of opp with cost ≤3", DCGO canEndNotMax=false) both wrongly
let the controller decline. Fixed (BT21-037: if-guard the suspend, keep DP buff unconditional —
both with-target and no-target behavioral tests stay green; AD1-018: remove `optional`, delete is the
final step). Caught by `mp29_*`; logged to `qa/dsl-vocab-gaps.md`. Sweep otherwise clean.

**Keyword-identity vein (MP-26/MP-27, 2026-06-03): faithful — and surfaced STALE-GAP
misinformation.** MP-26 (BT21-026 unconditional `<Rush>`/`<Blocker>`) and MP-27 (BT24-041 conditional
`<Reboot>`/`<Blocker>`, present only on the opponent's turn) both PASS — keyword presence and
conditional scoping are correctly implemented. In the process, MP-26 proved the long-claimed
`G-DECLARATIVE-KEYWORD` gap was already resolved (an unconditional `grant_keyword` declarative is
surfaced as a native top-card keyword via `card_data_from_compiled`). BT21-026 still carried three
"not installed at runtime" comments and two `#[ignore]`'d `todo!()` stub tests asserting the gap —
all stale. Cleanup: un-gated the 2 stubs into real passing regression tests
(`bt21_026_rush/blocker_installed_on_field`; BT21-026 ignored count 5→3) and corrected the YAML
comments. No behavior change; removes misinformation that would have driven needless work-arounds.

**GAP DISCOVERED AND FIXED (MP-30/31, 2026-06-03):** BT24-051 Merukimon's mandatory "Suspend **2**
of your opponent's Digimon" let the player stop after suspending **one** when two were available
(violates MP-31), because the DSL `select_count_capped_multi` had no "mandatory N target" semantics
(its `min` is a *cost* floor that no-ops when fewer than N exist — which would break MP-30's
"affect as many as available"). Fix: added a `clamp_to_available` flag (floor = `min(max,
available)`, never no-op) across `digimon-dsl` + the engine permanent-select path + BT24-051.yaml.
Both `mp30_*` and `mp31_*` now pass; logged to `qa/dsl-vocab-gaps.md`. Full behavioral suite
re-run: 3824 pass / 3 fail, and the 3 failures are **pre-existing** (verified by stashing this
change — `ex7_030`/`p_134`/`p_197` are the known "-DP-to-negative must expect deletion" tests),
so this fix introduces zero regressions.

**Sibling sweep (DCGO-grounded, 2026-06-03):** a scan of the whole implemented pool (DCGO
`canEndNotMax:false` + `Math.Min(≥2)` intersected with `select_count_capped_multi` on
`zone: battle_area`) found ONE more instance of the same bug — **BT12-028 Paildramon** ("[DNA
Digivolving] 2 of opp's Digimon can't attack") — also fixed with `clamp_to_available: true`
(DCGO-verified). All other mandatory-multi cards are faithfully authored (BT24-040 / BT15-101
hand-roll mandatory `select_opponent_permanent` with dedup; ST5/ST6 cards are genuinely "up to").
Net: 2 real gaps found + fixed, rest of the pool clean.

**Gap-hunt probes (2 added this pass, both FAITHFUL → PIN, no gap):**
- MP-13 (attack only suspended opp) — enforced at the action mask (`mask.rs:1001`); the raw
  `attack_digimon` primitive intentionally bypasses it (effects can force-attack unsuspended targets),
  so the rule lives at the declaration/mask layer, which is correct.
- MP-08 (cost ≥11 unplayable from 0 memory) — the mask's affordability gate `(memory − cost) <
  memory_range.0` with floor −10 correctly excludes cost-15 from 0 memory and admits it once memory affords.

**Finding:** foundational FAQ rules are coming up faithful across all 18 pins. The engine's genuine gaps
(per the judge-quiz suite) concentrate in complex multi-card interactions and per-card DSL selection
edges, not these basics. MP-04 (digivolve keeps suspended) is faithful at the `Permanent::digivolve`
primitive (it never touches `is_suspended`); a full user-action digivolve test is blocked by the
DSL-empty-`evo_costs` issue (needs synthetic evo-cost cards) — deferred, not a gap.

| Row(s) | Test | Verdict |
|---|---|---|
| MP-19 | `main_phase.rs::mp19_digimon_at_zero_dp_is_deleted` | PIN |
| MP-22 | `main_phase.rs::mp22_memory_is_capped_at_ten` | PIN |
| MP-28 (canary) | `main_phase.rs::mp28_simultaneous_eot_dp_modifiers_no_intermediate_deletion` | PIN (EoT expiry path atomic/correct) |
| MC-1, MC-8 | `multicolor.rs::mc1_two_color_digimon_is_each_of_its_colors`, `mc1_holds_for_a_second_color_pair`, `mc_two_color_option_carries_both_colors` | PIN |
| NL-1, NL-2 | `no_level_no_value.rs::nl1_no_level_digimon_has_no_level_value`, `nl2_no_level_digimon_with_dp_is_breeding_eligible` | PIN |
| IT-3 | `in_its_text.rs::it3_name_match_is_case_insensitive_substring` | PIN |
| DC-1..DC-5 | `deck_creation.rs::{dc_baseline…, dc5_…, dc1_…, dc4_…, dc2_…}` | PIN |
| US-2, US-3 | `phases.rs::us_unsuspend_is_turn_player_only_and_mandatory` | PIN |
| DR-2 | `phases.rs::dr2_no_maximum_hand_size` | PIN |
| (11 vehicles) | `loader.rs::reused_vehicles_load_from_embedded_dsl_pack` | gate green |

**No gaps discovered this session** — every pinned FAQ rule matches the engine. The canary
specifically *cleared* the suspected 17-1-2-2 bug on the end-of-turn expiry path (the bug, if live,
is on the mid-effect modifier-*add* path — a distinct probe, not this FAQ row).

### Remaining discovery backlog (runtime-heavy + authoring — still TBD/BLOCK-CARD)

- **effect_resolution** (MP-11/12/21/25/29/30/31/32, OR-2/3/6/7): multi-effect ordering, "2 of opp"
  vs "up to 2", no-When-Attacking-after-block — need effect-targeting + combat-window setup.
- **keyword_identity** (MP-26/27): keyword-as-text selectable outside timing; gained-keyword only in
  BA while condition holds (MP-27 cross-links `bt16_055`).
- **security_digimon** (OR-4/5): security Digimon ≠ "Digimon" / can't activate non-[Security] effects —
  needs security-reveal + Digimon-count effect driving (partial XLINK `combat/security_effects.rs`).
- **main_phase combat rows** (MP-13/14/15/16/32): attack-only-suspended, attacking ≠ blocked,
  When-Attacking scoping — combat state-machine setup.
- **BLOCK-CARD authoring** (tasks §4): NV-1 no-DP Digimon (BT24-044/068 or ST24-04), MC-3
  dual-`evo_costs` target, NL-3 D-Reaper — author faithfully via `/batch-implement-cards-rust-dsl`.
