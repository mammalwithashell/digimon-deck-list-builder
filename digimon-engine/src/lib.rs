pub mod enums;
pub mod rules;
pub mod card_data;
pub mod card_registry;
pub mod card_source;
pub mod permanent;
pub mod player;
pub mod game;
pub mod game_actions;
pub mod game_phases;
pub mod action;
pub mod phases;
pub mod tensor;
pub mod modifiers;
pub mod effect;
pub mod effect_context;
pub mod cards;
pub mod combat;
pub mod selection;
pub mod effect_queue;
pub mod dna_digivolve;
pub mod debug_runner;
pub mod runners;
pub mod logger;
pub mod recorder;

// Re-export key types at crate root
pub use enums::*;
pub use rules::{CardRestriction, Rules};
pub use card_data::CardData;
pub use card_registry::CardRegistry;
pub use card_source::{CardHandle, CardSource};
pub use permanent::{Permanent, PermanentHandle};
pub use player::Player;
pub use game::Game;
pub use tensor::{build_tensor, TENSOR_SIZE};
pub use action::build_action_mask;
pub use modifiers::{ModifierEntry, ModifierRegistry};
pub use effect::{CardEffect, Effect, EffectBuilder};
pub use effect_context::{EffectContext, EffectReadContext};
pub use cards::{build_registry, CardEffectRegistry};
pub use combat::AttackResult;
pub use debug_runner::{DebugRunner, DebugRunnerBuilder};
pub use runners::HeadlessRunner;
pub use logger::{GameLogger, SilentLogger, VerboseLogger};
pub use recorder::{GameRecorder, InitialState, PlayerInitialState, RecordedAction};
pub use selection::{
    AttackState, AttackTarget, DeclineCallback, EffectChoiceEntry, EffectQueue, PendingAttack,
    PendingSelection, QueuedEffect, SelectionCallback, SelectionError, SelectionKind,
    TriggerSource,
};
