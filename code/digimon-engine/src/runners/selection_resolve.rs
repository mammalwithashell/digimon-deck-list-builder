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
            "card pick '{}' not found in {:?} prompt (zone owner {}, valid {:?})",
            want, pending.kind, zone_owner, valid
        ));
    }

    // ── Single-value payloads ────────────────────────────────────────────
    if let Some(b) = payload.bool_value {
        if !b {
            if pending.is_optional {
                return Ok(Some(PASS));
            }
            return Err(format!(
                "recorded 'no' on a non-optional {:?} prompt (valid {:?})",
                pending.kind, valid
            ));
        }
        // Affirmative: unambiguous only when the prompt has one accept path.
        if valid.len() == 1 {
            return Ok(Some(valid[0]));
        }
        return Err(format!(
            "recorded 'yes' but {:?} prompt has {} accept ids {:?}",
            pending.kind,
            valid.len(),
            valid
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
