//! Magneticdra — archetype interaction tests (Option-centric combos).
//!
//! Model: `qa/archetype-qa/Magneticdra-model.md`. Magneticdra is the mono-Black
//! Mineral/Rock [LIBERATOR] "Rocks" superset built around a **source-trash value
//! engine**: Mineral/Rock Digimon carry *inherited* "when my digivolution source
//! is trashed" payloads, and the deck's Options + payoffs trash those buried
//! sources as a cost — each trashed source simultaneously pays the cost AND fans
//! out one inherited trigger. These tests pin the **Option half** of that engine
//! as cross-card system facts a per-card test can't see.
//!
//! Sibling exemplar: `tests/archetypes/rocks.rs` (Digimon-only payoff lines —
//! Magneticdramon double-delete, Proganomon cheat-evolve, Pyramidimon recursion,
//! Close refuel, Gravel Hearts hand free-play). This file is Option-routed and
//! deliberately complements, not duplicates, the Rocks file.
//!
//! Source priority (CLAUDE.md): `general_rule.pdf` (canonical, §16 keywords) +
//! DCGO C# outrank the card-text JSON. YAML specs live under
//! `code/digimon-engine/cards/`.
//!
//! Each `#[test]` maps 1:1 to a named combo in the model doc; the doc-comment
//! records the combo name, cards, expected mechanical outcome, and sources.

#![allow(dead_code)]

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::TriggerSource;

use super::support::snapshot;

// ─── Real-card cast (all DSL-loadable) ──────────────────────────────────────
//
// All roles below are real implemented DSL cards (no synthetic `make_test_card`):
//
//   BT4-072 Gogmamon          Black Lv.5 DP 7000, [Rock] trait — the Mineral/Rock
//                             carrier that holds combo sources. Rock satisfies the
//                             `host_permanent_trait_has: Mineral|Rock` inherited
//                             gate; Black is a legal base for the black-filtered
//                             Option digivolves. Its own [Main] Digi-Burst is
//                             player-activated (never auto-fires) and its inherited
//                             +1000 DP only applies when it is itself a source.
//   BT14-009 Gotsumon         Red Lv.3, [Rock] trait, NO inherited body — the
//                             plain Mineral/Rock filler source: trashable by a
//                             Mineral/Rock cost filter, but fans out NO delete.
//                             (Its top-card [All Turns] never applies while it is a
//                             buried source — only inherited bodies apply, and it
//                             has none.)
//   ST1-05 Birdramon / ST2-05 Ikkakumon / ST3-06 Gatomon
//                             vanilla Lv.4 cost-4 Digimon — neutral cost-≤4 opp
//                             victims for the inherited delete fan-outs.
//   ST5-05 Commandramon       Black Lv.3 vanilla — the C2 colour anchor.
//   ST1-09 MetalGreymon       Red Lv.5 (inherited-only) — the C3N non-black base.
//   ST1-04 Dracomon           Red Lv.3 vanilla — deck pad / security fodder.
//   BT23-005 Elizamon         Lv.3 [LIBERATOR] — the C2 reveal-search LIBERATOR
//                             bucket target.

/// Drive every pending selection by always taking the first valid action (or
/// PASS on a candidate-less optional prompt), bounded so a logic bug surfaces as
/// a loop-exhaustion panic rather than a hang. Mirrors the proven driver from
/// `tests/archetypes/rocks.rs`.
fn drive_first_valid(runner: &mut DebugRunner, max_steps: usize) {
    use digimon_engine::action::space::PASS;
    for _ in 0..max_steps {
        let Some(view) = runner.pending_selection_view() else {
            return;
        };
        let player = view.selecting_player;
        let action = view.valid_action_ids.first().copied().unwrap_or(PASS);
        if runner.execute_action(player, action).is_err() {
            return;
        }
    }
    panic!("drive_first_valid exhausted {max_steps} steps without draining the selection queue");
}

/// Restore a card's real `evo_costs` digivolve requirement (color/level/cost)
/// after it was loaded via `dsl_card`. The embedded DSL test loader populates a
/// card's `alt_paths` (used by the player-action digivolve mask) but leaves
/// `evo_costs` empty (`card_data_from_compiled` in `debug_runner.rs`), and the
/// engine's `effect_initiated_digivolve` requirement check matches on
/// `evo_costs`. Effect-initiated digivolves onto a real DSL card therefore can't
/// satisfy the requirement until the card's true `evo_costs` (from `cards.json`)
/// are present. This restores the **faithful** card metadata — it does NOT
/// weaken the digivolve requirement; it makes the real requirement enforceable
/// in the test harness. (Engine/test gap routed to the trackers, see notes.)
fn set_evo_cost(runner: &mut DebugRunner, card_id: &str, color: CardColor, level: u8, cost: u16) {
    let cd = runner
        .game
        .card_data
        .iter_mut()
        .find(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("set_evo_cost: {card_id} not in card_data"));
    cd.evo_costs = vec![digimon_engine::card_data::EvoCost {
        card_color: color as u8,
        level,
        memory_cost: cost,
    }];
}

/// Seed `count` security cards for `player` (so a "trash top security" clause has
/// something to bite), using a real card that is already registered in
/// `card_data` (`base_id`). Returns nothing — read `snapshot().security` for
/// deltas.
fn seed_security(runner: &mut DebugRunner, player: usize, base_id: &str, count: usize) {
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == base_id)
        .unwrap_or_else(|| panic!("seed_security: {base_id} must be loaded into card_data"));
    for _ in 0..count {
        let cs = CardSource::new(idx, player as u8, runner.game.next_card_index());
        runner.game.players[player].security.push(cs);
    }
}

// ─── Combo C1: Zofr Kabus source-trash fan-out removal ───────────────────────

/// C1 — "Zofr Kabus source-trash fan-out removal".
///
/// - Cards: EX8-070 Zofr Kabus (Option, hand) + EX8-047 Sunarizamon (buried as a
///   digivolution source under a Mineral carrier — the inherited-delete payload)
///   + an opponent Digimon with play cost ≤4 (the inherited fan-out victim).
/// - Expected mechanical outcome: play Zofr Kabus `[Main]`. Its cost is "trash 1
///   digivolution card of 1 of your [Mineral]/[Rock] Digimon". Trashing the buried
///   EX8-047 source **(a)** grants the chosen carrier Collision + Piercing +
///   Reboot + +3000 DP until end of opponent's turn, and **(b)** fires EX8-047's
///   inherited "when my source is trashed from a Mineral/Rock Digimon → delete 1
///   opp Digimon with play cost ≤4" body, deleting an opponent Digimon. Board
///   diff: carrier source count −1; carrier gains Collision/Piercing/Reboot +
///   ≥+3000 DP; **opp battle area −1 Digimon (cost ≤4)** + opp trash +1.
/// - Rules/keyword basis: "By trashing … digivolution card" = cost; the source
///   trash dispatches the trashed source's inherited `OnDigivolutionCardTrashed`
///   trigger (gated on `host_permanent_trait_has: Mineral|Rock`). Keyword grants:
///   Collision §16-29, Piercing §16-6, Reboot. Impl:
///   `code/digimon-engine/cards/ex8/EX8-070.yaml` (Main trash + buff),
///   inherited delete `code/digimon-engine/cards/ex8/EX8-047.yaml`.
///
/// The Option-as-cost → inherited-fan-out chain is invisible to a per-card test:
/// EX8-070's own test grants the buff but never trashes a *real* inherited source.
#[test]
fn c1_zofr_kabus_source_trash_fans_out_inherited_delete_and_buffs() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 (Zofr Kabus) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .dsl_card("BT4-072") // Gogmamon — Black/Rock Lv.5 carrier
        .expect("BT4-072 (Gogmamon) in embedded DSL pack")
        .dsl_card("ST1-05") // Birdramon — vanilla cost-4 opp victim
        .expect("ST1-05 (Birdramon) in embedded DSL pack")
        .dsl_card("ST2-05") // Ikkakumon — vanilla cost-4 opp survivor
        .expect("ST2-05 (Ikkakumon) in embedded DSL pack")
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // Opponent fields two cost-≤4 Digimon: one is the inherited fan-out victim,
    // the other survives (proving exactly one extra delete from the fan-out).
    runner.place_on_field(1, "ST1-05", None);
    runner.place_on_field(1, "ST2-05", None);

    // Carrier (Rock trait) with exactly one buried source: EX8-047. The
    // single source makes Zofr Kabus's source pick forced onto the inherited card.
    let carrier = runner.place_on_field(0, "BT4-072", None);
    runner.push_source(carrier, "EX8-047");

    let before = snapshot(&runner);

    // Play Zofr Kabus from hand: select carrier → trash its EX8-047 source →
    // buff carrier → fan out EX8-047's inherited cost-≤4 delete (pick a victim).
    runner.play(0, 0);
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // (a) The buff landed on the carrier: Collision + Piercing + Reboot + +3000 DP.
    assert!(
        runner.game.has_keyword(carrier, Keyword::Collision),
        "Zofr Kabus must grant Collision to the chosen carrier"
    );
    assert!(
        runner.game.has_keyword(carrier, Keyword::Piercing),
        "Zofr Kabus must grant Piercing to the chosen carrier"
    );
    assert!(
        runner.game.has_keyword(carrier, Keyword::Reboot),
        "Zofr Kabus must grant Reboot to the chosen carrier"
    );
    let dp = runner.dp_of(carrier).expect("carrier has DP");
    assert!(
        dp >= 7000 + 3000,
        "Zofr Kabus must add at least +3000 DP to the carrier (BT4-072 base 7000, got {dp})"
    );

    // (a) The EX8-047 source left the carrier stack (cost paid). `card_sources`
    // includes the top card, so a single-source carrier drops from 2 → 1.
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        1,
        "the trashed EX8-047 source must leave the carrier's digivolution stack \
         (only the top carrier card should remain)"
    );

    // (b) The inherited fan-out deleted exactly one cost-≤4 opponent Digimon.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "EX8-047's inherited 'trash → delete opp cost ≤4' must remove exactly 1 opponent Digimon \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "the deleted opponent Digimon must land in the opponent's trash"
    );
}

/// C1 unhappy path: trashing a NON-inherited-delete source (a plain Mineral
/// filler instead of EX8-047) still pays the cost and applies the buff, but
/// fires NO cost-≤4 delete — the fan-out is source-specific. This is the
/// system-level fact a per-card EX8-070 test can't express.
#[test]
fn c1_zofr_kabus_non_inherited_source_buffs_but_does_not_fan_out_delete() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-070")
        .expect("EX8-070 (Zofr Kabus) in embedded DSL pack")
        .dsl_card("BT4-072") // Gogmamon — Black/Rock Lv.5 carrier
        .expect("BT4-072 (Gogmamon) in embedded DSL pack")
        .dsl_card("BT14-009") // Gotsumon — plain Rock filler (no inherited body)
        .expect("BT14-009 (Gotsumon) in embedded DSL pack")
        .dsl_card("ST1-05") // Birdramon — vanilla cost-4 opp Digimon
        .expect("ST1-05 (Birdramon) in embedded DSL pack")
        .dsl_card("ST2-05") // Ikkakumon — vanilla cost-4 opp Digimon
        .expect("ST2-05 (Ikkakumon) in embedded DSL pack")
        .hand(0, &["EX8-070"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(1, "ST1-05", None);
    runner.place_on_field(1, "ST2-05", None);

    let carrier = runner.place_on_field(0, "BT4-072", None);
    // The only buried source is a plain Rock filler (BT14-009) with no inherited
    // body — it pays the cost but fans out no delete.
    runner.push_source(carrier, "BT14-009");

    let before = snapshot(&runner);
    runner.play(0, 0);
    drive_first_valid(&mut runner, 30);
    let after = snapshot(&runner);

    // The buff still applies (the cost was paid by trashing the filler).
    assert!(
        runner.game.has_keyword(carrier, Keyword::Collision),
        "the buff must still apply when trashing a non-inherited source"
    );
    // But NO opponent Digimon is deleted: the plain filler has no fan-out body.
    assert_eq!(
        after.field[1], before.field[1],
        "trashing a non-inherited-delete source must NOT fan out any delete \
         (before={}, after={})",
        before.field[1], after.field[1],
    );
}

// ─── Combo C2: Gravel Hearts free-play (from trash) → Sunarizamon search ──────

/// C2 — "Gravel Hearts free-play → recur Sunarizamon search".
///
/// - Cards: EX10-069 Unique Emblem: Gravel Hearts (Option, hand) + EX8-047
///   Sunarizamon available **in trash** (the recursion branch).
/// - Expected mechanical outcome: play Gravel Hearts `[Main]`, choose "From
///   trash", then free-play 1 [Sunarizamon] from trash without paying its cost;
///   Gravel Hearts then places itself in the battle area as a Delay Option. The
///   free-played EX8-047 Sunarizamon's `[On Play]` fires: reveal top 3, add 1
///   Mineral/Rock + 1 LIBERATOR to hand, rest to bottom. Board diff: battle area
///   +1 Sunarizamon (cost paid: 0); **trash −1 Sunarizamon** (recursion out of
///   trash); hand +up to 2 search cards (minus the Gravel Hearts that left hand);
///   Gravel Hearts → battle area (OnSuspend Delay armed).
/// - Rules/keyword basis: "play … without paying the cost" (free-play, no memory
///   for the body); Delay placement §16-16. Impl:
///   `code/digimon-engine/cards/ex10/EX10-069.yaml` (`play_from_trash_free`);
///   On-Play search `code/digimon-engine/cards/ex8/EX8-047.yaml`.
///
/// The Rocks exemplar (`rocks.rs` C5) covers the HAND branch; this pins the
/// distinct **trash-recursion** branch + the chained On-Play search.
#[test]
fn c2_gravel_hearts_free_plays_sunarizamon_from_trash_and_searches() {
    use digimon_engine::enums::DelayTrigger;
    use digimon_engine::selection::OptionPlayResult;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-069")
        .expect("EX10-069 (Gravel Hearts) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        // ST5-05 Commandramon — Black Lv.3 anchor (satisfies the Black Option's
        // colour requirement at play time).
        .dsl_card("ST5-05")
        .expect("ST5-05 (Commandramon) in embedded DSL pack")
        // Deck fodder so EX8-047's reveal-top-3 search has both buckets to fill:
        // BT14-009 Gotsumon ([Rock]) + BT23-005 Elizamon ([LIBERATOR]) + a pad.
        .dsl_card("BT14-009")
        .expect("BT14-009 (Gotsumon) in embedded DSL pack")
        .dsl_card("BT23-005")
        .expect("BT23-005 (Elizamon) in embedded DSL pack")
        .dsl_card("ST1-04")
        .expect("ST1-04 (Dracomon) in embedded DSL pack")
        .hand(0, &["EX10-069"])
        .deck(0, &["BT14-009", "BT23-005", "ST1-04"])
        .deck(1, &["ST1-04"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "ST5-05", Some(0));
    // EX8-047 seeded in trash — the recursion target.
    runner.inject_trash(0, "EX8-047");
    runner.game.enter_main_phase();

    let before = snapshot(&runner);

    // Gravel Hearts [Main] parks on its zone-choice selection.
    let gh_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "EX10-069")
        .expect("Gravel Hearts in hand");
    assert_eq!(
        runner.game.play_option_from_hand(0, gh_idx),
        OptionPlayResult::Pending,
        "EX10-069 [Main] must park on its zone-choice selection"
    );
    // Drive the option pipeline. At the zone EffectChoice prompt, explicitly pick
    // the SECOND branch ("From trash") — drive_first_valid would take "From hand"
    // (index 0). All other prompts (any option-play replacement window, the trash
    // pick, EX8-047's On-Play reveal-3 two-pick, the self-placement) take the
    // first valid action. The branch is identified by SelectionKind::EffectChoice
    // so it works regardless of where the zone prompt sits in the queue.
    {
        use digimon_engine::action::space::PASS;
        use digimon_engine::selection::SelectionKind;
        let mut chose_trash = false;
        for _ in 0..40 {
            let Some(view) = runner.pending_selection_view() else {
                break;
            };
            let action = if !chose_trash && view.kind == SelectionKind::EffectChoice {
                assert!(
                    view.valid_action_ids.len() >= 2,
                    "the zone choice must expose both 'From hand' and 'From trash'"
                );
                chose_trash = true;
                view.valid_action_ids[1] // "From trash"
            } else {
                view.valid_action_ids.first().copied().unwrap_or(PASS)
            };
            if runner.execute_action(view.selecting_player, action).is_err() {
                break;
            }
        }
        assert!(chose_trash, "the Gravel Hearts zone EffectChoice must have appeared");
    }
    let after = snapshot(&runner);

    // The Sunarizamon was recurred OUT of trash onto the field.
    let suna_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX8-047");
    assert!(
        suna_on_field,
        "the free-played EX8-047 Sunarizamon must be on the field"
    );
    assert!(
        after.trash[0] < before.trash[0],
        "the recurred Sunarizamon must leave trash (before={}, after={})",
        before.trash[0],
        after.trash[0],
    );

    // The free-play costs NO memory for the body: only Gravel Hearts' own cost
    // (3) is paid. If the body had been charged its cost-3 too, memory would have
    // dropped by 6.
    assert_eq!(
        before.memory - after.memory,
        3,
        "only Gravel Hearts' own cost (3) may be paid; the free-played Sunarizamon \
         must add no further memory drain (before={}, after={})",
        before.memory,
        after.memory,
    );

    // EX8-047's On-Play search ran: net hand grew (Gravel Hearts left hand, but
    // the two-pick added up to 2 cards, so hand is at least its pre-play size).
    assert!(
        after.hand[0] >= before.hand[0],
        "EX8-047 On-Play search must add cards: hand should not shrink below its \
         pre-play size despite Gravel Hearts leaving hand (before={}, after={})",
        before.hand[0],
        after.hand[0],
    );

    // Gravel Hearts placed itself as an OnSuspend-gated Delay Option.
    let gh = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "EX10-069")
        .expect("Gravel Hearts must be placed in the battle area as a Delay option");
    assert!(
        matches!(
            gh.option_state,
            OptionState::Delayed {
                trigger: DelayTrigger::OnEvent(EffectTiming::OnSuspend),
                ..
            }
        ),
        "Gravel Hearts must be parked as an OnSuspend Delay; got {:?}",
        gh.option_state
    );
}

// ─── Combo C3: Black Scramble cost-reduced digivolve into the payoff ─────────

/// C3 — "Black Scramble cost-reduced digivolve into payoff".
///
/// - Cards: LM-031 Black Scramble (Option, hand) + a Black Mineral carrier on
///   field (Lv.5 base) + EX10-033 Pyramidimon (Black Lv.6, real evo cost from
///   Lv.5 black = 3, per `cards.json` `evo_costs`) as the in-hand digivolve target.
/// - Expected mechanical outcome: play Black Scramble `[Main]`. Choose the
///   on-field black Digimon, then EX10-033 from hand; it performs an
///   effect-initiated digivolve with the digivolution cost reduced by 3
///   (`effect_initiated_digivolve cost {reduce: 3}`), then Black Scramble places
///   itself as a Delay Option. Board diff: the chosen permanent's top card
///   becomes EX10-033 (prior top now a source beneath); hand −1 (the evo target)
///   plus Black Scramble leaving hand; **memory paid for the digivolve =
///   max(0, evoCost − 3) = max(0, 3 − 3) = 0** (the model's "Pyramidimon → 0" is
///   correct against the real evo cost of 3); Black Scramble → battle area as a
///   Delay. Total memory drop = Black Scramble's own cost (2) + 0 = 2.
/// - Rules/keyword basis: effect-initiated digivolve + cost reduction; Delay
///   §16-16. Impl: `code/digimon-engine/cards/lm/LM-031.yaml` (Clause A
///   `effect_initiated_digivolve cost {reduce:3}`, `color_is: black` filters);
///   EX10-033 evo cost from `data/cards.json` (`evo_costs: [{color:5, level:5,
///   memory_cost:3}]`).
///
/// The black-colour filter on both the base and the hand target is a
/// system-level gate (see the unhappy path below).
#[test]
fn c3_black_scramble_cost_reduced_digivolve_into_pyramidimon() {
    use digimon_engine::selection::OptionPlayResult;

    let mut runner = DebugRunner::builder()
        .dsl_card("LM-031")
        .expect("LM-031 (Black Scramble) in embedded DSL pack")
        .dsl_card("EX10-033")
        .expect("EX10-033 (Pyramidimon) in embedded DSL pack")
        .dsl_card("BT4-072") // Gogmamon — Black/Rock Lv.5 base
        .expect("BT4-072 (Gogmamon) in embedded DSL pack")
        .hand(0, &["LM-031", "EX10-033"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    // Restore EX10-033's real digivolve requirement (Lv.5 black, cost 3) — the
    // DSL test loader leaves evo_costs empty (see set_evo_cost doc-comment).
    set_evo_cost(&mut runner, "EX10-033", CardColor::Black, 5, 3);

    // The Black Lv.5 base on field (placed a past turn so it is a legal
    // digivolve base this turn) — the digivolve source.
    let base = runner.place_on_field(0, "BT4-072", Some(0));

    let before = snapshot(&runner);

    let bs_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LM-031")
        .expect("Black Scramble in hand");
    let res = runner.game.play_option_from_hand(0, bs_idx);
    assert_eq!(
        res,
        OptionPlayResult::Pending,
        "LM-031 [Main] must park on its (optional) base-selection"
    );
    // Drive: pick the black base → pick EX10-033 in hand → effect-digivolve −3 →
    // resolve EX10-033's [When Digivolving] follow-ups → place Black Scramble.
    drive_first_valid(&mut runner, 40);
    let after = snapshot(&runner);

    // The base's top card is now Pyramidimon.
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX10-033",
        "the chosen black base must have digivolved into EX10-033 Pyramidimon"
    );
    // Memory paid for the digivolve = max(0, 3 − 3) = 0; plus Black Scramble's
    // own play cost (2). Total memory drop = 2.
    assert_eq!(
        before.memory - after.memory,
        2,
        "memory drop must be Black Scramble's own cost (2) + cost-reduced digivolve \
         residual (max(0, 3−3)=0) = 2 (before={}, after={})",
        before.memory,
        after.memory,
    );
    // Black Scramble placed itself as a Delay Option.
    let bs_parked = runner.game.players[0].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == "LM-031"
            && matches!(p.option_state, OptionState::Delayed { .. })
    });
    assert!(
        bs_parked,
        "Black Scramble must place itself in the battle area as a Delay Option"
    );
}

/// C3 unhappy path: a NON-black base is not a legal selection for Black
/// Scramble's `color_is: black` base filter. With only a Red base on field, the
/// `[Main]` clause finds no eligible base and the cost-reduced digivolve cannot
/// run — the system-level colour gate. (LM-031 is fully optional via "may", so
/// it still places itself as a Delay; the digivolve simply does not occur.)
#[test]
fn c3_black_scramble_non_black_base_does_not_digivolve() {
    // ST1-09 MetalGreymon — a RED Lv.5 base, excluded by LM-031's color_is:
    // black filter (its only printed effect is inherited, never active as a top
    // card, and it is never digivolved here).
    let mut runner = DebugRunner::builder()
        .dsl_card("LM-031")
        .expect("LM-031 (Black Scramble) in embedded DSL pack")
        .dsl_card("EX10-033")
        .expect("EX10-033 (Pyramidimon) in embedded DSL pack")
        .dsl_card("ST1-09")
        .expect("ST1-09 (MetalGreymon) in embedded DSL pack")
        .hand(0, &["LM-031", "EX10-033"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let base = runner.place_on_field(0, "ST1-09", None);

    let bs_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "LM-031")
        .expect("Black Scramble in hand");
    let _ = runner.game.play_option_from_hand(0, bs_idx);
    drive_first_valid(&mut runner, 30);

    // The red base must NOT have digivolved — it is excluded by the black filter.
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "ST1-09",
        "a non-black base must not be a legal Black Scramble digivolve source"
    );
}

// ─── Combo C4: Defense Training Delay cost-reduced digivolve ──────────────────

/// C4 — "Defense Training Delay cost-reduced digivolve".
///
/// - Cards: P-107 Defense Training (Option) placed as an armed Delay in the
///   battle area + a black Digimon card in hand (EX10-033 Pyramidimon) + a black
///   Mineral base on field as the digivolve target.
/// - Expected mechanical outcome: activate Defense Training's `<Delay>` by
///   trashing it: choose 1 of your Digimon, then a **black** Digimon card in
///   hand; it digivolves into that card for digivolution cost reduced by 2
///   (`effect_initiated_digivolve cost {reduce: 2}`). Board diff: Defense
///   Training → trash (Delay consumed); the chosen permanent's top card becomes
///   EX10-033 (prior top beneath as a source); hand −1; memory paid =
///   max(0, evoCost − 2) = max(0, 3 − 2) = 1 (EX10-033's real Lv.5 black evo
///   cost is 3 per `data/cards.json`); the new Digimon's `[When Digivolving]`
///   fires.
/// - Rules/keyword basis: Delay §16-16 (activated by trashing the placed card on
///   a later main phase); effect-initiated digivolve −2. Impl:
///   `code/digimon-engine/cards/p/P-107.yaml` (`kind: delay`,
///   `effect_initiated_digivolve cost {reduce:2}`).
///
/// This pins the Delay form of the Option cost-reduction line (distinct from
/// C3's in-hand `[Main]` form): the digivolve fires only via the armed `<Delay>`.
#[test]
fn c4_defense_training_delay_cost_reduced_digivolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-107")
        .expect("P-107 (Defense Training) in embedded DSL pack")
        .dsl_card("EX10-033")
        .expect("EX10-033 (Pyramidimon) in embedded DSL pack")
        .dsl_card("BT4-072") // Gogmamon — Black/Rock Lv.5 base
        .expect("BT4-072 (Gogmamon) in embedded DSL pack")
        .hand(0, &["EX10-033"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    // Restore EX10-033's real digivolve requirement (Lv.5 black, cost 3).
    set_evo_cost(&mut runner, "EX10-033", CardColor::Black, 5, 3);

    // The black Lv.5 base on field (placed a past turn) — the digivolve target.
    let base = runner.place_on_field(0, "BT4-072", Some(0));

    // Install Defense Training as an armed Delay Option with its placing turn in
    // the PAST (turn 0) so §16-16 ("after the placing turn") permits activation
    // on the current turn (turn 1). `install_field_option_as_delay` reads P-107's
    // own delay_trigger (MainPhaseActivated) from its compiled effects.
    let dt = {
        let data_idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "P-107")
            .expect("P-107 registered");
        let next = runner.game.next_card_index();
        let cs = CardSource::new(data_idx, 0, next);
        runner.game.install_field_option_as_delay(cs, 0, 0)
    };
    // The placing turn (0) is strictly before the current turn (1) — activation
    // is now legal. (The same-turn-placement unhappy path — a Delay cannot fire
    // on its placing turn, §16-16 — is asserted for Gravel Hearts in rocks.rs C5;
    // the gating function `delayed_option_main_activation_available` returns false
    // when `turn_count == placed_on_turn`, which is the same rule applied here.)
    assert!(
        runner.game.delayed_option_main_activation_available(dt),
        "Defense Training's <Delay> must be activatable after its placing turn (§16-16)"
    );

    let before = snapshot(&runner);

    // Activate the Delay (trash P-107 → choose base → choose EX10-033 → −2 evo).
    assert!(
        runner.game.activate_delayed_option_main(dt),
        "activating Defense Training's <Delay> must dispatch its body"
    );
    drive_first_valid(&mut runner, 40);
    let after = snapshot(&runner);

    // The base digivolved into EX10-033 Pyramidimon.
    assert_eq!(
        runner.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "EX10-033",
        "the Delay must digivolve the chosen base into EX10-033 Pyramidimon"
    );
    // Defense Training was consumed (trashed) by activating the Delay. (Trash
    // may also grow from EX10-033's own [When Digivolving] source-trash clause,
    // so assert P-107 specifically reached trash rather than a raw count.)
    let p107_in_trash = runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "P-107");
    assert!(
        p107_in_trash,
        "activating the <Delay> must trash Defense Training (P-107)"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "P-107"),
        "Defense Training must no longer be a field Option after the Delay is consumed"
    );
    // Memory paid = max(0, evoCost(3) − 2) = 1.
    assert_eq!(
        before.memory - after.memory,
        1,
        "the −2 cost-reduced digivolve of EX10-033 (real evo cost 3) must pay 1 memory \
         (before={}, after={})",
        before.memory,
        after.memory,
    );
}

// ─── Combo C5: Magneticdramon source-trash double removal ────────────────────

/// C5 — "Magneticdramon source-trash double removal".
///
/// - Cards: EX10-036 Magneticdramon (payoff) + EX8-047 Sunarizamon (the
///   inherited-delete payload) + Mineral fillers to complete the 3-source cost +
///   opponent cost-≤4 Digimon + opp top security.
/// - Expected mechanical outcome: on Magneticdramon's `[When Digivolving]`,
///   trash exactly 3 [Mineral]/[Rock] sources → delete 1 chosen opp Digimon +
///   trash opp top security (active clause). Then a trashed inherited-delete
///   source fires "trash → delete opp cost ≤4", deleting a SECOND opp Digimon.
///   Board diff: opp battle area −2 Digimon, opp security −1.
///
///   FAITHFULNESS / MODEL CORRECTION (filed, matches `Rocks-model.md` C1
///   correction): **EX8-047's traits are `[Reptile, LIBERATOR]`**
///   (`cards/ex8/EX8-047.yaml`), NOT Mineral/Rock. Magneticdramon's cost filter
///   is `trait_has: Mineral|Rock` (`cards/ex10/EX10-036.yaml`), so EX8-047 is
///   **never selectable** as one of the 3 trashed sources via this line — its
///   inherited delete therefore cannot fire from Magneticdramon's cost. To
///   produce the faithful double-delete the trashed inherited source must itself
///   be Mineral/Rock-trait. EX8-048 Landramon (Mineral-trait, same cost-≤4
///   inherited body) is the correct partner; the Rocks exemplar's C1 already
///   pins exactly this with EX8-048. This test therefore loads the model-named
///   EX8-047 AND uses a Mineral-trait inherited-delete carrier source so the
///   faithful double-delete is reproducible without weakening the assertion, and
///   asserts EX8-047 cannot participate.
/// - Rules/keyword basis: "by trashing 3 … sources" = cost; source trash
///   dispatches each inherited `OnDigivolutionCardTrashed`; Fragment(3) §16-36.
///   Impl: `cards/ex10/EX10-036.yaml`, inherited `cards/ex8/EX8-047.yaml`.
#[test]
fn c5_magneticdramon_source_trash_double_removal() {
    // A Mineral-trait inherited-delete source that CAN be trashed by
    // Magneticdramon's Mineral/Rock cost filter and carries the same cost-≤4
    // delete body as EX8-047. (EX8-048 Landramon is the faithful partner; the
    // model named EX8-047 which is Reptile-trait and cannot participate — see the
    // doc-comment correction. We load EX8-047 too, to assert it is excluded.)
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-036")
        .expect("EX10-036 (Magneticdramon) in embedded DSL pack")
        .dsl_card("EX8-047")
        .expect("EX8-047 (Sunarizamon) in embedded DSL pack")
        .dsl_card("EX8-048")
        .expect("EX8-048 (Landramon) in embedded DSL pack")
        .dsl_card("BT14-009") // Gotsumon — plain Rock filler source #1
        .expect("BT14-009 (Gotsumon) in embedded DSL pack")
        .dsl_card("EX8-046") // Gotsumon — plain Rock filler source #2 (Blocker-only inherited)
        .expect("EX8-046 (Gotsumon) in embedded DSL pack")
        .dsl_card("BT4-072") // Gogmamon — Black/Rock Lv.5 carrier
        .expect("BT4-072 (Gogmamon) in embedded DSL pack")
        .dsl_card("ST1-05") // Birdramon — vanilla cost-4 opp Digimon
        .expect("ST1-05 (Birdramon) in embedded DSL pack")
        .dsl_card("ST2-05") // Ikkakumon — vanilla cost-4 opp Digimon
        .expect("ST2-05 (Ikkakumon) in embedded DSL pack")
        .dsl_card("ST3-06") // Gatomon — vanilla cost-4 opp Digimon
        .expect("ST3-06 (Gatomon) in embedded DSL pack")
        .dsl_card("ST1-04") // Dracomon — security fodder
        .expect("ST1-04 (Dracomon) in embedded DSL pack")
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // EX8-047 is Reptile/LIBERATOR — confirm it is NOT Mineral/Rock, so it can
    // never satisfy Magneticdramon's trait-filtered cost.
    let suna = runner
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == "EX8-047")
        .expect("EX8-047 loaded");
    assert!(
        !suna.traits.iter().any(|t| t == "Mineral" || t == "Rock"),
        "EX8-047 must be Reptile/LIBERATOR (not Mineral/Rock) — it cannot be a \
         Magneticdramon cost source; traits={:?}",
        suna.traits
    );

    // Opponent fields three cost-≤4 Digimon: 1 active-clause victim + 1 fan-out
    // victim + 1 survivor (proving exactly one extra delete from the fan-out).
    runner.place_on_field(1, "ST1-05", None);
    runner.place_on_field(1, "ST2-05", None);
    runner.place_on_field(1, "ST3-06", None);
    seed_security(&mut runner, 1, "ST1-04", 2);

    // Carrier stack: the Mineral inherited-delete source (EX8-048) + 2 plain Rock
    // fillers under a Rock carrier, so Clause A's "trash 3" picks all three
    // Mineral/Rock-trait cards (EX8-048 fans out a second delete).
    let carrier = runner.place_on_field(0, "BT4-072", None);
    runner.push_source(carrier, "EX8-048");
    runner.push_source(carrier, "BT14-009");
    runner.push_source(carrier, "EX8-046");

    let magnet = runner.place_on_field(0, "EX10-036", None);

    let before = snapshot(&runner);
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(magnet));
    runner.game.drain_effect_queue();
    drive_first_valid(&mut runner, 50);
    let after = snapshot(&runner);

    // Active delete (1) + EX8-048 inherited fan-out delete (1) = 2 removed.
    assert_eq!(
        after.field[1],
        before.field[1] - 2,
        "Clause A active delete + EX8-048 inherited fan-out must remove 2 opponent Digimon \
         (before={}, after={})",
        before.field[1],
        after.field[1],
    );
    // Clause A trashes the opponent's top security card.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "Clause A must trash the opponent's top security card (before={}, after={})",
        before.security[1],
        after.security[1],
    );
}
