//! Security-check phase order — Comprehensive Rules §13-1-8 (Ver. 4.3; the
//! base-repo Ver. 3.6 PDF numbers the same procedure §13-1-7).
//!
//! Each checked card resolves in this order:
//!
//! * §13-1-8-1 — the top security card is revealed.
//! * §13-1-8-2 — *effects triggered by the security check* resolve. The
//!   revealed card's own `[Security]` effect goes first (§15-16-10-2), then
//!   the "when a security check is performed" observers on **either**
//!   player's battle area (DCGO `AutoProcessing.GetSkillInfos` walks
//!   `Players_ForTurnPlayer`, turn player first — e.g. BT20-005's
//!   attacker-side "gain <Jamming> for the turn" ESS).
//! * §13-1-8-3 — the battle against a Security Digimon (§13-1-8-3-1), or no
//!   battle at all when the checked card is not a Digimon (§13-1-8-3-2).
//! * §13-1-8-4 — the checked card is trashed unless it belongs to an area.
//!
//! Before this module landed the engine ran the battle *before* the
//! observers (gap `G-ENGINE-SECURITY-CHECK-OBSERVERS-AFTER-BATTLE`), so a
//! DP modifier granted by an observer could never influence the battle it
//! was printed to influence. Every test below uses a DP modifier (or a
//! shared ordering log) as the witness because that is exactly the
//! observable the order changes.

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Expiry};

type Log = Arc<Mutex<Vec<&'static str>>>;

fn digimon(card_id: &str, dp: i32) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        link_text: String::new(),
        requirement_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn option(card_id: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 2,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        link_text: String::new(),
        requirement_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// "[All Turns] When a security check is performed, the attacking Digimon
/// gets `dp_delta` DP for the turn" — an `OnSecurityCheck` observer that
/// also records its firing in `log` under `label`.
struct CheckObserver {
    log: Log,
    label: &'static str,
    dp_delta: i32,
}

impl CardEffect for CheckObserver {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.log.clone();
        let label = self.label;
        let dp_delta = self.dp_delta;
        vec![Effect::on_security_check(c)
            .name("When a security check is performed, attacker DP shift")
            .process(move |ctx| {
                log.lock().unwrap().push(label);
                if dp_delta != 0 {
                    if let Some(attacker) = ctx.attacker() {
                        ctx.add_dp_modifier(attacker, dp_delta, Expiry::EndOfTurn);
                    }
                }
            })
            .build()]
    }
}

/// "[Security] The attacking Digimon gets `dp_delta` DP for the turn" on the
/// checked card itself — the §15-16-10-2 "[Security] first" witness.
struct SecurityDpShift {
    log: Log,
    dp_delta: i32,
}

impl CardEffect for SecurityDpShift {
    fn effects(&self, c: CardHandle) -> Vec<Effect> {
        let log = self.log.clone();
        let dp_delta = self.dp_delta;
        vec![Effect::security(c)
            .name("[Security] attacker DP shift")
            .process(move |ctx| {
                log.lock().unwrap().push("security-skill");
                if let Some(attacker) = ctx.attacker() {
                    ctx.add_dp_modifier(attacker, dp_delta, Expiry::EndOfTurn);
                }
            })
            .build()]
    }
}

fn new_log() -> Log {
    Arc::new(Mutex::new(Vec::new()))
}

/// (a) §13-1-8-2 before §13-1-8-3-1: a *defender-side* observer that gives
/// the attacker -2000 DP resolves before the battle, so an attacker that
/// would have beaten the Security Digimon on printed DP (5000 vs 4000) now
/// loses it (3000 vs 4000) and is deleted.
#[test]
fn defender_observer_dp_penalty_resolves_before_security_battle() {
    let log = new_log();
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 5000))
        .add_card(digimon("OBS-DEF", 1000))
        .add_card(digimon("SEC-DIGI", 4000))
        .security(1, &["SEC-DIGI"])
        .start();
    r.register_effect(
        "OBS-DEF",
        Arc::new(CheckObserver {
            log: log.clone(),
            label: "observer-def",
            dp_delta: -2000,
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _obs = r.place_on_field(1, "OBS-DEF", Some(0));

    let result = r.attack_player(atk, 1, false);
    assert!(r.game.pending_selection.is_none(), "no choice should be pending");

    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["observer-def"],
        "the defender's observer fires once for the check"
    );
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "§13-1-8-2 then §13-1-8-3-1: the observer's -2000 DP applies BEFORE \
         the battle, so 3000 vs 4000 deletes the attacker"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "the attacker lost the security battle and left the field"
    );
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.trash_size(1), 1, "§13-1-8-4: the checked card is trashed");
}

/// (b) §13-1-8-2 before §13-1-8-3-1 from the *attacker's* side: an
/// observer on the attacking player's battle area ("when a security check is
/// performed, the attacking Digimon gets +2000 DP") resolves before the
/// battle, so an attacker that would have lost on printed DP (3000 vs 4000)
/// now wins (5000 vs 4000) and survives. Observers on either side are
/// collected (DCGO `GetSkillInfos` → `Players_ForTurnPlayer`).
#[test]
fn attacker_observer_dp_bonus_resolves_before_security_battle() {
    let log = new_log();
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 3000))
        .add_card(digimon("OBS-ATK", 1000))
        .add_card(digimon("SEC-DIGI", 4000))
        .security(1, &["SEC-DIGI"])
        .start();
    r.register_effect(
        "OBS-ATK",
        Arc::new(CheckObserver {
            log: log.clone(),
            label: "observer-atk",
            dp_delta: 2000,
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _obs = r.place_on_field(0, "OBS-ATK", Some(0));

    let result = r.attack_player(atk, 1, false);
    assert!(r.game.pending_selection.is_none(), "no choice should be pending");

    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["observer-atk"],
        "an attacker-side observer fires for the check too"
    );
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "§13-1-8-2 then §13-1-8-3-1: the observer's +2000 DP applies BEFORE \
         the battle, so 5000 vs 4000 keeps the attacker alive"
    );
    assert_eq!(
        r.battle_area_size(0),
        2,
        "attacker and its observer both remain on the field"
    );
    assert_eq!(
        r.effective_dp(atk),
        Some(5000),
        "the for-the-turn bonus is still on the attacker after the check"
    );
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.trash_size(1), 1, "§13-1-8-4: the checked card is trashed");
}

/// (c) §15-16-10-2: the checked card's own `[Security]` effect resolves
/// first among the §13-1-8-2 triggers, then the observers, and only then the
/// §13-1-8-3-1 battle. Both DP shifts (-1000 from `[Security]`, -1000 from
/// the observer) must land before the compare: 5500 - 2000 = 3500 < 4000.
#[test]
fn security_effect_resolves_before_observers_and_both_before_battle() {
    let log = new_log();
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 5500))
        .add_card(digimon("OBS-DEF", 1000))
        .add_card(digimon("SEC-WIT", 4000))
        .security(1, &["SEC-WIT"])
        .start();
    r.register_effect(
        "SEC-WIT",
        Arc::new(SecurityDpShift {
            log: log.clone(),
            dp_delta: -1000,
        }),
    );
    r.register_effect(
        "OBS-DEF",
        Arc::new(CheckObserver {
            log: log.clone(),
            label: "observer-def",
            dp_delta: -1000,
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _obs = r.place_on_field(1, "OBS-DEF", Some(0));

    let result = r.attack_player(atk, 1, false);
    assert!(r.game.pending_selection.is_none(), "no choice should be pending");

    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["security-skill", "observer-def"],
        "§15-16-10-2: the [Security] effect resolves before the observers"
    );
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "§13-1-8-2 then §13-1-8-3-1: both pre-battle DP shifts apply before \
         the compare, so 3500 vs 4000 deletes the attacker"
    );
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.trash_size(1), 1, "§13-1-8-4: the checked card is trashed");
}

/// (d) §13-1-8-3-2: a non-Digimon security card has no Security Digimon, so
/// after the §13-1-8-2 triggers resolve (observers on both sides, turn
/// player first — §15-? simultaneous-trigger ordering) the procedure skips
/// the battle and goes straight to §13-1-8-4 (trash). The attacker is
/// untouched and no DP compare happens.
#[test]
fn non_digimon_security_card_runs_observers_skips_battle_and_trashes() {
    let log = new_log();
    let mut r = DebugRunner::builder()
        .add_card(digimon("ATK", 3000))
        .add_card(digimon("OBS-ATK", 1000))
        .add_card(digimon("OBS-DEF", 1000))
        .add_card(option("SEC-OPT"))
        .security(1, &["SEC-OPT"])
        .start();
    r.register_effect(
        "OBS-ATK",
        Arc::new(CheckObserver {
            log: log.clone(),
            label: "observer-atk",
            dp_delta: 0,
        }),
    );
    r.register_effect(
        "OBS-DEF",
        Arc::new(CheckObserver {
            log: log.clone(),
            label: "observer-def",
            dp_delta: 0,
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _obs_atk = r.place_on_field(0, "OBS-ATK", Some(0));
    let _obs_def = r.place_on_field(1, "OBS-DEF", Some(0));

    let result = r.attack_player(atk, 1, false);
    assert!(r.game.pending_selection.is_none(), "no choice should be pending");

    assert_eq!(
        log.lock().unwrap().as_slice(),
        &["observer-atk", "observer-def"],
        "both sides' observers fire for the check; the turn player's (the \
         attacker's) resolves first"
    );
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "§13-1-8-3-2: no Security Digimon, so no battle and the attacker survives"
    );
    assert_eq!(
        r.battle_area_size(0),
        2,
        "attacker and its observer both remain on the field"
    );
    assert_eq!(r.battle_area_size(1), 1, "the defender's observer stays too");
    assert_eq!(
        r.effective_dp(atk),
        Some(3000),
        "no battle, no DP shift: the attacker is untouched"
    );
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.trash_size(1), 1, "§13-1-8-4: the checked Option is trashed");
}
