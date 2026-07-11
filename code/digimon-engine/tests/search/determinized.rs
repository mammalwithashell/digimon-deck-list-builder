//! Tasks 2.3 + 2.5 — determinized aggregation (PIMC / IS-MCTS) and the
//! no-X-ray guard.
//!
//! The strong form of the no-X-ray invariant: `determinized_search` is a
//! function of the VIEWER'S INFOSET (+ seeds), not of the true hidden
//! arrangement. Permuting the opponent's concealed cards hidden-to-hidden
//! must produce a bitwise-identical `SearchResult`; a public change must
//! not.

use std::collections::HashMap;

use digimon_engine::action::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, PlayerId};
use digimon_engine::game::Game;
use digimon_engine::player::OriginalDeckCardCount;
use digimon_engine::runners::HeadlessRunner;
use digimon_engine::search::{
    determinized_search, DeterminizationMode, DeterminizedConfig, EvalRequest, Evaluation,
    PolicyValueFn, SearchConfig, SearchResult, UniformEvaluator,
};

use crate::support::{
    advance_random, decision_player, legal_actions, load_card_data, make_digimon, run_big_stack,
    Lcg, ST1_DECK, ST5_DECK,
};

fn owned(deck: &[&str]) -> Vec<String> {
    deck.iter().map(|s| s.to_string()).collect()
}

/// Real-deck mid-game state rooted at a viewer decision (the materialization
/// contract: never root at an opponent-owned pending selection).
fn midgame_state(
    card_data: &HashMap<String, digimon_engine::card_data::CardData>,
    seed: u64,
    decisions: u32,
) -> (Game, PlayerId) {
    let runner = HeadlessRunner::new(
        owned(ST1_DECK),
        owned(ST5_DECK),
        card_data,
        false,
        false,
        false,
        Some(seed),
    )
    .expect("build headless runner");
    let mut game = runner.game;
    let mut lcg = Lcg(seed ^ 0x9E3779B97F4A7C15);
    advance_random(&mut game, &mut lcg, decisions);
    assert!(!game.game_over, "mid-game fixture ended too early");
    let viewer = decision_player(&game);
    (game, viewer)
}

fn pimc_config(simulations: u32, worlds: u32, seed: u64) -> DeterminizedConfig {
    DeterminizedConfig {
        worlds,
        mode: DeterminizationMode::Pimc,
        search: SearchConfig {
            simulations,
            seed,
            ..SearchConfig::default()
        },
    }
}

fn ismcts_config(simulations: u32, seed: u64) -> DeterminizedConfig {
    DeterminizedConfig {
        worlds: 1, // ignored by IsMcts (re-determinizes per iteration)
        mode: DeterminizationMode::IsMcts,
        search: SearchConfig {
            simulations,
            seed,
            ..SearchConfig::default()
        },
    }
}

fn most_visited(result: &SearchResult) -> u16 {
    result
        .root_visits
        .iter()
        .max_by_key(|(_, v)| *v)
        .expect("non-empty root visits")
        .0
}

/// The forced-win board from `perfect_information.rs`: turn player has an
/// unsuspended attacker vs an empty security; PASS loses to the opponent's
/// counter-attack. Hidden zones (decks) exist, so determinization has
/// something to re-deal.
///
/// DebugRunner boards are staged outside the normal deal, so
/// `Player::original_deck` (which the Phase 1 invariant checker audits copy
/// limits against) starts empty — declare the fixture's decklist explicitly.
fn forced_win_board() -> (
    DebugRunner,
    HashMap<String, digimon_engine::card_data::CardData>,
) {
    let mut cards = HashMap::new();
    cards.insert("ATK".to_string(), make_digimon("ATK", CardColor::Red, 5000));
    cards.insert(
        "DEF".to_string(),
        make_digimon("DEF", CardColor::Blue, 3000),
    );
    let mut r = DebugRunner::builder()
        .with_card_data(cards.clone())
        .deck(0, &["DEF", "DEF", "DEF", "DEF"])
        .deck(1, &["DEF", "DEF", "DEF", "DEF"])
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    r.place_on_field(tp, "ATK", Some(0));
    r.place_on_field(opp, "DEF", Some(0));
    r.game.enter_main_phase();
    for p in r.game.players.iter_mut() {
        p.original_deck = vec![
            OriginalDeckCardCount {
                card_id: "ATK".to_string(),
                count: 4,
                is_digitama: false,
            },
            OriginalDeckCardCount {
                card_id: "DEF".to_string(),
                count: 8,
                is_digitama: false,
            },
        ];
    }
    (r, cards)
}

/// Deterministic evaluator whose VALUE depends on the observation (FNV hash
/// of the tensor bits, squashed to [-1, 1]); policy uniform over legal.
///
/// The uniform stub is blind to anything the tree never proves terminal, so
/// a public infoset difference could vacuously produce identical results.
/// This probe makes the search output sensitive to every evaluated world
/// state: identical worlds keep identical results (the invariance
/// direction), while any observable difference perturbs leaf values (the
/// sensitivity direction).
struct ObsProbeEvaluator;

impl PolicyValueFn for ObsProbeEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        requests
            .iter()
            .map(|r| {
                let legal: Vec<usize> = r
                    .mask
                    .iter()
                    .enumerate()
                    .filter(|(_, &m)| m > 0.5)
                    .map(|(i, _)| i)
                    .collect();
                let mut policy = vec![0.0f32; r.mask.len()];
                let p = 1.0 / legal.len().max(1) as f32;
                for i in legal {
                    policy[i] = p;
                }
                let mut h: u32 = 2166136261;
                for &x in &r.obs {
                    h = (h ^ x.to_bits()).wrapping_mul(16777619);
                }
                let value = (h % 2001) as f32 / 1000.0 - 1.0;
                Evaluation { policy, value }
            })
            .collect()
    }
}

/// PIMC end-to-end: K worlds, summed root visits still find the forced win,
/// and the budget accounting matches the documented split
/// (`per_world = simulations / worlds`, remainder dropped).
#[test]
fn pimc_finds_forced_win_and_sums_visits_across_worlds() {
    run_big_stack(|| {
        let (r, cards) = forced_win_board();
        let tp = r.game.turn_player();
        let registry = CardRegistry::from_cards(&cards);
        let mut evaluator = UniformEvaluator;
        let config = pimc_config(160, 4, 42);
        let result = determinized_search(&registry, &r.game, tp, &mut evaluator, &config);

        assert_eq!(result.simulations_run, 160, "4 worlds x 40 sims");
        let visit_sum: u32 = result.root_visits.iter().map(|(_, v)| *v).sum();
        assert_eq!(visit_sum, 160, "every sim visits exactly one root edge");

        let win_action = encode_attack(0, SECURITY_TARGET);
        assert_eq!(most_visited(&result), win_action);
        assert!(result.root_value > 0.5, "root value {}", result.root_value);
        // PASS is on the visit list (legal) but dominated.
        assert!(result.root_visits.iter().any(|(a, _)| *a == PASS));
    });
}

/// IS-MCTS end-to-end on the same board (single infoset-keyed tree,
/// per-iteration re-determinization).
#[test]
fn ismcts_finds_forced_win() {
    run_big_stack(|| {
        let (r, cards) = forced_win_board();
        let tp = r.game.turn_player();
        let registry = CardRegistry::from_cards(&cards);
        let mut evaluator = UniformEvaluator;
        let config = ismcts_config(120, 42);
        let result = determinized_search(&registry, &r.game, tp, &mut evaluator, &config);

        assert_eq!(result.simulations_run, 120);
        let win_action = encode_attack(0, SECURITY_TARGET);
        assert_eq!(most_visited(&result), win_action);
        assert!(result.root_value > 0.5, "root value {}", result.root_value);
    });
}

/// Both modes are deterministic per seed and (almost surely) seed-sensitive.
#[test]
fn determinized_search_is_deterministic_per_seed() {
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        let (game, viewer) = midgame_state(&card_data, 31, 24);

        for mode in [DeterminizationMode::Pimc, DeterminizationMode::IsMcts] {
            let config = DeterminizedConfig {
                worlds: 4,
                mode,
                search: SearchConfig {
                    simulations: 48,
                    seed: 7,
                    ..SearchConfig::default()
                },
            };
            let mut e1 = UniformEvaluator;
            let a = determinized_search(&registry, &game, viewer, &mut e1, &config);
            let mut e2 = UniformEvaluator;
            let b = determinized_search(&registry, &game, viewer, &mut e2, &config);
            assert_eq!(a, b, "{mode:?}: same seed must reproduce exactly");

            let differs = (8u64..=10).any(|seed| {
                let mut e = UniformEvaluator;
                let cfg = DeterminizedConfig {
                    search: SearchConfig {
                        seed,
                        ..config.search.clone()
                    },
                    ..config.clone()
                };
                determinized_search(&registry, &game, viewer, &mut e, &cfg) != a
            });
            assert!(
                differs,
                "{mode:?}: different seeds should change the result"
            );
        }
    });
}

/// TASK 2.5 — the no-X-ray guard, strong form.
///
/// Permuting the opponent's concealed cards HIDDEN-TO-HIDDEN (a hand card
/// swapped with a differently-named deck card — invisible to the viewer's
/// infoset, per Phase 1's own invariance test) must leave the search result
/// BITWISE IDENTICAL: the search may only consume the infoset. A PUBLIC
/// change (opponent hand card moved to trash) must change the result.
#[test]
fn search_is_invariant_to_hidden_permutation_but_not_public_change() {
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        let (game, viewer) = midgame_state(&card_data, 51, 30);
        let opp = 1 - viewer;
        assert!(
            !game.players[opp as usize].hand.is_empty()
                && !game.players[opp as usize].deck.is_empty(),
            "fixture needs concealed opponent cards to permute"
        );

        // Hidden-to-hidden permutation: swap an opponent hand card with a
        // differently-named opponent deck card (same technique as
        // tests/determinization/infoset_tests.rs).
        let mut hidden_variant = game.clone();
        {
            let p = &hidden_variant.players[opp as usize];
            let mut found = None;
            'outer: for (hi, hc) in p.hand.iter().enumerate() {
                for (di, dc) in p.deck.iter().enumerate() {
                    if hc.card_id(&hidden_variant.card_data)
                        != dc.card_id(&hidden_variant.card_data)
                    {
                        found = Some((hi, di));
                        break 'outer;
                    }
                }
            }
            let (hi, di) = found.expect("opponent hand+deck contain two distinct ids");
            let p = &mut hidden_variant.players[opp as usize];
            let (hand, deck) = (&mut p.hand, &mut p.deck);
            std::mem::swap(&mut hand[hi], &mut deck[di]);
        }

        // Public change: an opponent hand card becomes visible in the trash.
        let mut public_variant = game.clone();
        {
            let p = &mut public_variant.players[opp as usize];
            let c = p.hand.pop().expect("opponent has a hand");
            p.trash.push(c);
        }

        // The obs-sensitive probe evaluator makes the guard sharp in both
        // directions: any difference in any evaluated world state perturbs
        // leaf values, so "identical results" really means "the search saw
        // only the infoset" — not "the uniform stub was blind to it".
        for (mode, sims) in [
            (DeterminizationMode::Pimc, 96u32),
            (DeterminizationMode::IsMcts, 48u32),
        ] {
            let config = DeterminizedConfig {
                worlds: 4,
                mode,
                search: SearchConfig {
                    simulations: sims,
                    seed: 99,
                    ..SearchConfig::default()
                },
            };
            let mut e1 = ObsProbeEvaluator;
            let base = determinized_search(&registry, &game, viewer, &mut e1, &config);
            let mut e2 = ObsProbeEvaluator;
            let hidden = determinized_search(&registry, &hidden_variant, viewer, &mut e2, &config);
            assert_eq!(
                base, hidden,
                "{mode:?}: a hidden-to-hidden permutation leaked into the search \
                 (the search consumed more than the infoset)"
            );
        }

        // Public sensitivity (PIMC): the infoset differs, so the result must.
        let config = pimc_config(96, 4, 99);
        let mut e1 = ObsProbeEvaluator;
        let base = determinized_search(&registry, &game, viewer, &mut e1, &config);
        let mut e3 = ObsProbeEvaluator;
        let public = determinized_search(&registry, &public_variant, viewer, &mut e3, &config);
        assert_ne!(
            base, public,
            "a public change (opponent hand -> trash) must be observable"
        );
    });
}

/// TASK 2.5 — world hygiene + mask discipline on a messy mid-game state.
/// Every materialized world carries no reveal source and passes the Phase 1
/// invariant checker (both debug_asserted inside `determinized_search` —
/// this test completing IS the guard), and every aggregated root action was
/// legal at the true root.
#[test]
fn determinized_search_worlds_are_hygienic_and_respect_the_root_mask() {
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        let (game, viewer) = midgame_state(&card_data, 61, 20);

        let root_mask = digimon_engine::build_action_mask(&game, viewer);
        let legal = legal_actions(&root_mask);

        // PIMC budget accounting: 3 worlds x floor(100/3)=33 sims each.
        let config = pimc_config(100, 3, 5);
        let mut evaluator = UniformEvaluator;
        let result = determinized_search(&registry, &game, viewer, &mut evaluator, &config);
        assert_eq!(
            result.simulations_run, 99,
            "3 worlds x 33 sims (remainder dropped)"
        );
        let visit_sum: u32 = result.root_visits.iter().map(|(_, v)| *v).sum();
        assert_eq!(visit_sum, 99);

        // Budget is a CEILING even when worlds > simulations: sims=1,
        // worlds=8 must clamp to ONE 1-sim world, never 8 sims.
        let tiny = pimc_config(1, 8, 5);
        let mut evaluator = UniformEvaluator;
        let result = determinized_search(&registry, &game, viewer, &mut evaluator, &tiny);
        assert_eq!(
            result.simulations_run, 1,
            "worlds must clamp to the simulation budget"
        );

        // simulations=0 does no search work but still enumerates the root's
        // legal actions with zero visits (matching the core searcher).
        let zero = pimc_config(0, 8, 5);
        let mut evaluator = UniformEvaluator;
        let result = determinized_search(&registry, &game, viewer, &mut evaluator, &zero);
        assert_eq!(result.simulations_run, 0);
        assert!(
            !result.root_visits.is_empty(),
            "root edges still enumerated"
        );
        assert!(result.root_visits.iter().all(|(_, v)| *v == 0));
        for (action, _) in &result.root_visits {
            assert!(
                legal.contains(action),
                "root action {action} not legal at the true root"
            );
        }

        // IS-MCTS: completes, and every visited root action is masked-legal.
        // (Its first simulation evaluates the root, so total visits = sims-1.)
        let config = ismcts_config(40, 5);
        let mut evaluator = UniformEvaluator;
        let result = determinized_search(&registry, &game, viewer, &mut evaluator, &config);
        assert_eq!(result.simulations_run, 40);
        let visit_sum: u32 = result.root_visits.iter().map(|(_, v)| *v).sum();
        assert_eq!(
            visit_sum, 39,
            "IS-MCTS spends its first sim on the root eval"
        );
        for (action, _) in &result.root_visits {
            assert!(
                legal.contains(action),
                "root action {action} not legal at the true root"
            );
        }
    });
}
