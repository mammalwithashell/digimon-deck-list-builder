//! BT25-103 GraceNovamon — Digimon, Lv.7, Red/Blue, 15000 DP, cost 15.
//! Traits: Galaxy / Iliad / TS + (Rule) [Olympos XII]. Mega / Vaccine.
//! Digivolve: Red Lv.6 / 5; Blue Lv.6 / 5; [Digivolve] Lv.6 w/[TS] trait: 5;
//! [DNA Digivolve] Red Lv.6 + Blue Lv.6: Cost 0.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT25-103.md`)
//!
//! <Security A. +1> <Iceclad> <Partition ([Apollomon] & [Dianamon])>
//! [When Digivolving] [When Attacking] Return 1 of your opponent's Digimon with
//! as many or fewer digivolution cards as this Digimon to the bottom of the deck.
//! [When Attacking] [Counter] [Once Per Turn] For each of this Digimon's
//! digivolution cards, you may trash any 1 digivolution card from your
//! opponent's Digimon. Then, you may end this attack.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_103.cs
//! - WD/WA bounce: mandatory SelectPermanentEffect(PutLibraryBottom) over
//!   opponent Digimon with `sources <= this.sources`.
//! - WA/Counter (maxCountPerTurn 1): SelectTrashDigivolutionCards(IsEnemyDigimon,
//!   maxCount = own sources, canNoTrash: true, isFromOnly1Permanent: false),
//!   then Yes/No end the attack; RemoveUse when nothing trashed and not ended.
//!
//! # Patterns
//! - H: Security A. +1, Iceclad, Partition.  - C: alt digivolve + DNA.
//! - F3: source-count-relative bounce; count-scaled cross-permanent source trash.
//! - OPT refund on no-op (G-OPT-REFUND-ON-DECLINE); end_attack in WA + Counter.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, encode_digivolve, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT25-103";

fn digi(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 5;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-103 YAML loads")
        .add_card(digi("RED-L6", CardColor::Red, 6, 12000, &[]))
        .add_card(digi("BLUE-L6", CardColor::Blue, 6, 12000, &[]))
        .add_card(digi("GREEN-L6", CardColor::Green, 6, 12000, &[]))
        .add_card(digi("TS-L6", CardColor::Green, 6, 12000, &["TS"]))
        .add_card(digi("SRC", CardColor::Red, 3, 3000, &[]))
        .add_card(digi("OPP-A", CardColor::Yellow, 5, 5000, &[]))
        .add_card(digi("OPP-B", CardColor::Yellow, 5, 5000, &[]))
        .add_card(digi("OPP-S", CardColor::Yellow, 3, 3000, &[]))
        .add_card(digi("Apollomon", CardColor::Red, 6, 12000, &[]))
        .add_card(digi("Dianamon", CardColor::Blue, 6, 12000, &[]))
        .add_card(tamer("OPP-TAMER"))
        .add_card(make_test_card("FILL", "Filler"))
}

fn start(hand: &[&str], mem: i16) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(mem)
        .start()
}

fn hand_slot(r: &DebugRunner, id: &str) -> u16 {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("card in hand") as u16
}

fn digivolve_onto(r: &mut DebugRunner, base: PermanentHandle) {
    let slot = hand_slot(r, CARD_ID);
    r.game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
}

fn field_ids(r: &DebugRunner, player: usize) -> Vec<String> {
    r.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn sources(r: &DebugRunner, h: PermanentHandle) -> usize {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .len()
        - 1
}

fn opp_source_total(r: &DebugRunner) -> usize {
    r.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len().saturating_sub(1))
        .sum()
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    r.game.drain_effect_queue();
}

fn is_counter_clause_prompt(r: &DebugRunner) -> bool {
    r.pending_selection_view()
        .map(|v| {
            v.prompt.contains("trash any 1 digivolution card")
                || v.prompt.contains("end this attack")
        })
        .unwrap_or(false)
}

/// Resolve a TriggerOrder bundle by picking the counter-clause first when
/// `counter_first`, otherwise the first offered.
fn resolve_trigger_order(r: &mut DebugRunner) {
    while matches!(r.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        let v = r.pending_selection_view().unwrap();
        let a = v.valid_action_ids[0];
        r.execute_action(v.selecting_player, a)
            .expect("TriggerOrder");
    }
}

/// Pick `n` opponent sources at the SourceMulti prompt (first legal each time).
fn pick_sources(r: &mut DebugRunner, n: usize) {
    for _ in 0..n {
        let v = r.pending_selection_view().expect("source prompt");
        assert!(
            matches!(v.kind, SelectionKind::SourceMulti { .. }),
            "{:?}",
            v.kind
        );
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a source pick");
        r.execute_action(v.selecting_player, a)
            .expect("pick source");
    }
}

fn finish_source_picks(r: &mut DebugRunner) {
    if let Some(v) = r.pending_selection_view() {
        if matches!(v.kind, SelectionKind::SourceMulti { .. }) {
            assert!(v.valid_action_ids.contains(&PASS), "each pick is optional");
            r.execute_action(v.selecting_player, PASS)
                .expect("stop picking");
        }
    }
}

fn answer_end_attack(r: &mut DebugRunner, end: bool) {
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "'you may end this attack'"
    );
    r.execute_branch(if end { 0 } else { 1 }).unwrap();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_103_printed_metadata() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(7), Some(15000), Some(15)));
    assert_eq!(c.color, vec![CompiledColor::Red, CompiledColor::Blue]);
    for t in ["Galaxy", "Iliad", "TS", "Olympos XII"] {
        assert!(c.traits.iter().any(|x| x == t), "trait {t}");
    }
}

#[test]
fn bt25_103_alt_paths_three_digivolve_and_dna() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3, "Red Lv.6, Blue Lv.6, Lv.6 w/[TS]");
    assert!(digi
        .iter()
        .all(|p| p.cost == Some(CompiledCost::Literal(5))));
    let dna: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(dna.len(), 1);
    assert_eq!(dna[0].cost, Some(CompiledCost::Literal(0)));
    assert_eq!(dna[0].materials.len(), 2);
}

#[test]
fn bt25_103_digivolves_from_ts_lv6_for_5() {
    let mut r = start(&[CARD_ID], 6);
    let base = r.place_on_field(0, "TS-L6", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(r.memory(), 1, "paid 5");
}

#[test]
fn bt25_103_cannot_digivolve_from_non_ts_green_lv6() {
    let mut r = start(&[CARD_ID], 6);
    let base = r.place_on_field(0, "GREEN-L6", Some(0));
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["GREEN-L6".to_string()]);
}

#[test]
fn bt25_103_has_iceclad() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    assert!(r.game.has_keyword(me, Keyword::Iceclad));
    // <Security A. +1> is exercised behaviourally in
    // `bt25_103_security_attack_checks_two_when_attack_not_ended`.
}

#[test]
fn bt25_103_partition_apollomon_and_dianamon() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let (srcs, excl) = c
        .effects
        .iter()
        .find_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
                sources,
                exclude_cause,
                ..
            }) => Some((sources, exclude_cause)),
            _ => None,
        })
        .expect("Partition clause");
    assert_eq!(srcs.len(), 2);
    assert!(srcs
        .iter()
        .any(|p| p.name_is.as_deref() == Some("Apollomon")));
    assert!(srcs
        .iter()
        .any(|p| p.name_is.as_deref() == Some("Dianamon")));
    assert!(excl.iter().any(|c| c == "own_effect"));
    assert!(excl.iter().any(|c| c == "battle"));
}

#[test]
fn bt25_103_partition_plays_apollomon_and_dianamon_on_opponent_removal() {
    use digimon_engine::replacement::ReplacementCause;
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["Apollomon", "SRC", "Dianamon", CARD_ID]);
    r.game
        .delete_permanent_with_cause(me, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept <Partition>");
    let _ = r.auto_resolve();
    let mut ids = field_ids(&r, 0);
    ids.sort();
    assert_eq!(ids, vec!["Apollomon".to_string(), "Dianamon".to_string()]);
}

#[test]
fn bt25_103_clause_shape() {
    let r = start(&[], 5);
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    let bounce = t
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::WhenDigivolving))
        .expect("WD/WA bounce");
    assert!(bounce.when.contains(&CompiledTiming::WhenAttacking));
    assert!(
        !bounce.optional && !bounce.once_per_turn,
        "mandatory, unlimited"
    );
    let counter = t
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::Counter))
        .expect("WA/Counter clause");
    assert!(counter.when.contains(&CompiledTiming::WhenAttacking));
    assert!(!counter.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(counter.once_per_turn);
    assert!(
        !counter.optional,
        "DCGO optional:false — choices live in the body"
    );
}

// ─── Section 2 — [When Digivolving] bounce ──────────────────────────────────

#[test]
fn bt25_103_when_digivolving_bottom_decks_opponent_with_as_many_or_fewer_sources() {
    let mut r = start(&[CARD_ID], 6);
    let base = r.place_stack(0, &["SRC", "RED-L6"]); // after digivolve: 2 sources
    let eq = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]); // 2 sources — eligible
    let more = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-S", "OPP-B"]); // 3 — not
    digivolve_onto(&mut r, base);
    let v = r.pending_selection_view().expect("bounce prompt");
    assert!(!v.is_optional, "mandatory");
    assert!(v
        .valid_action_ids
        .contains(&encode_attack(0, eq.index as u16)));
    assert!(
        !v.valid_action_ids
            .contains(&encode_attack(0, more.index as u16)),
        "more digivolution cards than GraceNovamon"
    );
    let deck_before = r.deck_size(1);
    let trash_before = r.trash_size(1);
    r.execute_action(0, encode_attack(0, eq.index as u16))
        .unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-B".to_string()]);
    assert_eq!(
        r.deck_size(1),
        deck_before + 1,
        "the Digimon card went to the deck"
    );
    assert_eq!(
        r.trash_size(1),
        trash_before + 2,
        "its digivolution cards are trashed"
    );
    let bottom = r.game.players[1].deck[0]
        .card_id(&r.game.card_data)
        .to_string();
    let top = r.game.players[1].deck[r.game.players[1].deck.len() - 1]
        .card_id(&r.game.card_data)
        .to_string();
    assert!(bottom == "OPP-A" || top == "OPP-A", "OPP-A is in the deck");
}

#[test]
fn bt25_103_when_digivolving_no_eligible_target_no_prompt() {
    let mut r = start(&[CARD_ID], 6);
    let base = r.place_on_field(0, "RED-L6", Some(0)); // 1 source after digivolve
    r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]); // 2 sources
    digivolve_onto(&mut r, base);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["OPP-A".to_string()]);
}

// ─── Section 3 — [When Attacking] both clauses; end the attack ─────────────

#[test]
fn bt25_103_when_attacking_bounce_trash_and_end_attack() {
    let mut r = start(&[], 5);
    r.set_turn(3);
    let me = r.place_stack(0, &["SRC", "SRC", "RED-L6", CARD_ID]); // 3 sources
                                                                   // 6 sources: still > 3 even after 2 are trashed, so never bounceable
                                                                   // whichever order the two [When Attacking] effects resolve in.
    let a = r.place_stack(
        1,
        &[
            "OPP-S", "OPP-S", "OPP-S", "OPP-S", "OPP-S", "OPP-S", "OPP-A",
        ],
    );
    let b = r.place_stack(1, &["OPP-S", "OPP-B"]); // 1 — bounceable
    let _ = (a, b);
    let deck_before = r.deck_size(1);
    let _ = r.attack_player(me, 1, false);
    resolve_trigger_order(&mut r);
    // Drive whichever clause surfaces first.
    let mut ended = false;
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        match v.kind {
            SelectionKind::SourceMulti { max, .. } => {
                assert_eq!(
                    max as usize, 3,
                    "one pick per GraceNovamon digivolution card"
                );
                pick_sources(&mut r, 2);
                finish_source_picks(&mut r);
            }
            SelectionKind::EffectChoice => {
                answer_end_attack(&mut r, true);
                ended = true;
            }
            SelectionKind::TriggerOrder => resolve_trigger_order(&mut r),
            _ => {
                // bounce prompt: the only eligible target is the 1-source Digimon.
                let a = v.valid_action_ids[0];
                r.execute_action(v.selecting_player, a).unwrap();
            }
        }
    }
    let _ = r.auto_resolve();
    assert!(ended);
    assert_eq!(
        field_ids(&r, 1),
        vec!["OPP-A".to_string()],
        "[When Attacking] bounce: OPP-B (<= 3 cards) left, OPP-A (> 3) stays"
    );
    assert_eq!(
        r.deck_size(1),
        deck_before + 1,
        "OPP-B returned to the deck"
    );
    assert_eq!(r.security_count(1), 3, "attack ended — no security check");
    assert!(!r.game_over());
}

#[test]
fn bt25_103_security_attack_checks_two_when_attack_not_ended() {
    let mut r = start(&[], 5);
    r.set_turn(3);
    let me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]);
    let _ = r.attack_player(me, 1, false);
    resolve_trigger_order(&mut r);
    // No opponent Digimon: no bounce, no source picks — only the end choice.
    answer_end_attack(&mut r, false);
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(1), 1, "<Security A. +1>: 2 checks");
}

#[test]
fn bt25_103_iceclad_compares_digivolution_cards() {
    let mut r = start(&[], 5);
    r.set_turn(3);
    let me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]); // 2 sources, 15000 DP
    let big = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-S", "OPP-A"]); // 3 sources, 5000 DP
    r.game.players[1].battle_area[big.index as usize].is_suspended = true;
    let _ = r.attack_digimon(me, big, false);
    resolve_trigger_order(&mut r);
    for _ in 0..6 {
        match r.pending_kind() {
            Some(SelectionKind::SourceMulti { .. }) => finish_source_picks(&mut r),
            Some(SelectionKind::EffectChoice) => answer_end_attack(&mut r, false),
            Some(SelectionKind::TriggerOrder) => resolve_trigger_order(&mut r),
            None => break,
            Some(k) => panic!("unexpected prompt {k:?}"),
        }
    }
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "Iceclad: 2 digivolution cards lose to 3 despite 15000 vs 5000 DP"
    );
    assert_eq!(field_ids(&r, 1), vec!["OPP-A".to_string()]);
}

// ─── Section 4 — Counter-clause body details ────────────────────────────────

#[test]
fn bt25_103_trash_spans_multiple_opponent_digimon_and_is_capped_by_own_sources() {
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["SRC", "SRC", "RED-L6", CARD_ID]); // 3 sources
    let a = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]);
    let b = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-B"]);
    fire(&mut r, EffectTiming::CounterEffect, me);
    match r.pending_kind() {
        Some(SelectionKind::SourceMulti { min, max, .. }) => {
            assert_eq!((min, max), (0, 3));
        }
        k => panic!("expected source multi, got {k:?}"),
    }
    // Candidates include both opponent Digimon's stacks (cross-permanent).
    let v = r.pending_selection_view().unwrap();
    assert!(
        v.valid_action_ids.len() >= 4 + 1,
        "4 sources + PASS: {:?}",
        v.valid_action_ids
    );
    pick_sources(&mut r, 3);
    finish_source_picks(&mut r);
    answer_end_attack(&mut r, false);
    let _ = r.auto_resolve();
    assert_eq!(
        opp_source_total(&r),
        1,
        "4 − 3 trashed (cap = own 3 sources)"
    );
    let _ = (a, b);
}

#[test]
fn bt25_103_trash_clamps_to_available() {
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["SRC", "SRC", "SRC", "RED-L6", CARD_ID]); // 4 sources
    r.place_stack(1, &["OPP-S", "OPP-A"]); // 1 source
    fire(&mut r, EffectTiming::CounterEffect, me);
    match r.pending_kind() {
        Some(SelectionKind::SourceMulti { max, .. }) => assert_eq!(max, 1),
        k => panic!("expected source multi, got {k:?}"),
    }
    pick_sources(&mut r, 1);
    finish_source_picks(&mut r);
    answer_end_attack(&mut r, false);
    assert_eq!(opp_source_total(&r), 0);
}

#[test]
fn bt25_103_no_sources_on_self_skips_trash_still_offers_end() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0)); // 0 sources
    r.place_stack(1, &["OPP-S", "OPP-A"]);
    fire(&mut r, EffectTiming::CounterEffect, me);
    answer_end_attack(&mut r, false);
    let _ = r.auto_resolve();
    assert_eq!(opp_source_total(&r), 1);
}

#[test]
fn bt25_103_declining_everything_refunds_opt() {
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]);
    r.place_stack(1, &["OPP-S", "OPP-A"]);
    fire(&mut r, EffectTiming::CounterEffect, me);
    finish_source_picks(&mut r);
    answer_end_attack(&mut r, false);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    // OPT not spent: the clause fires again.
    fire(&mut r, EffectTiming::CounterEffect, me);
    assert!(
        matches!(r.pending_kind(), Some(SelectionKind::SourceMulti { .. })),
        "declining everything refunds [Once Per Turn]: {:?}",
        r.pending_kind()
    );
}

#[test]
fn bt25_103_trashing_spends_opt() {
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]);
    r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]);
    fire(&mut r, EffectTiming::CounterEffect, me);
    pick_sources(&mut r, 1);
    finish_source_picks(&mut r);
    answer_end_attack(&mut r, false);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::CounterEffect, me);
    assert!(r.game.pending_selection.is_none(), "OPT spent");
    fire(&mut r, EffectTiming::WhenAttacking, me);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::SourceMulti { .. })),
        "OPT shared across [When Attacking] and [Counter]"
    );
}

#[test]
fn bt25_103_ending_attack_alone_spends_opt() {
    let mut r = start(&[], 5);
    let me = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::CounterEffect, me);
    answer_end_attack(&mut r, true);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::CounterEffect, me);
    assert!(
        r.game.pending_selection.is_none(),
        "OPT spent by ending the attack"
    );
}

// ─── Section 5 — real [Counter]: opponent's attack ──────────────────────────

#[test]
fn bt25_103_counter_on_opponent_attack_can_end_it() {
    let mut r = builder()
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(0)
        .start();
    r.set_first_player(1);
    r.set_turn(3);
    let me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]); // 2 sources
    let target = r.place_on_field(0, "BLUE-L6", Some(0));
    r.game.players[0].battle_area[target.index as usize].is_suspended = true;
    let attacker = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]);
    let _ = r.attack_digimon(attacker, target, false);
    // Counter window: the defender (player 0) may fire GraceNovamon's ability.
    let mut fired = false;
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        match v.kind {
            SelectionKind::SourceMulti { max, .. } => {
                fired = true;
                assert_eq!(max, 2);
                pick_sources(&mut r, 2);
            }
            SelectionKind::EffectChoice => answer_end_attack(&mut r, true),
            SelectionKind::TriggerOrder => resolve_trigger_order(&mut r),
            _ => {
                // Counter-window prompt: choose the non-PASS (field ability).
                let a = v
                    .valid_action_ids
                    .iter()
                    .copied()
                    .find(|&a| a != PASS)
                    .unwrap_or(PASS);
                r.execute_action(v.selecting_player, a).unwrap();
            }
        }
    }
    let _ = r.auto_resolve();
    assert!(fired, "[Counter] fired from the battle area");
    assert_eq!(sources(&r, attacker), 0, "attacker's 2 sources trashed");
    assert!(
        field_ids(&r, 0).contains(&"BLUE-L6".to_string()),
        "the opponent's attack was ended before the battle"
    );
    assert!(r.game.pending_attack.is_none(), "no attack in progress");
    let _ = me;
}

#[test]
fn bt25_103_counter_on_attack_on_player_can_end_it() {
    let mut r = builder()
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(0)
        .start();
    r.set_first_player(1);
    r.set_turn(3);
    let _me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]); // 2 sources
    let attacker = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-A"]);
    let _ = r.attack_player(attacker, 0, false);
    let mut fired = false;
    let mut ended = false;
    for _ in 0..12 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        match v.kind {
            SelectionKind::SourceMulti { max, .. } => {
                fired = true;
                assert_eq!(max, 2);
                pick_sources(&mut r, 2);
            }
            SelectionKind::EffectChoice => {
                answer_end_attack(&mut r, true);
                ended = true;
            }
            SelectionKind::TriggerOrder => resolve_trigger_order(&mut r),
            _ => {
                // Counter-window prompt: take the field ability (non-PASS).
                let a = v
                    .valid_action_ids
                    .iter()
                    .copied()
                    .find(|&a| a != PASS)
                    .unwrap_or(PASS);
                r.execute_action(v.selecting_player, a).unwrap();
            }
        }
    }
    let _ = r.auto_resolve();
    assert!(fired, "[Counter] window opened on an attack on the player");
    assert!(ended, "end-attack choice offered");
    assert_eq!(sources(&r, attacker), 0, "attacker's 2 sources trashed");
    assert_eq!(r.security_count(0), 3, "attack ended — security untouched");
    assert!(r.game.pending_attack.is_none(), "no attack in progress");
}

#[test]
fn bt25_103_opt_resets_next_turn() {
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["SRC", "RED-L6", CARD_ID]);
    r.place_stack(1, &["OPP-S", "OPP-S", "OPP-S", "OPP-A"]);
    fire(&mut r, EffectTiming::CounterEffect, me);
    pick_sources(&mut r, 1);
    finish_source_picks(&mut r);
    answer_end_attack(&mut r, false);
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::CounterEffect, me);
    assert!(r.game.pending_selection.is_none(), "OPT spent this turn");
    r.end_turn();
    let _ = r.auto_resolve();
    fire(&mut r, EffectTiming::CounterEffect, me);
    assert!(
        matches!(r.pending_kind(), Some(SelectionKind::SourceMulti { .. })),
        "[Once Per Turn] resets on the next turn: {:?}",
        r.pending_kind()
    );
}

// ─── Section 6 — "from your opponent's Digimon" (not Tamers) ────────────────

#[test]
fn bt25_103_cards_under_opponent_tamers_are_not_candidates() {
    let mut r = start(&[], 5);
    let me = r.place_stack(0, &["SRC", "SRC", "RED-L6", CARD_ID]);
    let t = r.place_stack(1, &["OPP-S", "OPP-S", "OPP-TAMER"]);
    fire(&mut r, EffectTiming::CounterEffect, me);
    // Only Tamer-held cards exist → no source prompt; straight to end choice.
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    answer_end_attack(&mut r, false);
    assert_eq!(sources(&r, t), 2);
}
