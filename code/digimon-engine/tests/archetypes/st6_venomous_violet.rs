//! ST-6 Venomous Violet — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/st6-venomous-violet-model.md` (the trash-as-resource
//! Purple deck — fill the trash to flip threshold payoffs, recur Digimon out of
//! it for free, and convert your own deletions into tempo). Each `#[test]` maps
//! 1:1 to a ranked combo in that model and asserts the *system-level* outcome
//! the combo claims — several real implemented ST-6 DSL cards working together —
//! which no per-card `cards_behavioral/st6/*.rs` test sees in isolation.
//!
//! Combos under test (model "Ranked interactions to test"):
//!   1. Trash-threshold beatdown (rank 1) — WereGarurumon's inherited `[Your
//!      Turn]` +2000-DP aura gated on `trash >= 5`; the trash-FILL engine
//!      (Pagumon `[On Deletion]` / Gabumon `[When Attacking]`) is what crosses
//!      the threshold.
//!   2. Death Claw -> Matt memory loop (rank 2) — Death Claw's optional
//!      self-sacrifice deletion feeds Matt Ishida's `[Your Turn]`
//!      deletion-observer for +1 memory; the opponent deletion AND Matt's reaction
//!      are both gated on the self-delete actually happening (DCGO `successProcess`).
//!   3. Nail Bone double-recur (rank 3) — free-play a purple Lv3 + a purple Lv4
//!      from trash with their `[On Play]` effects suppressed.
//!
//! Cards are loaded from the embedded DSL pack via `dsl_card("ST6-XX")`;
//! synthetic `make_test_card` bodies are used only as neutral filler / carrier
//! bodies / opponent targets. Source priority for expected outcomes: printed
//! text (`cards/st6/<ID>.json` + `cards/st6/<ID>.yaml`) -> `general_rule.pdf`
//! (§16 keyword semantics, §6/§11 deletion + DP) -> DCGO C#
//! (`DCGO/Assets/Scripts/CardEffect/ST6/Purple/ST6_XX.cs`).

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, GamePhase, PlayerId};
use digimon_engine::selection::{OptionPlayResult, TriggerSource};

// ─── Combo-piece fixtures ────────────────────────────────────────────────────

/// A neutral Lv.4 Purple body to sit on TOP of WereGarurumon in a stack, so the
/// only DP swing we measure is WereGarurumon's *inherited* trash-threshold aura
/// — not any of its own (top-card) effects. Purple Lv4 so the stack is a legal
/// `Lv4 -> Lv5` digivolution shape.
fn make_lv4_body(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.colors = vec![digimon_engine::enums::CardColor::Purple];
    c
}

/// A neutral own Lv.4 Digimon for Death Claw to sacrifice. Plain body so the
/// only deletion observers in play are Matt's (the combo under test).
fn make_expendable(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// A neutral opponent Lv.4 Digimon — a legal Death Claw target (level <= 4).
fn make_opp_lv4(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// A neutral filler used purely to pad a trash zone above a count threshold.
fn make_trash_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Seed `count` copies of `card_id` into `player`'s trash zone (real
/// `CardSource`s, mirroring `place_on_field`'s construction). Trash threshold
/// auras and `play_from_trash_free` read this zone, so this is how we set up
/// "5+ cards in trash" / "an eligible recursion target sits in trash".
fn seed_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str, count: usize) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("seed_trash: unknown card_id {card_id}"));
    for _ in 0..count {
        let next_idx = runner.game.next_card_index();
        let mut card = CardSource::new(data_idx, player, next_idx);
        card.card_index = next_idx;
        runner.game.players[player as usize].trash.push(card);
    }
}

fn trash_ids(runner: &DebugRunner, player: PlayerId) -> Vec<String> {
    runner
        .game
        .player(player)
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_ids(runner: &DebugRunner, player: PlayerId) -> Vec<String> {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn hand_ids(runner: &DebugRunner, player: PlayerId) -> Vec<String> {
    runner
        .game
        .player(player)
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// Fire a permanent's triggered timing window directly (enqueue + drain) — the
/// proven harness pattern for `[On Deletion]` / `[When Attacking]` windows that
/// `place_*` setup builds the board for but does not itself fire.
fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

// ─── Combo 1 (rank 1): Trash-threshold beatdown ──────────────────────────────

/// **ST6-11 WereGarurumon** inherited aura: `[Your Turn]` while you have 5 or
/// more cards in your trash, the bearer gets **+2000 DP**.
///
/// Combo: WereGarurumon sits as an inherited digivolution source under a Lv.4
/// Purple body (`place_stack(0, ["ST6-11", <body>])`). The buff is a pure
/// function of (your turn ∧ `trash >= 5`), measured via `effective_dp` (which
/// folds inherited-source declarative DP auras into the bearer's combined DP —
/// the value combat actually compares). Expected outcome:
///   - your turn + trash >= 5 -> base + 2000;
///   - your turn + trash < 5  -> base (the count gate turns the aura off);
///   - opponent's turn + trash >= 5 -> base (the `[Your Turn]` gate turns it off).
///
/// This is the system fact a per-card test can't express: the deck's payoff DP
/// is *conditioned on a trash count* that the rest of the deck deliberately
/// fills. The trash-FILL half of the engine — Pagumon `[On Deletion]` trashing
/// the top 2, Gabumon `[When Attacking]` draw-1-then-trash-1 — is what crosses
/// the threshold, so we drive those windows to push trash from 4 to 5 and show
/// the buff flip on.
///
/// Sources: printed text `cards/st6/ST6-11.json` + YAML aura
/// (`scope: inherited`, `active_when {your_turn, count_gte trash 5}`,
/// `dp_modifier 2000`); fill engine `cards/st6/ST6-01.yaml` (`on_deletion
/// trash_from_top 2`) + `ST6-03.yaml` (`when_attacking draw 1 / trash 1`); DCGO
/// `ST6_11.cs` / `ST6_01.cs` / `ST6_03.cs`; DP comparison `general_rule.pdf` §11.
#[test]
fn weregarurumon_trash_threshold_grants_plus_2000_only_on_your_turn_at_five_trash() {
    let body = make_lv4_body("VV-BODY", 6000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-11")
        .expect("ST6-11 (WereGarurumon) in embedded DSL pack")
        .dsl_card("ST6-01")
        .expect("ST6-01 (Pagumon) in embedded DSL pack")
        .dsl_card("ST6-03")
        .expect("ST6-03 (Gabumon) in embedded DSL pack")
        .add_card(body)
        .deck(0, &["ST6-03", "ST6-03", "ST6-03", "ST6-03"]) // top-of-deck fodder for trashes/draws
        .memory(0)
        .build();
    // Your turn (player 0). place_stack places on player 0's field.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Stack [WereGarurumon (inherited aura), neutral Lv4 body].
    let stack = runner.place_stack(0, &["ST6-11", "VV-BODY"]);
    let base = runner.game.players[0].battle_area[stack.index as usize]
        .base_dp(&runner.game.card_data)
        .expect("carrier base DP");

    // ── trash < 5 on your turn → no buff. ──
    seed_trash(&mut runner, 0, "ST6-01", 4);
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.trash_size(0),
        4,
        "precondition: 4 cards in trash (below the WereGarurumon threshold)"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(base),
        "below 5 trash, WereGarurumon's [Your Turn] +2000 aura is off"
    );

    // ── Trash-FILL engine crosses the threshold (4 → 5). ──
    // Pagumon [On Deletion] trashes the top 2 of deck. Drive its window from an
    // inherited-source carrier so a real card fires the fill, not a raw trash().
    let pagumon_carrier = runner.place_stack(0, &["ST6-01", "VV-BODY"]);
    let trash_before_fill = runner.trash_size(0);
    let deck_before_fill = runner.deck_size(0);
    fire(&mut runner, EffectTiming::OnDeletion, pagumon_carrier);
    assert_eq!(
        runner.trash_size(0),
        trash_before_fill + 2,
        "Pagumon [On Deletion] trashes the top 2 cards of deck (the trash-fill engine)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before_fill - 2,
        "the 2 filled cards leave the deck"
    );

    // ── trash >= 5 on your turn → +2000. ──
    runner.game.tick_declarative_effects();
    assert!(
        runner.trash_size(0) >= 5,
        "trash is now at/above the WereGarurumon threshold (was {trash_before_fill}, +2)"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(base + 2000),
        "at 5+ trash on your turn, WereGarurumon grants the bearer +2000 DP"
    );

    // ── Opponent's turn → the [Your Turn] gate turns the aura off even though
    // trash is still 5+. ──
    runner.game.turn_player_idx = 1;
    runner.game.tick_declarative_effects();
    assert!(
        runner.trash_size(0) >= 5,
        "trash count is unchanged on the opponent's turn"
    );
    assert_eq!(
        runner.effective_dp(stack),
        Some(base),
        "the +2000 is [Your Turn]-only — it turns off on the opponent's turn"
    );
}

// ─── Combo 2 (rank 2): Death Claw → Matt memory loop ──────────────────────────

/// **ST6-15 Death Claw** `[Main]` (option): *you may delete 1 of your Digimon to
/// delete 1 of your opponent's Lv.4-or-lower Digimon* + **ST6-14 Matt Ishida**
/// `[Your Turn]`: *when one of your Digimon is deleted, you may suspend this
/// Tamer to gain 1 memory.*
///
/// Happy path: with Matt on the field (unsuspended) and an expendable own
/// Digimon, playing Death Claw and **accepting** the self-delete:
///   1. trashes the expendable own Digimon (own field −1);
///   2. that deletion fires Matt's `[Your Turn]` observer — player 0 accepts the
///      optional response, Matt **suspends** and player 0 gains **+1 memory**;
///   3. Death Claw's `successProcess` then deletes the opponent Lv.4 Digimon
///      (opp field −1).
///
/// The net is a 1-for-1 removal that also nets a tamer-suspend for +1 memory —
/// the deck's tempo engine, and a true multi-card loop no per-card test sees.
///
/// Sources: `ST6_15.cs` (own-delete `canNoSelect: true`, opponent delete only
/// inside `SelectPermanentCoroutine`'s `successProcess`), `ST6_14.cs`
/// (`OnDestroyedAnyone`, `IsOwnerTurn`, suspend cost, `AddMemory(1)`); YAML
/// `cards/st6/ST6-15.yaml` (`select_own_permanent optional`, then `if
/// binding_present sacrifice` → opp delete) + `ST6-14.yaml`
/// (`on_any_deletion optional`, `your_turn`, `activation_cost suspend_self`,
/// `gain_memory 1`); deletion timing `general_rule.pdf` §6/§9.
#[test]
fn death_claw_self_sacrifice_feeds_matt_then_deletes_opponent() {
    let expendable = make_expendable("VV-SAC", 4000);
    let opp = make_opp_lv4("VV-OPP", 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-15")
        .expect("ST6-15 (Death Claw) in embedded DSL pack")
        .dsl_card("ST6-14")
        .expect("ST6-14 (Matt Ishida) in embedded DSL pack")
        .add_card(expendable)
        .add_card(opp)
        .hand(0, &["ST6-15"])
        .memory(3)
        .build();
    // Your turn (player 0) — Matt's [Your Turn] observer only fires on your turn.
    // Death Claw is an Option: its `[Main]` body resolves through the Option
    // lifecycle (`play_option_from_hand` → pay use cost → OptionMain → dispose
    // to trash), which is gated on the Main phase. `.build()` leaves the game
    // in Mulligan and does NOT apply `.memory(3)` (only `.start()` does), so we
    // set both explicitly — the same `.build()`-style manual setup the other
    // ST starter archetype tests use (cf. st4_giga_green `make_p0_turn`).
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    runner.game.current_phase = GamePhase::Main;
    runner.game.set_memory(3);

    runner.place_on_field(0, "ST6-14", Some(0)); // Matt, unsuspended
    runner.place_on_field(0, "VV-SAC", Some(0)); // expendable own Digimon
    runner.place_on_field(1, "VV-OPP", Some(0)); // opponent Lv4 target

    let memory_before = runner.memory();

    // Play Death Claw (Option) through the real Option lifecycle → pays its
    // use cost (1) and installs the optional own-Digimon select. The body
    // parks a selection, so the play returns `Pending`.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "Death Claw's [Main] installs the optional self-delete selection (Pending)"
    );
    let sac_prompt = runner
        .pending_selection()
        .expect("Death Claw installs the optional self-delete selection");
    assert!(
        sac_prompt.is_optional,
        "the self-delete is a 'you may' — the prompt must allow declining"
    );
    // Accept the self-delete: pick the only legal own Digimon (the expendable
    // body — Matt is a Tamer, not a Digimon, so it is never a target).
    let sac_pick = sac_prompt.valid_action_ids[0];
    runner
        .execute_action(0, sac_pick)
        .expect("delete the expendable own Digimon");

    // The self-deletion fires Matt's optional [Your Turn] response AND Death
    // Claw's opponent-delete success branch. `auto_resolve` accepts every
    // pending prompt by its first valid action: Matt's cost-bearing optional
    // trigger surfaces a single-entry TriggerOrder prompt whose first action is
    // "resolve" (suspend-self → +1 memory) rather than PASS, and the
    // opponent-delete prompt's first action is the lone Lv.4 target — so the
    // loop drives the full happy path regardless of trigger ordering, and the
    // Standard Option then disposes itself to trash once the body finishes.
    let _ = runner.auto_resolve();

    // Matt paid the cost by suspending itself.
    let matt = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "ST6-14")
        .expect("Matt is still on the field");
    assert!(
        runner.game.players[0].battle_area[matt].is_suspended,
        "Matt suspends to pay its [Your Turn] response cost"
    );
    // +1 memory from Matt's reaction. Net of Death Claw's own 1-cost use:
    // memory_before − 1 (Death Claw use cost) + 1 (Matt) = memory_before. The
    // suspend assertion above already proves Matt's +1 fired; this pins the
    // arithmetic (the loop is memory-neutral, not a net +1 — the printed Death
    // Claw use cost is paid first).
    assert_eq!(
        runner.memory(),
        memory_before,
        "Matt's +1 memory exactly offsets Death Claw's 1-cost play \
         (memory_before − 1 + 1 = memory_before)"
    );
    // The opponent Lv.4 Digimon was deleted by Death Claw's successProcess.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "Death Claw deletes the opponent Lv.4 Digimon after the self-delete succeeds"
    );
    assert_eq!(
        runner.trash_size(1),
        1,
        "the deleted opponent Digimon lands in the opponent's trash"
    );
    // The expendable own Digimon is gone; only Matt remains on player 0's field.
    assert_eq!(
        field_ids(&runner, 0),
        vec!["ST6-14".to_string()],
        "the sacrificed own Digimon left the field; only Matt remains"
    );
}

/// Unhappy path — **declining** the optional self-delete short-circuits the whole
/// combo. Death Claw's opponent deletion lives inside the self-delete's success
/// branch (DCGO `SelectPermanentCoroutine` → `successProcess`; DSL `if
/// binding_present: sacrifice`), so with no sacrifice:
///   - the opponent Lv.4 Digimon is **not** deleted;
///   - Matt never sees a deletion, so it does **not** fire (no suspend, no memory).
///
/// This pins the gate the happy path depends on: the removal AND the memory loop
/// are both conditional on the player actually paying the self-sacrifice.
#[test]
fn declining_death_claw_self_delete_spares_the_opponent_and_silences_matt() {
    let expendable = make_expendable("VV-SAC", 4000);
    let opp = make_opp_lv4("VV-OPP", 5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-15")
        .expect("ST6-15 (Death Claw) in embedded DSL pack")
        .dsl_card("ST6-14")
        .expect("ST6-14 (Matt Ishida) in embedded DSL pack")
        .add_card(expendable)
        .add_card(opp)
        .hand(0, &["ST6-15"])
        .memory(3)
        .build();
    // Same `.build()`-style manual setup as the happy-path test: Main phase
    // (the Option lifecycle is Main-gated) and explicit memory (`.build()`
    // does not apply `.memory(3)`).
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    runner.game.current_phase = GamePhase::Main;
    runner.game.set_memory(3);

    let matt = runner.place_on_field(0, "ST6-14", Some(0));
    runner.place_on_field(0, "VV-SAC", Some(0));
    runner.place_on_field(1, "VV-OPP", Some(0));

    let memory_before = runner.memory();

    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "Death Claw's [Main] installs the optional self-delete selection (Pending)"
    );
    let sac_prompt = runner
        .pending_selection()
        .expect("Death Claw installs the optional self-delete selection");
    assert!(
        sac_prompt.is_optional,
        "the self-delete must be declinable"
    );
    // Decline the self-delete (PASS the optional own-Digimon select).
    runner
        .decline_optional_trigger()
        .expect("decline Death Claw's self-delete");
    let _ = runner.auto_resolve();

    // No deletion happened anywhere: own board intact, opponent board intact.
    // Death Claw itself is an Option — it resolves out of `pending_option` and
    // disposes to the controller's trash, so it is NEVER a battle-area
    // permanent. Player 0's battle area is exactly Matt + the expendable body.
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "declining means no own Digimon is sacrificed (Matt + expendable remain; \
         Death Claw the Option is disposed to trash, never on the field)"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "with no self-delete, Death Claw's opponent deletion never runs (successProcess gate)"
    );
    assert_eq!(
        runner.trash_size(1),
        0,
        "no opponent Digimon is trashed"
    );
    // Death Claw was used and disposed: it lands in player 0's trash even when
    // the optional deletion is declined (the play cost is spent regardless).
    assert!(
        trash_ids(&runner, 0).contains(&"ST6-15".to_string()),
        "the used Death Claw Option disposes to player 0's trash; trash={:?}",
        trash_ids(&runner, 0)
    );
    // Matt never reacted: still unsuspended, no memory gained.
    assert!(
        !runner.game.players[0].battle_area[matt.index as usize].is_suspended,
        "no deletion occurred, so Matt's [Your Turn] observer never fires"
    );
    // Declining spends only Death Claw's own 1-cost (no Matt +1 to offset it),
    // so memory ends exactly 1 below the start — the inverse of the happy path,
    // where Matt's +1 cancels the cost.
    assert_eq!(
        runner.memory(),
        memory_before - 1,
        "no Matt reaction: only Death Claw's 1-cost is spent (memory_before − 1)"
    );
}

// ─── Combo 3 (rank 3): Nail Bone double-recur, On Play suppressed ─────────────

/// **ST6-16 Nail Bone** `[Main]` (option): *you may play 1 purple Lv.3 and 1
/// purple Lv.4 Digimon from your trash without paying their costs; any `[On
/// Play]` effects on Digimon played this way don't activate.*
///
/// Combo: seed player 0's trash with exactly one eligible purple Lv.3 (ST6-04
/// Dracmon — which itself has an `[On Play]` Option-return), one eligible purple
/// Lv.4 (ST6-06 Garurumon), one ineligible card (a non-purple filler), and a
/// purple cost-1 Option (another Death Claw) that Dracmon's `[On Play]` *would*
/// return if it fired. Activating Nail Bone and selecting both eligible Digimon:
///   - exactly the Lv.3 AND the Lv.4 are played to player 0's field (field +2);
///   - both leave the trash (the 2 recurred cards −, the rest untouched);
///   - their `[On Play]` effects are **suppressed** — Dracmon's Option-return
///     does NOT fire, so the seeded Death Claw stays in trash and no `[On Play]`
///     return-selection is ever installed;
///   - the ineligible filler is never a candidate and never moves;
///   - no memory is paid for the recurred bodies (they are free plays).
///
/// `[On Play]` suppression on a free-play-from-trash is the exact mechanic that
/// makes this a *recursion* tool rather than a re-trigger tool — the system fact
/// the combo claims, invisible to a per-card test of Dracmon or Garurumon alone.
///
/// Sources: `ST6_16.cs` (`PlayPermanentCards ... activateETB: false`, one Lv3 +
/// one Lv4 capped); YAML `cards/st6/ST6-16.yaml` (two `play_from_trash_free`
/// with `suppress_on_play: true`); Dracmon `[On Play]` `cards/st6/ST6-04.yaml`
/// (the effect proven suppressed). On Play suppression `general_rule.pdf` §9 +
/// the printed "don't activate" clause.
#[test]
fn nail_bone_recurs_lv3_and_lv4_from_trash_with_on_play_suppressed() {
    let filler = {
        let mut c = make_trash_filler("VV-FILLER");
        // Non-purple, so it can never be a Nail Bone candidate.
        c.colors = vec![digimon_engine::enums::CardColor::Red];
        c
    };
    // A purple Digimon on player 0's field satisfies the Option color
    // requirement (an Option is only playable with a color-matching Digimon/Tamer
    // in play — `option_color_match_available`). The Death Claw tests get this
    // from Matt on the field; Nail Bone's board is otherwise empty.
    let purple_anchor = {
        let mut c = make_test_card("VV-ANCHOR", "VV-ANCHOR");
        c.card_kind = CardKind::Digimon;
        c.level = Some(3);
        c.dp = Some(3000);
        c.colors = vec![digimon_engine::enums::CardColor::Purple];
        c
    };

    let mut runner = DebugRunner::builder()
        .dsl_card("ST6-16")
        .expect("ST6-16 (Nail Bone) in embedded DSL pack")
        .dsl_card("ST6-04")
        .expect("ST6-04 (Dracmon, purple Lv3 with [On Play]) in embedded DSL pack")
        .dsl_card("ST6-06")
        .expect("ST6-06 (Garurumon, purple Lv4) in embedded DSL pack")
        .dsl_card("ST6-15")
        .expect("ST6-15 (Death Claw, purple cost-1 Option) in embedded DSL pack")
        .add_card(filler)
        .add_card(purple_anchor)
        .hand(0, &["ST6-16"])
        .memory(10)
        .build();
    // Nail Bone is an Option (cost 7): it resolves through the Option lifecycle
    // (Main-gated), so set the Main phase and apply memory explicitly — `.build()`
    // leaves Mulligan and does not apply `.memory(10)`.
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    runner.game.current_phase = GamePhase::Main;
    runner.game.set_memory(10);

    // Seed trash: 1 eligible Lv3, 1 eligible Lv4, 1 ineligible filler, and a
    // purple cost-1 Option that Dracmon's [On Play] WOULD return if it fired.
    seed_trash(&mut runner, 0, "ST6-04", 1); // Dracmon (purple Lv3)
    seed_trash(&mut runner, 0, "ST6-06", 1); // Garurumon (purple Lv4)
    seed_trash(&mut runner, 0, "VV-FILLER", 1); // ineligible (non-purple)
    seed_trash(&mut runner, 0, "ST6-15", 1); // Death Claw — Dracmon [On Play] bait

    // Color anchor: a purple Digimon in play so Nail Bone (an Option) is legal.
    runner.place_on_field(0, "VV-ANCHOR", Some(0));

    let field_before = runner.battle_area_size(0);
    let trash_before = runner.trash_size(0);
    let memory_before = runner.memory();
    let hand_before = runner.hand_size(0);

    // Activate Nail Bone from hand through the real Option lifecycle (pays its
    // 7-cost, fires the OptionMain body, then disposes itself to trash). The
    // body parks the Lv.3 trash selection, so the play returns `Pending`.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "Nail Bone's [Main] installs the purple-Lv3 trash selection (Pending)"
    );

    // First selection: the purple Lv.3 candidate (only Dracmon qualifies).
    let lv3_prompt = runner
        .pending_selection()
        .expect("Nail Bone installs the purple-Lv3 trash selection");
    assert_eq!(
        lv3_prompt.valid_action_ids.len(),
        1,
        "only the seeded purple Lv.3 (Dracmon) is a legal Lv.3 candidate — the \
         non-purple filler and the Option are excluded"
    );
    let lv3_pick = lv3_prompt.valid_action_ids[0];
    runner.execute_action(0, lv3_pick).expect("recur Dracmon");

    // Second selection: the purple Lv.4 candidate (only Garurumon qualifies).
    let lv4_prompt = runner
        .pending_selection()
        .expect("Nail Bone installs the purple-Lv4 trash selection");
    assert_eq!(
        lv4_prompt.valid_action_ids.len(),
        1,
        "only the seeded purple Lv.4 (Garurumon) is a legal Lv.4 candidate"
    );
    let lv4_pick = lv4_prompt.valid_action_ids[0];
    runner.execute_action(0, lv4_pick).expect("recur Garurumon");

    // No further selection should be pending: Dracmon's [On Play] is SUPPRESSED,
    // so its Option-return never installs a prompt.
    assert!(
        runner.pending_selection().is_none(),
        "Dracmon's [On Play] is suppressed by Nail Bone — no Option-return prompt installs"
    );

    // Both Digimon are now on player 0's field (field +2).
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 2,
        "Nail Bone free-plays exactly the Lv.3 and the Lv.4 to the field"
    );
    let field = field_ids(&runner, 0);
    assert!(
        field.iter().any(|id| id == "ST6-04") && field.iter().any(|id| id == "ST6-06"),
        "both Dracmon and Garurumon are on the field; field={field:?}"
    );

    // Net trash change: the 2 recurred Digimon LEAVE (−2), but Nail Bone — an
    // Option — disposes itself to trash after resolving (+1), for a net −1. The
    // ineligible filler and the suppressed-On-Play bait (Death Claw) are
    // untouched.
    assert_eq!(
        runner.trash_size(0),
        trash_before - 1,
        "the 2 recurred Digimon leave the trash (−2) and the used Nail Bone Option \
         disposes back into it (+1) → net −1"
    );
    let remaining = trash_ids(&runner, 0);
    assert!(
        !remaining.contains(&"ST6-04".to_string()) && !remaining.contains(&"ST6-06".to_string()),
        "both recurred Digimon (Dracmon, Garurumon) left the trash; trash={remaining:?}"
    );
    assert!(
        remaining.contains(&"VV-FILLER".to_string()),
        "the ineligible non-purple filler is never a candidate and stays in trash; trash={remaining:?}"
    );
    assert!(
        remaining.contains(&"ST6-15".to_string()),
        "Dracmon's [On Play] return is suppressed, so the Death Claw bait stays in trash; trash={remaining:?}"
    );
    assert!(
        remaining.contains(&"ST6-16".to_string()),
        "the used Nail Bone Option disposes back to player 0's trash; trash={remaining:?}"
    );

    // The recurred bodies are free plays — only Nail Bone's own 7-cost is paid,
    // and the suppressed Dracmon [On Play] never returned anything to hand.
    assert_eq!(
        runner.memory(),
        memory_before - 7,
        "Nail Bone free-plays the recurred Digimon — the only cost is Nail Bone's own 7"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "only Nail Bone itself left the hand; the suppressed [On Play] returns nothing"
    );
}
