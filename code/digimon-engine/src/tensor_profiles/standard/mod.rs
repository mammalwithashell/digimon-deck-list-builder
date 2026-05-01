use crate::tensor_profiles::TensorProfile;

pub mod v1;

pub const DEFAULT_PROFILE: TensorProfile = v1::PROFILE;

pub fn profile_by_version(version: u32) -> Option<TensorProfile> {
    match version {
        1 => Some(v1::PROFILE),
        _ => None,
    }
}
