//! EX13 "Witchelny" slice — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/witchelny-ex13-model.md`. The Candlemon (EX13-025)
//! → FlameWizardmon (EX13-029) → Mistymon (EX13-033) line, with the
//! out-of-slice EX13 partners EX13-004 DemiMeramon (egg) and EX13-037 Dynasmon
//! (Lv.6 top end), both already IMPLEMENTED.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0{25,29,33}.rs`;
//! this file asserts only cross-card SYSTEM facts (combos C1..C7 of the model).
//!
//! Real DSL cards fill every role. Fillers / opponents are effectless vanilla
//! ST3 Digimon: ST3-02 Salamon (Lv.3 3000), ST3-03 Tapirmon (Lv.3 4000),
//! ST3-06 Gatomon (Lv.4 5000), ST3-10 Magnadramon (Lv.6 12000) — none has
//! [Witchelny] in its text.
//!
//! No DCGO C# exists for the three slice cards; printed text (official Bandai
//! DB bundles) governs, with DCGO BT18/Yellow/BT18_030.cs as the sibling for
//! the shared inherited leave-prevention. Rules: general_rule.pdf 15-7 (by-X
//! costs), 15-8-3-2 (triggers wait for the current effect), 15-8-5 (would-leave
//! replacement).

#![allow(dead_code)]

use std::collections::VecDeque;

use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, HAND_EFFECT_START, PASS, PLAY_HAND_START, TARGETS_PER_ATTACKER,
};
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{PendingSelectionView, SelectionKind, TriggerSource};

use super::support::dsl_builder;

// ─── Card ids ────────────────────────────────────────────────────────────────

const DEMIMERAMON: &str = "EX13-004";
const CANDLEMON: &str = "EX13-025";
const FLAMEWIZ: &str = "EX13-029";
const MISTYMON: &str = "EX13-033";
const DYNASMON: &str = "EX13-037";
/// Vanilla filler (deck / security) and non-[Witchelny] Lv.3 base, 3000.
const SALAMON: &str = "ST3-02";
/// Vanilla opponent, Lv.3 4000.
const TAPIRMON: &str = "ST3-03";
/// Vanilla opponent, Lv.4 5000.
const GATOMON: &str = "ST3-06";
/// Vanilla opponent, Lv.6 12000.
const MAGNADRAMON: &str = "ST3-10";

const REAL: &[&str] = &[
    DEMIMERAMON,
    CANDLEMON,
    FLAMEWIZ,
    MISTYMON,
    DYNASMON,
    SALAMON,
    TAPIRMON,
    GATOMON,
    MAGNADRAMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn builder(own_security: usize) -> DebugRunnerBuilder {
    dsl_builder(REAL)
        .deck(0, &[SALAMON; 8])
        .deck(1, &[SALAMON; 8])
        .security(0, &vec![SALAMON; own_security])
        .security(1, &[SALAMON; 5])
}

fn start(b: DebugRunnerBuilder) -> DebugRunner {
    let mut r = b.start();
    r.skip_mulligan();
    r
}

fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn security_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].security)
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn top_id(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string()
}

fn find_perm(r: &DebugRunner, p: u8, id: &str) -> Option<PermanentHandle> {
    field_ids(r, p)
        .iter()
        .position(|f| f == id)
        .map(|i| PermanentHandle {
            player: p,
            index: i as u8,
        })
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn hand_action(r: &DebugRunner, view: &PendingSelectionView, p: u8, id: &str) -> Option<u16> {
    let slot = hand_ids(r, p).iter().position(|h| h == id)? as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
}

fn field_action(view: &PendingSelectionView, h: PermanentHandle) -> Option<u16> {
    view.valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(ATTACK_START)
                .is_some_and(|o| (o % TARGETS_PER_ATTACKER) as u8 == h.index)
        })
}

/// Pick the route whose label names `cost` at a digivolve-route EffectChoice.
fn route_action(view: &PendingSelectionView, cost: u16) -> Option<u16> {
    if view.kind != SelectionKind::EffectChoice {
        return None;
    }
    view.effect_choices
        .as_ref()?
        .iter()
        .find(|c| c.label.contains(&format!("{cost}")))
        .map(|c| c.action_id)
}

fn accept(view: &PendingSelectionView) -> Option<u16> {
    view.valid_action_ids.iter().copied().find(|&a| a != PASS)
}

/// Drive pending prompts with `policy`; a `None` from the policy falls back to
/// PASS on optional prompts / the first legal action on mandatory ones.
fn drive(
    r: &mut DebugRunner,
    mut policy: impl FnMut(&DebugRunner, &PendingSelectionView) -> Option<u16>,
) {
    for _ in 0..64 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        let a = policy(r, &view).unwrap_or_else(|| {
            if view.is_optional || view.valid_action_ids.contains(&PASS) {
                PASS
            } else {
                *view.valid_action_ids.first().expect("legal action")
            }
        });
        r.execute_action(view.selecting_player, a)
            .unwrap_or_else(|e| panic!("action {a} on {view:?}: {e:?}"));
    }
    panic!("drive did not converge");
}

/// Opponent-field picker: answers each OppField prompt with the next card id in
/// `queue` (by the card currently on top of that permanent). Records every
/// OppField prompt it answered so tests can assert how many removals fired.
struct OppPicks {
    queue: VecDeque<&'static str>,
    answered: Vec<&'static str>,
}

impl OppPicks {
    fn new(q: &[&'static str]) -> Self {
        Self {
            queue: q.iter().copied().collect(),
            answered: vec![],
        }
    }
    fn pick(&mut self, r: &DebugRunner, v: &PendingSelectionView) -> Option<u16> {
        let id = self.queue.pop_front()?;
        let h = find_perm(r, 1, id)?;
        let a = field_action(v, h);
        if a.is_some() {
            self.answered.push(id);
        }
        a
    }
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

// ─── C1 Witchelny discount route into FlameWizardmon removal ─────────────────

/// **C1 Witchelny discount route** — EX13-025 Candlemon + EX13-029
/// FlameWizardmon. Candlemon's (Rule) Trait [Witchelny] unlocks the printed
/// "[Digivolve] Lv.3 w/[Witchelny] in text: Cost 2" route. FlameWizardmon's
/// [When Digivolving] then pays its top security (4 → 3, 15-7-4), gives
/// Magnadramon -4000 (12000 → 8000) and — now at 3 security — deletes the
/// 4000-DP Tapirmon.
#[test]
fn c1_candlemon_unlocks_cost_2_flamewizardmon_whose_wd_trash_enables_the_delete() {
    let mut r = start(builder(4).hand(0, &[FLAMEWIZ]).memory(5));
    let base = r.place_stack(0, &[CANDLEMON]);
    let magna = r.place_on_field(1, MAGNADRAMON, Some(0));
    r.place_on_field(1, TAPIRMON, Some(0));

    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    let mut picks = OppPicks::new(&[MAGNADRAMON, TAPIRMON]);
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::Replacement => accept(v), // FW's outer "by trashing" accept
        SelectionKind::OppField => picks.pick(r, v),
        _ => route_action(v, 2),
    });

    assert_eq!(top_id(&r, base), FLAMEWIZ);
    assert_eq!(
        r.memory(),
        3,
        "[Witchelny] route: Cost 2 (not the printed 3)"
    );
    assert_eq!(r.security_count(0), 3, "the [WD] cost trashed 1 security");
    assert_eq!(r.effective_dp(magna), Some(8000), "-4000 for the turn");
    assert_eq!(picks.answered, vec![MAGNADRAMON, TAPIRMON]);
    assert_eq!(
        field_ids(&r, 1),
        vec![MAGNADRAMON],
        "4000-DP Tapirmon deleted at 3 security"
    );
}

/// **C1b unhappy path** — the discount needs [Witchelny] in the base's text:
/// on vanilla yellow Lv.3 Salamon only the printed Yellow Lv.3 / 3 circle
/// applies.
#[test]
fn c1b_non_witchelny_base_pays_the_printed_cost_3() {
    let mut r = start(builder(5).hand(0, &[FLAMEWIZ]).memory(5));
    let base = r.place_stack(0, &[SALAMON]);

    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    drive(&mut r, |_, v| match v.kind {
        SelectionKind::Replacement => Some(PASS), // decline the [WD] cost
        SelectionKind::EffectChoice => {
            assert!(
                route_action(v, 2).is_none(),
                "no cost-2 route on a non-[Witchelny] base"
            );
            None
        }
        _ => None,
    });
    assert_eq!(top_id(&r, base), FLAMEWIZ);
    assert_eq!(r.memory(), 2, "printed Yellow Lv.3 circle: Cost 3");
    assert_eq!(
        r.security_count(0),
        5,
        "declined [WD] cost → security untouched"
    );
}

// ─── C2 FlameWizardmon's cost feeds Mistymon's observer ──────────────────────

/// **C2 Double removal** — EX13-029 + EX13-025 + EX13-033. With Mistymon
/// already on the field, FlameWizardmon's [WD] cost (trash top security)
/// is a "security stack removed from" event: after FW's effect finishes
/// (15-8-3-2) Mistymon's [All Turns][OPT] observer fires — -6000 and, at 3
/// security, delete ≤6000. FW: Magnadramon -4000 (8000), delete Tapirmon;
/// Mistymon: Magnadramon -6000 (2000), delete Magnadramon. Gatomon survives.
#[test]
fn c2_flamewizardmon_cost_triggers_mistymon_for_a_second_removal() {
    let mut r = start(builder(4).hand(0, &[FLAMEWIZ]).memory(5));
    let base = r.place_stack(0, &[CANDLEMON]);
    r.place_on_field(0, MISTYMON, Some(0));
    r.place_on_field(1, MAGNADRAMON, Some(0));
    r.place_on_field(1, GATOMON, Some(0));
    r.place_on_field(1, TAPIRMON, Some(0));

    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    let mut picks = OppPicks::new(&[MAGNADRAMON, TAPIRMON, MAGNADRAMON, MAGNADRAMON]);
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::Replacement => accept(v),
        SelectionKind::OppField => picks.pick(r, v),
        _ => route_action(v, 2),
    });

    assert_eq!(top_id(&r, base), FLAMEWIZ);
    assert_eq!(r.security_count(0), 3, "exactly one security card paid");
    assert_eq!(
        picks.answered,
        vec![MAGNADRAMON, TAPIRMON, MAGNADRAMON, MAGNADRAMON],
        "FW (-4000, delete) then Mistymon (-6000, delete) — four opponent picks"
    );
    assert_eq!(
        field_ids(&r, 1),
        vec![GATOMON],
        "two opponent Digimon removed by one digivolve"
    );
}

// ─── C3 Mistymon WD: placement funds the attack cost at the threshold ────────

/// **C3 Placement-funded attack** — EX13-033 onto EX13-029 (on EX13-025),
/// placing a second EX13-025 from hand. Mistymon digivolves for 3 (Lv.4
/// w/[Witchelny] route); [WD] places Candlemon as bottom security (3 → 4),
/// then pays the top (4 → 3) so Mistymon attacks. That payment fires
/// Mistymon's own observer at 3 security: Magnadramon -6000 (6000) → deleted.
/// The attack checks 1 opponent security.
#[test]
fn c3_mistymon_places_witchelny_card_pays_it_back_and_attacks_while_deleting() {
    let mut r = start(builder(3).hand(0, &[MISTYMON, CANDLEMON]).memory(5));
    let base = r.place_stack(0, &[CANDLEMON, FLAMEWIZ]);
    r.place_on_field(1, MAGNADRAMON, Some(0));

    r.game
        .decode_action(encode_digivolve(0, base.index as u16), 0);
    let mut picks = OppPicks::new(&[MAGNADRAMON, MAGNADRAMON]);
    let mut paid = false;
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::Hand => hand_action(r, v, 0, CANDLEMON),
        SelectionKind::EffectChoice if v.prompt.contains("By trashing") => {
            paid = true;
            v.effect_choices.as_ref().map(|c| c[0].action_id)
        }
        SelectionKind::OwnField => field_action(v, base),
        SelectionKind::OppField => picks.pick(r, v),
        _ => route_action(v, 3).or_else(|| {
            // Attack-target prompt: the opponent's security.
            v.valid_action_ids.iter().copied().find(|&a| {
                a.checked_sub(ATTACK_START)
                    .is_some_and(|o| o % TARGETS_PER_ATTACKER == TARGETS_PER_ATTACKER - 1)
            })
        }),
    });

    assert_eq!(top_id(&r, base), MISTYMON);
    assert_eq!(r.memory(), 2, "[Witchelny] route: Cost 3");
    assert!(paid, "the by-trashing cost was offered and paid");
    let sec = security_ids(&r, 0);
    assert_eq!(sec.len(), 3, "placed (+1) then paid (-1)");
    assert_eq!(
        sec[0], CANDLEMON,
        "Candlemon placed as the bottom security card"
    );
    assert_eq!(
        picks.answered,
        vec![MAGNADRAMON, MAGNADRAMON],
        "observer: -6000 then delete"
    );
    assert!(
        field_ids(&r, 1).is_empty(),
        "Magnadramon 12000-6000 = 6000 → deleted"
    );
    assert_eq!(
        r.security_count(1),
        4,
        "Mistymon's granted attack checked 1 security"
    );
}

// ─── C4 Stacked inherited saves feed Mistymon's observer ─────────────────────

/// **C4 Stacked saves** — Mistymon on [Candlemon, FlameWizardmon]. Both
/// sources carry the inherited "[Witchelny]-text carrier won't leave by
/// opponent's effects by trashing top security" (distinct cards → distinct
/// [Once Per Turn]s; DCGO BT18_030.cs). The first opponent-effect deletion is
/// prevented (4 → 3 security) and that trash fires Mistymon's observer:
/// Magnadramon -6000 → deleted. A second deletion is prevented by the other
/// source (3 → 2); a third goes through.
#[test]
fn c4_two_source_saves_each_trash_security_and_the_first_feeds_mistymon() {
    let mut r = start(builder(4));
    let misty = r.place_stack(0, &[CANDLEMON, FLAMEWIZ, MISTYMON]);
    r.place_on_field(1, MAGNADRAMON, Some(0));

    // Removal #1 — saved; Mistymon's observer deletes Magnadramon.
    r.game
        .delete_permanent_with_cause(misty, ReplacementCause::OpponentEffect);
    let mut picks = OppPicks::new(&[MAGNADRAMON, MAGNADRAMON]);
    let mut saves = 0;
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::Replacement => {
            saves += 1;
            accept(v)
        }
        SelectionKind::OppField => picks.pick(r, v),
        _ => None,
    });
    assert_eq!(saves, 1);
    assert_eq!(top_id(&r, misty), MISTYMON, "it doesn't leave");
    assert_eq!(r.security_count(0), 3);
    assert_eq!(picks.answered, vec![MAGNADRAMON, MAGNADRAMON]);
    assert!(
        field_ids(&r, 1).is_empty(),
        "the save's security trash fed Mistymon's delete"
    );

    // Removal #2 — the OTHER source's inherited still has its OPT.
    r.game
        .delete_permanent_with_cause(misty, ReplacementCause::OpponentEffect);
    let mut saves2 = 0;
    drive(&mut r, |_, v| match v.kind {
        SelectionKind::Replacement => {
            saves2 += 1;
            accept(v)
        }
        _ => None,
    });
    assert_eq!(saves2, 1, "second source's save offered");
    assert_eq!(field_ids(&r, 0), vec![MISTYMON.to_string()]);
    assert_eq!(r.security_count(0), 2);

    // Removal #3 — both OPTs spent: it leaves.
    r.game
        .delete_permanent_with_cause(misty, ReplacementCause::OpponentEffect);
    drive(&mut r, |_, v| {
        assert_ne!(
            v.kind,
            SelectionKind::Replacement,
            "no third save this turn"
        );
        None
    });
    assert!(field_ids(&r, 0).is_empty(), "third removal goes through");
    assert_eq!(r.security_count(0), 2);
}

// ─── C5 Candlemon start-of-main + Mistymon + recycle ─────────────────────────

/// **C5 Main-phase engine** — EX13-025 + EX13-033 (+ EX13-029 recycled).
/// Candlemon at 3 security trashes its top card, draws 1, gains 1 memory; at
/// 2 security it places FlameWizardmon (Witchelny-text) as bottom security.
/// The trash also fires Mistymon's observer: Magnadramon -6000 → deleted.
#[test]
fn c5_candlemon_main_phase_trash_triggers_mistymon_and_recycles_flamewizardmon() {
    let mut r = start(builder(3).hand(0, &[FLAMEWIZ]).memory(3));
    let candle = r.place_on_field(0, CANDLEMON, Some(0));
    r.place_on_field(0, MISTYMON, Some(0));
    r.place_on_field(1, MAGNADRAMON, Some(0));
    let hand_before = r.hand_size(0);

    fire(&mut r, EffectTiming::StartOfYourMainPhase, candle);
    let mut picks = OppPicks::new(&[MAGNADRAMON, MAGNADRAMON]);
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::Hand => hand_action(r, v, 0, FLAMEWIZ),
        SelectionKind::OppField => picks.pick(r, v),
        SelectionKind::EffectChoice if v.prompt.contains("top or bottom") => {
            v.effect_choices.as_ref().map(|c| c[0].action_id)
        }
        _ => None,
    });

    assert_eq!(r.memory(), 4, "+1 memory");
    let sec = security_ids(&r, 0);
    assert_eq!(sec.len(), 3, "3 → trash → 2 → place → 3");
    assert_eq!(
        sec[0], FLAMEWIZ,
        "FlameWizardmon recycled as the bottom security card"
    );
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "Draw 1 offsets the placed card"
    );
    assert_eq!(picks.answered, vec![MAGNADRAMON, MAGNADRAMON]);
    assert!(
        field_ids(&r, 1).is_empty(),
        "Mistymon's observer deleted Magnadramon"
    );
}

// ─── C6 DemiMeramon mid-attack climb into FlameWizardmon ─────────────────────

/// **C6 Egg-driven climb** — EX13-004 + EX13-025 + EX13-029. Candlemon (on
/// DemiMeramon) attacks; DemiMeramon's inherited [WA] digivolves it into
/// FlameWizardmon from hand at the [Witchelny] route minus 1 (2 - 1 = 1), then
/// trashes top security (5 → 4). FW's [WD] is a derived trigger: pays again
/// (4 → 3), Magnadramon -4000, then deletes Tapirmon. The attack then checks
/// 1 opponent security.
///
/// FAILS today: FlameWizardmon's [WD] resolves INSIDE DemiMeramon's body —
/// before its "if this effect digivolved, trash your top security card" — so
/// the [WD] sees 4 security (no delete) instead of 3. general_rule.pdf
/// 15-8-3-2 and DCGO (EX13_004.cs `successProcess` runs right after
/// `PlayCardClass.PlayCard()` merely *stacks* the [WD] via
/// `autoProcessing.StackSkillInfos`) both put the trash first. Filed as
/// G-ENGINE-EFFECT-DIGIVOLVE-WD-DRAINS-MID-BODY (docs/RUST_ENGINE_GAPS.md).
#[test]
#[ignore = "G-ENGINE-EFFECT-DIGIVOLVE-WD-DRAINS-MID-BODY: effect-initiated digivolve drains [When Digivolving] mid-body (15-8-3-2)"]
fn c6_demimeramon_attack_climbs_into_flamewizardmon_for_1_and_chains_its_wd() {
    let mut r = start(builder(5).hand(0, &[FLAMEWIZ]).memory(5));
    let atk = r.place_stack(0, &[DEMIMERAMON, CANDLEMON]);
    let magna = r.place_on_field(1, MAGNADRAMON, Some(0));
    r.place_on_field(1, TAPIRMON, Some(0));

    r.attack_player(atk, 1, false);
    let mut picks = OppPicks::new(&[MAGNADRAMON, TAPIRMON]);
    drive(&mut r, |r, v| match v.kind {
        SelectionKind::Hand => hand_action(r, v, 0, FLAMEWIZ),
        SelectionKind::Replacement => accept(v),
        SelectionKind::OppField => picks.pick(r, v),
        _ => route_action(v, 1),
    });

    assert_eq!(top_id(&r, atk), FLAMEWIZ, "digivolved mid-attack");
    assert_eq!(r.memory(), 4, "[Witchelny] route 2, reduced by 1");
    assert_eq!(r.security_count(0), 3, "DemiMeramon trash + FW [WD] cost");
    assert_eq!(r.effective_dp(magna), Some(8000));
    assert_eq!(
        field_ids(&r, 1),
        vec![MAGNADRAMON],
        "Tapirmon deleted at 3 security"
    );
    assert_eq!(
        r.security_count(1),
        4,
        "the attack continued and checked 1 security"
    );
}

// ─── C7 Mistymon inherited unsuspends Dynasmon ───────────────────────────────

/// **C7 Ready again** — Dynasmon on [Candlemon, FlameWizardmon, Mistymon].
/// Dynasmon attacks (suspends); its [When Attacking] trashes top security;
/// Mistymon's inherited "[All Turns][OPT] when your security stack is removed
/// from, this Digimon may unsuspend" readies Dynasmon.
#[test]
fn c7_dynasmon_attack_trash_lets_mistymon_inherited_unsuspend_it() {
    let mut r = start(builder(5).memory(5));
    let dyn_h = r.place_stack(0, &[CANDLEMON, FLAMEWIZ, MISTYMON, DYNASMON]);

    r.attack_player(dyn_h, 1, false);
    let mut unsuspend_offered = false;
    drive(&mut r, |_, v| match v.kind {
        SelectionKind::Replacement if v.prompt.contains(MISTYMON) => {
            unsuspend_offered = true;
            accept(v)
        }
        SelectionKind::Replacement => accept(v),
        _ => None,
    });

    assert!(
        r.security_count(0) < 5,
        "Dynasmon's [WA] trashed own security"
    );
    assert!(
        unsuspend_offered,
        "Mistymon's inherited may-unsuspend offered"
    );
    assert!(
        !suspended(&r, dyn_h),
        "Dynasmon unsuspended after attacking"
    );
    assert_eq!(top_id(&r, dyn_h), DYNASMON);
}
