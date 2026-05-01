//! Action-mask + observation-tensor test binary — covers the mask
//! generation, tensor encoding, card-registry indexing, and hidden-info
//! redaction. These tests share the MaskParity / TensorParity scaffolding
//! and benefit from being compiled as a single binary.

mod action_explain;
mod action_main_effects_parity;
mod breeding_selection_mask;
mod card_registry_parity;
mod dp_budget_selection_mask;
mod mask_end_of_turn_parity;
mod mask_main_effects_parity;
mod mask_main_parity;
mod source_selection_mask;
mod tensor_and_mask;
mod tensor_helpers;
mod tensor_hidden_info;
mod tensor_source_contributions;
