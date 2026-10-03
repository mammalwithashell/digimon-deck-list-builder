//! EX13 "Examon / holy warrior (Royal Knights Assembly)" slice — archetype
//! interaction tests.
//!
//! Model: `qa/archetype-qa/royal-knights-ex13-model.md`. EX13's Royal Knights
//! are a toolbox of Lv.6 / Lv.7 [Royal Knight] payoffs, each with its own
//! Assembly -5 recipe, glued together by a handful of cross-card enablers:
//! Gigimon's (EX13-001) Tamer-triggered cheap digivolve into Gallantmon,
//! Leopardmon's (EX13-043) suspend-scaled "play a [Royal Knight] from hand",
//! Magnamon's (EX13-020) end-of-turn [Royal Knight] unsuspend, Jesmon's
//! (EX13-014) "any of your Digimon played" removal, and Dynasmon's (EX13-037)
//! "security stacks removed from" punisher. Examon (EX13-045) closes via a
//! Green Lv.6 + Blue Lv.6 DNA digivolve.
//!
//! Per-card behaviour lives in `tests/cards_behavioral/ex13/ex13_0xx.rs`; this
//! file asserts only the cross-card SYSTEM facts no per-card test can see.
//!
//! Real DSL cards fill every role — no synthetic cards. Vanilla (effectless)
//! starter Digimon are the stack bases, DNA partner and opponent targets:
//! ST1-02 Biyomon, ST1-05 Birdramon, ST1-10 Phoenixmon, ST2-02 Gomamon,
//! ST2-10 Plesiomon, ST4-09 Okuwamon. ST1-08 Garudamon (red Lv.5) is the
//! Gallantmon digivolve base; its only effect is a [When Digivolving] that
//! never fires here (it is always the base, never the digivolving card).
//! EX13-068 Takato Matsuki is the red Tamer (no [On Play]; its start-of-turn
//! clauses never fire mid-turn).
//!
//! Blocked (not authored, see the model's "Blocked combos"): EX13-023
//! UlforceVeedramon, EX13-016 Omnimon and EX13-077 Omnimon: Merciful Mode have
//! no YAML.

#![allow(dead_code)]

use digimon_engine::action::space::{
    encode_attack, encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::GamePhase;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::PendingSelectionView;

use super::support::{dsl_builder, snapshot};

// ─── Card ids ────────────────────────────────────────────────────────────────

const GIGIMON: &str = "EX13-001";
const JESMON: &str = "EX13-014";
const GALLANTMON: &str = "EX13-015";
const MAGNAMON: &str = "EX13-020";
const DYNASMON: &str = "EX13-037";
const LEOPARDMON: &str = "EX13-043";
const EXAMON: &str = "EX13-045";
const GANKOOMON: &str = "EX13-061";
const CRANIAMON: &str = "EX13-062";
/// Red Tamer, no [On Play] — Gigimon's inherited trigger.
const TAKATO: &str = "EX13-068";
/// Vanilla red Lv.3 3000 DP, play cost 2 — filler / opponent target.
const BIYOMON: &str = "ST1-02";
/// Vanilla red Lv.4 5000 DP, play cost 4 — opponent target.
const BIRDRAMON: &str = "ST1-05";
/// Red Lv.5 — Gallantmon's "Red Lv.5" digivolve base (WD never fires here).
const GARUDAMON: &str = "ST1-08";
/// Vanilla red Lv.6 12000 DP — the "12000 DP or higher" opponent.
const PHOENIXMON: &str = "ST1-10";
/// Vanilla blue Lv.3 3000 DP, play cost 2 — opponent target.
const GOMAMON: &str = "ST2-02";
/// Vanilla blue Lv.6 — the Blue Lv.6 half of Examon's DNA recipe.
const PLESIOMON: &str = "ST2-10";
/// Vanilla green Lv.5 — Leopardmon's "Green Lv.5" digivolve base.
const OKUWAMON: &str = "ST4-09";

const REAL: &[&str] = &[
    GIGIMON, JESMON, GALLANTMON, MAGNAMON, DYNASMON, LEOPARDMON, EXAMON, GANKOOMON, CRANIAMON,
    TAKATO, BIYOMON, BIRDRAMON, GARUDAMON, PHOENIXMON, GOMAMON, PLESIOMON, OKUWAMON,
];

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn builder() -> DebugRunnerBuilder {
    dsl_builder(REAL)
}

fn start(b: DebugRunnerBuilder) -> DebugRunner {
    let mut runner = b
        .deck(0, &[BIYOMON; 10])
        .deck(1, &[BIYOMON; 10])
        .security(1, &[BIYOMON; 5])
        .start();
    runner.skip_mulligan();
    runner.game.current_phase = GamePhase::Main;
    runner
}

fn ids_of(runner: &DebugRunner, cards: &[CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    ids_of(runner, &runner.game.players[p as usize].hand)
}

fn field_ids(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_names(runner: &DebugRunner, p: u8) -> Vec<String> {
    runner.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_name(&runner.game.card_data).to_string())
        .collect()
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    ids_of(
        runner,
        &runner.game.players[h.player as usize].battle_area[h.index as usize].card_sources,
    )
}

fn hand_index(runner: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(runner, p)
        .iter()
        .position(|h| h == id)
        .unwrap_or_else(|| panic!("{id} must be in player {p}'s hand: {:?}", hand_ids(runner, p)))
}

fn find_perm(runner: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let index = field_ids(runner, p)
        .iter()
        .position(|f| f == id)
        .unwrap_or_else(|| panic!("{id} must be on player {p}'s field: {:?}", field_ids(runner, p)));
    PermanentHandle {
        player: p,
        index: index as u8,
    }
}

fn is_suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn view(runner: &DebugRunner) -> PendingSelectionView {
    runner
        .pending_selection_view()
        .expect("a prompt must be pending")
}

fn view_dbg(runner: &DebugRunner) -> String {
    format!("{:?}", runner.pending_selection_view())
}

fn act(runner: &mut DebugRunner, action: u16) {
    let v = view(runner);
    assert!(
        v.valid_action_ids.contains(&action),
        "action {action} must be legal; view={v:?}"
    );
    runner
        .execute_action(v.selecting_player, action)
        .expect("action accepted");
}

fn pass(runner: &mut DebugRunner) {
    let v = view(runner);
    assert!(v.is_optional, "PASS must be legal; view={v:?}");
    runner.execute_action(v.selecting_player, PASS).expect("pass");
}

/// Take the first non-PASS option of the pending prompt.
fn accept(runner: &mut DebugRunner) {
    let v = view(runner);
    let a = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("non-PASS option; view={v:?}"));
    runner.execute_action(v.selecting_player, a).expect("accept");
}

/// Pick `id` from a hand prompt (PLAY_HAND or HAND_EFFECT encoding).
fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let slot = hand_index(runner, 0, id) as u16;
    let v = view(runner);
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| v.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable; view={v:?}"));
    runner.execute_action(v.selecting_player, a).expect("pick hand");
}

/// If the pending prompt is a simultaneous-trigger ordering / branch choice,
/// pick the entry whose label contains `needle` (case-insensitive). Returns
/// whether a choice was made.
fn choose_labelled(runner: &mut DebugRunner, needle: &str) -> bool {
    let Some(v) = runner.pending_selection_view() else {
        return false;
    };
    let Some(choices) = v.effect_choices.clone() else {
        return false;
    };
    let needle = needle.to_lowercase();
    let Some(idx) = choices
        .iter()
        .position(|c| c.label.to_lowercase().contains(&needle))
    else {
        return false;
    };
    runner.execute_branch(idx).expect("choose labelled branch");
    true
}

/// Digivolve hand card `id` onto `base` (single-route digivolutions only).
fn digivolve(runner: &mut DebugRunner, id: &str, base: PermanentHandle) {
    let slot = hand_index(runner, 0, id) as u16;
    runner
        .game
        .decode_action(encode_digivolve(slot, base.index as u16), 0);
}

/// Action id for a pick over BOTH players' fields (`select_any_permanent`).
fn any_side(h: PermanentHandle) -> u16 {
    encode_attack(h.player as u16, h.index as u16)
}

/// Action id for a single-side field pick (own-only / opponent-only prompts).
fn one_side(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 1 — Takato (red Tamer) → Gigimon inherited → Gallantmon for 1 →
//           Gallantmon [When Digivolving] trashes their top security
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-001 Gigimon (bottom digivolution card — inherited enabler),
/// ST1-08 Garudamon (red Lv.5 carrier), EX13-068 Takato Matsuki (red Tamer),
/// EX13-015 Gallantmon (hand), ST1-05 Birdramon (5000-DP opponent).
///
/// Expected mechanical outcome: playing a red Tamer fires Gigimon's inherited
/// [Your Turn][OPT] on its carrier, which digivolves into the [Gallantmon]-named
/// hand card at its Red Lv.5 cost 3 reduced by 2 → **1**. Gallantmon's
/// [When Digivolving] then finds no 12000-DP-or-higher opponent Digimon, so
/// "this effect didn't delete" → trash their top security card. Net: memory
/// −4 (Takato) −1 (digivolve), opponent security 5 → 4, Birdramon untouched,
/// a 5-card Gigimon…Gallantmon stack.
///
/// Sources: printed text `data/card_bundles/EX13-001.md`, `EX13-015.md`;
/// DCGO `EX13/Red/EX13_001.cs` (OnEnterFieldAnyone, red Tamer, reduceCost 2),
/// `EX13/Red/EX13_015.cs` (DeletePeremanentAndProcessAccordingToResult →
/// failedToDelete → trash security); `general_rule.pdf` inherited effects
/// (digivolution cards' inherited text applies to the Digimon they're under).
#[test]
fn takato_triggers_gigimon_into_gallantmon_for_1_which_trashes_security() {
    let mut runner = start(builder().hand(0, &[TAKATO, GALLANTMON]).memory(10));
    let carrier = runner.place_stack(0, &[GIGIMON, BIYOMON, BIRDRAMON, GARUDAMON]);
    runner.place_on_field(1, BIRDRAMON, Some(0));
    let before = snapshot(&runner);

    let slot = hand_index(&runner, 0, TAKATO);
    runner.play(0, slot).expect("Takato plays");
    assert_eq!(before.memory - runner.memory(), 4, "Takato costs 4");

    let v = view(&runner);
    assert!(v.is_optional, "Gigimon: \"may digivolve\"; view={v:?}");
    pick_hand(&mut runner, GALLANTMON);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    assert_eq!(
        before.memory - after.memory,
        4 + 1,
        "Red Lv.5 cost 3 reduced by 2 = 1; view={}",
        view_dbg(&runner)
    );
    assert_eq!(
        stack_ids(&runner, carrier),
        vec![GIGIMON, BIYOMON, BIRDRAMON, GARUDAMON, GALLANTMON],
        "Gigimon's carrier digivolved into Gallantmon"
    );
    assert_eq!(after.security[1], 4, "no 12000+ target → trash their top security");
    assert_eq!(field_ids(&runner, 1), vec![BIRDRAMON], "5000 DP is not a delete target");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 2 — Gallantmon's security trash feeds Dynasmon's [All Turns] punisher
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-015 Gallantmon (digivolving onto ST1-08 Garudamon), EX13-037
/// Dynasmon (on field), ST1-05 Birdramon (opponent).
///
/// Expected mechanical outcome: Gallantmon's [When Digivolving] finds no
/// 12000+ target and trashes the opponent's top security card. That is "security
/// stacks are removed from", so Dynasmon's [All Turns][OPT] gives 1 opponent
/// Digimon −12000 DP (Birdramon 5000 → deleted at the rule check) and, because
/// we have 3 or fewer security cards, <Recovery +1> (3 → 4).
///
/// Sources: printed text `data/card_bundles/EX13-015.md`, `EX13-037.md`;
/// DCGO `EX13/Red/EX13_015.cs` (no EX13_037.cs — printed text governs);
/// `general_rule.pdf` rule processing (0-DP Digimon deleted at the rule check,
/// EX13-060 Q&A), Recovery (§16 keyword).
#[test]
fn gallantmon_security_trash_triggers_dynasmon_minus_12000_and_recovery() {
    let mut runner = start(
        builder()
            .hand(0, &[GALLANTMON])
            .security(0, &[BIYOMON; 3])
            .memory(10),
    );
    let base = runner.place_on_field(0, GARUDAMON, Some(0));
    runner.place_on_field(0, DYNASMON, Some(0));
    runner.place_on_field(1, BIRDRAMON, Some(0));
    let before = snapshot(&runner);

    digivolve(&mut runner, GALLANTMON, base);
    let mut punished = false;
    for _ in 0..8 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.prompt.contains("-12000") {
            let h = find_perm(&runner, 1, BIRDRAMON);
            act(&mut runner, one_side(h));
            punished = true;
            continue;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);
    assert!(punished, "Dynasmon's trigger opened; view={}", view_dbg(&runner));
    assert_eq!(before.memory - after.memory, 3, "Red Lv.5 digivolve: cost 3");
    assert_eq!(after.security[1], 4, "Gallantmon trashed their top security");
    assert!(field_ids(&runner, 1).is_empty(), "Birdramon −12000 → deleted");
    assert_eq!(after.security[0], 4, "≤3 security → <Recovery +1>");
}

/// Unhappy path of Combo 2: when Gallantmon CAN delete (a 12000-DP Phoenixmon),
/// no security is removed, so Dynasmon's punisher never fires — the opponent's
/// Birdramon survives and our security stays at 3.
#[test]
fn gallantmon_deleting_instead_leaves_dynasmon_dormant() {
    let mut runner = start(
        builder()
            .hand(0, &[GALLANTMON])
            .security(0, &[BIYOMON; 3])
            .memory(10),
    );
    let base = runner.place_on_field(0, GARUDAMON, Some(0));
    runner.place_on_field(0, DYNASMON, Some(0));
    runner.place_on_field(1, BIRDRAMON, Some(0));
    runner.place_on_field(1, PHOENIXMON, Some(0));

    digivolve(&mut runner, GALLANTMON, base);
    for _ in 0..6 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        assert!(!v.prompt.contains("-12000"), "Dynasmon must stay dormant; view={v:?}");
        if v.prompt.contains("12000 DP or higher") {
            let h = find_perm(&runner, 1, PHOENIXMON);
            act(&mut runner, one_side(h));
            continue;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert_eq!(field_ids(&runner, 1), vec![BIRDRAMON], "Phoenixmon deleted, Birdramon not");
    assert_eq!(runner.security_count(1), 5, "deleted → no security trash");
    assert_eq!(runner.security_count(0), 3, "no Recovery");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 3 — Gankoomon played under Jesmon: one play → removal + two tokens
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-014 Jesmon (on field), EX13-061 Gankoomon (played from hand),
/// ST1-02 Biyomon + ST1-05 Birdramon (opponent).
///
/// Expected mechanical outcome: playing Gankoomon fires BOTH Jesmon's
/// [All Turns][OPT] "when any of your Digimon are played" (delete 1 opponent
/// lowest-DP Digimon → Biyomon, then play an [Atho, René & Por] Token) and
/// Gankoomon's own [On Play] (play a [Hinukamuy] Token, then shield a white
/// Digimon). The two token plays are themselves "Digimon played" but Jesmon's
/// clause is [Once Per Turn], so only ONE opponent Digimon dies. Board after:
/// Jesmon, Gankoomon, Atho, Hinukamuy vs. Birdramon.
///
/// Sources: printed text `data/card_bundles/EX13-014.md`, `EX13-061.md`;
/// no EX13_014.cs / EX13_061.cs at the submodule (printed text governs);
/// `general_rule.pdf` simultaneous triggers (turn player chooses order) and
/// [Once Per Turn].
#[test]
fn gankoomon_under_jesmon_deletes_once_and_fields_both_tokens() {
    let mut runner = start(builder().hand(0, &[GANKOOMON]).memory(10));
    runner.place_on_field(0, JESMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    runner.place_on_field(1, BIRDRAMON, Some(0));

    let slot = hand_index(&runner, 0, GANKOOMON);
    runner.play(0, slot).expect("Gankoomon plays");

    let mut deletes_offered = 0;
    for _ in 0..16 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.prompt.contains("lowest DP") {
            deletes_offered += 1;
            let h = find_perm(&runner, 1, BIYOMON);
            act(&mut runner, one_side(h));
            continue;
        }
        if choose_labelled(&mut runner, "Play 1") {
            continue;
        }
        accept(&mut runner);
    }
    let _ = runner.auto_resolve();

    assert_eq!(deletes_offered, 1, "Jesmon's clause is Once Per Turn");
    assert_eq!(field_ids(&runner, 1), vec![BIRDRAMON], "only the lowest DP died");
    let names = field_names(&runner, 0);
    assert_eq!(names.len(), 4, "Jesmon + Gankoomon + 2 tokens: {names:?}");
    assert!(names.iter().any(|n| n.contains("Atho")), "Atho token: {names:?}");
    assert!(names.iter().any(|n| n.contains("Hinukamuy")), "Hinukamuy token: {names:?}");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 4 — Leopardmon: suspend first, then the [Royal Knight] play is cheaper
// ═══════════════════════════════════════════════════════════════════════════

/// Set up Leopardmon's digivolve: Okuwamon base, Craniamon in hand, opponent
/// Biyomon (3000, lowest) + Phoenixmon (12000).
fn leopardmon_board() -> (DebugRunner, PermanentHandle) {
    let mut runner = start(builder().hand(0, &[LEOPARDMON, CRANIAMON]).memory(10));
    let base = runner.place_on_field(0, OKUWAMON, Some(0));
    runner.place_on_field(1, BIYOMON, Some(0));
    runner.place_on_field(1, PHOENIXMON, Some(0));
    (runner, base)
}

/// Resolve Leopardmon's OP/WD "suspend 1 Digimon, then bottom-deck the
/// opponent's lowest DP" clause: suspend Phoenixmon (or decline), bottom-deck
/// Biyomon.
fn resolve_suspend_clause(runner: &mut DebugRunner, suspend: bool) {
    let v = view(runner);
    assert!(v.prompt.contains("suspend 1 Digimon"), "suspend prompt; view={v:?}");
    if suspend {
        let h = find_perm(runner, 1, PHOENIXMON);
        act(runner, any_side(h));
    } else {
        pass(runner);
    }
    let v = view(runner);
    assert!(v.prompt.contains("bottom of the deck"), "bottom-deck prompt; view={v:?}");
    let h = find_perm(runner, 1, BIYOMON);
    act(runner, one_side(h));
}

/// Resolve Leopardmon's WD/WA "play a [Royal Knight] from hand" clause by
/// picking Craniamon.
fn resolve_play_clause(runner: &mut DebugRunner) {
    let v = view(runner);
    assert!(v.prompt.contains("Royal Knight"), "RK play prompt; view={v:?}");
    pick_hand(runner, CRANIAMON);
}

/// Cards: EX13-043 Leopardmon (digivolving onto ST4-09 Okuwamon), EX13-062
/// Craniamon (the [Royal Knight] played from hand), ST1-02 Biyomon + ST1-10
/// Phoenixmon (opponent).
///
/// Expected mechanical outcome: Leopardmon's two [When Digivolving] clauses
/// trigger together; resolving the suspend clause FIRST (suspend Phoenixmon,
/// bottom-deck Biyomon) means the play clause sees 1 suspended Digimon, so
/// Craniamon costs 12 − 4 − 1 = **7**. Net memory: −3 (Green Lv.5 digivolve)
/// −7 = 0 (turn does not pass). Biyomon sits at the bottom of the opponent's
/// deck; Phoenixmon is suspended.
///
/// Sources: printed text `data/card_bundles/EX13-043.md`, `EX13-062.md`;
/// no EX13_043.cs (printed text governs); `general_rule.pdf` simultaneous
/// triggers — the turn player chooses the resolution order.
#[test]
fn leopardmon_suspends_first_so_craniamon_costs_7() {
    let (mut runner, base) = leopardmon_board();
    let mem0 = runner.memory();

    digivolve(&mut runner, LEOPARDMON, base);
    assert_eq!(mem0 - runner.memory(), 3, "Green Lv.5 digivolve: cost 3");
    assert!(
        choose_labelled(&mut runner, "slot 1"),
        "two WD triggers → order choice; view={}",
        view_dbg(&runner)
    );
    resolve_suspend_clause(&mut runner, true);
    let _ = choose_labelled(&mut runner, "slot 2");
    resolve_play_clause(&mut runner);
    let _ = runner.auto_resolve();

    assert_eq!(mem0 - runner.memory(), 3 + 7, "Craniamon at 12 − 4 − 1 suspended");
    assert_eq!(runner.turn_player(), 0, "memory 0 keeps the turn");
    let f = field_ids(&runner, 0);
    assert!(f.contains(&LEOPARDMON.to_string()) && f.contains(&CRANIAMON.to_string()), "{f:?}");
    assert_eq!(field_ids(&runner, 1), vec![PHOENIXMON]);
    assert!(is_suspended(&runner, find_perm(&runner, 1, PHOENIXMON)));
    assert_eq!(
        runner.game.players[1]
            .deck
            .first()
            .map(|c| c.card_id(&runner.game.card_data).to_string()),
        Some(BIYOMON.to_string()),
        "Biyomon went to the bottom of its owner's deck"
    );
}

/// Ordering half of Combo 4: resolving the PLAY clause first means no Digimon
/// is suspended yet, so Craniamon costs the bare 12 − 4 = **8** (one more than
/// the suspend-first line) even though Phoenixmon is suspended afterwards.
#[test]
fn leopardmon_playing_before_suspending_pays_the_full_minus_4() {
    let (mut runner, base) = leopardmon_board();
    let mem0 = runner.memory();

    digivolve(&mut runner, LEOPARDMON, base);
    assert!(
        choose_labelled(&mut runner, "slot 2"),
        "order choice; view={}",
        view_dbg(&runner)
    );
    resolve_play_clause(&mut runner);
    let mem_after_play = runner.memory();
    let _ = choose_labelled(&mut runner, "slot 1");
    resolve_suspend_clause(&mut runner, true);
    let _ = runner.auto_resolve();

    assert_eq!(mem0 - mem_after_play, 3 + 8, "no suspended Digimon yet: 12 − 4");
    assert!(field_ids(&runner, 0).contains(&CRANIAMON.to_string()));
    assert_ne!(runner.turn_player(), 0, "12 − 4 from 7 memory overspends → the turn passes");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 5 — Craniamon attacks (suspend → board wipe), Magnamon wakes it at
//           end of turn (unsuspend → +3000)
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-062 Craniamon (attacker), EX13-020 Magnamon ×2 (one on the
/// field — printed [End of Your Turn] — and one as a digivolution card under a
/// vanilla ST1-10 Phoenixmon — inherited [End of Your Turn]), ST1-02 Biyomon +
/// ST2-02 Gomamon (cost 2) and ST1-05 Birdramon (cost 4) on the opponent's side.
///
/// Expected mechanical outcome: attacking suspends Craniamon → its
/// [All Turns][OPT] "when this Digimon suspends" may delete ALL the opponent's
/// lowest-play-cost Digimon (both cost-2 Rookies; Birdramon stays). At end of
/// turn the two Magnamon clauses (printed + inherited — separate [Once Per
/// Turn] effects) each unsuspend one [Royal Knight]: Craniamon (→ its "when
/// this Digimon unsuspends" +3000 DP, 12000 → 15000, observable while the
/// second Magnamon clause is still pending) and the suspended field Magnamon.
/// Both stand untapped as <Blocker>s for the opponent's turn; Craniamon's
/// +3000 ("until your turn ends") has expired by then.
///
/// Sources: printed text `data/card_bundles/EX13-062.md`, `EX13-020.md`;
/// DCGO `EX13/Black/EX13_062.cs` (OnTappedAnyone → DestroyPermanentsClass over
/// IsMinCost; OnUnTappedAnyone +3000 UntilOwnerTurnEnd), `EX13/Blue/EX13_020.cs`
/// ([End of Your Turn] unsuspend Free/Royal Knight); `general_rule.pdf`
/// inherited effects + [Once Per Turn] is per effect. Note: Craniamon is
/// PARTIAL only for the unsuspend-PHASE / <Reboot> path
/// (G-ENGINE-PHASE-UNSUSPEND-NO-ONUNSUSPEND); Magnamon's unsuspend is
/// effect-driven, the path that IS implemented.
#[test]
fn craniamon_attack_wipes_lowest_cost_then_magnamons_wake_it_for_plus_3000() {
    let mut runner = start(builder().memory(3));
    let cran = runner.place_on_field(0, CRANIAMON, Some(0));
    let mag = runner.place_on_field(0, MAGNAMON, Some(0));
    runner.place_stack(0, &[MAGNAMON, PHOENIXMON]);
    runner.place_on_field(1, BIYOMON, Some(0));
    runner.place_on_field(1, GOMAMON, Some(0));
    runner.place_on_field(1, BIRDRAMON, Some(0));

    let _ = runner.attack_player(cran, 1, false);
    let mut wipe_offered = false;
    for _ in 0..10 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.prompt.contains(CRANIAMON) {
            assert!(v.is_optional, "\"you may delete\"");
            accept(&mut runner);
            wipe_offered = true;
            continue;
        }
        if v.is_optional {
            pass(&mut runner);
        } else {
            accept(&mut runner);
        }
    }
    let _ = runner.auto_resolve();
    assert!(wipe_offered, "attack-suspend fired Craniamon's clause");
    assert_eq!(field_ids(&runner, 1), vec![BIRDRAMON], "both cost-2 Digimon deleted");
    assert!(is_suspended(&runner, cran), "attacked → suspended");
    assert_eq!(runner.effective_dp(cran), Some(12_000));

    // The field Magnamon also attacked earlier this turn (state seeded directly
    // so its own [When Attacking] stays out of this combo).
    runner.game.players[0].battle_area[mag.index as usize].is_suspended = true;
    // Leave the opponent memory so their turn waits at Main (no auto-pass back).
    runner.game.set_memory(-3);
    runner.end_turn();

    let mut wake_prompts = 0;
    let mut cran_dp_during_eot = Vec::new();
    for _ in 0..12 {
        if runner.turn_player() != 0 {
            break;
        }
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.effect_choices.is_some() {
            // Trigger order: resolve Craniamon's unsuspend buff as soon as it
            // is queued, otherwise take the first pending clause.
            if !choose_labelled(&mut runner, CRANIAMON) {
                runner.execute_branch(0).expect("order");
            }
        } else if v.prompt.contains("Royal Knight") {
            wake_prompts += 1;
            let target = if is_suspended(&runner, cran) { cran } else { mag };
            act(&mut runner, one_side(target));
        } else {
            accept(&mut runner);
        }
        if runner.turn_player() == 0 {
            cran_dp_during_eot.push(runner.effective_dp(cran).unwrap_or(0));
        }
    }
    let _ = runner.auto_resolve();

    assert_eq!(wake_prompts, 2, "printed + inherited Magnamon each offered a wake");
    assert!(
        cran_dp_during_eot.contains(&15_000),
        "unsuspend → +3000 DP during our end of turn: {cran_dp_during_eot:?}"
    );
    assert_eq!(runner.turn_player(), 1, "opponent's turn now");
    assert!(!is_suspended(&runner, cran), "Craniamon woke (not by our unsuspend phase)");
    assert!(!is_suspended(&runner, mag), "the field Magnamon woke too");
    assert_eq!(runner.effective_dp(cran), Some(12_000), "+3000 expired with our turn");
}

// ═══════════════════════════════════════════════════════════════════════════
// Combo 6 — Leopardmon + Blue Lv.6 DNA → Examon: +10000 to the team, forced
//           attack + battle, Magnamon wakes Examon at end of turn
// ═══════════════════════════════════════════════════════════════════════════

/// Cards: EX13-043 Leopardmon (Green Lv.6 DNA half), ST2-10 Plesiomon (Blue
/// Lv.6 DNA half), EX13-045 Examon (hand), EX13-020 Magnamon (bystander +
/// end-of-turn waker), ST1-05 Birdramon (opponent).
///
/// Expected mechanical outcome: DNA digivolving the [Royal Knight] Leopardmon
/// with Plesiomon into Examon costs **0** (Green Lv.6 + Blue Lv.6: Cost 0);
/// its "if DNA digivolving" [When Digivolving] gives ALL our Digimon +10000
/// (Magnamon 7000 → 17000, Examon 15000 → 25000), makes Examon attack
/// (security, <Security A. +1> → 2 checks) and then lets it battle Birdramon
/// (deleted). At end of turn Magnamon unsuspends the [Royal Knight] Examon,
/// so the deck's closer is back up as a <Blocker>.
///
/// Sources: printed text `data/card_bundles/EX13-045.md`, `EX13-020.md`;
/// DCGO `EX13/Green/EX13_045.cs` (AddJogressConditionClass green6 + blue6
/// cost 0; IsJogress → SelectAttackEffect + ChangeDigimonDPPlayerEffect;
/// IBattle), `EX13/Blue/EX13_020.cs`; `general_rule.pdf` DNA digivolution.
#[test]
fn leopardmon_dna_examon_buffs_team_attacks_and_magnamon_wakes_it() {
    let mut runner = start(builder().hand(0, &[EXAMON]).memory(3));
    runner.place_on_field(0, LEOPARDMON, Some(0));
    runner.place_on_field(0, PLESIOMON, Some(0));
    runner.place_on_field(0, MAGNAMON, Some(0));
    runner.place_on_field(1, BIRDRAMON, Some(0));
    let mem0 = runner.memory();

    assert!(runner.game.initiate_dna_digivolve(0, 0), "DNA action legal");
    runner.game.resolve_selection(0, 0).expect("first DNA material");
    runner.game.resolve_selection(0, 1).expect("second DNA material");

    let ex = find_perm(&runner, 0, EXAMON);
    let mag = find_perm(&runner, 0, MAGNAMON);
    assert_eq!(runner.memory(), mem0, "Green Lv.6 + Blue Lv.6 DNA: Cost 0");
    let mut stack = stack_ids(&runner, ex);
    stack.sort();
    let mut want = vec![EXAMON.to_string(), LEOPARDMON.to_string(), PLESIOMON.to_string()];
    want.sort();
    assert_eq!(stack, want, "both Lv.6 Digimon went under Examon");
    assert_eq!(runner.effective_dp(ex), Some(25_000), "Examon +10000");
    assert_eq!(runner.effective_dp(mag), Some(17_000), "Magnamon +10000 (all your Digimon)");

    // Forced attack at security; keep the target (decline Raid); battle Birdramon.
    let v = view(&runner);
    assert!(!v.is_optional, "\"this Digimon attacks\" is mandatory; view={v:?}");
    act(&mut runner, encode_attack(ex.index as u16, SECURITY_TARGET));
    let mut battled = false;
    for _ in 0..12 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        if v.prompt.contains("Raid") {
            pass(&mut runner);
            continue;
        }
        if v.prompt.contains("battle") && !battled {
            let h = find_perm(&runner, 1, BIRDRAMON);
            act(&mut runner, one_side(h));
            battled = true;
            continue;
        }
        if v.is_optional {
            pass(&mut runner);
        } else {
            accept(&mut runner);
        }
    }
    let _ = runner.auto_resolve();
    assert!(battled, "Examon's may-battle opened; view={}", view_dbg(&runner));
    assert!(field_ids(&runner, 1).is_empty(), "25000 Examon beat Birdramon");
    assert_eq!(runner.security_count(1), 5 - 2, "<Security A. +1>: 2 checks");
    let ex = find_perm(&runner, 0, EXAMON);
    assert!(is_suspended(&runner, ex), "Examon attacked");

    // Leave the opponent memory so their turn waits at Main (no auto-pass back
    // into our own unsuspend phase, which would mask Magnamon's work).
    runner.game.set_memory(-3);
    runner.end_turn();
    let v = view(&runner);
    assert!(v.prompt.contains("Royal Knight"), "Magnamon's EoT unsuspend; view={v:?}");
    act(&mut runner, one_side(ex));
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1, "opponent's turn now");
    assert!(!is_suspended(&runner, ex), "Magnamon woke the [Royal Knight] Examon");
}
