pub mod serialization;
#[cfg(feature = "dsl-yaml-loader")]
pub use digimon_dsl as dsl;

pub mod action;
pub mod card_data;
pub mod card_registry;
pub mod card_source;
pub mod cards;
pub mod combat;
pub mod debug_runner;
pub mod deck_tools;
pub mod digixros;
pub mod dna_digivolve;
#[cfg(feature = "dsl-yaml-loader")]
pub mod dsl_bridge;
#[cfg(feature = "dsl-yaml-loader")]
pub mod dsl_cards;
#[cfg(feature = "dsl-yaml-loader")]
pub mod dsl_registry;
pub mod effect;
pub mod effect_context;
pub mod effect_queue;
pub mod enums;
pub mod events;
pub mod game;
pub mod game_actions;
pub mod game_phases;
pub mod inference;
pub mod logger;
pub mod modifiers;
pub mod observation;
pub mod option_lifecycle;
pub mod permanent;
pub mod phases;
pub mod player;
pub mod policies;
pub mod recorder;
pub mod replacement;
pub(crate) mod resource_flow;
pub mod rules;
pub mod runners;
pub mod scheduled_effects;
pub mod selection;
pub mod tensor;
pub mod tensor_profiles;
pub use tensor_profiles as tensor_profile;
pub mod tensor_v2_full;
pub mod tensor_v2_lite;
pub mod token_registry;
pub mod trigger_context;

// Re-export key types at crate root
pub use crate::events::GameEvent;
pub use action::build_action_mask;
pub use card_data::CardData;
pub use card_registry::CardRegistry;
pub use card_source::{CardHandle, CardSource};
pub use cards::{build_registry, CardEffectRegistry};
pub use combat::AttackResult;
pub use debug_runner::{DebugRunner, DebugRunnerBuilder};
pub use effect::{CardEffect, Effect, EffectBuilder};
pub use effect_context::{CountCappedZone, DistinctByMode, EffectContext, EffectReadContext};
pub use enums::*;
pub use game::Game;
pub use inference::{load_policy, InferenceError, OnnxLstmPolicy, OnnxMlpPolicy, OnnxPolicy};
pub use logger::{GameLogger, SilentLogger, VerboseLogger};
pub use modifiers::{ModifierEntry, ModifierRegistry, PlayerModifierEntry};
pub use permanent::{Permanent, PermanentHandle};
pub use player::Player;
pub use policies::{greedy_action, GreedyPolicy, Policy, RandomPolicy};
pub use recorder::{
    GameRecorder, InitialState, PlayerInitialState, RecordedAction, TensorSnapshot,
};
pub use rules::{CardRestriction, Rules};
pub use runners::HeadlessRunner;
pub use selection::{
    AttackState, AttackTarget, DeclineCallback, EffectChoiceEntry, EffectQueue, PendingAttack,
    PendingSelection, PendingSelectionView, QueuedEffect, SelectionCallback, SelectionError,
    SelectionKind, TriggerSource, UnionZoneSet,
};
pub use tensor::{build_tensor, TENSOR_SIZE};
pub use token_registry::{TokenDef, TokenRegistry};
pub use trigger_context::{AttackTargetChange, AttackTargetChangeReason, TriggerContext};
