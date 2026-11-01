#![cfg(feature = "portable-policy-math")]
use digimon_engine::inference::masked_softmax;

#[test]
fn portable_softmax_masks_and_normalizes_extreme_logits() {
    let p = masked_softmax(&[f32::NAN, -1000., 0., -1.], &[0., 1., 1., 1.]).unwrap();
    assert_eq!(p[0].to_bits(), 0);
    assert_eq!(p[1].to_bits(), 0);
    assert!(p[2] > p[3] && p[3] > 0.);
    assert!((p.iter().sum::<f32>() - 1.).abs() <= f32::EPSILON);
    assert!(masked_softmax(&[f32::NAN], &[1.]).is_err());
    assert!(masked_softmax(&[0.], &[0.]).is_err());
    assert!(masked_softmax(&[0.], &[]).is_err());
}

#[test]
fn portable_exponential_accuracy_and_monotonicity_on_softmax_domain() {
    let mut previous = 0.;
    for i in 0..=25600 {
        let x = -100. + i as f32 / 256.;
        let actual = libm::expf(x);
        let reference = (x as f64).exp() as f32;
        assert!(actual >= previous && actual <= 1.);
        assert!(actual.to_bits().abs_diff(reference.to_bits()) <= 2);
        previous = actual;
    }
    assert_eq!(libm::expf(f32::NEG_INFINITY), 0.);
    assert_eq!(libm::expf(0.), 1.);
}

#[test]
fn saved_discrepant_operands_are_stable_in_new_path() {
    for x in [f32::from_bits(3199710904), f32::from_bits(3189940136)] {
        let first = masked_softmax(&[0., x], &[1., 1.]).unwrap();
        for _ in 0..32 {
            let next = masked_softmax(&[0., x], &[1., 1.]).unwrap();
            assert_eq!(
                first.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                next.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
            );
        }
    }
}
