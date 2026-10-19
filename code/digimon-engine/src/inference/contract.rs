//! Model semantics must match the runtime profile, even when shapes agree.
use crate::tensor_profiles::TensorProfile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelContract {
    pub observation_profile: String,
    pub tensor_version: u16,
    pub feature_schema_version: String,
    pub tensor_size: usize,
    pub tensor_layout_hash: String,
    pub action_space_size: usize,
    pub action_schema_version: u32,
}

impl ModelContract {
    pub fn for_profile(profile: &TensorProfile) -> Self {
        Self {
            observation_profile: profile.id.into(),
            tensor_version: profile.tensor_version,
            feature_schema_version: profile.feature_schema_version.into(),
            tensor_size: profile.tensor_size,
            tensor_layout_hash: profile.layout_hash.into(),
            action_space_size: crate::action::ACTION_SPACE_SIZE,
            action_schema_version: crate::action::space::SCHEMA_VERSION,
        }
    }
    pub fn validate_for_profile(&self, profile: &TensorProfile) -> Result<(), String> {
        let expected = Self::for_profile(profile);
        if self != &expected {
            return Err(format!(
                "model contract mismatch: expected {expected:?}, found {self:?}"
            ));
        }
        Ok(())
    }
    pub fn parse_and_validate(raw: &str, profile: &TensorProfile) -> Result<Self, String> {
        let contract: Self = serde_json::from_str(raw)
            .map_err(|e| format!("missing or malformed digimon_contract metadata: {e}"))?;
        contract.validate_for_profile(profile)?;
        Ok(contract)
    }
}

pub(crate) fn validate_session_contract(
    session: &ort::session::Session,
    profile: &TensorProfile,
) -> Result<(), super::InferenceError> {
    let metadata = session.metadata().map_err(|e| {
        super::InferenceError::LoadFailed(format!("cannot read model contract: {e}"))
    })?;
    let raw = metadata.custom("digimon_contract").ok_or_else(|| {
        super::InferenceError::LoadFailed("missing embedded digimon_contract metadata".into())
    })?;
    ModelContract::parse_and_validate(&raw, profile).map_err(super::InferenceError::LoadFailed)?;
    validate_outlet(
        session
            .inputs()
            .iter()
            .find(|v| v.name() == "obs")
            .map(|v| v.dtype()),
        profile.tensor_size,
        "obs",
    )?;
    validate_outlet(
        session
            .outputs()
            .iter()
            .find(|v| v.name() == "logits")
            .map(|v| v.dtype()),
        crate::action::ACTION_SPACE_SIZE,
        "logits",
    )
}

fn validate_outlet(
    dtype: Option<&ort::value::ValueType>,
    expected: usize,
    label: &str,
) -> Result<(), super::InferenceError> {
    use ort::value::TensorElementType;
    let valid = dtype.is_some_and(|dtype| {
        dtype.tensor_type() == Some(TensorElementType::Float32)
            && dtype
                .tensor_shape()
                .is_some_and(|shape| shape.len() == 2 && shape[1] == expected as i64)
    });
    if !valid {
        return Err(super::InferenceError::ShapeMismatch(format!(
            "{label} must be a rank-2 float32 tensor with final dimension {expected}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod dimension_tests {
    use super::*;
    use ort::value::{TensorElementType, ValueType};
    fn tensor(shape: Vec<i64>, ty: TensorElementType) -> ValueType {
        let rank = shape.len();
        ValueType::Tensor {
            ty,
            shape: shape.into(),
            dimension_symbols: ort::value::SymbolicDimensions::empty(rank),
        }
    }
    #[test]
    fn actual_float_dimensions_must_match_even_with_valid_metadata() {
        let current = crate::tensor_profiles::default_profile();
        let good = tensor(
            vec![-1, current.tensor_size as i64],
            TensorElementType::Float32,
        );
        assert!(validate_outlet(Some(&good), current.tensor_size, "obs").is_ok());
        for bad in [
            tensor(vec![-1, 8], TensorElementType::Float32),
            tensor(
                vec![-1, current.tensor_size as i64],
                TensorElementType::Int64,
            ),
            tensor(vec![current.tensor_size as i64], TensorElementType::Float32),
            tensor(vec![-1, -1], TensorElementType::Float32),
        ] {
            assert!(validate_outlet(Some(&bad), current.tensor_size, "obs").is_err());
        }
        assert!(validate_outlet(None, current.tensor_size, "obs").is_err());
    }
}

#[cfg(test)]
mod action_semantic_tests {
 use super::*;
 #[test] fn identical_shapes_with_old_action_semantics_are_rejected() {
  let profile=crate::tensor_profiles::default_profile();
  let current=ModelContract::for_profile(&profile);
  let mut old=current.clone(); old.action_schema_version=current.action_schema_version+1;
  assert_eq!(old.tensor_size,current.tensor_size); assert_eq!(old.action_space_size,current.action_space_size);
  assert!(old.validate_for_profile(&profile).is_err());
  assert!(ModelContract::parse_and_validate(&serde_json::to_string(&current).unwrap(),&profile).is_ok());
 }
 #[test] fn missing_action_semantics_cannot_be_inferred_from_shape() {
  let profile=crate::tensor_profiles::default_profile();
  let mut value=serde_json::to_value(ModelContract::for_profile(&profile)).unwrap();
  value.as_object_mut().unwrap().remove("action_schema_version");
  assert!(ModelContract::parse_and_validate(&value.to_string(),&profile).is_err());
 }
}
