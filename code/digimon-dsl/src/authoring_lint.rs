//! Authoring lints over the RAW card YAML: shapes that parse, compile and run
//! but that no printed card means. Each rule is a class that reached logged
//! games before it was caught (bug scan over 1,224 games), so the rule is an
//! error, not a warning.
//!
//! The walk is over the untyped YAML value on purpose: both shapes can sit at
//! any depth (a formula inside a nested `if`, a granted effect's `when:`), and
//! a typed walk would have to be kept in step with every new step variant.
//!
//! Rules
//!
//! * `unfiltered-battle-area-count` -- `card_count_in_zone` over `battle_area`
//!   with no `filter`. An unfiltered count is `battle_area.len()`: every
//!   permanent, including an Option card an effect placed in the battle area
//!   (CR 4.3 4-28-1, 17-1-3-2-2: it stays there as an Option card) and Tamers
//!   (4-4-1). No printed card counts "permanents"; they count "Digimon",
//!   "Digimon and Tamers", "Tamers", ... so the filter must be spelled out.
//!   ShineGreymon (AD1-016) "-3000 DP for each of your Digimon and Tamers" and
//!   Medusamon (BT24-017) "+2000 DP for each of your opponent's Digimon" both
//!   counted Digital Accident Tactics Squad (P-235) / a Tamer this way.
//!
//! * `self-and-observer-timing` -- one `when:` list mixing a card's OWN timing
//!   (`on_play`, `when_digivolving`, `when_attacking`, `on_deletion`) with the
//!   board-wide OBSERVER timing of the same event family (`on_digivolve`,
//!   `on_enter_field_anyone`, `on_any_digimon_played`, `on_ally_played`,
//!   `on_ally_attack`, `on_opponent_attack`, `on_any_deletion`). The printed
//!   tag pair "[On Play] [When Digivolving]" is `[on_play, when_digivolving]`;
//!   `on_digivolve` fires whenever ANY Digimon on either side digivolves.
//!   Snatchmon (BT21-058) was authored `[on_play, on_digivolve]` and re-ran its
//!   whole effect on every digivolution in the game. A printed observer ("When
//!   any of your Digimon are played or digivolve") uses observer timings only;
//!   "When Digimon attack" is `[on_ally_attack, on_opponent_attack]` (the
//!   observer's own attack is inside `on_ally_attack`), never `when_attacking`.
//!
//! * `refund-after-explicit-accept` -- a `refund_opt` inside a clause whose
//!   activation passes an explicit accept: an `optional: true` replacement
//!   (the "May accept replacement" prompt), an `outer_prompt: true` clause, or
//!   a [Main] / [Counter] clause (activation-type, CR 4.3 15-8-4 /
//!   15-16-14-1: activated by declaring it). Whether such a refund is right
//!   depends on which path reaches it, and the YAML shape cannot tell. The
//!   [Once Per Turn] table (qa/rules/mechanics-gaps-2026-10.md W14) decides;
//!   the accept prompt is DCGO's skippable Yes/No, not printed, and for a
//!   printed "you may X" the choice to activate is the choice to do X:
//!   - a. declined at the accept prompt: kept (no refund step needed);
//!   - b. accepted, X was possible, the player chose not to do X: KEPT
//!     (official Q1818, Q4086, Q6946) -- the refund is right;
//!   - W13. "you may A. Then, you may B." with both halves declined: KEPT
//!     (owner decision W13, 2026-10-21) -- the refund is right;
//!   - c. the player chose X, whatever then happened to it: SPENT (CR 4.3
//!     15-14-1-4, official Q5426) -- a refund is wrong;
//!   - d. nothing was possible but the effect was offered: SPENT (CR 4.3
//!     15-14-1-4); where DCGO does not offer the effect in that state the
//!     clause mirrors that with a `condition:` instead -- a refund is wrong;
//!   - e. mandatory content ran, or an opponent's choice was processed:
//!     SPENT (official Q4739, Q5056, Q5171, Q5330) -- a refund is wrong;
//!   - f. accepted, then the optional cost of a "by X, ..." effect was
//!     declined: KEPT on the closed list [`COST_DECLINED_BY_OWNER_RULING`]
//!     by owner rulings of 2026-11-23 ("parity with DCGO": Slayerdramon
//!     (EX13-024) and ten clauses where DCGO `RemoveUse()`s a declined cost)
//!     -- the refund is right there; on every other card the current
//!     behaviour (spent) stands until the owner rules, so a refund is wrong.
//!   So every such clause is listed in [`REFUND_AFTER_ACCEPT_REVIEWED`] with
//!   the row its refund path is reached on (b, W13, or f by owner ruling;
//!   [`RefundRow`] has no variant for c, d or e), why it is reached on no
//!   other, and the behavioural tests that prove it. An unlisted clause is an
//!   error, and the
//!   pool guards fail on an entry whose clause no longer refunds behind an
//!   accept ([`stale_refund_reviews`]) or whose named test is gone. The table
//!   is a review record, not a check of the row: the lint cannot see which
//!   path reaches a `refund_opt`, so the named tests carry the proof.
//!   Erlangmon (EX12-034) once refunded on its nothing-playable path (row d)
//!   and was used twice in one turn in logged games.
//!   A declared [Main] is not an unprinted accept. Its use accounting follows
//!   the card's DCGO script, by the owner ruling of 2026-11-24 ("Match dcgo
//!   on the main use"; that this covers every per-turn [Main] path by path is
//!   the orchestrator's interpretation): where DCGO `RemoveUse()`s a declared
//!   [Main] that did nothing, the use is kept; everywhere else it is spent.
//!   The owner chose DCGO parity over the CR 4.3 15-8-4-1/-2, 15-14-1-4
//!   reading, reversing the 2026-10-25 record (docs/RUST_ENGINE_GAPS.md). A
//!   [Main] refund is a reviewed entry on [`RefundRow::MainPassedByOwnerRuling`]
//!   and only for the closed list [`MAIN_REFUND_BY_OWNER_RULING`]; the
//!   engine's main-declaration loop guard keeps the kept use from being a
//!   free repeatable no-op (docs/RUST_ENGINE_API.md "Main-declaration guard").

use serde_yml::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoringFinding {
    pub rule: &'static str,
    pub path: String,
    pub message: String,
}

const SELF_TIMINGS: &[&str] = &["on_play", "when_digivolving", "when_attacking", "on_deletion"];
/// Activation-type timings: the player activates the effect by declaring it.
const DECLARED_TIMINGS: &[&str] = &["main", "main_on_field", "main_from_hand", "main_from_trash", "counter"];

/// Why a clause mapping's activation is an explicit choice, if it is.
fn explicit_accept(clause: &serde_yml::Mapping) -> Option<String> {
    let is_true = |k: &str| clause.get(k).and_then(Value::as_bool) == Some(true);
    if clause.get("kind").and_then(Value::as_str) == Some("replacement") && is_true("optional") {
        return Some("an optional replacement (accepted at its \"May accept replacement\" prompt)".into());
    }
    if is_true("outer_prompt") {
        return Some("an `outer_prompt: true` clause (accepted at its outer prompt)".into());
    }
    let timings: Vec<&str> = match clause.get("when") {
        Some(Value::String(s)) => vec![s.as_str()],
        Some(Value::Sequence(items)) => items.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };
    timings
        .into_iter()
        .find(|t| DECLARED_TIMINGS.contains(t))
        .map(|t| format!("a `{t}` clause (activation-type: activated by declaring it)"))
}

fn contains_key(value: &Value, wanted: &str) -> bool {
    match value {
        Value::Mapping(map) => map
            .iter()
            .any(|(k, v)| k.as_str() == Some(wanted) || contains_key(v, wanted)),
        Value::Sequence(items) => items.iter().any(|v| contains_key(v, wanted)),
        _ => false,
    }
}
const OBSERVER_TIMINGS: &[&str] = &[
    "on_digivolve",
    "on_enter_field_anyone",
    "on_any_digimon_played",
    "on_ally_played",
    "on_ally_attack",
    "on_opponent_attack",
    "on_any_deletion",
];

/// The rows of the [Once Per Turn] table (qa/rules/mechanics-gaps-2026-10.md
/// W14) on which a refund behind an explicit accept is right. Rows c, d and e
/// spend the use, so they have no variant: a clause that refunds on them
/// cannot be listed. Row f is right only by a per-card owner ruling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefundRow {
    /// b: accepted, the printed "you may X" was possible, and the player chose
    /// not to do X (official Q1818, Q4086, Q6946).
    DeclinedWhilePossible,
    /// W13: "you may A. Then, you may B." with both halves declined
    /// (owner decision 2026-10-21).
    BothHalvesDeclined,
    /// f: the optional cost of a "by X, ..." effect was declined after the
    /// accept, on a clause the owner has ruled keeps the use then. Only the
    /// clauses in [`COST_DECLINED_BY_OWNER_RULING`] (owner rulings
    /// 2026-11-23: Slayerdramon (EX13-024), then "parity with DCGO" for ten
    /// more); on every other card a declined "by X" cost spends the use
    /// today, and none may be listed on this row without its own ruling.
    CostDeclinedByOwnerRuling,
    /// A declared [Main] [Once Per Turn] whose pick was passed (or had no
    /// candidate), on a clause whose DCGO script `RemoveUse()`s that path:
    /// KEPT by the owner ruling of 2026-11-24 ("Match dcgo on the main use").
    /// Only the clauses in [`MAIN_REFUND_BY_OWNER_RULING`]; a [Main] whose
    /// DCGO script has no `RemoveUse()` on the path keeps spending.
    MainPassedByOwnerRuling,
}

/// The closed list of `(card, clause)` the owner has ruled onto row f: a
/// declined "by X" cost after the accept keeps the [Once Per Turn] there.
/// Owner rulings of 2026-11-23 ("keep parity with dcgo"): Slayerdramon
/// (EX13-024), whose DCGO script `RemoveUse()`s an unpaid suspend, and the
/// ten clauses below, where DCGO likewise lets the cost be declined after the
/// accept and `RemoveUse()`s it. It is not a pool-wide rule: Ogremon (X
/// Antibody) (P-250), Hyogamon (BT24-026), MetalEtemon (EX5-054), Apocalymon
/// (BT15-102) and DemiMeramon (EX8-006) keep "a declined cost spends the
/// use", which DCGO does there too. A new card needs its own ruling before it
/// is added here (qa/rules/mechanics-gaps-2026-10.md W14 row (f)).
pub const COST_DECLINED_BY_OWNER_RULING: &[(&str, &str)] = &[
    ("EX13-024", "effects[3]"),
    ("EX13-024", "effects[4]"),
    ("BT25-084", "effects[1]"),
    ("BT26-044", "effects[2]"),
    ("EX13-015", "effects[4]"),
    ("EX13-027", "effects[1]"),
    ("EX13-028", "effects[2]"),
    ("EX13-032", "effects[0]"),
    ("EX13-048", "effects[1]"),
    ("EX13-052", "effects[2]"),
    ("EX13-059", "effects[1]"),
    ("EX13-059", "effects[2]"),
];

/// The closed list of declared [Main] refunds: owner ruling 2026-11-24,
/// "Match dcgo on the main use" (DCGO parity; that it covers every per-turn
/// [Main] path by path is the orchestrator's interpretation). Each clause's
/// DCGO script `RemoveUse()`s the declared [Main] when it did nothing
/// (`if (!isUsed)` / `if (!digivolved)`), and the clause refunds on exactly
/// that path. A [Main] whose DCGO script has no `RemoveUse()` there (Logimon
/// (BT25-052), Logamon (BT25-070), Greymon (BT23-008), ...) keeps spending
/// and must not be listed; NightChiropmon (BT26-070) and Richard Sampson
/// (EX13-071) refund in DCGO only on a cancelled cost pick that ours does not
/// surface (declaring and cancelling is the same as not declaring), so they
/// are not listed.
pub const MAIN_REFUND_BY_OWNER_RULING: &[(&str, &str)] = &[
    ("BT26-012", "effects[0]"),
    ("BT26-021", "effects[1]"),
    ("EX12-013", "effects[0]"),
    ("EX12-041", "effects[0]"),
    ("EX12-043", "effects[0]"),
    ("EX13-074", "effects[2]"),
];

/// One reviewed `refund_opt` behind an explicit accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewedRefund {
    pub card: &'static str,
    /// The clause's path in the spec, as the lint reports it without the
    /// trailing `.process` (`effects[5]`).
    pub clause: &'static str,
    pub row: RefundRow,
    /// Why the refund path is reached on `row` and on no spending row.
    pub reason: &'static str,
    /// The behavioural tests that carry the proof, each as
    /// `<file under code/digimon-engine/tests>::<test fn>`: the listed row
    /// keeps the use, choosing X spends it (row c), and the nothing-possible
    /// state spends it or is not offered (row d). Where one half is always
    /// possible, so that no row-d state exists, `reason` says which.
    pub tests: &'static [&'static str],
}

/// Every production clause with a `refund_opt` behind an explicit accept,
/// reviewed against the [Once Per Turn] table.
///
/// This is a review record, not a proof. The lint sees that a clause refunds
/// behind an accept; it cannot see which path reaches the refund, so a clause
/// that also refunds on a spending row (an unconditional `else: refund_opt`
/// that runs when nothing was possible, row d) could still be listed as row
/// b. What proves an entry is its `tests`: behavioural tests for the kept
/// row, for row c and for the row-d state. The pool guards fail on a new
/// unlisted clause (the lint), on a stale entry ([`stale_refund_reviews`])
/// and, in `cards_behavioral`, on a named test that no longer exists.
pub const REFUND_AFTER_ACCEPT_REVIEWED: &[ReviewedRefund] = &[
    ReviewedRefund {
        card: "AD1-024",
        clause: "effects[3]",
        row: RefundRow::BothHalvesDeclined,
        reason: concat!(
            "Imperialdramon: Fighter Mode prints \"you may suspend 1 of your opponent's Digimon and ",
            "unsuspend this Digimon. Then, if played or digivolved by effects, you may return ...\": ",
            "W13. Its outer_prompt is DCGO's unprinted skippable Yes/No (AD1_024.cs SetIsSkippable). ",
            "The refund needs every pick absent, so no half executed, and an opponent Digimon or this ",
            "Digimon being suspended, so something was possible: with neither, DCGO still offers the ",
            "effect and accepting it spends the use (row d)."
        ),
        tests: &[
            "cards_behavioral/ad1/ad1_024.rs::ad1_024_all_turns_accept_then_decline_everything_keeps_opt",
            "cards_behavioral/ad1/ad1_024.rs::ad1_024_all_turns_is_once_per_turn",
            "cards_behavioral/ad1/ad1_024.rs::ad1_024_all_turns_accepted_with_nothing_possible_spends_opt",
        ],
    },
    ReviewedRefund {
        card: "BT25-103",
        clause: "effects[4]",
        row: RefundRow::BothHalvesDeclined,
        reason: concat!(
            "GraceNovamon [When Attacking] [Counter] \"you may trash ... Then, you may end this attack\": W13, ",
            "which the owner decision names (\"DCGO has the Dianamon/GraceNovamon effect correct\"); one clause ",
            "for both timings (Q1550), as DCGO BT25_103.cs's one shared effect. The refund needs nothing trashed ",
            "AND \"Don't end the attack\", so no half executed. No row-d state exists: the end-attack half is ",
            "offered even with nothing to trash.",
        ),
        tests: &[
            "cards_behavioral/bt25/bt25_103.rs::bt25_103_when_attacking_declining_everything_keeps_opt",
            "cards_behavioral/bt25/bt25_103.rs::bt25_103_counter_declining_everything_keeps_opt",
            "cards_behavioral/bt25/bt25_103.rs::bt25_103_when_attacking_trashing_spends_opt_for_counter",
            "cards_behavioral/bt25/bt25_103.rs::bt25_103_ending_attack_alone_spends_opt",
            "cards_behavioral/bt25/bt25_103.rs::bt25_103_no_sources_on_self_skips_trash_still_offers_end",
        ],
    },
    ReviewedRefund {
        card: "BT26-055",
        clause: "effects[1]",
        row: RefundRow::BothHalvesDeclined,
        reason: concat!(
            "Giromon [On Play] [When Digivolving] [Counter] \"You may place ... Then, you may delete ...\": W13 at ",
            "every timing (Q1550; DCGO BT26_055.cs one shared effect). The refund needs nothing placed and ",
            "\"Don't delete\". No row-d state exists: the delete half is offered even with an empty hand and no ",
            "opponent Digimon, because Giromon is itself [Ver.3].",
        ),
        tests: &[
            "cards_behavioral/bt26/bt26_055.rs::bt26_055_doing_nothing_refunds_once_per_turn",
            "cards_behavioral/bt26/bt26_055.rs::bt26_055_counter_doing_nothing_keeps_once_per_turn",
            "cards_behavioral/bt26/bt26_055.rs::bt26_055_counter_placing_spends_once_per_turn_for_every_timing",
            "cards_behavioral/bt26/bt26_055.rs::bt26_055_empty_hand_still_offers_the_delete_half",
        ],
    },
    ReviewedRefund {
        card: "BT26-060",
        clause: "effects[5]",
        row: RefundRow::DeclinedWhilePossible,
        reason: concat!(
            "Chronomon: Destroy Mode prints \"you may delete 1 of your opponent's Digimon\"; the outer prompt is ",
            "DCGO's skippable Yes/No. The refund runs only when the delete pick is passed; the clause's ",
            "`condition:` (an opponent Digimon exists, DCGO CanActivateCondition) means a target was always ",
            "possible (row b, Q1818) and the effect is not offered without one (row d). A chosen target spends ",
            "it (row c).",
        ),
        tests: &[
            "cards_behavioral/bt26/bt26_060.rs::bt26_060_accepted_then_declined_delete_keeps_the_once_per_turn",
            "cards_behavioral/bt26/bt26_060.rs::bt26_060_accepted_and_delete_chosen_spends_the_once_per_turn",
            "cards_behavioral/bt26/bt26_060.rs::bt26_060_not_offered_when_the_opponent_has_no_digimon",
        ],
    },
    ReviewedRefund {
        card: "EX12-017",
        clause: "effects[3]",
        row: RefundRow::BothHalvesDeclined,
        reason: concat!(
            "WarGreymon [Counter] \"2 of your Digimon may DNA digivolve ... Then, you may change the attack ",
            "target\": W13 (DCGO EX12_017.cs RemoveUse when neither happened). The refund needs both the DNA ",
            "pair and the redirect pick absent. No row-d state exists: the redirect is offered even without a ",
            "DNA pair, because WarGreymon is itself a candidate.",
        ),
        tests: &[
            "cards_behavioral/ex12/ex12_017.rs::ex12_017_counter_both_halves_declined_keeps_the_once_per_turn",
            "cards_behavioral/ex12/ex12_017.rs::ex12_017_counter_redirect_chosen_spends_the_once_per_turn",
            "cards_behavioral/ex12/ex12_017.rs::redirect_is_offered_even_without_a_dna",
        ],
    },
    ReviewedRefund {
        card: "EX12-034",
        clause: "effects[2]",
        row: RefundRow::DeclinedWhilePossible,
        reason: concat!(
            "Erlangmon prints \"you may play 1 level 5 or lower [SW] ... card\"; the replacement's accept is DCGO's ",
            "skippable prompt. The refund runs only on \"Don't play\" while a zone held a playable card (row b, ",
            "Q1818); with nothing playable it does not run and the use is spent (row d: DCGO offers the effect ",
            "and its RemoveUse sits inside `if (validHandCard || validDigivolutionCard)`).",
        ),
        tests: &[
            "cards_behavioral/ex12/ex12_034.rs::ex12_034_leave_accepted_then_dont_play_keeps_once_per_turn",
            "cards_behavioral/ex12/ex12_034.rs::ex12_034_leave_accepted_and_card_played_spends_once_per_turn",
            "cards_behavioral/ex12/ex12_034.rs::ex12_034_leave_with_no_playable_card_plays_nothing",
        ],
    },
    ReviewedRefund {
        card: "EX13-024",
        clause: "effects[3]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Slayerdramon \"by suspending 1 of your such Digimon, they don't leave\": owner ruling 2026-11-23 ",
            "(qa/rules/mechanics-gaps-2026-10.md W14), for this card only: the suspend declined keeps the use ",
            "(DCGO EX13_024.cs RemoveUse). The replacement is offered only while a payer exists (`active_when`), ",
            "so the refund runs only on a decline while the suspend was possible; a suspended Digimon spends it.",
        ),
        tests: &[
            "ex13_024_plural_prevention.rs::face_cost_picker_decline_keeps_opt_and_clones",
            "ex13_024_plural_prevention.rs::successful_face_payment_consumes_opt_with_another_payer_available",
            "cards_behavioral/ex13/ex13_024.rs::ex13_024_no_unsuspended_text_digimon_means_no_offer",
        ],
    },
    ReviewedRefund {
        card: "EX13-024",
        clause: "effects[4]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Slayerdramon's inherited copy of effects[3]: same ruling, separate [Once Per Turn].",
        ),
        tests: &[
            "ex13_024_plural_prevention.rs::inherited_cost_picker_decline_keeps_opt_and_clones",
            "ex13_024_plural_prevention.rs::successful_inherited_payment_consumes_opt_with_another_payer_available",
            "cards_behavioral/ex13/ex13_024.rs::ex13_024_inherited_no_unsuspended_text_digimon_means_no_offer",
        ],
    },
    ReviewedRefund {
        card: "BT25-084",
        clause: "effects[1]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Titamon \"by trashing 2 cards in your hand, it doesn't leave\": owner ruling 2026-11-23, parity with DCGO ",
            "(BT25_084.cs: 2 cards or none, RemoveUse when fewer than 2 were discarded). Offered only with 2 cards in ",
            "hand (`active_when`, DCGO CanActivateCondition), so the refund runs only on a decline of a payable cost; ",
            "once the first card is chosen the second pick is mandatory (Q6398), and 2 trashed spends the use.",
        ),
        tests: &[
            "cards_behavioral/bt25/bt25_084.rs::bt25_084_leave_prevention_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/bt25/bt25_084.rs::bt25_084_leave_prevention_paid_spends_opt_and_second_pick_is_mandatory",
            "cards_behavioral/bt25/bt25_084.rs::bt25_084_leave_prevention_not_offered_with_one_hand_card",
        ],
    },
    ReviewedRefund {
        card: "BT26-044",
        clause: "effects[2]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Lilamon's inherited \"by trashing the bottom face-down card from under any of your Tamers, it doesn't ",
            "leave\": owner ruling 2026-11-23, parity with DCGO (BT26_044.cs: canNoSelect Tamer pick, RemoveUse when ",
            "none chosen). Offered only while a Tamer has a face-down card (the step's 15-7-4 guard), so the refund ",
            "runs only on a declined Tamer pick; a chosen Tamer spends the use.",
        ),
        tests: &[
            "cards_behavioral/bt26/bt26_044.rs::bt26_044_inherited_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/bt26/bt26_044.rs::bt26_044_inherited_paid_spends_opt",
            "cards_behavioral/bt26/bt26_044.rs::bt26_044_inherited_not_offered_without_a_face_down_tamer_card",
        ],
    },
    ReviewedRefund {
        card: "EX13-015",
        clause: "effects[4]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Gallantmon \"by deleting 1 of your opponent's 9000 DP or lower Digimon, it doesn't leave\": owner ruling ",
            "2026-11-23, parity with DCGO (EX13_015.cs: canNoSelect pick, RemoveUse unless a Digimon was chosen). ",
            "Offered only while such a Digimon exists (leading-pick preflight), so the refund runs only on a declined ",
            "pick; a chosen Digimon spends the use even when its deletion is prevented (Q7247: it then leaves).",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_015.rs::ex13_015_replacement_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_015.rs::ex13_015_replacement_is_once_per_turn",
            "cards_behavioral/ex13/ex13_015.rs::ex13_015_replacement_prevented_payment_spends_opt",
            "cards_behavioral/ex13/ex13_015.rs::ex13_015_no_9000_or_lower_target_means_it_leaves",
        ],
    },
    ReviewedRefund {
        card: "EX13-027",
        clause: "effects[1]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Chuumon's inherited \"by deleting 1 other Digimon with [Sukamon] in its name, it doesn't leave\": owner ",
            "ruling 2026-11-23, parity with DCGO (EX13_027.cs: canNoSelect pick over either battle area, Q7287; ",
            "RemoveUse unless a Digimon was chosen). Offered only while such a Digimon exists (leading-pick ",
            "preflight), so the refund runs only on a declined pick; a chosen Digimon spends the use.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_027.rs::ex13_027_inherited_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_027.rs::ex13_027_inherited_once_per_turn_and_resets",
            "cards_behavioral/ex13/ex13_027.rs::ex13_027_inherited_not_offered_without_other_sukamon",
        ],
    },
    ReviewedRefund {
        card: "EX13-028",
        clause: "effects[2]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Sukamon's inherited copy of Chuumon's clause: owner ruling 2026-11-23, parity with DCGO (EX13_028.cs, ",
            "same body as EX13_027.cs; Q7288). Offered only while another [Sukamon]-named Digimon exists, so the ",
            "refund runs only on a declined pick; a chosen Digimon spends the use.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_028.rs::ex13_028_inherited_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_028.rs::ex13_028_inherited_once_per_turn",
            "cards_behavioral/ex13/ex13_028.rs::ex13_028_inherited_not_offered_without_other_sukamon",
        ],
    },
    ReviewedRefund {
        card: "EX13-032",
        clause: "effects[0]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Chirinmon \"by trashing your top security card or the bottom face-down card from under any of your ",
            "Tamers\": owner ruling 2026-11-23, parity with DCGO (EX13_032.cs: one choice of each payable cost plus ",
            "\"Don't pay the cost\", a canNoSelect Tamer pick, RemoveUse unless a cost was paid). Offered only while a ",
            "cost is payable (`condition`, DCGO AdditionalActivateCondition), so both refunds run only on a decline of ",
            "a payable cost; a paid cost spends the use.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_032.rs::ex13_032_dont_pay_keeps_opt_and_does_nothing",
            "cards_behavioral/ex13/ex13_032.rs::ex13_032_tamer_cost_declined_at_the_tamer_pick_keeps_opt",
            "cards_behavioral/ex13/ex13_032.rs::ex13_032_paid_cost_spends_opt",
            "cards_behavioral/ex13/ex13_032.rs::ex13_032_no_payable_cost_does_not_offer_the_clause",
        ],
    },
    ReviewedRefund {
        card: "EX13-048",
        clause: "effects[1]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Kotemon's inherited \"by deleting 1 of your other Digimon with [Knightmon] in its text, it doesn't ",
            "leave\": owner ruling 2026-11-23, parity with DCGO (EX13_048.cs: canNoSelect pick, RemoveUse unless a ",
            "Digimon was chosen). Offered only while such a Digimon exists, so the refund runs only on a declined ",
            "pick; a chosen Digimon spends the use.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_048.rs::ex13_048_inherited_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_048.rs::ex13_048_inherited_once_per_turn",
            "cards_behavioral/ex13/ex13_048.rs::ex13_048_inherited_not_offered_without_other_knightmon_text_digimon",
        ],
    },
    ReviewedRefund {
        card: "EX13-052",
        clause: "effects[2]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "Gladimon's inherited copy of Kotemon's clause: owner ruling 2026-11-23, parity with DCGO (EX13_052.cs: ",
            "canNoSelect pick, RemoveUse when none chosen). Offered only while another of your [Knightmon]-text ",
            "Digimon exists, so the refund runs only on a declined pick; a chosen Digimon spends the use.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_052.rs::ex13_052_inherited_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_052.rs::ex13_052_inherited_once_per_turn",
            "cards_behavioral/ex13/ex13_052.rs::ex13_052_inherited_not_offered_without_other_knightmon",
        ],
    },
    ReviewedRefund {
        card: "EX13-059",
        clause: "effects[1]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "BigMamemon [End of Your Turn] \"by deleting 1 of your Digimon with [Mamemon] in its name, delete 1 of ",
            "your opponent's Digimon with the lowest play cost\": owner ruling 2026-11-23, parity with DCGO ",
            "(EX13_059.cs: canNoSelect pick, RemoveUse when none chosen, reward only on a deletion). Offered only while ",
            "a [Mamemon] of yours exists (`condition`); face-up, BigMamemon itself always can pay, so no unpayable ",
            "state exists. A chosen Digimon spends the use, also when its deletion is prevented.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_eot_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_eot_is_once_per_turn_and_resets_next_turn",
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_eot_prevented_payment_spends_opt_without_reward",
        ],
    },
    ReviewedRefund {
        card: "EX13-059",
        clause: "effects[2]",
        row: RefundRow::CostDeclinedByOwnerRuling,
        reason: concat!(
            "BigMamemon's inherited [End of Your Turn] (delete all lowest-play-cost Digimon): same ruling and DCGO ",
            "flow as effects[1], separate [Once Per Turn] (`EX13_059_ESS_EOT`). Not offered without a [Mamemon] of yours.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_inherited_eot_accept_then_decline_cost_keeps_opt",
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_inherited_eot_paid_spends_opt",
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_inherited_eot_prevented_payment_spends_opt_without_reward",
            "cards_behavioral/ex13/ex13_059.rs::ex13_059_inherited_eot_not_offered_without_a_mamemon",
        ],
    },
    ReviewedRefund {
        card: "BT26-012",
        clause: "effects[0]",
        row: RefundRow::MainPassedByOwnerRuling,
        reason: concat!(
            "Manekimon [Main] [Once Per Turn] \"You may play or use 1 [TB] trait card from your hand with the cost reduced ",
            "by 2\": DCGO BT26_012.cs:138 `if (!isUsed) activateClass.RemoveUse();`, isUsed set at :81 once a card is ",
            "picked, before the play. The refund is the `else` of `binding_present: pick`, so it runs only when no card was ",
            "picked; a picked card spends the use whatever the play does. Not offered without a playable [TB] card (DCGO ",
            "CanUseCondition, same filter as the pick).",
        ),
        tests: &[
            "cards_behavioral/bt26/bt26_012.rs::bt26_012_main_declared_then_pick_passed_keeps_the_once_per_turn",
            "cards_behavioral/bt26/bt26_012.rs::bt26_012_main_plays_tb_digimon_with_cost_reduced_by_2",
            "cards_behavioral/bt26/bt26_012.rs::bt26_012_main_pass_then_state_change_redeclares_and_spends",
            "cards_behavioral/bt26/bt26_012.rs::bt26_012_main_unavailable_without_tb_card",
        ],
    },
    ReviewedRefund {
        card: "BT26-021",
        clause: "effects[1]",
        row: RefundRow::MainPassedByOwnerRuling,
        reason: concat!(
            "Gekomon [Main] [Once Per Turn] \"You may play 1 [TS] trait Tamer card from your trash with the cost reduced by ",
            "2\": DCGO BT26_021.cs:184 `if (!isUsed) activateClass.RemoveUse();`, isUsed set at :138 once a card is chosen, ",
            "before the play. The refund is the `then` of `binding_absent: tamer_pick`. Not offered without a [TS] Tamer in ",
            "trash.",
        ),
        tests: &[
            "cards_behavioral/bt26/bt26_021.rs::bt26_021_main_pass_refunds_once_per_turn",
            "cards_behavioral/bt26/bt26_021.rs::bt26_021_main_plays_ts_tamer_from_trash_reduced_once_per_turn",
            "cards_behavioral/bt26/bt26_021.rs::bt26_021_main_pass_then_state_change_redeclares_and_spends",
            "cards_behavioral/bt26/bt26_021.rs::bt26_021_main_needs_ts_tamer_in_trash",
        ],
    },
    ReviewedRefund {
        card: "EX12-013",
        clause: "effects[0]",
        row: RefundRow::MainPassedByOwnerRuling,
        reason: concat!(
            "BetelGammamon [Main] [Once Per Turn] \"You may play or use 1 card with [Gammamon] in its text or the [VB] ",
            "trait from your hand with the cost reduced by 2\": DCGO EX12_013.cs:181 `if (!isUsed) ",
            "activateClass.RemoveUse();`, isUsed set at :97 once a card is picked. The refund is the `else` of ",
            "`binding_present: vb_pick`. Not offered without a playable candidate (CanUseCondition).",
        ),
        tests: &[
            "cards_behavioral/ex12/ex12_013.rs::ex12_013_main_declared_then_pick_passed_keeps_the_once_per_turn",
            "cards_behavioral/ex12/ex12_013.rs::ex12_013_main_pass_then_state_change_redeclares_and_spends",
            "cards_behavioral/ex12/ex12_013.rs::ex12_013_main_not_activatable_with_only_unplayable_candidates",
        ],
    },
    ReviewedRefund {
        card: "EX12-041",
        clause: "effects[0]",
        row: RefundRow::MainPassedByOwnerRuling,
        reason: concat!(
            "Thundermon [Main] [Once Per Turn] \"You may play or use 1 card with the [Mutant] or [ME] trait from your hand ",
            "with the cost reduced by 2\": DCGO EX12_041.cs:180 `if (!isUsed) activateClass.RemoveUse();`, isUsed set at ",
            ":96 once a card is picked. The refund is the `else` of `binding_present: pick`. Not offered without a playable ",
            "candidate (CanUseCondition).",
        ),
        tests: &[
            "cards_behavioral/ex12/me_machine_puppet.rs::ex12_041_main_declared_then_pick_passed_keeps_the_once_per_turn",
            "cards_behavioral/ex12/me_machine_puppet.rs::ex12_041_main_pick_chosen_spends_the_once_per_turn",
            "cards_behavioral/ex12/me_machine_puppet.rs::ex12_041_main_pass_then_state_change_redeclares_and_spends",
            "cards_behavioral/ex12/me_machine_puppet.rs::ex12_041_main_never_offers_a_card_without_the_mutant_or_me_trait",
        ],
    },
    ReviewedRefund {
        card: "EX12-043",
        clause: "effects[0]",
        row: RefundRow::MainPassedByOwnerRuling,
        reason: concat!(
            "Hakubamon [Main] [Once Per Turn] \"You may play or use 1 card with the [SW] trait from your hand with the cost ",
            "reduced by 2\": DCGO EX12_043.cs:179 `if (!isUsed) activateClass.RemoveUse();`, isUsed set at :95 once a card ",
            "is picked. The refund is the `else` of `binding_present: sw_pick`. Not offered without a playable candidate ",
            "(CanUseCondition).",
        ),
        tests: &[
            "cards_behavioral/ex12/ex12_043.rs::ex12_043_main_declared_then_pick_passed_keeps_the_once_per_turn",
            "cards_behavioral/ex12/ex12_043.rs::ex12_043_main_pass_then_state_change_redeclares_and_spends",
            "cards_behavioral/ex12/ex12_043.rs::ex12_043_main_not_activatable_with_only_unplayable_candidates",
        ],
    },
    ReviewedRefund {
        card: "EX13-074",
        clause: "effects[2]",
        row: RefundRow::MainPassedByOwnerRuling,
        reason: concat!(
            "Rie Kishibe [Main] [Once Per Turn] \"... it may digivolve into [LordKnightmon] in the hand or trash ...\": ",
            "DCGO EX13_074.cs RemoveUse()s on \"Don't digivolve\" (:218-222) and `if (!digivolved)` (:245). The refund is ",
            "the `else` of `binding_present: lordknightmon`. DCGO does not offer the [Main] without a [LordKnightmon] in ",
            "hand or trash; ours does (the printed condition), and the empty pick then refunds, so the use is uncounted on ",
            "that path in both engines.",
        ),
        tests: &[
            "cards_behavioral/ex13/ex13_074.rs::ex13_074_main_may_decline",
            "cards_behavioral/ex13/ex13_074.rs::ex13_074_main_is_once_per_turn",
            "cards_behavioral/ex13/ex13_074.rs::ex13_074_main_decline_then_state_change_redeclares_and_spends",
            "cards_behavioral/ex13/ex13_074.rs::ex13_074_main_without_lordknightmon_keeps_the_once_per_turn",
        ],
    },
];

/// Whether `finding` is the reviewed refund `entry` of `card`.
fn reviewed_by(card: &str, finding: &AuthoringFinding, entry: &ReviewedRefund) -> bool {
    finding.rule == "refund-after-explicit-accept"
        && entry.card == card
        && finding.path.strip_suffix(".process").unwrap_or(&finding.path) == entry.clause
}

fn card_id(value: &Value) -> Option<&str> {
    value.as_mapping()?.get("card")?.as_str()
}

/// Lint one card's raw YAML text, ignoring [`REFUND_AFTER_ACCEPT_REVIEWED`].
pub fn check_authoring_yaml_unexempted(raw: &str) -> Vec<AuthoringFinding> {
    let mut out = Vec::new();
    if let Ok(value) = serde_yml::from_str::<Value>(raw) {
        walk(&value, "", &mut out);
    }
    out
}

/// Lint one card's raw YAML text against a reviewed-refund table.
pub fn check_authoring_yaml_with(raw: &str, reviewed: &[ReviewedRefund]) -> Vec<AuthoringFinding> {
    let mut out = check_authoring_yaml_unexempted(raw);
    let card = serde_yml::from_str::<Value>(raw)
        .ok()
        .and_then(|v| card_id(&v).map(str::to_string));
    if let Some(card) = card {
        out.retain(|f| !reviewed.iter().any(|entry| reviewed_by(&card, f, entry)));
    }
    out
}

/// Lint one card's raw YAML text. A file that does not parse yields no
/// findings here (the loader reports the parse error).
pub fn check_authoring_yaml(raw: &str) -> Vec<AuthoringFinding> {
    check_authoring_yaml_with(raw, REFUND_AFTER_ACCEPT_REVIEWED)
}

/// Whether the spec text `raw` still has the refund `entry` reviews.
pub fn refund_review_is_live(raw: &str, entry: &ReviewedRefund) -> bool {
    check_authoring_yaml_unexempted(raw)
        .iter()
        .any(|f| reviewed_by(entry.card, f, entry))
}

/// The clauses (`effects[i]`) of the spec text `raw` that put a `refund_opt`
/// in a [Main] clause (any `main*` timing), reviewed or not.
pub fn main_refund_clauses(raw: &str) -> Vec<String> {
    check_authoring_yaml_unexempted(raw)
        .into_iter()
        .filter(|f| f.rule == "refund-after-explicit-accept" && f.message.contains("a `main"))
        .map(|f| f.path.strip_suffix(".process").unwrap_or(&f.path).to_string())
        .collect()
}

/// Every `(card, clause)` in the production pool with a [Main] refund, sorted.
pub fn pool_main_refunds(cards_root: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for file in pool_card_files(cards_root) {
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Some(card) = serde_yml::from_str::<Value>(&raw).ok().and_then(|v| card_id(&v).map(str::to_string)) else {
            continue;
        };
        for clause in main_refund_clauses(&raw) {
            out.push((card.clone(), clause));
        }
    }
    out.sort();
    out
}

/// Entries of `reviewed` whose card spec under `cards_root` no longer has a
/// `refund_opt` behind an explicit accept at the listed clause; one line each.
pub fn stale_refund_reviews_with(cards_root: &std::path::Path, reviewed: &[ReviewedRefund]) -> Vec<String> {
    let files = pool_card_files(cards_root);
    reviewed
        .iter()
        .filter(|entry| {
            !files
                .iter()
                .filter(|f| f.file_stem().and_then(|s| s.to_str()) == Some(entry.card))
                .filter_map(|f| std::fs::read_to_string(f).ok())
                .any(|raw| refund_review_is_live(&raw, entry))
        })
        .map(|entry| format!("{} {} ({:?})", entry.card, entry.clause, entry.row))
        .collect()
}

/// [`stale_refund_reviews_with`] over [`REFUND_AFTER_ACCEPT_REVIEWED`].
pub fn stale_refund_reviews(cards_root: &std::path::Path) -> Vec<String> {
    stale_refund_reviews_with(cards_root, REFUND_AFTER_ACCEPT_REVIEWED)
}

/// Every production card spec under `cards_root` (`code/digimon-engine/cards`), sorted. `_examples/` holds DSL
/// documentation samples and `test/` synthetic engine fixtures; neither is a printed card. For the pool guards.
pub fn pool_card_files(cards_root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(sets) = std::fs::read_dir(cards_root) else {
        return files;
    };
    for set in sets.flatten() {
        let set = set.path();
        let name = set.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        if !set.is_dir() || name.starts_with('_') || name == "test" {
            continue;
        }
        let Ok(cards) = std::fs::read_dir(&set) else {
            continue;
        };
        for card in cards.flatten() {
            let card = card.path();
            if card.extension().and_then(|e| e.to_str()) == Some("yaml") {
                files.push(card);
            }
        }
    }
    files.sort();
    files
}

/// Run [`check_authoring_yaml`] over every production card spec; one line per finding.
pub fn check_pool(cards_root: &std::path::Path) -> Vec<String> {
    let mut findings = Vec::new();
    for file in pool_card_files(cards_root) {
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        for finding in check_authoring_yaml(&raw) {
            findings.push(format!(
                "{} [{}] {}: {}",
                file.file_name().and_then(|n| n.to_str()).unwrap_or_default(),
                finding.rule,
                finding.path,
                finding.message
            ));
        }
    }
    findings
}

fn zone_includes_battle_area(zone: Option<&Value>) -> bool {
    match zone {
        Some(Value::String(s)) => s == "battle_area",
        Some(Value::Sequence(items)) => items
            .iter()
            .any(|item| matches!(item, Value::String(s) if s == "battle_area")),
        _ => false,
    }
}

fn walk(value: &Value, path: &str, out: &mut Vec<AuthoringFinding>) {
    match value {
        Value::Mapping(map) => {
            if let (Some(process), Some(why)) = (map.get("process"), explicit_accept(map)) {
                if contains_key(process, "refund_opt") {
                    out.push(AuthoringFinding {
                        rule: "refund-after-explicit-accept",
                        path: if path.is_empty() { "process".into() } else { format!("{path}.process") },
                        message: format!(
                            "unreviewed `refund_opt` in {why}. Check which path reaches it against the [Once Per Turn] table (qa/rules/mechanics-gaps-2026-10.md W14): a refund is right only when the player chose not to do a printed \"you may X\" that was possible (row b: Q1818, Q4086, Q6946) or declined both halves of \"you may A. Then, you may B.\" (W13); it is wrong once the player chose X (row c: CR 4.3 15-14-1-4, Q5426), when nothing was possible but the effect was offered (row d), after mandatory content or an opponent's choice (row e: Q4739, Q5056), or when the declined thing is the optional cost of a \"by X, ...\" effect (row f; kept only by a per-card owner ruling). List the clause in REFUND_AFTER_ACCEPT_REVIEWED with its row, reason and tests, or gate or drop the refund"
                        ),
                    });
                }
            }
            for (key, child) in map {
                let Some(key) = key.as_str() else {
                    continue;
                };
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                if key == "card_count_in_zone" {
                    if let Value::Mapping(spec) = child {
                        // Absent, null, or an empty mapping (`filter: {}`): a
                        // present key that constrains nothing still counts
                        // every permanent.
                        let filter_missing = spec.get("filter").is_none_or(|filter| match filter {
                            Value::Null => true,
                            Value::Mapping(map) => map.is_empty(),
                            _ => false,
                        });
                        if zone_includes_battle_area(spec.get("zone")) && filter_missing {
                            out.push(AuthoringFinding {
                                rule: "unfiltered-battle-area-count",
                                path: child_path.clone(),
                                message: "card_count_in_zone over battle_area has no filter: it counts EVERY permanent, Option cards placed in the battle area and Tamers included (CR 4.3 4-28-1, 4-4-1). Spell out what the card counts, e.g. `filter: { kind: digimon }` or `filter: { any_of: [{ kind: digimon }, { kind: tamer }] }`".into(),
                            });
                        }
                    }
                }
                if key == "when" {
                    if let Value::Sequence(items) = child {
                        let timings: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
                        let own = timings.iter().copied().find(|t| SELF_TIMINGS.contains(t));
                        let observer = timings.iter().copied().find(|t| OBSERVER_TIMINGS.contains(t));
                        if let (Some(own), Some(observer)) = (own, observer) {
                            out.push(AuthoringFinding {
                                rule: "self-and-observer-timing",
                                path: child_path.clone(),
                                message: format!(
                                    "`when` mixes the card's own timing `{own}` with the board-wide observer timing `{observer}`. A printed \"[On Play] [When Digivolving]\" is `[on_play, when_digivolving]` (`on_digivolve` fires on EVERY Digimon's digivolution); a printed observer (\"When any ... are played or digivolve\", \"When Digimon attack\") uses observer timings only"
                                ),
                            });
                        }
                    }
                }
                walk(child, &child_path, out);
            }
        }
        Value::Sequence(items) => {
            for (index, child) in items.iter().enumerate() {
                walk(child, &format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(raw: &str) -> Vec<&'static str> {
        check_authoring_yaml(raw).into_iter().map(|f| f.rule).collect()
    }

    #[test]
    fn unfiltered_battle_area_count_is_rejected() {
        let raw = "effects:\n  - when: on_play\n    process:\n      - add_dp_modifier:\n          value:\n            formula:\n              base: 0\n              per:\n                card_count_in_zone:\n                  of: you\n                  zone: battle_area\n              delta: -3000\n";
        let findings = check_authoring_yaml(raw);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert_eq!(findings[0].rule, "unfiltered-battle-area-count");
        assert!(findings[0].path.ends_with("per.card_count_in_zone"));
    }

    #[test]
    fn filtered_battle_area_count_and_other_zones_pass() {
        let filtered = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: opponent, zone: battle_area, filter: { kind: digimon } }\n";
        assert!(rules(filtered).is_empty());
        let trash = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: you, zone: trash }\n";
        assert!(rules(trash).is_empty());
    }

    /// `filter: {}` is a present key with no constraint: the count is still
    /// every permanent, so it is the same finding as a missing filter.
    #[test]
    fn empty_filter_mapping_is_an_unfiltered_battle_area_count() {
        for filter in ["{}", "{ }", "null", "~"] {
            let raw = format!(
                "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: {{ of: you, zone: battle_area, filter: {filter} }}\n"
            );
            assert_eq!(rules(&raw), vec!["unfiltered-battle-area-count"], "filter: {filter}");
        }
        let list_zone = "effects:\n  - when: on_play\n    process:\n      - x:\n          per:\n            card_count_in_zone: { of: you, zone: [battle_area], filter: {} }\n";
        assert_eq!(rules(list_zone), vec!["unfiltered-battle-area-count"]);
    }

    #[test]
    fn on_play_with_board_wide_on_digivolve_is_rejected() {
        // Snatchmon (BT21-058): printed "[On Play] [When Digivolving]".
        let raw = "effects:\n  - when: [on_play, on_digivolve]\n    process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }

    #[test]
    fn when_attacking_with_opponent_attack_observer_is_rejected() {
        let raw = "effects:\n  - when: [when_attacking, on_opponent_attack]\n    process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }

    #[test]
    fn self_only_and_observer_only_lists_pass() {
        for when in [
            "[on_play, when_digivolving]",
            "[on_play, when_digivolving, when_attacking]",
            "[on_enter_field_anyone, on_digivolve]",
            "[on_ally_played, on_digivolve]",
            "[on_ally_attack, on_opponent_attack]",
            "on_digivolve",
        ] {
            let raw = format!("effects:\n  - when: {when}\n    process: []\n");
            assert!(rules(&raw).is_empty(), "{when}");
        }
    }

    const REFUND_TAIL: &str = "    process:\n      - select_hand: { of: you, bind_as: pick, optional: true, filter: {} }\n      - if:\n          condition: { binding_absent: pick }\n          then:\n            - refund_opt: {}\n";

    #[test]
    fn refund_after_an_explicit_accept_is_rejected() {
        for head in [
            // Erlangmon (EX12-034) / Slayerdramon (EX13-024) shape.
            "  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    once_per_turn: true\n",
            "  - when: on_add_to_deck\n    once_per_turn: true\n    optional: true\n    outer_prompt: true\n",
            "  - when: main_on_field\n    once_per_turn: true\n    optional: true\n",
            "  - when: counter\n    once_per_turn: true\n",
            "  - when: [when_attacking, counter]\n    once_per_turn: true\n",
        ] {
            let raw = format!("effects:\n{head}{REFUND_TAIL}");
            assert_eq!(rules(&raw), vec!["refund-after-explicit-accept"], "{head}");
        }
    }

    #[test]
    fn refund_without_an_explicit_accept_passes() {
        for head in [
            // Folded "may" (Q1818) and the W13 then-may shape: no outer accept.
            "  - when: when_attacking\n    once_per_turn: true\n",
            "  - when: on_play\n    once_per_turn: true\n    optional: true\n",
            // A mandatory (non-optional) replacement has no accept prompt.
            "  - kind: replacement\n    trigger: when_would_leave_battle_area\n    once_per_turn: true\n",
        ] {
            let raw = format!("effects:\n{head}{REFUND_TAIL}");
            assert!(rules(&raw).is_empty(), "{head}");
        }
        // An explicit-accept clause without a refund is fine.
        let raw = "effects:\n  - when: main_on_field\n    once_per_turn: true\n    process:\n      - draw: { of: you, count: 1 }\n";
        assert!(rules(raw).is_empty());
    }

    const TEST_REVIEW: &[ReviewedRefund] = &[ReviewedRefund {
        card: "TEST-1",
        clause: "effects[0]",
        row: RefundRow::DeclinedWhilePossible,
        reason: "test",
        tests: &[],
    }];

    fn rules_with(raw: &str) -> Vec<&'static str> {
        check_authoring_yaml_with(raw, TEST_REVIEW).into_iter().map(|f| f.rule).collect()
    }

    /// A reviewed entry clears exactly its card's clause: the same body on
    /// another card, or at another clause of the same card, is still flagged.
    #[test]
    fn reviewed_refund_applies_to_its_clause_only() {
        let clause = format!(
            "  - when: on_add_to_deck\n    once_per_turn: true\n    optional: true\n    outer_prompt: true\n{REFUND_TAIL}"
        );
        let reviewed = format!("card: TEST-1\neffects:\n{clause}");
        assert!(rules_with(&reviewed).is_empty());
        assert_eq!(
            check_authoring_yaml_unexempted(&reviewed)
                .into_iter()
                .map(|f| f.rule)
                .collect::<Vec<_>>(),
            vec!["refund-after-explicit-accept"]
        );
        let other_card = format!("card: TEST-2\neffects:\n{clause}");
        assert_eq!(rules_with(&other_card), vec!["refund-after-explicit-accept"]);
        let other_clause =
            format!("card: TEST-1\neffects:\n  - when: on_play\n    process: []\n{clause}");
        assert_eq!(rules_with(&other_clause), vec!["refund-after-explicit-accept"]);
    }

    /// Liveness: an entry is live while its clause still refunds behind an
    /// accept, and stale once the refund (or the accept) is gone or the
    /// clause moved.
    #[test]
    fn reviewed_refund_liveness() {
        let entry = &TEST_REVIEW[0];
        let live = format!("card: TEST-1\neffects:\n  - when: counter\n    once_per_turn: true\n{REFUND_TAIL}");
        assert!(refund_review_is_live(&live, entry));
        let no_refund = "card: TEST-1\neffects:\n  - when: counter\n    once_per_turn: true\n    process:\n      - draw: { of: you, count: 1 }\n";
        assert!(!refund_review_is_live(no_refund, entry));
        let no_accept = format!("card: TEST-1\neffects:\n  - when: on_play\n    once_per_turn: true\n{REFUND_TAIL}");
        assert!(!refund_review_is_live(&no_accept, entry));
        let moved = format!(
            "card: TEST-1\neffects:\n  - when: on_play\n    process: []\n  - when: counter\n    once_per_turn: true\n{REFUND_TAIL}"
        );
        assert!(!refund_review_is_live(&moved, entry), "the clause moved: re-review it");
    }

    /// Row f is a closed list: exactly Slayerdramon (EX13-024)'s two clauses
    /// and the ten "parity with DCGO" clauses of 2026-11-23, each reviewed on
    /// row f. An eleventh card fails here until it has its own ruling.
    #[test]
    fn owner_ruling_row_is_the_closed_list() {
        let on_row_f: Vec<(&str, &str)> = REFUND_AFTER_ACCEPT_REVIEWED
            .iter()
            .filter(|a| a.row == RefundRow::CostDeclinedByOwnerRuling)
            .map(|a| (a.card, a.clause))
            .collect();
        for ruled in COST_DECLINED_BY_OWNER_RULING {
            assert!(on_row_f.contains(ruled), "{ruled:?} is ruled onto row f but not reviewed there");
        }
        assert_eq!(on_row_f.len(), COST_DECLINED_BY_OWNER_RULING.len());
        for idiom in ["P-250", "BT24-026", "EX5-054", "BT15-102", "EX8-006"] {
            assert!(
                !COST_DECLINED_BY_OWNER_RULING.iter().any(|(c, _)| *c == idiom),
                "{idiom} keeps \"a declined cost spends the use\" (DCGO spends there too)"
            );
        }
    }

    /// The [Main] row is a closed list: exactly the clauses of the 2026-11-24
    /// ruling, each reviewed on that row, and none of the clauses DCGO spends.
    #[test]
    fn main_refund_row_is_the_closed_list() {
        let on_row: Vec<(&str, &str)> = REFUND_AFTER_ACCEPT_REVIEWED
            .iter()
            .filter(|a| a.row == RefundRow::MainPassedByOwnerRuling)
            .map(|a| (a.card, a.clause))
            .collect();
        for ruled in MAIN_REFUND_BY_OWNER_RULING {
            assert!(on_row.contains(ruled), "{ruled:?} is on the [Main] list but not reviewed there");
        }
        assert_eq!(on_row.len(), MAIN_REFUND_BY_OWNER_RULING.len());
        for spends in ["BT25-052", "BT25-070", "BT23-008", "BT26-070", "EX13-071"] {
            assert!(
                !MAIN_REFUND_BY_OWNER_RULING.iter().any(|(c, _)| *c == spends),
                "{spends}: DCGO keeps the declared [Main]'s use spent on every path ours reaches"
            );
        }
    }

    #[test]
    fn main_refund_clauses_finds_main_timings_only() {
        let main = format!("card: TEST-1\neffects:\n  - when: main_on_field\n    once_per_turn: true\n{REFUND_TAIL}");
        assert_eq!(main_refund_clauses(&main), vec!["effects[0]".to_string()]);
        let counter = format!("card: TEST-1\neffects:\n  - when: counter\n    once_per_turn: true\n{REFUND_TAIL}");
        assert!(main_refund_clauses(&counter).is_empty());
    }

    /// The production table: one entry per clause, each with a reason and
    /// the tests that prove it.
    #[test]
    fn reviewed_refund_table_is_well_formed() {
        for (i, a) in REFUND_AFTER_ACCEPT_REVIEWED.iter().enumerate() {
            assert!(!a.reason.trim().is_empty(), "{} {}", a.card, a.clause);
            assert!(a.clause.starts_with("effects["), "{} {}", a.card, a.clause);
            assert!(!a.tests.is_empty(), "{} {}: name the tests that prove the row", a.card, a.clause);
            if a.row == RefundRow::MainPassedByOwnerRuling {
                assert!(
                    MAIN_REFUND_BY_OWNER_RULING.contains(&(a.card, a.clause)),
                    "a [Main] refund needs DCGO RemoveUse on that path and the 2026-11-24 ruling's list \
                     ({} {}): add it to MAIN_REFUND_BY_OWNER_RULING only with a DCGO-script read",
                    a.card,
                    a.clause
                );
            }
            if a.row == RefundRow::CostDeclinedByOwnerRuling {
                assert!(
                    COST_DECLINED_BY_OWNER_RULING.contains(&(a.card, a.clause)),
                    "row f needs an owner ruling for that clause ({} {}): add it to \
                     COST_DECLINED_BY_OWNER_RULING only with the ruling",
                    a.card,
                    a.clause
                );
            }
            for t in a.tests {
                let (file, name) = t.split_once("::").unwrap_or(("", ""));
                assert!(
                    file.ends_with(".rs") && !name.is_empty(),
                    "{} {}: `{t}` is not <file>::<test fn>",
                    a.card,
                    a.clause
                );
            }
            for b in &REFUND_AFTER_ACCEPT_REVIEWED[i + 1..] {
                assert!(
                    (a.card, a.clause) != (b.card, b.clause),
                    "duplicate entry {} {}",
                    a.card,
                    a.clause
                );
            }
        }
    }

    #[test]
    fn nested_granted_effect_is_walked() {
        let raw = "effects:\n  - when: on_play\n    process:\n      - grant_triggered_effect:\n          effect:\n            when: [on_deletion, on_any_deletion]\n            process: []\n";
        assert_eq!(rules(raw), vec!["self-and-observer-timing"]);
    }
}
