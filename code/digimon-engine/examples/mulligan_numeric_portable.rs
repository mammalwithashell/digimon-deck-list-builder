#[cfg(not(feature = "portable-policy-math"))]
compile_error!("portable probe requires portable-policy-math");
// Offline portable numerical probe; no Game or search.
use digimon_engine::{
    inference::{masked_softmax, BatchedPolicyValueEvaluator},
    observation::ObservationProfileId,
};
use ndarray::Array2;
use ort::{session::Session, value::Value};
use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use std::path::Path;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 2, "request output");
    let output = Path::new(&args[1]);
    assert!(!output.exists(), "preserve earlier numeric probe");
    let request: Json = serde_json::from_slice(&std::fs::read(&args[0]).unwrap()).unwrap();
    let mut rows = Vec::new();
    for case in request["cases"].as_array().unwrap() {
        let bytes = std::fs::read(case["observation"].as_str().unwrap()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            case["observation_sha256"].as_str().unwrap()
        );
        assert_eq!(bytes.len() % 4, 0);
        let obs: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let model = Path::new(case["model"].as_str().unwrap());
        assert_eq!(
            format!("{:x}", Sha256::digest(std::fs::read(model).unwrap())),
            case["model_sha256"].as_str().unwrap()
        );
        let checked = BatchedPolicyValueEvaluator::load_for_profile(
            model,
            ObservationProfileId::StandardLiteV2,
        )
        .unwrap();
        checked.require_terminal_value().unwrap();
        drop(checked);
        let mut session = Session::builder()
            .unwrap()
            .with_intra_threads(1)
            .unwrap()
            .commit_from_file(model)
            .unwrap();
        let tensor =
            Value::from_array(Array2::from_shape_vec((1, obs.len()), obs).unwrap()).unwrap();
        let outputs = session.run(ort::inputs!["obs"=>tensor]).unwrap();
        let (_, logits) = outputs
            .get("logits")
            .unwrap()
            .try_extract_tensor::<f32>()
            .unwrap();
        let (_, values) = outputs
            .get("value")
            .unwrap()
            .try_extract_tensor::<f32>()
            .unwrap();
        let mut mask = vec![0.; logits.len()];
        mask[0] = 1.;
        mask[1] = 1.;
        let policy = masked_softmax(logits, &mask).unwrap();
        let max = logits[0].max(logits[1]);
        let centered = [logits[0] - max, logits[1] - max];
        let exps = [libm::expf(centered[0]), libm::expf(centered[1])];
        let mut logit_hash = Sha256::new();
        for logit in logits {
            logit_hash.update(logit.to_le_bytes());
        }
        rows.push(json!({"id":case["id"],"checkpoint":case["checkpoint"],
            "observation_sha256":case["observation_sha256"],"model_sha256":case["model_sha256"],
            "all_logits_sha256":format!("{:x}",logit_hash.finalize()),
            "legal_logits":[logits[0],logits[1]],"legal_logit_bits":[logits[0].to_bits(),logits[1].to_bits()],
            "centered":centered,"centered_bits":centered.map(f32::to_bits),
            "exp":exps,"exp_bits":exps.map(f32::to_bits),"denom_bits":(exps[0]+exps[1]).to_bits(),
            "policy":[policy[0],policy[1]],"policy_bits":[policy[0].to_bits(),policy[1].to_bits()],
            "value_bits":values[0].to_bits()}));
    }
    let mut domain_hash = Sha256::new();
    for i in 0..=25600 {
        let x = -100. + i as f32 / 256.;
        domain_hash.update(libm::expf(x).to_le_bytes());
    }
    std::fs::write(output,serde_json::to_vec_pretty(&json!({"math":"libm-0.2.15-software-expf","domain_exp_count":25601,"domain_exp_sha256":format!("{:x}",domain_hash.finalize()),"rows":rows,"scope":"Offline model/softmax numeric probe; no game, search, retry or scored result"})).unwrap()).unwrap();
}
