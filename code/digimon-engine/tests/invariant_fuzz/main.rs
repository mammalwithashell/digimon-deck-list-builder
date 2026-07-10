//! Tier-2 seeded random-game invariant smoke.
//!
//! Repro example:
//! `INVARIANT_FUZZ_SEED=202607080001 INVARIANT_FUZZ_STOP_AFTER_STEP=42 cargo test --test invariant_fuzz -- --nocapture`

use std::collections::{HashMap, HashSet};
use std::panic::{self, AssertUnwindSafe};

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::CONCEDE_GAME;
use digimon_engine::card_data::CardData;
use digimon_engine::cards::build_registry_cached;
use digimon_engine::enums::PlayerId;
use digimon_engine::game::Game;
use digimon_engine::runners::HeadlessRunner;

#[derive(Clone, Debug)]
struct DeckCandidate {
    label: String,
    deck: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
struct FuzzConfig {
    games: u64,
    max_steps: u32,
    base_seed: u64,
    single_seed: Option<u64>,
    stop_after_step: Option<u32>,
}

impl FuzzConfig {
    fn from_env() -> Self {
        let single_seed = env_u64("INVARIANT_FUZZ_SEED");
        Self {
            games: if single_seed.is_some() {
                1
            } else {
                env_u64("INVARIANT_FUZZ_GAMES").unwrap_or(4)
            },
            max_steps: env_u32("INVARIANT_FUZZ_MAX_STEPS").unwrap_or(120),
            base_seed: env_u64("INVARIANT_FUZZ_BASE_SEED").unwrap_or(20_260_708_000),
            single_seed,
            stop_after_step: env_u32("INVARIANT_FUZZ_STOP_AFTER_STEP"),
        }
    }

    fn seed_for_game(self, game_idx: u64) -> u64 {
        self.single_seed
            .unwrap_or_else(|| self.base_seed.wrapping_add(game_idx))
    }
}

fn env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|raw| raw.parse().ok())
}

fn env_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok().and_then(|raw| raw.parse().ok())
}

fn load_card_data() -> HashMap<String, CardData> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json");
    let text = std::fs::read_to_string(path).expect("read data/cards.json");
    CardData::load_from_str(&text).expect("parse data/cards.json")
}

fn load_implemented_decks(
    card_data: &HashMap<String, CardData>,
    implemented: &HashSet<String>,
) -> Vec<DeckCandidate> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/deck_library.json");
    let text = std::fs::read_to_string(path).expect("read data/deck_library.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse deck_library.json");
    let archetypes = value
        .get("archetypes")
        .and_then(serde_json::Value::as_object)
        .expect("deck_library.json has archetypes object");

    let mut decks = Vec::new();
    for (archetype_name, archetype) in archetypes {
        let Some(decklists) = archetype
            .get("decklists")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for decklist in decklists {
            let deck_id = decklist
                .get("deck_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown");
            let Some(deck) = parse_decklist(decklist.get("decklist")) else {
                continue;
            };
            if deck
                .iter()
                .all(|id| card_data.contains_key(id) && implemented.contains(id))
            {
                decks.push(DeckCandidate {
                    label: format!("{archetype_name}/{deck_id}"),
                    deck,
                });
            }
        }
    }
    decks.sort_by(|a, b| a.label.cmp(&b.label));
    decks
}

fn parse_decklist(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    match value? {
        serde_json::Value::String(raw) => serde_json::from_str(raw).ok(),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => None,
    }
}

/// Deterministic LCG so a reported seed reproduces deck choice and policy.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn usize(&mut self, upper: usize) -> usize {
        assert!(upper > 0, "LCG upper bound must be non-zero");
        (self.next() as usize) % upper
    }

    fn action(&mut self, mask: &[f32]) -> Option<u16> {
        let mut legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter(|(_, value)| **value > 0.5)
            .map(|(idx, _)| idx as u16)
            .collect();
        if legal.len() > 1 {
            legal.retain(|action| *action != CONCEDE_GAME);
        }
        if legal.is_empty() {
            return None;
        }
        Some(legal[self.usize(legal.len())])
    }
}

fn decision_player(game: &Game) -> PlayerId {
    if let Some(player) = game.mulligan_current_player() {
        return player;
    }
    if let Some(selection) = game.pending_selection.as_ref() {
        return selection.selecting_player;
    }
    game.turn_player()
}

fn choose_decks<'a>(
    seed: u64,
    decks: &'a [DeckCandidate],
) -> (&'a DeckCandidate, &'a DeckCandidate, u64) {
    let mut rng = Lcg(seed ^ 0xA5A5_5A5A_C3C3_3C3C);
    let first = rng.usize(decks.len());
    let mut second = rng.usize(decks.len());
    if decks.len() > 1 && second == first {
        second = (second + 1) % decks.len();
    }
    let game_seed = seed ^ rng.next().rotate_left(17);
    (&decks[first], &decks[second], game_seed)
}

fn assert_pending_resolvable(game: &Game, seed: u64, step: u32) {
    let Some(selection) = game.pending_selection.as_ref() else {
        return;
    };
    assert!(
        !selection.valid_action_ids.is_empty(),
        "seed {seed} step {step}: pending selection has no valid_action_ids\n{}",
        repro_hint(seed, step)
    );
    let mask = action_mask_with_context(game, selection.selecting_player, seed, step);
    let masked_actions: Vec<u16> = selection
        .valid_action_ids
        .iter()
        .copied()
        .filter(|action| {
            let idx = *action as usize;
            idx < mask.len() && mask[idx] > 0.5
        })
        .collect();
    assert!(
        !masked_actions.is_empty(),
        "seed {seed} step {step}: pending selection {:?} is stranded; valid_action_ids={:?}\n{}",
        selection.kind,
        selection.valid_action_ids,
        repro_hint(seed, step)
    );
}

fn action_mask_with_context(game: &Game, player: PlayerId, seed: u64, step: u32) -> Vec<f32> {
    panic::catch_unwind(AssertUnwindSafe(|| build_action_mask(game, player))).unwrap_or_else(|_| {
        panic!(
            "seed {seed} step {step}: build_action_mask panicked for player {player} phase {:?}\n{}",
            game.current_phase,
            repro_hint(seed, step)
        )
    })
}

fn assert_game_bounds(game: &Game, seed: u64, step: u32) {
    assert!(
        (-100..=100).contains(&game.memory),
        "seed {seed} step {step}: memory {} outside the invariant-fuzz runaway bound\n{}",
        game.memory,
        repro_hint(seed, step)
    );
    assert!(
        game.turn_count <= game.rules.max_turns + 1,
        "seed {seed} step {step}: turn_count {} exceeded max {}\n{}",
        game.turn_count,
        game.rules.max_turns,
        repro_hint(seed, step)
    );
    for player_id in 0..game.rules.player_count {
        let player = game.player(player_id);
        assert!(
            player.battle_area.len() <= game.rules.field_slots as usize,
            "seed {seed} step {step}: player {player_id} has {} battle slots, max {}\n{}",
            player.battle_area.len(),
            game.rules.field_slots,
            repro_hint(seed, step)
        );
        let field_sources: usize = player
            .battle_area
            .iter()
            .map(|permanent| permanent.card_sources.len())
            .sum();
        let breeding_sources = player
            .breeding_area
            .as_ref()
            .map(|permanent| permanent.card_sources.len())
            .unwrap_or(0);
        let total_known_cards = player.hand.len()
            + player.deck.len()
            + player.digitama_deck.len()
            + player.security.len()
            + player.trash.len()
            + field_sources
            + breeding_sources;
        assert!(
            total_known_cards <= 240,
            "seed {seed} step {step}: player {player_id} has implausible zone count {total_known_cards}\n{}",
            repro_hint(seed, step)
        );
    }
}

fn apply_action_with_context(game: &mut Game, action: u16, player: PlayerId, seed: u64, step: u32) {
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        game.decode_action(action, player);
    }));
    if result.is_err() {
        panic!(
            "seed {seed} step {step}: decode_action panicked for player {player}, action {action}\n{}",
            repro_hint(seed, step)
        );
    }
}

fn debug_step(game: &Game, seed: u64, step: u32, player: PlayerId, action: u16) {
    let pending = game
        .pending_selection
        .as_ref()
        .map(|selection| {
            format!(
                "kind={:?} player={} valid={:?} prompt={:?}",
                selection.kind,
                selection.selecting_player,
                selection.valid_action_ids,
                selection.prompt
            )
        })
        .unwrap_or_else(|| "none".to_string());
    eprintln!(
        "invariant_fuzz debug: seed={seed} step={step} player={player} action={action} phase={:?} turn={} memory={} pending={pending} digest={}",
        game.current_phase,
        game.turn_count,
        game.memory,
        game.verification_digest()
    );
}

fn repro_hint(seed: u64, step: u32) -> String {
    format!(
        "repro: set INVARIANT_FUZZ_SEED={seed} and INVARIANT_FUZZ_STOP_AFTER_STEP={step}, then run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test invariant_fuzz -- --nocapture`"
    )
}

fn fuzz_one_game(
    seed: u64,
    player1: &DeckCandidate,
    player2: &DeckCandidate,
    game_seed: u64,
    card_data: &HashMap<String, CardData>,
    config: FuzzConfig,
) -> u32 {
    let runner = HeadlessRunner::new(
        player1.deck.clone(),
        player2.deck.clone(),
        card_data,
        false,
        false,
        false,
        Some(game_seed),
    )
    .unwrap_or_else(|err| {
        panic!(
            "seed {seed}: failed to build HeadlessRunner for {} vs {}: {err}",
            player1.label, player2.label
        )
    });
    let mut game = runner.game;
    let mut policy = Lcg(seed ^ 0x9E37_79B9_7F4A_7C15);
    let mut steps = 0;

    while !game.game_over && steps < config.max_steps {
        assert_game_bounds(&game, seed, steps);
        assert_pending_resolvable(&game, seed, steps);

        let player = decision_player(&game);
        let mask = action_mask_with_context(&game, player, seed, steps);
        let action = policy.action(&mask).unwrap_or_else(|| {
            panic!(
                "seed {seed} step {steps}: empty action mask for player {player} phase {:?}\n{}",
                game.current_phase,
                repro_hint(seed, steps)
            )
        });
        assert!(
            (action as usize) < mask.len() && mask[action as usize] > 0.5,
            "seed {seed} step {steps}: policy chose unmasked action {action} for player {player}\n{}",
            repro_hint(seed, steps)
        );

        if config.stop_after_step == Some(steps) {
            debug_step(&game, seed, steps, player, action);
        }

        let before_digest = game.verification_digest();
        let mut clone =
            panic::catch_unwind(AssertUnwindSafe(|| game.clone())).unwrap_or_else(|_| {
                panic!(
                    "seed {seed} step {steps}: Game::clone panicked before action {action}\n{}",
                    repro_hint(seed, steps)
                )
            });

        apply_action_with_context(&mut game, action, player, seed, steps);
        apply_action_with_context(&mut clone, action, player, seed, steps);

        let original_digest = game.verification_digest();
        let clone_digest = clone.verification_digest();
        assert_eq!(
            original_digest,
            clone_digest,
            "seed {seed} step {steps}: clone digest diverged after player {player} action {action}\n{}",
            repro_hint(seed, steps)
        );
        assert!(
            game.game_over || original_digest != before_digest,
            "seed {seed} step {steps}: legal action {action} for player {player} made no digest progress\n{}",
            repro_hint(seed, steps)
        );

        assert_game_bounds(&game, seed, steps);
        assert_pending_resolvable(&game, seed, steps);
        steps += 1;
    }

    steps
}

#[test]
fn seeded_random_games_preserve_engine_invariants() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            let config = FuzzConfig::from_env();
            let card_data = load_card_data();
            let implemented: HashSet<String> = build_registry_cached()
                .registered_card_ids()
                .into_iter()
                .collect();
            let decks = load_implemented_decks(&card_data, &implemented);
            assert!(
                decks.len() >= 2,
                "invariant_fuzz needs at least two fully implemented deck_library decks, found {}",
                decks.len()
            );

            let mut total_steps = 0;
            for game_idx in 0..config.games {
                let seed = config.seed_for_game(game_idx);
                let (player1, player2, game_seed) = choose_decks(seed, &decks);
                let steps = fuzz_one_game(seed, player1, player2, game_seed, &card_data, config);
                total_steps += steps;
                eprintln!(
                    "invariant_fuzz game {game_idx}: seed={seed} game_seed={game_seed} steps={steps} decks={} vs {}",
                    player1.label, player2.label
                );
            }

            assert!(
                total_steps >= config.games as u32,
                "invariant_fuzz made no meaningful progress across {} games",
                config.games
            );
            eprintln!(
                "invariant_fuzz: {} game(s), {total_steps} random legal step(s), max_steps={}",
                config.games, config.max_steps
            );
        })
        .expect("spawn invariant fuzz thread")
        .join()
        .expect("invariant fuzz thread panicked");
}
