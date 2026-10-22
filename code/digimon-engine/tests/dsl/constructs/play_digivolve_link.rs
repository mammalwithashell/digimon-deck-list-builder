//! Real engine contracts for play, digivolve and link DSL constructs.
use digimon_engine::action::{mask::build_action_mask, space::*};
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const HATCH: &str = r#"
card: HATCH
name: Hatch
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - hatch: { of: you }
      - gain_memory: 1
"#;
const HATCH_OBS: &str = r#"
card: HATCH-OBS
name: Hatch observer
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_hatch
    process:
      - set_memory: 5
"#;
const DNA: &str = r#"
card: DNA
name: DNA Anchor
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - may_dna_digivolve_now:
          partner_filter: { of: you, kind: digimon }
          target_filter: { of: you, kind: digimon }
          cost: 0
          ignore_requirements: false
          optional: true
"#;
fn card(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(3000);
    c
}
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 20])
        .deck(1, &["PAD"; 20])
        .memory(5)
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn prompt(r: &DebugRunner, expected: &[u16], optional: bool) {
    let p = r.game.pending_selection.as_ref().expect("real choice");
    assert_eq!(p.valid_action_ids, expected);
    assert_eq!(p.is_optional, optional);
    assert!(r.game.pending_selection_resume.is_some());
    for (a, v) in build_action_mask(&r.game, p.selecting_player)
        .iter()
        .enumerate()
    {
        assert_eq!(
            *v,
            if expected.contains(&(a as u16)) || (optional && a == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "action {a}"
        );
    }
}
fn pick(r: &mut DebugRunner, a: u16) {
    r.execute_action(0, a).unwrap();
}
fn no_prompt(r: &DebugRunner) {
    assert!(r.game.pending_selection.is_none());
}
fn top(r: &DebugRunner, h: PermanentHandle) -> String {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&r.game.card_data)
        .to_owned()
}
fn hatch_runner(opponent: bool) -> DebugRunner {
    let yaml = if opponent {
        HATCH.replace("of: you", "of: opponent")
    } else {
        HATCH.to_owned()
    };
    let mut b = base()
        .from_dsl_yaml(&yaml)
        .unwrap()
        .from_dsl_yaml(HATCH_OBS)
        .unwrap()
        .memory(2);
    for id in ["EGG-A", "EGG-B"] {
        let mut c = card(id, CardColor::Red, 2);
        c.card_kind = CardKind::DigiEgg;
        b = b.add_card(c);
    }
    b.digitama(0, &["EGG-A", "EGG-B"])
        .digitama(1, &["EGG-A"])
        .start()
}
#[test]
fn hatch_top_and_observer_after_enclosing_body() {
    let mut r = hatch_runner(false);
    r.place_on_field(0, "HATCH-OBS", Some(0));
    let expected = r.game.players[0].digitama_deck.last().unwrap().handle();
    let h = r.place_on_field(0, "HATCH", Some(0));
    fire(&mut r, h);
    assert_eq!(
        r.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .handle(),
        expected
    );
    assert_eq!(r.game.players[0].digitama_deck.len(), 1);
    assert_eq!(r.game.memory, 5);
    no_prompt(&r);
}
#[test]
fn hatch_occupied_and_empty_noop() {
    for occupied in [false, true] {
        let mut r = hatch_runner(false);
        r.place_on_field(0, "HATCH-OBS", Some(0));
        if occupied {
            r.place_in_breeding(0, "EGG-B");
        } else {
            r.game.players[0].digitama_deck.clear();
        }
        let before = r.game.players[0].digitama_deck.len();
        let h = r.place_on_field(0, "HATCH", Some(0));
        fire(&mut r, h);
        assert_eq!(r.game.players[0].digitama_deck.len(), before);
        assert_eq!(r.game.memory, 3);
        assert_eq!(r.game.players[0].breeding_area.is_some(), occupied);
        no_prompt(&r);
    }
}
#[test]
fn hatch_opponent_breeding() {
    let mut r = hatch_runner(true);
    let h = r.place_on_field(0, "HATCH", Some(0));
    fire(&mut r, h);
    assert!(r.game.players[0].breeding_area.is_none());
    assert!(r.game.players[1].breeding_area.is_some());
    no_prompt(&r);
}
fn dna_runner(
    yaml: &str,
    reachable: bool,
) -> (
    DebugRunner,
    PermanentHandle,
    PermanentHandle,
    PermanentHandle,
) {
    let mut result = card("RESULT", CardColor::Red, 4);
    if reachable {
        let req = DnaRequirement {
            level: 3,
            card_colors: vec![CardColor::Red],
            name_contains: String::new(),
            text_contains: String::new(),
        };
        result.dna_costs = vec![DnaCost {
            requirement1: req.clone(),
            requirement2: req,
            memory_cost: 2,
        }];
    }
    let mut r = base()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(result)
        .add_card(card("PARTNER", CardColor::Red, 3))
        .add_card(card("OFF", CardColor::Blue, 3))
        .add_card(card("BAD", CardColor::Red, 4))
        .hand(0, &["RESULT", "BAD"])
        .start();
    let p = r.place_on_field(0, "PARTNER", Some(0));
    let off = r.place_on_field(0, "OFF", Some(0));
    let h = r.place_on_field(0, "DNA", Some(0));
    (r, h, p, off)
}
#[test]
fn may_dna_pair_gate_printed_cost_materials() {
    let (mut r, h, p, _) = dna_runner(DNA, true);
    fire(&mut r, h);
    prompt(&r, &[encode_attack(0, p.index as u16)], true);
    pick(&mut r, encode_attack(0, p.index as u16));
    prompt(&r, &[PLAY_HAND_START], true);
    pick(&mut r, PLAY_HAND_START);
    assert_eq!(r.game.memory, 3);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    let perm = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "RESULT")
        .unwrap();
    let ids: Vec<_> = perm
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data))
        .collect();
    assert!(ids.contains(&"DNA"));
    assert!(ids.contains(&"PARTNER"));
    no_prompt(&r);
}
#[test]
fn may_dna_no_reachable_target_no_prompt() {
    let (mut r, h, _, _) = dna_runner(DNA, false);
    fire(&mut r, h);
    no_prompt(&r);
    assert_eq!(r.game.memory, 5);
    assert_eq!(r.game.players[0].battle_area.len(), 3);
}
#[test]
fn may_dna_optional_pass_unchanged() {
    let (mut r, h, p, _) = dna_runner(DNA, true);
    fire(&mut r, h);
    prompt(&r, &[encode_attack(0, p.index as u16)], true);
    pick(&mut r, PASS);
    no_prompt(&r);
    assert_eq!(r.game.memory, 5);
    assert_eq!(r.game.players[0].hand.len(), 2);
    assert_eq!(r.game.players[0].battle_area.len(), 3);
    assert_eq!(top(&r, h), "DNA");
    assert_eq!(top(&r, p), "PARTNER");
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(
        r.game.players[0].battle_area[p.index as usize]
            .card_sources
            .len(),
        1
    );
}
#[test]
fn may_dna_ignore_requirements_authored_cost_mandatory_picks() {
    let yaml = DNA
        .replace("ignore_requirements: false", "ignore_requirements: true")
        .replace("cost: 0", "cost: 1")
        .replace("optional: true", "optional: false");
    let (mut r, h, p, off) = dna_runner(&yaml, false);
    fire(&mut r, h);
    prompt(
        &r,
        &[
            encode_attack(0, p.index as u16),
            encode_attack(0, off.index as u16),
        ],
        false,
    );
    pick(&mut r, encode_attack(0, off.index as u16));
    prompt(&r, &[PLAY_HAND_START, PLAY_HAND_START + 1], false);
    pick(&mut r, PLAY_HAND_START);
    assert_eq!(r.game.memory, 4);
    no_prompt(&r);
}
#[test]
fn may_dna_clause_optional_has_one_passable_prompt() {
    let yaml = DNA.replace("  - when: on_play", "  - when: on_play\n    optional: true");
    let (mut r, h, p, _) = dna_runner(&yaml, true);
    fire(&mut r, h);
    prompt(&r, &[encode_attack(0, p.index as u16)], true);
    pick(&mut r, PASS);
    no_prompt(&r);
}
#[test]
fn may_dna_clause_optional_ineligible_has_no_prompt() {
    let yaml = DNA.replace("  - when: on_play", "  - when: on_play\n    optional: true");
    let (mut r, h, _, _) = dna_runner(&yaml, false);
    fire(&mut r, h);
    no_prompt(&r);
}

#[test]
fn may_dna_outer_optional_mandatory_step_guard() {
    let yaml = DNA
        .replace("optional: true", "optional: false")
        .replace("  - when: on_play", "  - when: on_play\n    optional: true");
    for reachable in [false, true] {
        let (mut r, h, p, _) = dna_runner(&yaml, reachable);
        fire(&mut r, h);
        if !reachable {
            no_prompt(&r);
            continue;
        }
        prompt(&r, &[REPLACEMENT_ACCEPT], true);
        pick(&mut r, REPLACEMENT_ACCEPT);
        prompt(&r, &[encode_attack(0, p.index as u16)], false);
        pick(&mut r, encode_attack(0, p.index as u16));
        prompt(&r, &[PLAY_HAND_START], false);
        pick(&mut r, PLAY_HAND_START);
        assert_eq!(r.game.memory, 3);
        no_prompt(&r);
    }
}

mod app_fuse_port {
    // Effect-initiated App Fuse primitive — behavioral coverage.
    //
    // Drives the `app_fuse` DSL step through minimal `from_dsl_yaml` fixture cards
    // whose `[On Play]` clause carries a single `app_fuse` step. The result card is
    // a synthetic App-Fusion Digimon (`TEST-APPFUSE`, materials Kabemon & Gomimon),
    // mirroring `tests/cards_behavioral/bt25/app_fusion.rs`. Two engine-driven
    // selections: pick the field permanent, then the result card; then the commit
    // stacks the result on the host and folds the linked materials under it.

    use digimon_engine::action::space::{
        encode_attack, HAND_EFFECT_START, PASS, TRASH_EFFECT_START,
    };
    use digimon_engine::card_data::CardData;
    use digimon_engine::card_source::CardSource;
    use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
    use digimon_engine::enums::CardColor;
    use digimon_engine::permanent::PermanentHandle;
    use digimon_engine::selection::SelectionKind;

    /// Result card: App-Fusion Digimon whose materials are Kabemon & Gomimon.
    const APP_FUSE_RESULT: &str = r#"
card: TEST-APPFUSE
name: Test App Fusion Digimon
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 9000
traits: [Appmon]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

    /// A second App-Fusion result carrying a distinguishing trait, for the filter
    /// test. Same materials (Kabemon & Gomimon) so it is route-eligible.
    const APP_FUSE_RESULT_TAGGED: &str = r#"
card: TEST-APPFUSE-TAGGED
name: Test App Fusion Tagged
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 9000
traits: [Appmon, System]
alt_paths:
  - kind: app_fusion
    materials:
      - filter:
          name_in: [Kabemon, Gomimon]
    cost: 0
"#;

    /// Fixture: on play, app fuse from hand (no result filter). Clause is
    /// non-optional; the `app_fuse` step itself is optional ("may").
    const SRC_HAND: &str = r#"
card: DSL-APPFUSE-HAND
name: App Fuse Source Hand
kind: tamer
color: [green]
cost: 0
effects:
  - when: on_play
    process:
      - app_fuse: { from: hand, optional: true }
"#;

    /// Fixture: on play, app fuse from trash (no filter).
    const SRC_TRASH: &str = r#"
card: DSL-APPFUSE-TRASH
name: App Fuse Source Trash
kind: tamer
color: [green]
cost: 0
effects:
  - when: on_play
    process:
      - app_fuse: { from: trash, optional: true }
"#;

    /// Fixture: on play, app fuse from trash with a [System] result filter.
    const SRC_FILTER: &str = r#"
card: DSL-APPFUSE-FILTER
name: App Fuse Source Filter
kind: tamer
color: [green]
cost: 0
effects:
  - when: on_play
    process:
      - app_fuse:
          from: trash
          optional: true
          result_filter: { trait_has: System }
"#;

    fn named_digimon(card_id: &str, name: &str) -> CardData {
        let mut card = make_test_card(card_id, name);
        card.colors = vec![CardColor::Yellow];
        card.level = Some(4);
        card
    }

    fn builder() -> DebugRunnerBuilder {
        DebugRunner::builder()
            .from_dsl_yaml(SRC_HAND)
            .expect("SRC_HAND compiles")
            // Linked cards carry [Link] (Q4533; §17-1-3-2-6 / -7).
            .link_requirement("TEST-GOMIMON", 0)
            .from_dsl_yaml(SRC_TRASH)
            .expect("SRC_TRASH compiles")
            .from_dsl_yaml(SRC_FILTER)
            .expect("SRC_FILTER compiles")
            .from_dsl_yaml(APP_FUSE_RESULT)
            .expect("APP_FUSE_RESULT compiles")
            .from_dsl_yaml(APP_FUSE_RESULT_TAGGED)
            .expect("APP_FUSE_RESULT_TAGGED compiles")
            .add_card(named_digimon("TEST-KABEMON", "Kabemon"))
            .add_card(named_digimon("TEST-GOMIMON", "Gomimon"))
            .memory(5)
    }

    fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
        let idx = runner
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap();
        let iid = runner.game.next_card_index();
        runner.game.players[player]
            .trash
            .push(CardSource::new(idx, player as u8, iid));
    }

    /// Host with Kabemon top + Gomimon linked = 2 distinct named materials.
    fn fused_host(runner: &mut DebugRunner) -> PermanentHandle {
        let host = runner.place_on_field(0, "TEST-KABEMON", Some(0));
        runner.push_linked_owned(host, "TEST-GOMIMON", 0);
        host
    }

    fn hand_index(runner: &DebugRunner, card_id: &str) -> usize {
        runner.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&runner.game.card_data) == card_id)
            .expect("card in hand")
    }

    fn top_id(runner: &DebugRunner, host: PermanentHandle) -> String {
        runner.game.players[0].battle_area[host.index as usize]
            .top_card()
            .card_id(&runner.game.card_data)
            .to_string()
    }

    // ── 1. Hand happy path ────────────────────────────────────────────────────────

    #[test]
    fn app_fuse_hand_happy_path_stacks_result_and_consumes_link() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        let host = fused_host(&mut r);

        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);

        // Selection #1: pick the host permanent.
        assert_eq!(
            r.pending_kind(),
            Some(SelectionKind::Target),
            "perm selection installs"
        );
        let host_action = encode_attack(0, host.index as u16);
        assert!(
            r.pending_selection_view()
                .unwrap()
                .valid_action_ids
                .contains(&host_action),
            "host is an eligible app-fuse permanent"
        );
        r.execute_action(0, host_action).expect("pick host");

        // Selection #2: pick the result card in hand.
        assert_eq!(
            r.pending_kind(),
            Some(SelectionKind::Target),
            "result-card selection installs"
        );
        let card_action = HAND_EFFECT_START + hand_index(&r, "TEST-APPFUSE") as u16;
        r.execute_action(0, card_action).expect("pick result card");

        assert_eq!(
            top_id(&r, host),
            "TEST-APPFUSE",
            "result card stacked on top"
        );
        let perm = &r.game.players[0].battle_area[host.index as usize];
        assert!(perm.linked_cards.is_empty(), "linked materials consumed");
        assert!(
            perm.card_sources
                .iter()
                .any(|c| c.card_id(&r.game.card_data) == "TEST-GOMIMON"),
            "consumed link folded under the new top as a digivolution source"
        );
    }

    #[test]
    fn app_fuse_prompts_clone_faithfully() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        let host = fused_host(&mut r);

        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);

        let host_action = encode_attack(0, host.index as u16);
        assert_eq!(r.pending_kind(), Some(SelectionKind::Target));
        assert!(
            r.game.pending_selection_resume.is_some(),
            "App Fuse host prompt must park a data frame"
        );

        let mut clone_at_host = r.game.clone();
        clone_at_host
            .resolve_selection(0, host_action)
            .expect("clone resolves host prompt");
        assert!(
            clone_at_host.pending_selection.is_some(),
            "clone advances to result-card prompt"
        );
        assert!(
            clone_at_host.pending_selection_resume.is_some(),
            "App Fuse result prompt must park a data frame"
        );
        assert!(
            r.game.pending_selection.is_some(),
            "resolving the clone must leave the original at the host prompt"
        );

        r.execute_action(0, host_action)
            .expect("original picks host");
        let card_action = HAND_EFFECT_START + hand_index(&r, "TEST-APPFUSE") as u16;
        assert!(r.game.pending_selection_resume.is_some());

        let mut clone_at_result = r.game.clone();
        clone_at_result
            .resolve_selection(0, card_action)
            .expect("clone resolves result-card prompt");
        assert!(clone_at_result.pending_selection.is_none());
        assert_eq!(
            clone_at_result.players[0].battle_area[host.index as usize]
                .top_card()
                .card_id(&clone_at_result.card_data),
            "TEST-APPFUSE"
        );
        assert!(
            r.game.pending_selection.is_some(),
            "resolving the result clone must leave the original at the result prompt"
        );

        r.execute_action(0, card_action)
            .expect("original picks result card");
        assert_eq!(
            top_id(&r, host),
            clone_at_result.players[0].battle_area[host.index as usize]
                .top_card()
                .card_id(&clone_at_result.card_data)
        );
    }

    // ── 2. Trash happy path ─────────────────────────────────────────────────────

    #[test]
    fn app_fuse_trash_happy_path() {
        let mut r = builder().hand(0, &["DSL-APPFUSE-TRASH"]).start();
        let host = fused_host(&mut r);
        seed_trash(&mut r, 0, "TEST-APPFUSE");
        let trash_before = r.trash_size(0);

        let src_idx = hand_index(&r, "DSL-APPFUSE-TRASH");
        r.play(0, src_idx);

        r.execute_action(0, encode_attack(0, host.index as u16))
            .expect("pick host");
        let trash_pos = r.game.players[0]
            .trash
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE")
            .unwrap();
        r.execute_action(0, TRASH_EFFECT_START + trash_pos as u16)
            .expect("pick result from trash");

        assert_eq!(
            top_id(&r, host),
            "TEST-APPFUSE",
            "result card stacked from trash"
        );
        assert_eq!(r.trash_size(0), trash_before - 1, "result card left trash");
    }

    // ── 3. Result filter ────────────────────────────────────────────────────────

    #[test]
    fn app_fuse_result_filter_excludes_nonmatching_trash_card() {
        let mut r = builder().hand(0, &["DSL-APPFUSE-FILTER"]).start();
        let host = fused_host(&mut r);
        // Both results are route-eligible; only the tagged one has [System].
        seed_trash(&mut r, 0, "TEST-APPFUSE");
        seed_trash(&mut r, 0, "TEST-APPFUSE-TAGGED");

        let src_idx = hand_index(&r, "DSL-APPFUSE-FILTER");
        r.play(0, src_idx);
        r.execute_action(0, encode_attack(0, host.index as u16))
            .expect("pick host");

        // Selection #2 should offer ONLY the [System] result.
        let view = r
            .pending_selection_view()
            .expect("result selection installed");
        let tagged_pos = r.game.players[0]
            .trash
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE-TAGGED")
            .unwrap();
        let plain_pos = r.game.players[0]
            .trash
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "TEST-APPFUSE")
            .unwrap();
        assert!(
            view.valid_action_ids
                .contains(&(TRASH_EFFECT_START + tagged_pos as u16)),
            "the [System] result is offered"
        );
        assert!(
            !view
                .valid_action_ids
                .contains(&(TRASH_EFFECT_START + plain_pos as u16)),
            "the non-[System] result is filtered out"
        );
        super::prompt(&r, &[TRASH_EFFECT_START + tagged_pos as u16], true);
        r.execute_action(0, TRASH_EFFECT_START + tagged_pos as u16)
            .unwrap();
        assert_eq!(top_id(&r, host), "TEST-APPFUSE-TAGGED");
        super::no_prompt(&r);
    }

    // ── 4. Ineligible permanent not offered ─────────────────────────────────────

    #[test]
    fn app_fuse_ineligible_permanent_not_offered() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        let good = fused_host(&mut r);
        // A bare Kabemon (no linked second material) is NOT app-fuse-eligible.
        let bare = r.place_on_field(0, "TEST-KABEMON", Some(0));

        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);

        let view = r.pending_selection_view().expect("perm selection installs");
        assert!(
            view.valid_action_ids
                .contains(&encode_attack(0, good.index as u16)),
            "the fully-materialed host is eligible"
        );
        assert!(
            !view
                .valid_action_ids
                .contains(&encode_attack(0, bare.index as u16)),
            "the bare host (one material only) is NOT eligible"
        );
        super::prompt(&r, &[encode_attack(0, good.index as u16)], true);
        r.execute_action(0, encode_attack(0, good.index as u16))
            .unwrap();
        let action = HAND_EFFECT_START + hand_index(&r, "TEST-APPFUSE") as u16;
        super::prompt(&r, &[action], true);
        r.execute_action(0, action).unwrap();
        assert_eq!(top_id(&r, good), "TEST-APPFUSE");
        assert_eq!(top_id(&r, bare), "TEST-KABEMON");
        super::no_prompt(&r);
    }

    // ── 5. No eligible permanent → silent no-op ─────────────────────────────────

    #[test]
    fn app_fuse_no_eligible_permanent_is_silent_noop() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        // No host with materials on the field at all.
        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);
        assert!(
            r.pending_selection_view().is_none(),
            "no selection installs with no eligible host"
        );
    }

    // ── 6. Decline at permanent pick ────────────────────────────────────────────

    #[test]
    fn app_fuse_decline_at_permanent_pick_does_nothing() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        let host = fused_host(&mut r);
        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);

        assert!(r.pending_is_optional(), "perm selection is optional (may)");
        r.execute_action(0, PASS).expect("decline");
        assert_eq!(
            top_id(&r, host),
            "TEST-KABEMON",
            "host unchanged after decline"
        );
        assert!(r.pending_selection_view().is_none(), "no further selection");
    }

    // ── 7. Decline at card pick ─────────────────────────────────────────────────

    #[test]
    fn app_fuse_decline_at_card_pick_does_nothing() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        let host = fused_host(&mut r);
        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);

        r.execute_action(0, encode_attack(0, host.index as u16))
            .expect("pick host");
        assert!(r.pending_is_optional(), "card selection is optional (may)");
        r.execute_action(0, PASS).expect("decline card");
        assert_eq!(
            top_id(&r, host),
            "TEST-KABEMON",
            "host unchanged after declining the card pick"
        );
    }

    // ── 8. Requires two distinct named materials ────────────────────────────────

    #[test]
    fn app_fuse_two_candidates_and_pass_at_either_pick_preserves_state() {
        for stage in 0..3 {
            let mut r = builder()
                .add_card(named_digimon("WRONG", "Wrong material"))
                .link_requirement("WRONG", 0)
                .hand(0, &["TEST-APPFUSE", "TEST-APPFUSE-TAGGED", "TEST-GOMIMON"])
                .start();
            let a = fused_host(&mut r);
            let b = fused_host(&mut r);
            let wrong = r.place_on_field(0, "TEST-KABEMON", Some(0));
            r.push_linked_owned(wrong, "WRONG", 0);
            r.game.tick_declarative_effects();
            let f = r.place_on_field(0, "DSL-APPFUSE-HAND", Some(0));
            let hand: Vec<_> = r.game.players[0].hand.iter().map(|c| c.handle()).collect();
            let sources: Vec<_> = r.game.players[0].battle_area[a.index as usize]
                .card_sources
                .iter()
                .map(|c| c.handle())
                .collect();
            let links: Vec<_> = r.game.players[0].battle_area[a.index as usize]
                .linked_cards
                .iter()
                .map(|c| c.handle())
                .collect();
            super::fire(&mut r, f);
            super::prompt(
                &r,
                &[
                    encode_attack(0, a.index as u16),
                    encode_attack(0, b.index as u16),
                ],
                true,
            );
            if stage == 0 {
                r.execute_action(0, PASS).unwrap();
            } else {
                r.execute_action(0, encode_attack(0, a.index as u16))
                    .unwrap();
                super::prompt(&r, &[HAND_EFFECT_START, HAND_EFFECT_START + 1], true);
                r.execute_action(
                    0,
                    if stage == 1 {
                        PASS
                    } else {
                        HAND_EFFECT_START + 1
                    },
                )
                .unwrap();
            }
            super::no_prompt(&r);
            assert_eq!(r.game.memory, 5);
            assert_eq!(top_id(&r, b), "TEST-KABEMON");
            assert_eq!(top_id(&r, wrong), "TEST-KABEMON");
            if stage < 2 {
                assert_eq!(
                    r.game.players[0]
                        .hand
                        .iter()
                        .map(|c| c.handle())
                        .collect::<Vec<_>>(),
                    hand
                );
                let host = &r.game.players[0].battle_area[a.index as usize];
                assert_eq!(
                    host.card_sources
                        .iter()
                        .map(|c| c.handle())
                        .collect::<Vec<_>>(),
                    sources
                );
                assert_eq!(
                    host.linked_cards
                        .iter()
                        .map(|c| c.handle())
                        .collect::<Vec<_>>(),
                    links
                );
            } else {
                assert_eq!(top_id(&r, a), "TEST-APPFUSE-TAGGED");
                assert!(r.game.players[0].battle_area[a.index as usize]
                    .linked_cards
                    .is_empty());
            }
        }
    }

    #[test]
    fn app_fuse_requires_two_distinct_named_materials() {
        let mut r = builder()
            .hand(0, &["DSL-APPFUSE-HAND", "TEST-APPFUSE"])
            .start();
        // Host: Kabemon top + Kabemon linked = SAME name twice → not eligible
        // (App Fusion needs two DISTINCT named conditions).
        let host = r.place_on_field(0, "TEST-KABEMON", Some(0));
        r.push_linked_owned(host, "TEST-KABEMON", 0);

        let src_idx = hand_index(&r, "DSL-APPFUSE-HAND");
        r.play(0, src_idx);
        assert!(
            r.pending_selection_view().is_none(),
            "duplicate-name materials do not satisfy App Fusion"
        );
    }
}

mod play_hand {
    use super::*;
    use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
    use digimon_engine::effect::{CardEffect, Effect};
    use digimon_engine::effect_context::EffectContext;
    use std::sync::Arc;
    const HAND: &str = r#"
card: HAND
name: Hand fixture
kind: tamer
color: [red]
cost: 0
effects:
 - when: on_play
   process:
    - select_hand: { of: you, bind_as: picked, prompt: Choose a hand card, optional: true, filter: { trait_has: Target } }
    - play_or_use_from_hand: { of: you, hand_index: picked, cost_delta: { reduce: 3 } }
"#;
    struct OptionMainDraw;

    impl CardEffect for OptionMainDraw {
        fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<Effect> {
            vec![Effect::when_attacking(card)
                .option_main()
                .name("OptionMain draw 1")
                .process(|ctx: &mut EffectContext| {
                    ctx.draw(ctx.player, 1);
                })
                .build()]
        }
    }

    fn digimon(id: &str, cost: u16) -> CardData {
        let mut card = make_test_card(id, id);
        card.card_kind = CardKind::Digimon;
        card.level = Some(4);
        card.dp = Some(4000);
        card.play_cost = cost;
        card
    }

    fn option(id: &str, cost: u16) -> CardData {
        let mut card = make_test_card(id, id);
        card.card_kind = CardKind::Option;
        card.level = None;
        card.dp = None;
        card.play_cost = cost;
        card
    }

    fn dual(id: &str, play_cost: u16, use_cost: u16) -> CardData {
        let mut card = make_test_card(id, id);
        card.card_kind = CardKind::Dual;
        card.play_cost = play_cost;
        card.dual = Some(DualCardData {
            digimon: DualDigimonFace {
                level: 4,
                dp: 4000,
                colors: vec![],
                traits: vec![],
                evo_costs: vec![],
                effect_text: String::new(),
                inherited_text: String::new(),
                keywords: vec![],
            },
            option: DualOptionFace {
                use_cost,
                colors: vec![],
                effect_text: String::new(),
                security_text: String::new(),
                keywords: vec![],
            },
        });
        card
    }

    fn runner(yaml: &str, id: &str) -> DebugRunner {
        let mut b = base().from_dsl_yaml(yaml).unwrap().memory(10);
        for mut c in [
            digimon("H-DIGI", 5),
            option("H-OPT", 5),
            dual("H-DUAL", 5, 5),
        ] {
            c.traits = vec!["Target".into()];
            b = b.add_card(c);
        }
        let mut r = b.hand(0, &[id, "H-DIGI", "PAD"]).start();
        r.register_effect("H-OPT", Arc::new(OptionMainDraw));
        r.register_effect("H-DUAL", Arc::new(OptionMainDraw));
        let h = r.place_on_field(0, "HAND", Some(0));
        fire(&mut r, h);
        prompt(&r, &[PLAY_HAND_START, PLAY_HAND_START + 1], true);
        r
    }
    #[test]
    fn play_or_use_from_hand_bound_digimon_reduced() {
        let mut r = runner(HAND, "H-DIGI");
        pick(&mut r, PLAY_HAND_START);
        assert_eq!(r.game.memory, 8);
        assert_eq!(r.game.players[0].battle_area.len(), 2);
        assert_eq!(
            top(
                &r,
                PermanentHandle {
                    player: 0,
                    index: 1
                }
            ),
            "H-DIGI"
        );
        no_prompt(&r);
    }
    #[test]
    fn play_or_use_from_hand_bound_option_main_and_trash() {
        let mut r = runner(HAND, "H-OPT");
        let deck = r.game.players[0].deck.len();
        pick(&mut r, PLAY_HAND_START);
        assert_eq!(r.game.memory, 8);
        assert_eq!(r.game.players[0].hand.len(), 3);
        assert_eq!(r.game.players[0].deck.len(), deck - 1);
        assert_eq!(
            r.game.players[0].trash[0].card_id(&r.game.card_data),
            "H-OPT"
        );
        no_prompt(&r);
    }
    #[test]
    fn play_or_use_from_hand_missing_delta_full_cost() {
        let yaml = HAND.replace(", cost_delta: { reduce: 3 }", "");
        let mut r = runner(&yaml, "H-DIGI");
        pick(&mut r, PLAY_HAND_START);
        assert_eq!(r.game.memory, 5);
        no_prompt(&r);
    }
    #[test]
    fn play_or_use_from_hand_declined_unchanged() {
        let mut r = runner(HAND, "H-DIGI");
        pick(&mut r, PASS);
        assert_eq!(r.game.memory, 10);
        assert_eq!(r.game.players[0].hand.len(), 3);
        assert_eq!(r.game.players[0].battle_area.len(), 1);
        no_prompt(&r);
    }
    #[test]
    fn play_or_use_from_hand_dual_two_mandatory_faces() {
        for face in 0..2 {
            let mut r = runner(HAND, "H-DUAL");
            pick(&mut r, PLAY_HAND_START);
            prompt(&r, &[HAND_EFFECT_START, HAND_EFFECT_START + 1], false);
            let p = r.game.pending_selection.as_ref().unwrap();
            assert_eq!(p.kind, SelectionKind::EffectChoice);
            let labels: Vec<_> = p
                .effect_choices
                .as_ref()
                .unwrap()
                .iter()
                .map(|e| e.label.as_str())
                .collect();
            assert_eq!(labels, ["Play as Digimon", "Use as Option"]);
            pick(&mut r, HAND_EFFECT_START + face);
            assert_eq!(r.game.memory, 8);
            if face == 0 {
                assert_eq!(r.game.players[0].battle_area.len(), 2);
                assert_eq!(
                    top(
                        &r,
                        PermanentHandle {
                            player: 0,
                            index: 1
                        }
                    ),
                    "H-DUAL"
                );
            } else {
                assert_eq!(r.game.players[0].battle_area.len(), 1);
                assert_eq!(
                    r.game.players[0].trash[0].card_id(&r.game.card_data),
                    "H-DUAL"
                );
                assert_eq!(r.game.players[0].hand.len(), 3);
            }
            no_prompt(&r);
        }
    }
}
const ROUTE: &str = r#"
card: ROUTE
name: Route fixture
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
 - when: on_play
   process:
    - select_hand: { of: you, bind_as: picked, prompt: Choose a hand card, optional: true, filter: { can_digivolve_from_source: true } }
"#;
#[test]
fn can_digivolve_from_source_routes_inversion_and_prohibition() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::ModifierEntry;
    for mode in 0..3 {
        let yaml = if mode == 1 {
            ROUTE.replace(
                "can_digivolve_from_source: true",
                "can_digivolve_from_source: false",
            )
        } else {
            ROUTE.to_owned()
        };
        let mut ok = card("ROUTE-OK", CardColor::Red, 4);
        ok.evo_costs = vec![digimon_engine::card_data::EvoCost {
            card_color: CardColor::Red as u8,
            level: 3,
            memory_cost: 2,
        }];
        let mut bad = ok.clone();
        bad.card_id = "ROUTE-BAD".into();
        bad.evo_costs[0].card_color = CardColor::Blue as u8;
        let mut r = base()
            .from_dsl_yaml(&yaml)
            .unwrap()
            .add_card(ok)
            .add_card(bad)
            .hand(0, &["ROUTE-OK", "ROUTE-BAD", "PAD"])
            .start();
        let h = r.place_on_field(0, "ROUTE", Some(0));
        if mode == 2 {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
            );
        }
        fire(&mut r, h);
        if mode == 2 {
            no_prompt(&r);
        } else {
            let expected = if mode == 0 {
                vec![PLAY_HAND_START]
            } else {
                vec![PLAY_HAND_START + 1, PLAY_HAND_START + 2]
            };
            prompt(&r, &expected, true);
            pick(&mut r, PASS);
            no_prompt(&r);
        }
    }
}

mod links {
    use super::*;
    use digimon_engine::enums::LinkCardSource;
    const LINKER: &str = r#"
card: LINKER
name: Linker
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
traits: [Social]
effects:
 - kind: link_condition
   cost: 2
   filter: { trait_has: LinkHost }
 - scope: linked
   when: when_linked
   process:
    - draw: { of: you, count: 1 }
"#;
    const HOST: &str = r#"
card: HOST
name: Host
kind: digimon
level: 4
color: [green]
cost: 4
dp: 4000
traits: [LinkHost]
effects:
 - kind: link_condition
   cost: 0
   filter: { trait_has: LinkHost }
 - when: when_card_linked_to_this
   process:
    - gain_memory: 3
"#;
    const ANY: &str = r#"
card: ANY
name: Any Link
kind: tamer
color: [green]
cost: 0
effects:
 - when: on_any_link
   trigger_if: { event_target_owner: you }
   process:
    - draw: { of: you, count: 1 }
"#;
    const WINDOW: &str = r#"
card: WINDOW
name: Would Link Host
kind: digimon
level: 4
color: [green]
cost: 4
dp: 4000
traits: [LinkHost]
effects:
 - when: when_would_link_to_this
   optional: true
   active_when: { would_link_card_trait_any_of: [Social, Tool] }
   process:
    - reduce_link_cost: { amount: 1 }
"#;
    const OUTSIDE: &str = r#"
card: OUTSIDE
name: Outside
kind: tamer
color: [green]
cost: 0
effects:
 - when: on_play
   process:
    - reduce_link_cost: { amount: 1 }
 - when: when_digivolving
   condition: { would_link_card_trait_any_of: [Social] }
   process:
    - gain_memory: 7
"#;
    const COUNT: &str = r#"
card: COUNT
name: Link count
kind: tamer
color: [green]
cost: 0
effects:
 - when: on_play
   process:
    - select_own_permanent:
        bind_as: picked
        prompt: Choose a linked host
        filter: { kind: digimon, link_card_count: { filter: { trait_has: Social }, at_least: 2 } }
"#;
    fn builder() -> DebugRunnerBuilder {
        let mut c = card("PLAIN", CardColor::Green, 4);
        c.traits = vec!["LinkHost".into()];
        let sibling = LINKER
            .split(" - scope: linked")
            .next()
            .unwrap()
            .replace("card: LINKER", "card: SIBLING")
            .replace("traits: [Social]", "traits: [Beast]");
        base()
            .add_card(c)
            .from_dsl_yaml(LINKER)
            .unwrap()
            .from_dsl_yaml(&sibling)
            .unwrap()
            .from_dsl_yaml(HOST)
            .unwrap()
            .from_dsl_yaml(ANY)
            .unwrap()
    }
    fn start_link(
        r: &mut DebugRunner,
        src: PermanentHandle,
        hosts: &[PermanentHandle],
        chosen: PermanentHandle,
    ) {
        r.game.enter_main_phase();
        let a = FIELD_EFFECT_START
            + src.index as u16 * EFFECTS_PER_PERMANENT
            + FIELD_EFFECT_SLOT_FOR_LINK;
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        let actions: Vec<_> = hosts
            .iter()
            .map(|h| encode_attack(0, h.index as u16))
            .collect();
        // Link host choice carries a cancel path.
        let optional = r.game.pending_selection.as_ref().unwrap().is_optional;
        prompt(r, &actions, optional);
        pick(r, encode_attack(0, chosen.index as u16));
    }
    fn effect_link(r: &mut DebugRunner, host: PermanentHandle) {
        let c = r.game.players[host.player as usize].hand[0].handle();
        assert!(r
            .game
            .link_chosen_card_into_host(host, c, LinkCardSource::Hand(host.player)));
        r.game.drain_effect_queue();
    }
    #[test]
    fn when_linked_once_not_on_play_or_sibling() {
        let mut r = builder().start();
        let h = r.place_on_field(0, "PLAIN", Some(0));
        r.give_link_plus(h, 1);
        let l = r.place_on_field(0, "LINKER", Some(0));
        fire(&mut r, l);
        assert_eq!(r.game.players[0].hand.len(), 0);
        start_link(&mut r, l, &[h], h);
        assert_eq!(r.game.players[0].hand.len(), 1);
        assert_eq!(r.game.memory, 3);
        no_prompt(&r);
        let l = r.place_on_field(0, "SIBLING", Some(0));
        start_link(&mut r, l, &[h], h);
        assert_eq!(r.game.players[0].hand.len(), 1);
        assert_eq!(r.game.players[0].battle_area[0].linked_cards.len(), 2);
        no_prompt(&r);
    }
    #[test]
    fn when_linked_effect_api_fires() {
        let mut r = builder().hand(0, &["LINKER"]).start();
        let h = r.place_on_field(0, "PLAIN", Some(0));
        effect_link(&mut r, h);
        assert_eq!(r.game.players[0].hand.len(), 1);
        assert_eq!(r.game.players[0].deck.len(), 19);
        assert_eq!(r.game.memory, 5);
        no_prompt(&r);
    }
    #[test]
    fn when_card_linked_to_this_host_only_and_not_on_play() {
        let mut r = builder().start();
        let a = r.place_on_field(0, "HOST", Some(0));
        let b = r.place_on_field(0, "HOST", Some(0));
        fire(&mut r, a);
        assert_eq!(r.game.memory, 5);
        let l = r.place_on_field(0, "SIBLING", Some(0));
        start_link(&mut r, l, &[a, b], a);
        assert_eq!(r.game.memory, 6);
        let l = r.place_on_field(0, "SIBLING", Some(0));
        start_link(&mut r, l, &[a, b], b);
        assert_eq!(r.game.memory, 7);
        no_prompt(&r);
    }
    #[test]
    fn when_card_linked_to_this_not_when_it_becomes_link() {
        let mut r = builder().start();
        let b = r.place_on_field(0, "HOST", Some(0));
        let a = r.place_on_field(0, "HOST", Some(0));
        start_link(&mut r, a, &[b], b);
        assert_eq!(r.game.memory, 8);
        no_prompt(&r);
    }
    #[test]
    fn on_any_link_own_owner_filter_and_opponent_exclusion() {
        for owner in [0, 1] {
            let mut r = builder().hand(owner, &["SIBLING"]).start();
            r.place_on_field(0, "ANY", Some(0));
            let h = r.place_on_field(owner, "PLAIN", Some(0));
            effect_link(&mut r, h);
            assert_eq!(r.game.players[0].hand.len(), if owner == 0 { 1 } else { 0 });
            no_prompt(&r);
        }
    }
    #[test]
    fn on_any_link_raw_sees_both_sides() {
        let raw = ANY
            .replace("   trigger_if: { event_target_owner: you }\n", "")
            .replace("draw: { of: you, count: 1 }", "gain_memory: 1");
        for (observer, owner) in [(0, 1), (1, 0)] {
            let mut r = builder()
                .from_dsl_yaml(&raw)
                .unwrap()
                .hand(owner, &["SIBLING"])
                .start();
            r.place_on_field(observer, "ANY", Some(0));
            let h = r.place_on_field(owner, "PLAIN", Some(0));
            effect_link(&mut r, h);
            assert_eq!(r.game.memory, if observer == 0 { 6 } else { 4 });
            no_prompt(&r);
        }
    }
    #[test]
    fn on_any_link_not_play_or_digivolve() {
        let mut evo = card("NEXT", CardColor::Green, 5);
        evo.evo_costs = vec![digimon_engine::card_data::EvoCost {
            card_color: CardColor::Green as u8,
            level: 4,
            memory_cost: 1,
        }];
        let mut r = builder().add_card(evo).hand(0, &["NEXT"]).start();
        r.place_on_field(0, "ANY", Some(0));
        let h = r.place_on_field(0, "PLAIN", Some(0));
        fire(&mut r, h);
        assert_eq!(r.game.players[0].hand.len(), 1);
        let before = r.game.players[0].deck.len();
        r.game.enter_main_phase();
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.game.decode_action(action, 0);
        assert_eq!(top(&r, h), "NEXT");
        assert_eq!(r.game.players[0].deck.len(), before - 1);
        assert_eq!(r.game.players[0].hand.len(), 1);
        assert_eq!(r.game.memory, 4);
        no_prompt(&r);
    }
    fn window_runner(
        yaml: &str,
        trait_name: &str,
    ) -> (DebugRunner, PermanentHandle, PermanentHandle) {
        let linker = LINKER
            .split(" - scope: linked")
            .next()
            .unwrap()
            .replace("traits: [Social]", &format!("traits: [{trait_name}]"))
            .replace("card: LINKER", "card: WINDOW-LINKER");
        let mut r = builder()
            .from_dsl_yaml(yaml)
            .unwrap()
            .from_dsl_yaml(&linker)
            .unwrap()
            .start();
        let h = r.place_on_field(0, "WINDOW", Some(0));
        let l = r.place_on_field(0, "WINDOW-LINKER", Some(0));
        (r, h, l)
    }
    #[test]
    fn would_link_window_before_payment_accept_decline_and_floor() {
        for (amount, accept, expected) in [(1, true, 4), (1, false, 3), (5, true, 5)] {
            let yaml = WINDOW.replace("amount: 1", &format!("amount: {amount}"));
            let (mut r, h, l) = window_runner(&yaml, "Social");
            start_link(&mut r, l, &[h], h);
            assert_eq!(r.game.memory, 5);
            assert!(r.game.players[0].battle_area[0].linked_cards.is_empty());
            prompt(&r, &[REPLACEMENT_ACCEPT], true);
            assert_eq!(
                r.game.pending_selection.as_ref().unwrap().source_card,
                r.game.players[0].battle_area[h.index as usize]
                    .top_card()
                    .handle()
            );
            pick(&mut r, if accept { REPLACEMENT_ACCEPT } else { PASS });
            assert_eq!(r.game.memory, expected);
            assert_eq!(r.game.players[0].battle_area[0].linked_cards.len(), 1);
            no_prompt(&r);
        }
    }
    #[test]
    fn would_link_trait_any_of_and_case_sensitive() {
        for (trait_name, eligible) in [
            ("Social", true),
            ("Tool", true),
            ("Beast", false),
            ("social", false),
        ] {
            let (mut r, h, l) = window_runner(WINDOW, trait_name);
            start_link(&mut r, l, &[h], h);
            if eligible {
                prompt(&r, &[REPLACEMENT_ACCEPT], true);
                pick(&mut r, REPLACEMENT_ACCEPT);
                assert_eq!(r.game.memory, 4);
            } else {
                assert_eq!(r.game.memory, 3);
            }
            no_prompt(&r);
        }
    }
    #[test]
    fn would_link_different_host_no_prompt() {
        let (mut r, h, l) = window_runner(WINDOW, "Social");
        let other = r.place_on_field(0, "PLAIN", Some(0));
        start_link(&mut r, l, &[h, other], other);
        assert_eq!(r.game.memory, 3);
        assert!(r.game.players[0].battle_area[h.index as usize]
            .linked_cards
            .is_empty());
        no_prompt(&r);
    }
    #[test]
    fn reduce_link_cost_no_leak_next_link() {
        let (mut r, h, l) = window_runner(WINDOW, "Social");
        r.give_link_plus(h, 1);
        start_link(&mut r, l, &[h], h);
        pick(&mut r, REPLACEMENT_ACCEPT);
        let l = r.place_on_field(0, "SIBLING", Some(0));
        start_link(&mut r, l, &[h], h);
        assert_eq!(r.game.memory, 2);
        assert_eq!(r.game.players[0].battle_area[0].linked_cards.len(), 2);
        no_prompt(&r);
    }
    #[test]
    fn reduce_link_cost_and_trait_predicate_outside_window_noop() {
        let mut r = builder().from_dsl_yaml(OUTSIDE).unwrap().start();
        let f = r.place_on_field(0, "OUTSIDE", Some(0));
        fire(&mut r, f);
        r.game
            .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(f));
        r.game.drain_effect_queue();
        assert_eq!(r.game.memory, 5);
        let h = r.place_on_field(0, "PLAIN", Some(0));
        let l = r.place_on_field(0, "SIBLING", Some(0));
        start_link(&mut r, l, &[h], h);
        assert_eq!(r.game.memory, 3);
        no_prompt(&r);
    }
    #[test]
    fn link_card_count_filtered_empty_filter_and_unreachable() {
        for mode in 0..3 {
            let yaml = match mode {
                1 => COUNT.replace("at_least: 2", "at_least: 3"),
                2 => COUNT.replace("filter: { trait_has: Social }", "filter: {}"),
                _ => COUNT.to_owned(),
            };
            let mut r = builder().from_dsl_yaml(&yaml).unwrap().start();
            let h2 = r.place_on_field(0, "PLAIN", Some(0));
            let h1 = r.place_on_field(0, "PLAIN", Some(0));
            r.place_on_field(0, "PLAIN", Some(0));
            for h in [h2, h1] {
                r.give_link_plus(h, 2);
                r.push_linked_owned(h, "LINKER", 0);
            }
            r.push_linked_owned(h2, "LINKER", 0);
            r.push_linked_owned(h1, "SIBLING", 0);
            r.game.tick_declarative_effects();
            let f = r.place_on_field(0, "COUNT", Some(0));
            fire(&mut r, f);
            if mode == 1 {
                no_prompt(&r);
            } else {
                let expected = if mode == 0 {
                    vec![encode_attack(0, h2.index as u16)]
                } else {
                    vec![
                        encode_attack(0, h2.index as u16),
                        encode_attack(0, h1.index as u16),
                    ]
                };
                prompt(&r, &expected, false);
                pick(&mut r, expected[0]);
                no_prompt(&r);
            }
        }
    }
}

mod reducers {
    use super::*;
    const ARM: &str = r#"
card: ARM
name: Armed reducer
kind: tamer
color: [green]
cost: 0
effects:
 - when: on_play
   process:
    - arm_digivolve_cost_reducer: { amount: 5, single_fire: true, target_color: green, suspend_cost: true }
"#;
    const G4: &str = r#"
card: G4
name: Green four
kind: digimon
level: 4
color: [green]
cost: 6
dp: 5000
alt_paths:
 - kind: digivolve
   from: { level_eq: 3 }
   cost: 6
"#;
    fn runner(
        yaml: &str,
    ) -> (
        DebugRunner,
        PermanentHandle,
        PermanentHandle,
        PermanentHandle,
    ) {
        let mut r = base()
            .memory(10)
            .from_dsl_yaml(yaml)
            .unwrap()
            .from_dsl_yaml(G4)
            .unwrap()
            .add_card(card("GREEN3", CardColor::Green, 3))
            .add_card(card("RED3", CardColor::Red, 3))
            .hand(0, &["G4", "G4"])
            .start();
        let g = r.place_on_field(0, "GREEN3", Some(0));
        let red = r.place_on_field(0, "RED3", Some(0));
        let other = r.place_on_field(0, "GREEN3", Some(0));
        let a = r.place_on_field(0, "ARM", Some(0));
        fire(&mut r, a);
        assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
        r.game.enter_main_phase();
        (r, g, red, other)
    }
    fn evolve(r: &mut DebugRunner, h: PermanentHandle) {
        let a = encode_digivolve(0, h.index as u16);
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
    }
    #[test]
    fn arm_digivolve_cost_reducer_optional_then_mandatory_suspend() {
        let (mut r, g, red, other) = runner(ARM);
        let sleeping = r.place_on_field(0, "GREEN3", Some(0));
        r.game.players[0].battle_area[sleeping.index as usize].is_suspended = true;
        r.place_on_field(1, "GREEN3", Some(0));
        evolve(&mut r, g);
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().kind,
            SelectionKind::EffectChoice
        );
        prompt(&r, &[HAND_EFFECT_START], true);
        pick(&mut r, HAND_EFFECT_START);
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().kind,
            SelectionKind::OwnField
        );
        prompt(
            &r,
            &[
                encode_attack(0, g.index as u16),
                encode_attack(0, red.index as u16),
                encode_attack(0, other.index as u16),
            ],
            false,
        );
        pick(&mut r, encode_attack(0, other.index as u16));
        assert!(r.game.players[0].battle_area[other.index as usize].is_suspended);
        assert_eq!(top(&r, g), "G4");
        assert_eq!(r.game.memory, 9);
        assert!(r.game.player_digivolve_cost_reducers.is_empty());
        no_prompt(&r);
    }
    #[test]
    fn arm_digivolve_cost_reducer_wrong_color_retains() {
        let (mut r, _, red, _) = runner(ARM);
        evolve(&mut r, red);
        assert_eq!(r.game.memory, 4);
        assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
        no_prompt(&r);
    }
    #[test]
    fn arm_digivolve_cost_reducer_decline_consumes_single_fire() {
        let (mut r, g, _, other) = runner(ARM);
        evolve(&mut r, g);
        prompt(&r, &[HAND_EFFECT_START], true);
        pick(&mut r, PASS);
        assert_eq!(r.game.memory, 4);
        assert!(r.game.player_digivolve_cost_reducers.is_empty());
        evolve(&mut r, other);
        assert_eq!(top(&r, other), "G4");
        no_prompt(&r);
    }
    #[test]
    fn arm_digivolve_cost_reducer_end_turn_expiry() {
        let (mut r, _, _, _) = runner(ARM);
        r.pass_turn();
        assert!(r.game.player_digivolve_cost_reducers.is_empty());
    }
    #[test]
    fn arm_digivolve_cost_reducer_no_suspend_silent_and_multi_fire() {
        for single in [true, false] {
            let yaml = ARM
                .replace("amount: 5", "amount: 1")
                .replace(", target_color: green, suspend_cost: true", "")
                .replace("single_fire: true", &format!("single_fire: {single}"));
            let (mut r, g, _, other) = runner(&yaml);
            evolve(&mut r, g);
            assert_eq!(r.game.memory, 5);
            no_prompt(&r);
            assert_eq!(
                r.game.player_digivolve_cost_reducers.len(),
                usize::from(!single)
            );
            if !single {
                evolve(&mut r, other);
                assert_eq!(r.game.memory, 0);
                assert_eq!(r.game.player_digivolve_cost_reducers.len(), 1);
                no_prompt(&r);
            }
        }
    }
}
