//! Spec-shape lints run by `dsl-lint` (and, for the printed-text rule, the
//! engine's `tests/dsl/spec_lint.rs` CI gate).
//!
//! - [`check_security_attack_self_aura`] — **error**: an unconditional,
//!   face-up-scope, self-target `kind: aura` with `security_attack:` on a card
//!   whose printed face ALREADY carries an innate `<Security A. ±N>`. The engine
//!   counts the printed keyword (`security_attack_keyword_bonus`, parsed off the
//!   face text) AND the aura's `SecurityAttackChange` modifier, so a real-text
//!   game checks one extra security card. The grant de-dup that single-counts a
//!   printed keyword only covers `kind: grant_keyword`; author the printed
//!   keyword as `kind: grant_keyword, keyword: SecurityAttackPlus, value: N`
//!   (G-ENGINE-SECURITY-ATTACK-AURA-PLUS-FACE-DOUBLE-COUNT).
//! - [`check_capped_multi_min_zero`] — **warning**: `select_count_capped_multi`
//!   with an explicit `min: 0` and no `optional_zero: true`. `min: 0` is the
//!   default and means "at least 1 pick unless `optional_zero`", so the
//!   explicit `min: 0` reads as "zero picks allowed" but is not (BT26-016).

use crate::clause::{ClauseScope, ClauseSpec, DeclarativeKind, TypedDeclarativeBody};
use crate::predicate::PredicateSpec;
use crate::spec::CardSpec;
use serde_yml::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecLintRule {
    /// Self `security_attack` aura on a card that prints `<Security A. ±N>`.
    SecurityAttackAuraDoubleCountsPrintedKeyword,
    /// `select_count_capped_multi { min: 0 }` without `optional_zero`.
    CappedMultiMinZeroWithoutOptionalZero,
}

#[derive(Debug, Clone)]
pub struct SpecLintFinding {
    pub rule: SpecLintRule,
    /// Location, e.g. `effects[0]` or `effects[2].process[1]`.
    pub path: String,
    pub message: String,
}

/// `target` names the aura's own carrier: absent / `{}` (the self-aura idiom)
/// or exactly `{ is_source: true }`.
fn is_self_target(target: &Option<PredicateSpec>) -> bool {
    match target {
        None => true,
        Some(p) => {
            *p == PredicateSpec::default()
                || *p
                    == PredicateSpec {
                        is_source: Some(true),
                        ..PredicateSpec::default()
                    }
        }
    }
}

fn self_sa_auras(prefix: &str, clauses: &[ClauseSpec], out: &mut Vec<String>) {
    for (i, c) in clauses.iter().enumerate() {
        let ClauseSpec::Declarative(d) = c else {
            continue;
        };
        if d.kind != DeclarativeKind::Aura
            || d.scope != ClauseScope::FaceUp
            || d.active_when.is_some()
        {
            continue;
        }
        let Ok(TypedDeclarativeBody::Aura(a)) = d.typed_body() else {
            continue;
        };
        if a.security_attack.is_some() && a.target_player.is_none() && is_self_target(&a.target) {
            out.push(format!("{prefix}[{i}]"));
        }
    }
}

/// Rule (a). `face_prints_security_attack` is whether the card's printed FACE
/// text (the main effect box, not the inherited / security sections) carries
/// an innate `<Security A. +N>` / `<Security A. -N>` — the caller decides with
/// the engine's own face-keyword parser (`parse_printed_keywords`) so the lint
/// matches exactly what the engine double-counts.
pub fn check_security_attack_self_aura(
    spec: &CardSpec,
    face_prints_security_attack: bool,
) -> Vec<SpecLintFinding> {
    if !face_prints_security_attack {
        return Vec::new();
    }
    let mut paths = Vec::new();
    self_sa_auras("effects", &spec.effects, &mut paths);
    if let Some(dual) = &spec.dual {
        self_sa_auras("dual.digimon.effects", &dual.digimon.effects, &mut paths);
    }
    paths
        .into_iter()
        .map(|path| SpecLintFinding {
            rule: SpecLintRule::SecurityAttackAuraDoubleCountsPrintedKeyword,
            path,
            message: "unconditional self `security_attack` aura on a card whose face prints \
                      `<Security A. ±N>`: the engine also counts the printed keyword, so this \
                      checks one extra security card. Author it as `kind: grant_keyword, \
                      keyword: SecurityAttackPlus, value: N` (single-counted when printed). \
                      G-ENGINE-SECURITY-ATTACK-AURA-PLUS-FACE-DOUBLE-COUNT."
                .into(),
        })
        .collect()
}

fn is_zero(v: &Value) -> bool {
    v.as_u64() == Some(0) || v.as_i64() == Some(0)
}

fn walk_capped_multi(v: &Value, path: &str, out: &mut Vec<SpecLintFinding>) {
    match v {
        Value::Mapping(m) => {
            for (k, child) in m {
                let key = k.as_str().unwrap_or("?");
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                if key == "select_count_capped_multi" {
                    if let Value::Mapping(args) = child {
                        let min_zero = args.get("min").map_or(false, is_zero);
                        let optional_zero = args
                            .get("optional_zero")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        if min_zero && !optional_zero {
                            out.push(SpecLintFinding {
                                rule: SpecLintRule::CappedMultiMinZeroWithoutOptionalZero,
                                path: child_path.clone(),
                                message: "`select_count_capped_multi` with `min: 0` but no \
                                          `optional_zero`: `min: 0` is the default and means \
                                          \"at least 1 pick unless `optional_zero`\". If zero \
                                          picks are legal, set `optional_zero: true` (and drop \
                                          `min: 0`); otherwise drop the misleading `min: 0`."
                                    .into(),
                            });
                        }
                    }
                }
                walk_capped_multi(child, &child_path, out);
            }
        }
        Value::Sequence(s) => {
            for (i, child) in s.iter().enumerate() {
                walk_capped_multi(child, &format!("{path}[{i}]"), out);
            }
        }
        Value::Tagged(t) => walk_capped_multi(&t.value, path, out),
        _ => {}
    }
}

/// Rule (b), over the raw YAML (the typed `min` defaults to 0, so an explicit
/// `min: 0` is only visible in the source text). Returns no findings for text
/// that is not valid YAML — the loader reports that.
pub fn check_capped_multi_min_zero(yaml: &str) -> Vec<SpecLintFinding> {
    let Ok(v) = serde_yml::from_str::<Value>(yaml) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    walk_capped_multi(&v, "", &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(effects_yaml: &str) -> CardSpec {
        let y = format!(
            "card: TEST-1\nname: T\nkind: digimon\nlevel: 6\ncolor: [red]\ncost: 10\ndp: 12000\n\
             effects:\n{effects_yaml}"
        );
        serde_yml::from_str(&y).expect("test spec parses")
    }

    const SELF_AURA: &str = "  - kind: aura\n    target: {}\n    security_attack: 1\n";

    #[test]
    fn self_sa_aura_on_printed_keyword_is_flagged() {
        let s = spec(SELF_AURA);
        let f = check_security_attack_self_aura(&s, true);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].path, "effects[0]");
        assert_eq!(
            f[0].rule,
            SpecLintRule::SecurityAttackAuraDoubleCountsPrintedKeyword
        );
    }

    #[test]
    fn absent_target_and_is_source_target_count_as_self() {
        let s = spec(concat!(
            "  - kind: aura\n    security_attack: 1\n",
            "  - kind: aura\n    target: { is_source: true }\n    security_attack: 1\n",
        ));
        assert_eq!(check_security_attack_self_aura(&s, true).len(), 2);
    }

    #[test]
    fn self_sa_aura_without_printed_keyword_is_clean() {
        assert!(check_security_attack_self_aura(&spec(SELF_AURA), false).is_empty());
    }

    #[test]
    fn grant_keyword_form_is_clean() {
        let s = spec("  - kind: grant_keyword\n    keyword: SecurityAttackPlus\n    value: 1\n");
        assert!(check_security_attack_self_aura(&s, true).is_empty());
    }

    #[test]
    fn conditional_inherited_or_filtered_auras_are_clean() {
        let conditional = spec(
            "  - kind: aura\n    target: {}\n    security_attack: 1\n    \
             active_when: { your_turn: true }\n",
        );
        assert!(check_security_attack_self_aura(&conditional, true).is_empty());
        let inherited =
            spec("  - kind: aura\n    scope: inherited\n    target: {}\n    security_attack: 1\n");
        assert!(check_security_attack_self_aura(&inherited, true).is_empty());
        let filtered = spec(
            "  - kind: aura\n    target: { trait_has: Royal Knight }\n    security_attack: 1\n",
        );
        assert!(check_security_attack_self_aura(&filtered, true).is_empty());
        let dp_only = spec("  - kind: aura\n    target: {}\n    dp_modifier: 1000\n");
        assert!(check_security_attack_self_aura(&dp_only, true).is_empty());
    }

    const CAPPED_MIN0: &str = "\
effects:
  - when: on_play
    process:
      - if:
          condition: { binding_present: x }
          then:
            - select_count_capped_multi:
                of: you
                zone: trash
                max: 3
                min: 0
                filter: {}
                prompt: p
";

    #[test]
    fn capped_multi_min_zero_without_optional_zero_warns_with_nested_path() {
        let f = check_capped_multi_min_zero(CAPPED_MIN0);
        assert_eq!(f.len(), 1);
        assert_eq!(
            f[0].path,
            "effects[0].process[0].if.then[0].select_count_capped_multi"
        );
        assert_eq!(
            f[0].rule,
            SpecLintRule::CappedMultiMinZeroWithoutOptionalZero
        );
    }

    #[test]
    fn capped_multi_with_optional_zero_or_without_explicit_min_is_clean() {
        let with_oz = CAPPED_MIN0.replace("min: 0", "min: 0\n                optional_zero: true");
        assert!(check_capped_multi_min_zero(&with_oz).is_empty());
        let no_min = CAPPED_MIN0.replace("                min: 0\n", "");
        assert!(check_capped_multi_min_zero(&no_min).is_empty());
        let min_one = CAPPED_MIN0.replace("min: 0", "min: 1");
        assert!(check_capped_multi_min_zero(&min_one).is_empty());
    }
}
