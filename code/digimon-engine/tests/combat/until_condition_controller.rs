use std::sync::Arc;

use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::{ModifierEntry, ModifierSubject};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SourceSelectionRef;

fn digimon(card_id: &str, level: u8, dp: i32) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, card_id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(level);
    d.dp = Some(dp);
    d.colors = vec![CardColor::Red];
    d
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(digimon("SRC", 3, 2000))
        .add_card(digimon("HOST", 4, 4000))
        .add_card(digimon("OTHER", 4, 4000))
        .add_card(digimon("WATCHER", 3, 2000))
        .start()
}

fn memory_non_negative() -> digimon_engine::modifiers::UntilConditionFn {
    Arc::new(|game, _subject| game.memory >= 0)
}

#[test]
fn until_condition_installs_in_debug_and_evicts_after_predicate_false() {
    let mut r = runner();
    let host = r.place_on_field(0, "HOST", Some(0));

    r.game.modifiers.add(
        host,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, 0)
            .with_until_condition(memory_non_negative()),
    );

    assert_eq!(r.game.modifiers.sum(host, ModifierType::ChangeDp), 1000);
    r.game.set_memory(-1);

    assert_eq!(
        r.game.modifiers.sum(host, ModifierType::ChangeDp),
        0,
        "predicate false after a memory mutation must evict the modifier"
    );
    assert_eq!(r.game.until_condition_last_cycle_evaluations(), 1);
}

#[test]
fn until_condition_false_then_true_does_not_reinstall() {
    let mut r = runner();
    let host = r.place_on_field(0, "HOST", Some(0));

    r.game.modifiers.add(
        host,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, 0)
            .with_until_condition(memory_non_negative()),
    );

    r.game.set_memory(-1);
    r.game.set_memory(0);

    assert_eq!(
        r.game.modifiers.sum(host, ModifierType::ChangeDp),
        0,
        "UntilCondition removal is final for that installed entry"
    );
}

#[test]
fn until_condition_fifo_eviction_lets_later_predicates_see_prior_removals() {
    let mut r = runner();
    let host = r.place_on_field(0, "HOST", Some(0));

    r.game.modifiers.add(
        host,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::UntilCondition, 0)
            .with_until_condition(memory_non_negative()),
    );
    r.game.modifiers.add(
        host,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, 0)
            .with_until_condition(Arc::new(|game, subject| match subject {
                ModifierSubject::Permanent(target) => {
                    game.modifiers.has(target, ModifierType::CannotSuspend)
                }
                ModifierSubject::Player(_) => false,
            })),
    );

    r.game.set_memory(-1);

    assert!(!r.game.modifiers.has(host, ModifierType::CannotSuspend));
    assert_eq!(
        r.game.modifiers.sum(host, ModifierType::ChangeDp),
        0,
        "second predicate must evaluate after the first entry is evicted"
    );
    assert_eq!(r.game.until_condition_last_cycle_evaluations(), 2);
}

#[test]
fn until_condition_subject_scope_uses_installed_subject_not_event_subject() {
    let mut r = runner();
    let host = r.place_stack(0, &["SRC", "HOST"]);
    let other = r.place_on_field(0, "OTHER", Some(1));

    r.game.modifiers.add(
        host,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, 0)
            .with_until_condition(Arc::new(|game, subject| match subject {
                ModifierSubject::Permanent(target) => game
                    .player(target.player)
                    .battle_area
                    .get(target.index as usize)
                    .is_some_and(|perm| perm.stack_size() >= 2),
                ModifierSubject::Player(_) => false,
            })),
    );

    r.game.suspend(other);
    assert_eq!(
        r.game.modifiers.sum(host, ModifierType::ChangeDp),
        1000,
        "unrelated event subjects must not replace the installed subject"
    );

    let source_card = r.game.player(0).battle_area[host.index as usize].card_sources[0].handle();
    r.game.remove_source_ref(SourceSelectionRef {
        permanent: host,
        field_index: host.index,
        source_index: 0,
        card: source_card,
    });

    assert_eq!(
        r.game.modifiers.sum(host, ModifierType::ChangeDp),
        0,
        "source-count predicate must evict when the installed subject changes"
    );
}

struct MemoryFlipOnSuspend {
    watched: PermanentHandle,
}

impl CardEffect for MemoryFlipOnSuspend {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let watched = self.watched;
        vec![Effect::on_suspend(card)
            .name("memory flip on suspend")
            .process(move |ctx| {
                ctx.set_memory(-1);
                assert_eq!(
                    ctx.game.modifiers.sum(watched, ModifierType::ChangeDp),
                    1000,
                    "controller must not evict mid-observer after an internal state change"
                );
            })
            .build()]
    }
}

#[test]
fn until_condition_waits_until_top_level_event_drain_completes() {
    let mut r = runner();
    let watched = r.place_on_field(0, "HOST", Some(0));
    let watcher = r.place_on_field(0, "WATCHER", Some(1));

    r.game.modifiers.add(
        watched,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::UntilCondition, 0)
            .with_until_condition(memory_non_negative()),
    );
    r.game
        .effect_registry
        .insert("WATCHER", Arc::new(MemoryFlipOnSuspend { watched }));

    r.game.suspend(watcher);

    assert_eq!(
        r.game.modifiers.sum(watched, ModifierType::ChangeDp),
        0,
        "controller must evict once after the OnSuspend observer queue drains"
    );
    assert_eq!(r.game.until_condition_last_cycle_evaluations(), 1);
}
