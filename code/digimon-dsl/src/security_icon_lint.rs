//! Security-icon lint: cross-check a card spec's security-scoped clauses
//! against the card's PRINTED text (the official Bandai DB mirror,
//! `data/card_official.json`).
//!
//! The printed card carries two different "security" icons, and they are
//! different mechanics:
//!
//! - **Pink `{Security}`** — an effect that is active while the card sits FACE UP
//!   in the security stack (rule 15-14-5; BT26-082's official Q&A). It is printed
//!   inside the card's normal effect box, e.g.
//!   `{Security} [End of Opponent's Turn] Play this card …` or
//!   `{Security} [All Turns] All of your [TS] trait Digimon get +2000 DP`.
//!   DSL: a clause with `scope: security` (a triggered turn-boundary clause or a
//!   `kind: aura`). The engine activates it only while the card is face up.
//! - **Blue `[Security]`** — the ordinary security effect, activated when the card
//!   is flipped during a security check. Printed in its own "Security Effect"
//!   section (`[Security] Play this card without paying the cost.`).
//!   DSL: `when: on_security` (face-up scope, or `inherited` on an Option).
//!
//! A third, icon-less shape — "When effects trash this card from the security
//! stack, …" — is authored `scope: security` + `when: on_discard_security`; it
//! is neither icon and is exempt from these checks.
//!
//! The colour itself is not in the data, so this lint keys on spelling:
//! - pink: the literal `{Security}`, OR (older printings, e.g. ST20-15 "Island of
//!   Adventure") `[Security]` immediately followed by another `[Timing]` bracket
//!   OUTSIDE the "Security Effect" section — `[Security] [All Turns] …`;
//! - blue: a "Security Effect" section, or a `[Security]` NOT followed by a
//!   timing bracket (`[Security] Play this card …`).

use crate::clause::{ClauseScope, ClauseSpec, Timing, TimingSet};
use crate::spec::CardSpec;

/// One printed text section of a card (label + text), as stored in
/// `card_official.json`'s `text_sections`.
#[derive(Debug, Clone)]
pub struct PrintedSection {
    pub label: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityIconRule {
    /// `scope: security` clause, but the printed card has no pink `{Security}`.
    PinkClauseWithoutPinkIcon,
    /// The printed card has a pink `{Security}` effect, but no `scope: security`
    /// clause implements it (it was likely authored as `when: on_security`,
    /// which would make it fire from a security CHECK instead of while face up).
    PinkIconWithoutPinkClause,
    /// A `when: on_security` clause, but the printed card has no blue
    /// `[Security]` effect.
    BlueClauseWithoutBlueIcon,
}

#[derive(Debug, Clone)]
pub struct SecurityIconFinding {
    pub rule: SecurityIconRule,
    /// Clause location, e.g. `effects[3]` or `dual.option.effects[0]`; empty for
    /// card-level findings.
    pub path: String,
    pub message: String,
}

fn timings(set: &TimingSet) -> Vec<Timing> {
    match set {
        TimingSet::Single(t) => vec![*t],
        TimingSet::Multi(v) => v.clone(),
    }
}

/// `scope: security` clauses that implement a pink `{Security}` effect (i.e.
/// every security-scoped clause except the icon-less on_discard_security shape),
/// and `when: on_security` clauses (blue).
fn classify(prefix: &str, clauses: &[ClauseSpec], pink: &mut Vec<String>, blue: &mut Vec<String>) {
    for (i, c) in clauses.iter().enumerate() {
        let path = format!("{prefix}[{i}]");
        match c {
            ClauseSpec::Triggered(t) => {
                let ts = timings(&t.when);
                if t.scope == ClauseScope::Security {
                    let only_discard =
                        !ts.is_empty() && ts.iter().all(|x| *x == Timing::OnDiscardSecurity);
                    if !only_discard {
                        pink.push(path.clone());
                    }
                }
                if ts.contains(&Timing::OnSecurity) {
                    blue.push(path);
                }
            }
            ClauseSpec::Declarative(d) => {
                if d.scope == ClauseScope::Security {
                    pink.push(path);
                }
            }
        }
    }
}

fn is_security_effect_section(s: &PrintedSection) -> bool {
    s.label.eq_ignore_ascii_case("Security Effect")
}

/// For each literal `[Security]` in `text`, whether it is immediately followed
/// (after whitespace) by another `[...]` timing bracket. Older printings render
/// the pink face-up icon that way inside the main effect box —
/// `[Security] [All Turns] …` (ST20-15, ST21-15, EX8-068/069/071, BT19-100,
/// BT26-075) — while a blue security effect reads `[Security] Play this card …`.
fn security_tags(text: &str) -> impl Iterator<Item = bool> + '_ {
    const TAG: &str = "[Security]";
    text.match_indices(TAG)
        .map(move |(i, _)| text[i + TAG.len()..].trim_start().starts_with('['))
}

/// Check one spec against its printed sections. Returns no findings when the
/// printed text is unavailable (`printed` empty) — absence of data is not a
/// violation.
pub fn check_security_icons(
    spec: &CardSpec,
    printed: &[PrintedSection],
) -> Vec<SecurityIconFinding> {
    if printed.is_empty() {
        return Vec::new();
    }
    let printed_pink = printed.iter().any(|s| {
        s.text.contains("{Security}")
            || (!is_security_effect_section(s)
                && security_tags(&s.text).any(|followed_by_timing| followed_by_timing))
    });
    let printed_blue = printed.iter().any(|s| {
        is_security_effect_section(s)
            || security_tags(&s.text).any(|followed_by_timing| !followed_by_timing)
    });

    let mut pink = Vec::new();
    let mut blue = Vec::new();
    classify("effects", &spec.effects, &mut pink, &mut blue);
    if let Some(dual) = &spec.dual {
        classify(
            "dual.digimon.effects",
            &dual.digimon.effects,
            &mut pink,
            &mut blue,
        );
        classify(
            "dual.option.effects",
            &dual.option.effects,
            &mut pink,
            &mut blue,
        );
    }

    let mut out = Vec::new();
    if !printed_pink {
        for p in &pink {
            out.push(SecurityIconFinding {
                rule: SecurityIconRule::PinkClauseWithoutPinkIcon,
                path: p.clone(),
                message: "`scope: security` clause, but the printed card has no pink `{Security}` \
                          effect. Pink `{Security}` = active while FACE UP in the security stack; a \
                          blue `[Security]` effect (fires on a security check) is authored \
                          `when: on_security` instead."
                    .into(),
            });
        }
    }
    if printed_pink && pink.is_empty() {
        out.push(SecurityIconFinding {
            rule: SecurityIconRule::PinkIconWithoutPinkClause,
            path: String::new(),
            message: "the printed card has a pink `{Security}` effect (active while face up in the \
                      security stack) but no `scope: security` clause implements it. Author it as \
                      `scope: security` with its printed timing (or `kind: aura, scope: security`), \
                      not `when: on_security`."
                .into(),
        });
    }
    if !printed_blue {
        for p in &blue {
            out.push(SecurityIconFinding {
                rule: SecurityIconRule::BlueClauseWithoutBlueIcon,
                path: p.clone(),
                message: "`when: on_security` clause, but the printed card has no blue `[Security]` \
                          effect. If this is a pink `{Security}` effect, author it `scope: security` \
                          with its printed timing."
                    .into(),
            });
        }
    }
    out
}
