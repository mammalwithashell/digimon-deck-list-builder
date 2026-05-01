//! Phase E §E1 prerequisite — `battle_opponent_of` accessor on
//! `EffectReadContext` / `EffectContext`. Returns Some(opposing_combatant)
//! when `Game.pending_attack` is live and `self_handle` matches one side of
//! the pending battle; `None` otherwise.

use std::sync::{Arc, Mutex};

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;

fn fighter(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
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
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// An `OnDeletion` observer that captures `ctx.battle_opponent_of(self_handle)`
/// when the card it's registered on is deleted.
struct BattleOpponentCapture {
    /// The handle this observer will pass to `battle_opponent_of`.
    self_handle: Arc<Mutex<Option<PermanentHandle>>>,
    /// What `battle_opponent_of` returned.
    result: Arc<Mutex<Option<Option<PermanentHandle>>>>,
}

impl CardEffect for BattleOpponentCapture {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let self_handle_cell = self.self_handle.clone();
        let result_cell = self.result.clone();
        vec![Effect::on_deletion(card)
            .name("capture battle_opponent_of")
            .process(move |ctx| {
                // Read the handle that was stored before the attack
                let handle = self_handle_cell
                    .lock()
                    .unwrap()
                    .expect("self_handle must be set before attack fires");
                *result_cell.lock().unwrap() = Some(ctx.battle_opponent_of(handle));
            })
            .build()]
    }
}

/// Test: when the defender is deleted in a battle, its `OnDeletion` handler
/// calling `battle_opponent_of(defender_handle)` returns `Some(attacker_handle)`.
#[test]
fn battle_opponent_of_returns_attacker_when_self_is_defender() {
    // ATK (5000 DP) beats DEF (3000 DP) — DEF is deleted, observer fires.
    let self_handle_cell: Arc<Mutex<Option<PermanentHandle>>> = Arc::new(Mutex::new(None));
    let result_cell: Arc<Mutex<Option<Option<PermanentHandle>>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    r.register_effect(
        "DEF",
        Arc::new(BattleOpponentCapture {
            self_handle: self_handle_cell.clone(),
            result: result_cell.clone(),
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Store the defender handle before the attack so the observer can read it.
    *self_handle_cell.lock().unwrap() = Some(def);

    r.attack_digimon(atk, def, false);

    // DEF should have been deleted (lower DP).
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "DEF must be deleted (lower DP)"
    );

    let observed = result_cell
        .lock()
        .unwrap()
        .expect("OnDeletion must have fired");

    assert_eq!(
        observed,
        Some(atk),
        "battle_opponent_of(def) inside OnDeletion must return Some(atk)"
    );
}

/// Symmetric test: when the attacker is deleted (defender wins), its
/// `OnDeletion` handler calling `battle_opponent_of(attacker_handle)` returns
/// `Some(defender_handle)`.
#[test]
fn battle_opponent_of_returns_defender_when_self_is_attacker() {
    // ATK (2000 DP) loses to DEF (6000 DP) — ATK is deleted, observer fires.
    let self_handle_cell: Arc<Mutex<Option<PermanentHandle>>> = Arc::new(Mutex::new(None));
    let result_cell: Arc<Mutex<Option<Option<PermanentHandle>>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 2000))
        .add_card(fighter("DEF", 6000))
        .start();

    r.register_effect(
        "ATK",
        Arc::new(BattleOpponentCapture {
            self_handle: self_handle_cell.clone(),
            result: result_cell.clone(),
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    *self_handle_cell.lock().unwrap() = Some(atk);

    r.attack_digimon(atk, def, false);

    // ATK should have been deleted (lower DP).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "ATK must be deleted (lower DP)"
    );

    let observed = result_cell
        .lock()
        .unwrap()
        .expect("OnDeletion must have fired");

    assert_eq!(
        observed,
        Some(def),
        "battle_opponent_of(atk) inside OnDeletion must return Some(def)"
    );
}

/// Direct-player-attack test: when the attacker attacks the player directly
/// (no opposing Digimon), `battle_opponent_of(atk_handle)` must return `None`
/// because `effective_target` is `AttackTarget::Player(_)`, not a Digimon.
/// This covers the `Player(_) => None` arm in the accessor body.
#[test]
fn battle_opponent_of_returns_none_for_direct_player_attack() {
    // P0 has ATK + ALLY; P1 has no Digimon → attack resolves as a player attack.
    // ALLY's OnAllyAttack observer fires while pending_attack is live and
    // effective_target == Player(1). It calls battle_opponent_of(atk) → None.
    let result_cell: Arc<Mutex<Option<Option<PermanentHandle>>>> = Arc::new(Mutex::new(None));

    struct PlayerAttackCapture {
        atk_handle: Arc<Mutex<Option<PermanentHandle>>>,
        result: Arc<Mutex<Option<Option<PermanentHandle>>>>,
    }
    impl CardEffect for PlayerAttackCapture {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let handle_cell = self.atk_handle.clone();
            let result_cell = self.result.clone();
            vec![Effect::on_ally_attack(card)
                .name("player-attack battle_opponent_of None check")
                .process(move |ctx| {
                    let handle = handle_cell.lock().unwrap().expect("atk handle must be set");
                    *result_cell.lock().unwrap() = Some(ctx.battle_opponent_of(handle));
                })
                .build()]
        }
    }

    let atk_handle_cell: Arc<Mutex<Option<PermanentHandle>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("ALLY", 4000))
        .start();

    r.register_effect(
        "ALLY",
        Arc::new(PlayerAttackCapture {
            atk_handle: atk_handle_cell.clone(),
            result: result_cell.clone(),
        }),
    );

    // P1 intentionally has no Digimon — direct player attack.
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ally = r.place_on_field(0, "ALLY", Some(0));

    *atk_handle_cell.lock().unwrap() = Some(atk);

    r.attack_player(atk, 1, false);

    let observed = result_cell
        .lock()
        .unwrap()
        .expect("OnAllyAttack must have fired on ALLY");

    assert_eq!(
        observed, None,
        "battle_opponent_of(atk) must return None for a direct player attack \
         — effective_target is Player(1), not a Digimon"
    );
}

/// Non-combatant test: a bystander permanent on the field (not in the battle)
/// calling `battle_opponent_of(bystander_handle)` returns `None`.
#[test]
fn battle_opponent_of_returns_none_for_non_combatant() {
    // ATK beats DEF; BYSTANDER is on the field but not in the battle.
    // We verify None by reading pending_attack directly in an OnAllyAttack
    // observer (which fires while pending_attack is live).
    let result_cell: Arc<Mutex<Option<Option<PermanentHandle>>>> = Arc::new(Mutex::new(None));

    struct BystanderCapture {
        bystander_handle: Arc<Mutex<Option<PermanentHandle>>>,
        result: Arc<Mutex<Option<Option<PermanentHandle>>>>,
    }
    impl CardEffect for BystanderCapture {
        fn effects(&self, card: CardHandle) -> Vec<Effect> {
            let handle_cell = self.bystander_handle.clone();
            let result_cell = self.result.clone();
            vec![Effect::on_ally_attack(card)
                .name("bystander battle_opponent_of None check")
                .process(move |ctx| {
                    let handle = handle_cell.lock().unwrap().expect("handle must be set");
                    *result_cell.lock().unwrap() = Some(ctx.battle_opponent_of(handle));
                })
                .build()]
        }
    }

    let bystander_handle_cell: Arc<Mutex<Option<PermanentHandle>>> = Arc::new(Mutex::new(None));

    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .add_card(fighter("BYSTANDER", 4000))
        .start();

    r.register_effect(
        "BYSTANDER",
        Arc::new(BystanderCapture {
            bystander_handle: bystander_handle_cell.clone(),
            result: result_cell.clone(),
        }),
    );

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    let bystander = r.place_on_field(0, "BYSTANDER", Some(0));

    *bystander_handle_cell.lock().unwrap() = Some(bystander);

    r.attack_digimon(atk, def, false);

    let observed = result_cell
        .lock()
        .unwrap()
        .expect("OnAllyAttack must have fired on BYSTANDER");

    assert_eq!(
        observed, None,
        "battle_opponent_of(bystander) must return None — bystander is not a combatant"
    );
}
