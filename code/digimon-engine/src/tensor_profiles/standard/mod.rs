use crate::tensor_profiles::TensorProfile;

pub mod v1;
pub mod v2_full;
pub mod v2_lite;
pub mod v2_lite_deck;

pub const DEFAULT_PROFILE: TensorProfile = v2_lite_deck::PROFILE;

pub fn profile_by_version(version: u32) -> Option<TensorProfile> {
    match version {
        1 => Some(v1::PROFILE),
        2 => Some(v2_lite::PROFILE),
        _ => None,
    }
}
