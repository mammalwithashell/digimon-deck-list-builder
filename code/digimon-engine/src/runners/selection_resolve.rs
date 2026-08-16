//! Resolve DCGO semantic selection payloads against the engine's live
//! `PendingSelection` (task 3.5).
//!
//! The DCGO recorder cannot know the engine's candidate ordering or action-id
//! scheme, so selection rows carry SEMANTIC payloads — absolute frame targets,
//! card identities, counts, bools. At replay time the engine's own
//! `PendingSelection` (kind + `valid_action_ids` + `zone_owner` +
//! `effect_choices`) is authoritative, and this module maps each recorded pick
//! onto one of the ids the engine will actually accept.
//!
//! Multi-pick protocol: DCGO sends a whole pick-set in one RPC; the engine
//! re-installs a prompt after each pick. The caller loops `resolve_next` with
//! an incrementing `picks_done`, applying one id at a time, until `Ok(None)`.
//! After the payload's picks are exhausted, one trailing `PASS` is emitted iff
//! a prompt is still parked and PASS is legal (up-to-N prompts want an
//! explicit stop; exact-N prompts have already resolved by then).

use crate::action::space::{
    ATTACK_START, PASS, PLAY_HAND_START, SEL_MY_SECURITY_START, SEL_OPP_SECURITY_START,
    SEL_REVEAL_START, TARGETS_PER_ATTACKER, TRASH_EFFECT_START,
};
use crate::dcgo_recording::SelectionRow;
use crate::game::Game;
use crate::selection::SelectionKind;

/// Recognize a two-branch `EffectChoice` that is really a yes/no gate, and
/// return its `(affirmative_id, decline_id)`.
///
/// `select_effect_choice` in the DSL serves two structurally identical but
/// semantically different purposes:
///
///   1. A binary "you may" gate — `["Hatch", "Don't hatch"]`,
///      `["Play Tamer", "Decline"]`, `["Trash 1 from hand", "Skip"]`. Across
///      the card pool these are authored affirmative-first, decline-second.
///   2. A genuine either/or branch — `["From hand", "From trash"]`,
///      `["Top", "Bottom"]`, `["Your Trash", "Opponent's Trash"]`.
///
/// DCGO answers case 1 with a plain bool (its `SetBoolForPlayer` channel), so
/// the recording carries `bool_value` with no branch index. Case 2 cannot be
/// answered by a bool at all — "true" says nothing about hand vs. trash — and
/// DCGO records those as an int instead.
///
/// So a bool is mapped ONLY when the second branch is decline-shaped, which is
/// what distinguishes the two cases. Anything else returns `None` and fails
/// loudly rather than picking a branch on a coin flip: a wrong pick here would
/// silently corrupt the parity corpus, which is worse than a stalled replay.
fn yes_no_branches(pending: &crate::selection::PendingSelection) -> Option<(u16, u16)> {
    let entries = pending.effect_choices.as_ref()?;
    if entries.len() != 2 {
        return None;
    }
    let is_decline = |label: &str| {
        let l = label.trim().to_ascii_lowercase();
        l == "decline"
            || l == "skip"
            || l == "no"
            || l == "pass"
            || l.starts_with("don't")
            || l.starts_with("don’t")
            || l.starts_with("do not")
    };
    // Exactly one decline-shaped branch, and it must be the second — otherwise
    // this is an either/or prompt (or authored against the pool convention),
    // and a bool cannot address it.
    if is_decline(&entries[1].label) && !is_decline(&entries[0].label) {
        Some((entries[0].action_id, entries[1].action_id))
    } else {
        None
    }
}

/// Render an `EffectChoice` prompt's branch labels for an error message.
///
/// When a recorded payload can't be mapped, the labels are what tell you
/// whether the recorder lost prompt identity (DCGO's `generic_bool` fallback)
/// or the engine offered branches DCGO never showed. Empty string when the
/// prompt carries no choice entries.
fn describe_choices(pending: &crate::selection::PendingSelection) -> String {
    match &pending.effect_choices {
        Some(entries) if !entries.is_empty() => format!(
            " — choices: {:?}",
            entries
                .iter()
                .map(|e| format!("{} => {}", e.action_id, e.label))
                .collect::<Vec<_>>()
        ),
        _ => String::new(),
    }
}

/// The still-unplaced items of an in-flight ordered-permutation prompt, as
/// card IDs in `action_id - SEL_REVEAL_START` order.
///
/// `OrderedPermutation` is the one card-pick kind whose action IDs do NOT
/// index a game zone. The prompt re-installs itself after every pick over a
/// SHRINKING `remaining` list, so reveal index `i` means "the i-th card not
/// yet placed" — matching it against `game.revealed_cards` (which keeps every
/// revealed card, placed or not) picks the wrong card as soon as one is
/// ordered.
///
/// The list is observable for DSL cards, which park the permutation as a
/// `PermutationStep` frame on the resume stack. The legacy closure-based
/// installer in `effect_context::selections` keeps `remaining` captured inside
/// its `FnOnce`, out of reach of any caller; those prompts return `None` here
/// and surface as an honest resolution error rather than a wrong pick.
fn permutation_remaining_ids(game: &Game) -> Option<Vec<String>> {
    use crate::resume::ResumeFrame;

    let stack = game.pending_selection_resume.as_ref()?;
    // Frames run inner-to-outer, so the live prompt is the innermost one.
    let state = stack.frames.iter().rev().find_map(|f| match f {
        ResumeFrame::PermutationStep(s) => Some(s),
        _ => None,
    })?;

    Some(
        state
            .remaining
            .iter()
            .map(|h| card_id_for_handle(game, *h))
            .collect(),
    )
}

/// Resolve a `CardHandle` (a card's unique `card_index`) to its card ID.
///
/// Permutation items are typically mid-reveal, so `revealed_cards` is checked
/// first; the remaining zones cover `order_remainder`-style prompts whose items
/// have already been placed back on the deck. Returns an empty string for a
/// handle we cannot locate, which simply fails to match any recorded pick.
fn card_id_for_handle(game: &Game, handle: crate::card_source::CardHandle) -> String {
    let want = handle.0;
    let matches = |c: &&crate::card_source::CardSource| c.card_index == want;

    game.revealed_cards
        .iter()
        .find(matches)
        .or_else(|| {
            game.players.iter().find_map(|p| {
                p.deck
                    .iter()
                    .chain(p.hand.iter())
                    .chain(p.trash.iter())
                    .chain(p.security.iter())
                    .find(matches)
            })
        })
        .map(|c| c.card_id(&game.card_data).to_string())
        .unwrap_or_default()
}

/// Number of picks this payload carries (before any trailing PASS).
pub fn payload_pick_count(payload: &SelectionRow) -> usize {
    if let Some(t) = &payload.targets {
        t.len()
    } else if let Some(c) = &payload.card_ids {
        c.len()
    } else {
        // count / int / bool payloads are single decisions.
        1
    }
}

/// Resolve the next engine action id for `payload`, given `picks_done` picks
/// already applied. `Ok(None)` = payload fully consumed (caller stops).
/// `Err` = the payload cannot be mapped onto the current prompt — an honest
/// divergence, with enough detail to triage.
pub fn resolve_next(
    game: &Game,
    payload: &SelectionRow,
    picks_done: usize,
) -> Result<Option<u16>, String> {
    let n_picks = payload_pick_count(payload);

    let pending = match game.pending_selection.as_ref() {
        Some(p) => p,
        None => {
            // Engine has no prompt parked. If we already applied picks, the
            // selection resolved — done. If we applied none, the engine never
            // needed this prompt (e.g. it auto-resolves single-candidate
            // prompts DCGO still asks about) — skip the row.
            return Ok(None);
        }
    };

    // Payload exhausted → decide whether an explicit stop is needed.
    if picks_done >= n_picks {
        let declined = payload.cancel.unwrap_or(false);
        let multiselectish = matches!(
            pending.kind,
            SelectionKind::CountCappedMultiSelect { .. }
                | SelectionKind::SourceMulti { .. }
                | SelectionKind::RevealBucket { .. }
                | SelectionKind::DpBudget { .. }
                | SelectionKind::PlayCostBudget { .. }
        );
        let pass_legal = pending.is_optional;
        if picks_done > n_picks {
            // Trailing PASS already sent once; never loop.
            return Ok(None);
        }
        if (multiselectish || declined || n_picks == 0) && pass_legal {
            return Ok(Some(PASS));
        }
        return Ok(None);
    }

    // Cancelled prompt with no picks: decline outright.
    if payload.cancel.unwrap_or(false) && payload.targets.is_none() && payload.card_ids.is_none() {
        if pending.is_optional {
            return Ok(Some(PASS));
        }
        return Err(format!(
            "recorded cancel on a non-optional {:?} prompt",
            pending.kind
        ));
    }

    let valid = &pending.valid_action_ids;
    let accepts = |id: u16| valid.contains(&id);

    // ── Field-permanent picks ────────────────────────────────────────────
    if let Some(targets) = &payload.targets {
        let t = targets[picks_done];
        let slot = t.frame as u16;
        // Candidate encodings, most-specific first:
        //   OwnField/OppField: 100 + slot (side implicit in kind)
        //   AnyField:          100 + player*15 + slot
        let side_implicit = ATTACK_START + slot;
        let absolute = ATTACK_START + (t.player as u16) * TARGETS_PER_ATTACKER + slot;
        for cand in [side_implicit, absolute] {
            if accepts(cand) {
                return Ok(Some(cand));
            }
        }
        return Err(format!(
            "frame target (player {}, slot {}) matches none of {:?} (kind {:?})",
            t.player, slot, valid, pending.kind
        ));
    }

    // ── Card-identity picks (hand / trash / reveal / security) ───────────
    if let Some(card_ids) = &payload.card_ids {
        let want = &card_ids[picks_done];
        let zone_owner = pending
            .zone_owner
            .unwrap_or(pending.selecting_player);
        let find_id = |ids: &[String], base: u16| -> Option<u16> {
            ids.iter().enumerate().find_map(|(i, cid)| {
                let id = base + i as u16;
                (cid == want && accepts(id)).then_some(id)
            })
        };
        let zone_card_ids = |cards: &[crate::card_source::CardSource]| -> Vec<String> {
            cards
                .iter()
                .map(|c| c.card_id(&game.card_data).to_string())
                .collect()
        };
        let hit = match pending.kind {
            SelectionKind::Hand | SelectionKind::UnionZone { .. } => {
                find_id(&zone_card_ids(&game.player(zone_owner).hand), PLAY_HAND_START)
                    .or_else(|| find_id(&zone_card_ids(&game.player(zone_owner).trash), TRASH_EFFECT_START))
            }
            SelectionKind::Trash => {
                find_id(&zone_card_ids(&game.player(zone_owner).trash), TRASH_EFFECT_START)
            }
            SelectionKind::Reveal | SelectionKind::RevealBucket { .. } => {
                find_id(&zone_card_ids(&game.revealed_cards), SEL_REVEAL_START)
            }
            SelectionKind::OrderedPermutation { .. } => {
                permutation_remaining_ids(game).and_then(|ids| find_id(&ids, SEL_REVEAL_START))
            }
            SelectionKind::Security => {
                let base = if zone_owner == pending.selecting_player {
                    SEL_MY_SECURITY_START
                } else {
                    SEL_OPP_SECURITY_START
                };
                find_id(&zone_card_ids(&game.player(zone_owner).security), base)
            }
            _ => None,
        };
        if let Some(id) = hit {
            return Ok(Some(id));
        }
        return Err(format!(
            "card pick '{}' not found in {:?} prompt '{}' (zone owner {}, valid {:?})",
            want, pending.kind, pending.prompt, zone_owner, valid
        ));
    }

    // ── Single-value payloads ────────────────────────────────────────────
    if let Some(b) = payload.bool_value {
        // A binary "you may" gate that the engine models as a two-branch
        // EffectChoice rather than an optional prompt (see `yes_no_branches`).
        if let Some((yes, no)) = yes_no_branches(pending) {
            return Ok(Some(if b { yes } else { no }));
        }
        if !b {
            if pending.is_optional {
                return Ok(Some(PASS));
            }
            return Err(format!(
                "recorded 'no' on a non-optional {:?} prompt '{}' (valid {:?}){}",
                pending.kind,
                pending.prompt,
                valid,
                describe_choices(pending)
            ));
        }
        // Affirmative: unambiguous only when the prompt has one accept path.
        if valid.len() == 1 {
            return Ok(Some(valid[0]));
        }
        return Err(format!(
            "recorded 'yes' but {:?} prompt '{}' has {} accept ids {:?}{}",
            pending.kind,
            pending.prompt,
            valid.len(),
            valid,
            describe_choices(pending)
        ));
    }

    if let Some(v) = payload.int_value.or(payload.count.map(|c| c as i64)) {
        // EffectChoice: the int is DCGO's branch index — map through entries
        // by position (both sides present branches in card-text order).
        if let Some(entries) = &pending.effect_choices {
            if let Some(e) = entries.get(v as usize) {
                return Ok(Some(e.action_id));
            }
        }
        // Fallback: nth valid id (covers count prompts lowered to id lists).
        if v >= 0 {
            if let Some(id) = valid.get(v as usize) {
                return Ok(Some(*id));
            }
        }
        return Err(format!(
            "int payload {} out of range for {:?} prompt ({} valid ids)",
            v,
            pending.kind,
            valid.len()
        ));
    }

    Err(format!(
        "selection payload for prompt '{}' carries no usable fields (kind {:?})",
        payload.prompt, pending.kind
    ))
}
