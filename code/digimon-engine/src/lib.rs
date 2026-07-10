pub mod serialization;
#[cfg(feature = "dsl-yaml-loader")]
pub use digimon_dsl as dsl;

pub mod action;
pub mod aura;
pub mod card_data;
pub mod card_registry;
pub mod card_source;
pub mod card_store;
pub mod cards;
pub mod combat;
pub mod dcgo_recording;
pub mod debug_runner;
pub mod deck_tools;
pub mod deletion_batch;
pub mod determinize;
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
pub mod floating_modifier;
pub mod format;
pub mod game;
pub mod game_actions;
pub mod game_phases;
pub mod inference;
pub mod live_game;
pub mod logger;
pub mod modifiers;
pub mod observation;
pub mod opaque_deck;
pub mod option_lifecycle;
pub mod permanent;
pub mod phases;
pub mod player;
pub mod player_cost_reducer;
pub mod policies;
pub mod recorder;
pub mod replacement;
pub mod replay_corpus;
pub(crate) mod resource_flow;
pub mod resume;
pub mod rules;
pub mod runners;
pub mod scheduled_effects;
pub mod search;
pub mod selection;
pub mod selfplay;
pub mod tensor_profiles;
pub mod view;
pub use tensor_profiles as tensor_profile;
// Observation output port: the tensor builders live under `observation/`
// (read-only port — see its module doc). Re-exported here so the crate-root
// paths `crate::tensor`, `crate::tensor_v1`, … (used by PyO3 + Tauri) are
// preserved unchanged.
pub use observation::{tensor, tensor_v1, tensor_v2_full, tensor_v2_lite, tensor_v2_lite_deck};
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
pub use digixros::{
    ActiveDigiXrosWildcardSubstitution, DigiXrosDistinctBy, DigiXrosMaterialOrigin,
    DigiXrosMaterialZone, DigiXrosRecipeSlot, DigiXrosSelectedMaterial, DigiXrosTransaction,
    DigiXrosWildcardSubstitution, DigiXrosZoneAllowance,
};
pub use effect::{CardEffect, Effect, EffectBuilder};
pub use effect_context::{CountCappedZone, DistinctByMode, EffectContext, EffectReadContext};
pub use enums::*;
pub use game::{Game, TerminalOutcomeReason};
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
