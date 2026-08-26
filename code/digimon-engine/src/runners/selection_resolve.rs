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
    ATTACK_START, PASS, PLAY_HAND_START, SECURITY_TARGET, SEL_MY_SECURITY_START,
    SEL_OPP_SECURITY_START, SEL_REVEAL_START, TARGETS_PER_ATTACKER, TRASH_EFFECT_START,
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

/// Does this branch label describe the numeric value `want`?
///
/// The engine labels value-carrying branches with the value itself — e.g.
/// `"Digivolve for cost 2"`, `"Digivolve for cost 2 (App Fusion)"`. DCGO records
/// the chosen VALUE, so comparing against the label matches on what each side
/// thinks the option means rather than on where it happens to sit in a list.
///
/// Positional matching is the trap this avoids: with costs {0, 1} the value
/// equals the index and everything looks correct, then with costs {2, 3} the
/// same code silently picks the wrong branch.
///
/// Scans whole integer tokens so `"cost 2"` does not match `12` or `20`.
fn label_mentions_value(label: &str, want: i64) -> bool {
    let bytes = label.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if let Ok(n) = label[start..i].parse::<i64>() {
                if n == want {
                    return true;
                }
            }
        } else {
            i += 1;
        }
    }
    false
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

/// The selectable digivolution sources of an in-flight `SelectionKind::Material`
/// prompt, as `(card IDs bottom-up, range_start)` — `range_start` being the
/// action id of source index 0.
///
/// Material action IDs are the one card-pick encoding that does not name its
/// owner. A battle-area id is `SOURCE_SELECT_START + field_index *
/// SOURCES_PER_FIELD + source_index` (`material_zone_geometry`,
/// `effect_context/selections.rs`), so the FIELD index is recoverable by
/// arithmetic but the carrier's PLAYER is not — and `zone_owner` is `None` on
/// this kind. Guessing the side would mean picking a source index out of the
/// wrong stack, which resolves cleanly against the real carrier and silently
/// takes the wrong card. So the carrier comes from the engine, not from
/// arithmetic — the same shape as `permutation_remaining_ids` above: the DSL
/// installer (`install_select_material` in `dsl_cards/step/selections.rs`, the
/// only `EffectContext::select_material` caller in the crate) parks a
/// `ResumeFrame::RunTail { select_kind: ResumeSelectKind::Material { perm } }`
/// beside the prompt, and `perm` IS the carrier.
///
/// The recovered frame is then cross-checked against the live prompt: every one
/// of its `valid_action_ids` must fall inside that carrier's own 12-slot band.
/// `SelectionKind::Material` is heavily overloaded — DNA digivolution
/// (`game_actions/digivolve.rs`) and DigiXros material assembly
/// (`game_actions/misc.rs`) reuse the kind with completely different action-id
/// encodings (raw field indices, hand ids) — and a prompt from one of those, or
/// from a legacy closure-only installer with no data frame, returns `None` here
/// and surfaces as an honest resolution error rather than a wrong pick.
fn material_source_ids(game: &Game, valid: &[u16]) -> Option<(Vec<String>, u16)> {
    use crate::action::space::SOURCES_PER_FIELD;
    use crate::effect_context::selections::{material_zone_geometry, material_zone_slice};
    use crate::resume::{ResumeFrame, ResumeSelectKind};

    let stack = game.pending_selection_resume.as_ref()?;
    // Frames run inner-to-outer, so the live prompt is the innermost one —
    // same traversal as `permutation_remaining_ids`.
    let perm = stack.frames.iter().rev().find_map(|f| match f {
        ResumeFrame::RunTail {
            select_kind: ResumeSelectKind::Material { perm },
            ..
        } => Some(*perm),
        _ => None,
    })?;

    let (source_count, range_start) = material_zone_geometry(game, perm)?;
    let band = range_start..range_start.saturating_add(SOURCES_PER_FIELD);
    if valid.is_empty() || !valid.iter().all(|id| band.contains(id)) {
        return None;
    }

    // Mirror the installer: the top card (the active Digimon itself) is never a
    // candidate, so only `source_count` entries are addressable.
    let cap = source_count.min(SOURCES_PER_FIELD as usize);
    Some((
        material_zone_slice(game, perm)?
            .iter()
            .take(cap)
            .map(|c| c.card_id(&game.card_data).to_string())
            .collect(),
        range_start,
    ))
}

/// The selectable cards of an in-flight `SelectionKind::CountCappedMultiSelect`
/// prompt, as `(card IDs in zone-index order, range_start)` — `range_start`
/// being the action id of zone index 0.
///
/// `CountCappedMultiSelect` is the multi-pick kind whose variant carries no
/// zone: `{ min, max, picked, distinct }` says how MANY to take, never from
/// WHERE. Two facts are missing, and both are needed to name a card:
///
///   * the ZONE — recoverable by arithmetic, since the three bands are
///     disjoint (`PLAY_HAND_START` 0..30, `TRASH_EFFECT_START` 1150..1195,
///     `SOURCE_SELECT_START` 2000.. / `BREEDING_SOURCE_SELECT_START` 2168..);
///   * the zone's OWNER — which is NOT. All three installers hardcode
///     `zone_owner: None` (`install_count_capped_step` in
///     `effect_context/selections.rs`; `park_non_dsl_count_capped_state` and
///     `install_multipick_step` in `dsl_cards/step/selections.rs`), so
///     `resolve_next`'s `zone_owner.unwrap_or(selecting_player)` fallback reads
///     the SELECTING player's zone. That is wrong whenever a card makes you
///     pick out of the opponent's zone, which is live in the pool: BT18-019
///     lowers to `select_count_capped_multi { of: opponent, zone: trash,
///     max: 7, distinct_by: level }` (`cards/bt18/BT18-019.yaml:137`), and
///     `MultiPickState` models `of_player` and `selecting_player` as separate
///     fields precisely because they diverge. A wrong-owner read does not fail
///     loudly: index `i` of the selecting player's own trash may well hold the
///     wanted card id while `TRASH_EFFECT_START + i` sits in
///     `valid_action_ids`, so the pick is ACCEPTED and the wrong card leaves
///     the wrong trash.
///
/// So the zone comes from the engine, not from arithmetic — the same shape as
/// `material_source_ids` and `permutation_remaining_ids` above. Both data-VM
/// installers park a frame that states zone and owner outright:
/// `MultiPickState { zone, of_player, .. }` (the DSL `count_capped` family) and
/// `NonDslCountCappedState { zone, of_player, .. }` (keyword and engine-action
/// call sites — [Assembly] among them: `install_assembly_element` in
/// `game_actions/mod.rs` parks `CountCappedZone::Trash`).
///
/// This is the "make the variant carry its zone" option WITHOUT widening the
/// variant, which is the cheaper trade here: the kind's `Debug` rendering is a
/// live UI wire contract on both the browser and desktop pending-selection
/// wires (`format!("{:?}", kind)`, parsed by
/// `frontend/src/utils/trashSelection.ts`), and `SelectionKind` derives
/// `Copy + Hash`, which `CountCappedZone`'s `PermanentHandle` payload would
/// have to grow. The engine already carries the zone beside the prompt; it just
/// was not being read.
///
/// The recovered frame is then cross-checked against the live prompt: every
/// `valid_action_ids` entry must fall inside the named zone's own band. A
/// legacy closure-only prompt (`install_count_capped_step` parks no frame at
/// all) returns `None` here and surfaces as an honest resolution error rather
/// than a wrong pick.
fn count_capped_zone_ids(game: &Game, valid: &[u16]) -> Option<(Vec<String>, u16)> {
    use crate::action::space::{HAND_MAIN_LIMIT, SOURCES_PER_FIELD, TRASH_MAIN_LIMIT};
    use crate::effect_context::selections::{material_zone_geometry, material_zone_slice};
    use crate::effect_context::CountCappedZone;
    use crate::resume::ResumeFrame;

    let stack = game.pending_selection_resume.as_ref()?;
    // Frames run inner-to-outer, so the live prompt is the innermost one —
    // same traversal as `permutation_remaining_ids` / `material_source_ids`.
    let (zone, of_player) = stack.frames.iter().rev().find_map(|f| match f {
        ResumeFrame::MultiPickStep(s) => Some((s.zone, s.of_player)),
        ResumeFrame::NonDslCountCappedStep(s) => Some((s.zone, s.of_player)),
        _ => None,
    })?;

    let to_ids = |cards: &[crate::card_source::CardSource], cap: usize| -> Vec<String> {
        cards
            .iter()
            .take(cap)
            .map(|c| c.card_id(&game.card_data).to_string())
            .collect()
    };

    // `(ids, range_start, band_len)`, mirroring the installer's own
    // `(zone_len, range_start)` computation in `select_count_capped_multi_min`
    // so an index resolved here is an index the engine will accept.
    let (ids, range_start, band_len) = match zone {
        CountCappedZone::Hand => (
            to_ids(&game.player(of_player).hand, HAND_MAIN_LIMIT),
            PLAY_HAND_START,
            HAND_MAIN_LIMIT as u16,
        ),
        CountCappedZone::Trash => (
            to_ids(&game.player(of_player).trash, TRASH_MAIN_LIMIT),
            TRASH_EFFECT_START,
            TRASH_MAIN_LIMIT as u16,
        ),
        CountCappedZone::Material(perm) => {
            // Same geometry the `Material` arm uses: sources bottom-up, top
            // card excluded. The carrier's owner rides the handle, so
            // `of_player` is deliberately not consulted on this branch.
            let (source_count, base) = material_zone_geometry(game, perm)?;
            let cap = source_count.min(SOURCES_PER_FIELD as usize);
            (
                to_ids(material_zone_slice(game, perm)?, cap),
                base,
                SOURCES_PER_FIELD,
            )
        }
    };

    let band = range_start..range_start.saturating_add(band_len);
    if valid.is_empty() || !valid.iter().all(|id| band.contains(id)) {
        return None;
    }
    Some((ids, range_start))
}

/// Number of picks this payload carries (before any trailing PASS).
pub fn payload_pick_count(payload: &SelectionRow) -> usize {
    if let Some(t) = &payload.targets {
        t.len()
    } else if let Some(c) = &payload.card_ids {
        c.len()
    } else if payload.cancel.unwrap_or(false)
        && payload.int_value.is_none()
        && payload.count.is_none()
        && payload.bool_value.is_none()
    {
        // task_69f10a66 Family 3a root cause: a PURE-cancel payload is a
        // single decline delivered by the picks_done == 0 PASS — it carries
        // ZERO picks. Counting it as 1 made the `picks_done == n_picks`
        // trailing-PASS branch fire a PHANTOM SECOND PASS after the decline
        // resolved, and when the decline's resolution parks a NEW optional
        // prompt (exam witness EX12-063-inherited0: the declined mid-battle
        // <Barrier> commits the deletion, whose inherited [On Deletion]
        // "you may play" gate parks immediately), that phantom PASS silently
        // declined the new prompt — a choice the row never answered. The
        // engine's own trigger handling is sound (five DebugRunner
        // differentials pin it); the drop was this resolver's.
        0
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
        // A pure-cancel payload (n_picks == 0) aimed at a NON-optional
        // prompt is an honest divergence, not a silent stop — preserves the
        // error the dedicated cancel branch below used to raise before
        // pure-cancel payloads counted 0 picks (task_69f10a66 Family 3a).
        if declined && n_picks == 0 && picks_done == 0 && !pass_legal {
            return Err(format!(
                "recorded cancel on a non-optional {:?} prompt",
                pending.kind
            ));
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
        // DCGO's `SelectAttackEffect.SetAttackTarget` (the attack-target
        // prompt's chokepoint) uses `SecurityIndex = -1` as a sentinel for
        // "attack the player" rather than a battle-area permanent — see
        // its `__frame = permanentIndex < 0 ? -1 : ValidateFieldSlot(...)`.
        // `remap_selection_targets` in `runners/replay.rs` already treats
        // negative frames the same way ("-1 addresses the player
        // (security), not a permanent"). Casting a negative frame straight
        // to `u16` wraps to 65535 and blows up the `ATTACK_START` addition
        // below (the reported overflow panic), so the sign is handled
        // explicitly first: `-1` maps onto the engine's own
        // `SECURITY_TARGET` slot (reusing the exact same candidate
        // formulas that already resolve ordinary in-range frames), and any
        // other negative value — which no known DCGO chokepoint emits — is
        // an honest, legible `Err` rather than a guess.
        let slot: u16 = if t.frame >= 0 {
            t.frame as u16
        } else if t.frame == -1 {
            SECURITY_TARGET
        } else {
            return Err(format!(
                "frame target (player {}, frame {}) is not a valid battle-area slot or the \
                 -1 \"attack the player\" sentinel (kind {:?}, valid {:?})",
                t.player, t.frame, pending.kind, valid
            ));
        };
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
            // Digivolution sources of the prompt's carrier. Duplicate ids are
            // interchangeable copies, so they resolve by occurrence order —
            // `find_id` takes the FIRST matching index the prompt still
            // accepts, exactly as the Hand / Trash / Reveal arms above do
            // (a copy the install-time filter excluded is skipped, and the
            // scan continues to the next occurrence). Occurrence order here is
            // bottom-up: index 0 is the bottom card of the stack.
            SelectionKind::Material => {
                material_source_ids(game, valid).and_then(|(ids, base)| find_id(&ids, base))
            }
            // Hand / trash / material picks of a count-capped multi-select.
            // The kind names no zone and every installer leaves `zone_owner`
            // unset, so BOTH come from the parked data frame — see
            // `count_capped_zone_ids`. Duplicate ids are interchangeable
            // copies and resolve by occurrence order: `find_id` takes the
            // FIRST matching index the prompt still accepts, exactly as the
            // Hand / Trash / Reveal / Material arms above do, so a copy the
            // install-time filter excluded — or one already taken on an
            // earlier step and dropped from `valid_action_ids` — is skipped
            // and the scan continues to the next occurrence.
            SelectionKind::CountCappedMultiSelect { .. } => {
                count_capped_zone_ids(game, valid).and_then(|(ids, base)| find_id(&ids, base))
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

    // ── Count payloads (a VALUE, not an index) ───────────────────────────
    //
    // `SelectCountEffect` answers "how many?" / "which cost?" with the chosen
    // NUMBER — the button text is the value itself. Our engine models the same
    // question as an indexed branch list, so the value must first be located in
    // the option set DCGO offered.
    //
    // These were previously folded in with `int_value` via
    // `int_value.or(count)`, which indexed the branch list with a raw quantity:
    // `count: 2` became "pick branch 2". The visible half was an out-of-range
    // error; the dangerous half was `count: 0` resolving to branch 0 and
    // replaying clean while meaning something entirely different.
    if let Some(c) = payload.count {
        // Preferred path: the engine's own branch labels embed the value
        // ("Digivolve for cost 2"), so a recorded value can be matched directly.
        // This works on recordings made before the recorder emitted
        // `candidates`, and is stricter than positional matching — it compares
        // what each side thinks the option MEANS, not merely where it sits.
        if let Some(entries) = &pending.effect_choices {
            let matches: Vec<&crate::selection::EffectChoiceEntry> = entries
                .iter()
                .filter(|e| label_mentions_value(&e.label, c as i64))
                .collect();
            if matches.len() == 1 {
                return Ok(Some(matches[0].action_id));
            }
            if matches.len() > 1 {
                return Err(format!(
                    "count payload {} matches {} branches of the {:?} prompt '{}'                      — ambiguous{}",
                    c,
                    matches.len(),
                    pending.kind,
                    pending.prompt,
                    describe_choices(pending)
                ));
            }
        }

        let Some(candidates) = &payload.candidates else {
            return Err(format!(
                "count payload {} has no candidate set, so it cannot be mapped onto                  the {:?} prompt '{}' — re-record with a recorder that emits                  `candidates`{}",
                c,
                pending.kind,
                pending.prompt,
                describe_choices(pending)
            ));
        };
        let Some(index) = candidates.iter().position(|v| *v == c as i64) else {
            return Err(format!(
                "count payload {} is not in its own candidate set {:?} (prompt '{}')",
                c, candidates, pending.prompt
            ));
        };
        // The candidate set and the engine's branch list must describe the same
        // question. A length mismatch means they do not, and picking by index
        // anyway is how a wrong answer replays clean.
        if let Some(entries) = &pending.effect_choices {
            if entries.len() != candidates.len() {
                return Err(format!(
                    "count payload {} chose option {} of {:?}, but the {:?} prompt                      '{}' offers {} branch(es) — the two option sets disagree{}",
                    c,
                    index,
                    candidates,
                    pending.kind,
                    pending.prompt,
                    entries.len(),
                    describe_choices(pending)
                ));
            }
            if let Some(e) = entries.get(index) {
                return Ok(Some(e.action_id));
            }
        }
        if valid.len() == candidates.len() {
            if let Some(id) = valid.get(index) {
                return Ok(Some(*id));
            }
        }
        return Err(format!(
            "count payload {} chose option {} of {:?}, but the {:?} prompt '{}'              offers {} valid id(s)",
            c,
            index,
            candidates,
            pending.kind,
            pending.prompt,
            valid.len()
        ));
    }

    // ── Branch-index payloads ────────────────────────────────────────────
    if let Some(v) = payload.int_value {
        // EffectChoice: the int is DCGO's branch index — map through entries
        // by position (both sides present branches in card-text order).
        if let Some(entries) = &pending.effect_choices {
            if let Some(e) = entries.get(v as usize) {
                return Ok(Some(e.action_id));
            }
        }
        if v >= 0 {
            if let Some(id) = valid.get(v as usize) {
                return Ok(Some(*id));
            }
        }
        return Err(format!(
            "int payload {} out of range for {:?} prompt '{}' ({} valid ids){}",
            v,
            pending.kind,
            pending.prompt,
            valid.len(),
            describe_choices(pending)
        ));
    }

    Err(format!(
        "selection payload for prompt '{}' carries no usable fields (kind {:?})",
        payload.prompt, pending.kind
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_matching_finds_whole_integer_tokens_only() {
        assert!(label_mentions_value("Digivolve for cost 2", 2));
        assert!(label_mentions_value("Digivolve for cost 2 (App Fusion)", 2));
        assert!(!label_mentions_value("Digivolve for cost 12", 2));
        assert!(!label_mentions_value("Digivolve for cost 20", 2));
        assert!(label_mentions_value("Digivolve for cost 12", 12));
        assert!(!label_mentions_value("Play Tamer", 0));
    }

    #[test]
    fn label_matching_beats_positional_when_values_do_not_start_at_zero() {
        // The exact case that shipped a silently wrong answer: costs {2, 3}.
        // Positionally, count 2 indexes branch 2 -- out of range here, and on a
        // longer list simply the wrong branch. By label it is branch 0.
        let labels = ["Digivolve for cost 2", "Digivolve for cost 3"];
        let hit: Vec<usize> = (0..labels.len())
            .filter(|i| label_mentions_value(labels[*i], 2))
            .collect();
        assert_eq!(hit, vec![0]);

        let hit3: Vec<usize> = (0..labels.len())
            .filter(|i| label_mentions_value(labels[*i], 3))
            .collect();
        assert_eq!(hit3, vec![1]);
    }

    #[test]
    fn label_matching_agrees_on_the_coincidental_zero_based_case() {
        // Costs {0, 1}: the value happens to equal the index, which is exactly
        // why the old positional code looked correct in testing.
        let labels = ["Digivolve for cost 0", "Digivolve for cost 1"];
        for want in [0i64, 1i64] {
            let hit: Vec<usize> = (0..labels.len())
                .filter(|i| label_mentions_value(labels[*i], want))
                .collect();
            assert_eq!(hit, vec![want as usize]);
        }
    }

    // ── Field-permanent frame resolution: sentinel + overflow safety ─────
    //
    // Regression coverage for the reported panic: DCGO's `SelectAttackEffect`
    // chokepoint (`SetAttackTarget` in
    // `DCGO/Assets/Scripts/Script/SelectAttackEffect.cs`) records
    // `frame: -1` for "attack the player" instead of a battle-area
    // permanent — see its `const int SecurityIndex = -1;` and
    // `__frame = permanentIndex < 0 ? -1 : ValidateFieldSlot(...)`. Casting
    // that straight to `u16` wraps to 65535 and overflows the `ATTACK_START`
    // addition a few lines down.

    use crate::card_source::CardHandle;
    use crate::dcgo_recording::FrameTarget;
    use crate::debug_runner::DebugRunner;
    use crate::enums::{EffectSourceKind, GamePhase};
    use crate::selection::PendingSelection;

    /// Build a bare `SelectionRow` carrying only one `FrameTarget` pick —
    /// every other field empty, matching a decoded `SelectAttackEffect` row.
    fn attack_target_row(player: u8, frame: i32) -> SelectionRow {
        SelectionRow {
            step: 0,
            actor: player,
            prompt: "SelectAttackEffect".to_string(),
            phase: "Main".to_string(),
            targets: Some(vec![FrameTarget { player, frame }]),
            card_ids: None,
            indexes: None,
            count: None,
            candidates: None,
            int_value: None,
            bool_value: None,
            cancel: None,
            board_p0: None,
            board_p1: None,
            memory: None,
            mechanic: None,
            zone: None,
        }
    }

    /// Park a `Target`-kind pending selection — the kind an attack-target
    /// prompt installs (`effect_context::action::combat`'s
    /// `may_attack_now_optional_with_upgrade_and_summoning` /
    /// `select_redirect_attack_target`) — with the given valid action ids.
    fn park_attack_target_selection(game: &mut Game, valid_action_ids: Vec<u16>) {
        game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Target,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids,
            is_optional: true,
            prompt: "Select attack target".to_string(),
            effect_choices: None,
            source_card: CardHandle(0),
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            zone_owner: None,
        });
    }

    #[test]
    fn attack_target_sentinel_resolves_side_implicit_security_target() {
        let mut runner = DebugRunner::new();
        // side-implicit candidate: encode_attack(0, SECURITY_TARGET).
        park_attack_target_selection(&mut runner.game, vec![ATTACK_START + SECURITY_TARGET]);

        let payload = attack_target_row(1, -1);
        let result = resolve_next(&runner.game, &payload, 0);

        assert_eq!(result, Ok(Some(ATTACK_START + SECURITY_TARGET)));
    }

    #[test]
    fn attack_target_sentinel_resolves_absolute_security_target() {
        let mut runner = DebugRunner::new();
        // absolute candidate: encode_attack(player=1, SECURITY_TARGET).
        let absolute = ATTACK_START + TARGETS_PER_ATTACKER + SECURITY_TARGET;
        park_attack_target_selection(&mut runner.game, vec![absolute]);

        let payload = attack_target_row(1, -1);
        let result = resolve_next(&runner.game, &payload, 0);

        assert_eq!(result, Ok(Some(absolute)));
    }

    #[test]
    fn attack_target_sentinel_never_panics_when_unmatched() {
        let mut runner = DebugRunner::new();
        // Neither security-target candidate is offered — must be an honest
        // Err, not a panic, and not a wrong match either.
        park_attack_target_selection(&mut runner.game, vec![ATTACK_START]);

        let payload = attack_target_row(1, -1);
        let result = resolve_next(&runner.game, &payload, 0);

        assert!(result.is_err(), "expected Err, got {:?}", result);
    }

    #[test]
    fn unusable_negative_frame_returns_legible_err_not_panic() {
        let mut runner = DebugRunner::new();
        park_attack_target_selection(&mut runner.game, vec![ATTACK_START, ATTACK_START + 1]);

        // -5 is neither the -1 "attack the player" sentinel nor a valid
        // slot; no known DCGO chokepoint emits it. This is the "genuinely
        // unusable" case — it must name the offending value, not panic.
        let payload = attack_target_row(0, -5);
        let result = resolve_next(&runner.game, &payload, 0);

        let err = result.expect_err("frame -5 must not resolve to an action id");
        assert!(
            err.contains("-5"),
            "error must name the offending value, got: {err}"
        );
        assert!(
            err.contains("Target"),
            "error must name the prompt kind, got: {err}"
        );
    }

    #[test]
    fn ordinary_in_range_frame_still_resolves_side_implicit() {
        let mut runner = DebugRunner::new();
        // Regression guard: frame 0 must still resolve exactly as before —
        // encode_attack(0, 0).
        park_attack_target_selection(&mut runner.game, vec![ATTACK_START]);

        let payload = attack_target_row(1, 0);
        let result = resolve_next(&runner.game, &payload, 0);

        assert_eq!(result, Ok(Some(ATTACK_START)));
    }

    // ── Material picks answered by CARD IDENTITY ─────────────────────────
    //
    // The exam answers every selection by card identity on both sides (DCGO's
    // `SelectCardEffect.SetTargetCardAndIndicies` matches `select_card_ids`
    // against its scripted candidates and then sets `Indicies = null`), so a
    // scenario can never name a raw source index. Before the `Material` arm,
    // `resolve_next` dropped this kind through `_ => None` and the only shape
    // that lowered was `yes: true` (legal solely because `valid.len() == 1`),
    // which DCGO refuses outright — blocking EX12-031#effect#0 and
    // EX12-036#effect#2 (<Decode> source picks) at the wire, not at the rule.

    use crate::debug_runner::make_test_card;
    use crate::dsl_cards::bindings::Bindings;
    use crate::dsl_cards::step::StepRuntime;
    use crate::permanent::PermanentHandle;
    use crate::resume::{
        ResumeDecline, ResumeFrame, ResumeProvenance, ResumeSelectKind, ResumeStack,
    };
    use digimon_dsl::compiled::CompiledStep;
    use std::sync::Arc;

    /// A bare `SelectionRow` carrying one card-identity pick — the shape an
    /// exam step's `select: { cards: [X] }` lowers to.
    fn card_row(card_id: &str) -> SelectionRow {
        SelectionRow {
            step: 0,
            actor: 0,
            prompt: "SelectCardEffect".to_string(),
            phase: "Main".to_string(),
            targets: None,
            card_ids: Some(vec![card_id.to_string()]),
            indexes: None,
            count: None,
            candidates: None,
            int_value: None,
            bool_value: None,
            cancel: None,
            board_p0: None,
            board_p1: None,
            memory: None,
            mechanic: None,
            zone: None,
        }
    }

    /// Park a `Material` prompt exactly as `EffectContext::select_material`
    /// does (`effect_context/selections.rs`), plus the data frame its only
    /// caller — `install_select_material` in `dsl_cards/step/selections.rs` —
    /// parks beside it. `carrier` is the permanent whose digivolution sources
    /// are on offer; it is deliberately NOT recoverable from the action ids.
    fn park_material_selection(
        game: &mut Game,
        carrier: PermanentHandle,
        valid_action_ids: Vec<u16>,
    ) {
        let source_card = CardHandle(0);
        game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Material,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids,
            is_optional: false,
            prompt: "Choose a source to play".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            // `select_material` never sets a zone owner — so the carrier's
            // side cannot be inferred from the prompt either.
            zone_owner: None,
        });
        game.pending_selection_resume = Some(ResumeStack {
            frames: vec![ResumeFrame::RunTail {
                prov: ResumeProvenance {
                    source_card,
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    controller: 0,
                    override_pin: None,
                },
                select_kind: ResumeSelectKind::Material { perm: carrier },
                bind_as: None,
                inner_tail: Arc::new(vec![CompiledStep::GainMemory(0)]),
                outer_conts: Vec::new(),
                bindings: Bindings::new(),
                runtime: StepRuntime::default(),
                trigger_context: None,
                decline: ResumeDecline::None,
            }],
        });
    }

    /// A `DebugRunner` whose card pool holds the given ids (all distinct test
    /// cards), so `place_stack` can build carriers out of them.
    fn runner_with_cards(ids: &[&str]) -> DebugRunner {
        let mut builder = DebugRunner::builder();
        for id in ids {
            builder = builder.add_card(make_test_card(id, id));
        }
        builder.memory(0).start()
    }

    fn material_range_start(game: &Game, carrier: PermanentHandle) -> u16 {
        crate::effect_context::selections::material_zone_geometry(game, carrier)
            .expect("carrier has selectable sources")
            .1
    }

    #[test]
    fn material_pick_resolves_by_card_identity() {
        let mut runner = runner_with_cards(&["MAT-A", "MAT-B", "MAT-TOP"]);
        // Bottom-up: source 0 = MAT-A, source 1 = MAT-B, top = MAT-TOP.
        let carrier = runner.place_stack(0, &["MAT-A", "MAT-B", "MAT-TOP"]);
        let base = material_range_start(&runner.game, carrier);
        park_material_selection(&mut runner.game, carrier, vec![base, base + 1]);

        assert_eq!(
            resolve_next(&runner.game, &card_row("MAT-A"), 0),
            Ok(Some(base))
        );
        assert_eq!(
            resolve_next(&runner.game, &card_row("MAT-B"), 0),
            Ok(Some(base + 1))
        );
    }

    #[test]
    fn material_pick_reads_the_carrier_from_the_data_frame_not_the_selecting_player() {
        // The load-bearing case: the action ids encode only a FIELD index, so
        // a resolver that assumed the selecting player's side would read the
        // wrong stack. Both players hold a slot-0 stack containing MAT-B, at
        // DIFFERENT source indices, so a side mix-up returns a different id.
        let mut runner = runner_with_cards(&["MAT-A", "MAT-B", "MAT-TOP"]);
        let _p0_decoy = runner.place_stack(0, &["MAT-B", "MAT-A", "MAT-TOP"]);
        let carrier = runner.place_stack(1, &["MAT-A", "MAT-B", "MAT-TOP"]);
        let base = material_range_start(&runner.game, carrier);
        // Selecting player stays 0 while the carrier belongs to player 1.
        park_material_selection(&mut runner.game, carrier, vec![base, base + 1]);

        // On the carrier MAT-B is source 1; on player 0's decoy it is source 0.
        assert_eq!(
            resolve_next(&runner.game, &card_row("MAT-B"), 0),
            Ok(Some(base + 1))
        );
    }

    #[test]
    fn material_duplicate_ids_resolve_by_occurrence_order() {
        // Interchangeable copies: sources bottom-up are DUP, MAT-X, DUP. Like
        // the Hand / Trash / Reveal arms, the FIRST accepted occurrence wins —
        // here the bottom-most copy.
        let mut runner = runner_with_cards(&["MAT-DUP", "MAT-X", "MAT-TOP"]);
        let carrier = runner.place_stack(0, &["MAT-DUP", "MAT-X", "MAT-DUP", "MAT-TOP"]);
        let base = material_range_start(&runner.game, carrier);
        park_material_selection(&mut runner.game, carrier, vec![base, base + 1, base + 2]);

        assert_eq!(
            resolve_next(&runner.game, &card_row("MAT-DUP"), 0),
            Ok(Some(base))
        );
    }

    #[test]
    fn material_duplicate_skips_a_copy_the_prompt_does_not_accept() {
        // Same stack, but the install-time filter excluded the bottom copy —
        // the scan must continue to the next occurrence rather than returning
        // an id the engine would reject.
        let mut runner = runner_with_cards(&["MAT-DUP", "MAT-X", "MAT-TOP"]);
        let carrier = runner.place_stack(0, &["MAT-DUP", "MAT-X", "MAT-DUP", "MAT-TOP"]);
        let base = material_range_start(&runner.game, carrier);
        park_material_selection(&mut runner.game, carrier, vec![base + 1, base + 2]);

        assert_eq!(
            resolve_next(&runner.game, &card_row("MAT-DUP"), 0),
            Ok(Some(base + 2))
        );
    }

    #[test]
    fn material_top_card_is_never_a_candidate() {
        // `select_material` offers sources only — the active Digimon on top of
        // the stack is not one, so naming it must be an honest Err.
        let mut runner = runner_with_cards(&["MAT-A", "MAT-TOP"]);
        let carrier = runner.place_stack(0, &["MAT-A", "MAT-TOP"]);
        let base = material_range_start(&runner.game, carrier);
        park_material_selection(&mut runner.game, carrier, vec![base]);

        let err = resolve_next(&runner.game, &card_row("MAT-TOP"), 0)
            .expect_err("the top card is not a selectable material");
        assert!(err.contains("MAT-TOP"), "error must name the pick: {err}");
        assert!(err.contains("Material"), "error must name the kind: {err}");
    }

    #[test]
    fn material_without_a_data_frame_is_an_honest_err_not_a_guess() {
        // A closure-only Material installer parks no `ResumeSelectKind::Material`
        // frame, so the carrier is unknowable — and guessing a side would pick a
        // source index out of the wrong stack, which resolves cleanly and takes
        // the wrong card. Must fail loudly instead.
        let mut runner = runner_with_cards(&["MAT-A", "MAT-TOP"]);
        let carrier = runner.place_stack(0, &["MAT-A", "MAT-TOP"]);
        let base = material_range_start(&runner.game, carrier);
        park_material_selection(&mut runner.game, carrier, vec![base]);
        runner.game.pending_selection_resume = None;

        assert!(
            resolve_next(&runner.game, &card_row("MAT-A"), 0).is_err(),
            "a carrier-less Material prompt must not resolve to an action id"
        );
    }

    #[test]
    fn foreign_material_prompt_is_rejected_rather_than_mis_decoded() {
        // DNA digivolution and DigiXros assembly reuse `SelectionKind::Material`
        // with raw field / hand indices (`game_actions/digivolve.rs`,
        // `game_actions/misc.rs`). Those ids are nowhere near the carrier's
        // source band, so the band cross-check must reject them outright.
        let mut runner = runner_with_cards(&["MAT-A", "MAT-TOP"]);
        let carrier = runner.place_stack(0, &["MAT-A", "MAT-TOP"]);
        // Raw battle-area indices, exactly as `initiate_dna_digivolve` installs.
        park_material_selection(&mut runner.game, carrier, vec![0, 1]);

        assert!(
            resolve_next(&runner.game, &card_row("MAT-A"), 0).is_err(),
            "a DNA-style Material prompt must not be decoded as a source band"
        );
    }

    // ── CountCappedMultiSelect picks answered by CARD IDENTITY ───────────
    //
    // One variant over from `Material`, and blocking for the same reason:
    // `resolve_next` dropped `CountCappedMultiSelect` through `_ => None`, so
    // a scenario had no way to name a card in a multi-pick. Measured on
    // EX12-076#effect#0 (Susanoomon's <Rush>, reached via [Assembly -9]):
    //
    //   card pick 'EX12-009' not found in CountCappedMultiSelect
    //   { min: 0, max: 8, picked: 0, distinct: false }
    //   prompt 'Assembly: select 8 card(s) from trash to place under
    //   (No Selection to skip).' (zone owner 0, valid [1150, 1151, ... 1160])
    //
    // The ids are `TRASH_EFFECT_START`-based (`action/space.rs`), installed by
    // `install_assembly_element` (`game_actions/mod.rs`) via
    // `select_count_capped_multi_min(CountCappedZone::Trash, ..)`.

    use crate::effect_context::CountCappedZone;
    use crate::resume::{MultiPickState, NonDslCountCappedState, NonDslCountCappedTerminal};

    fn test_provenance() -> ResumeProvenance {
        ResumeProvenance {
            source_card: CardHandle(0),
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            controller: 0,
            override_pin: None,
        }
    }

    /// Park a `CountCappedMultiSelect` prompt exactly as the installers do:
    /// `zone_owner` is `None` at every one of the three construction sites
    /// (`install_count_capped_step`, `park_non_dsl_count_capped_state`,
    /// `install_multipick_step`), and the kind carries no zone — so neither
    /// the zone nor its owner is recoverable from the prompt alone.
    fn park_count_capped_prompt(
        game: &mut Game,
        selecting_player: crate::enums::PlayerId,
        valid_action_ids: Vec<u16>,
        frame: Option<ResumeFrame>,
    ) {
        game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::CountCappedMultiSelect {
                min: 0,
                max: 8,
                picked: 0,
                distinct: false,
            },
            selecting_player,
            previous_phase: GamePhase::Main,
            valid_action_ids,
            is_optional: true,
            prompt: "Assembly: select 8 card(s) from trash to place under \
                     (No Selection to skip)."
                .to_string(),
            effect_choices: None,
            source_card: CardHandle(0),
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            // Every installer hardcodes this — the resolver's
            // `unwrap_or(selecting_player)` fallback is a guess, not a read.
            zone_owner: None,
        });
        game.pending_selection_resume = frame.map(|f| ResumeStack { frames: vec![f] });
    }

    /// The frame `install_assembly_element` / `trash_opponent_hand_to_count_bound`
    /// park beside the prompt (`ResumeFrame::NonDslCountCappedStep`).
    fn non_dsl_frame(
        of_player: crate::enums::PlayerId,
        zone: CountCappedZone,
        candidate_actions: Vec<u16>,
    ) -> ResumeFrame {
        ResumeFrame::NonDslCountCappedStep(NonDslCountCappedState {
            prov: test_provenance(),
            of_player,
            zone,
            min: 0,
            max: 8,
            is_optional_zero: true,
            distinct_by: None,
            candidate_actions,
            accum: Vec::new(),
            prompt: "Assembly: select 8 card(s) from trash to place under \
                     (No Selection to skip)."
                .to_string(),
            previous_phase: GamePhase::Main,
            terminal: NonDslCountCappedTerminal::TrashOpponentHandToCount {
                opponent: of_player,
                bind_count_as: None,
            },
            outer_conts: Vec::new(),
        })
    }

    /// The frame the DSL `count_capped` family parks
    /// (`ResumeFrame::MultiPickStep`, `install_select_count_capped_multi`).
    fn multipick_frame(
        of_player: crate::enums::PlayerId,
        selecting_player: crate::enums::PlayerId,
        zone: CountCappedZone,
        range_start: u16,
        candidate_indices: Vec<usize>,
    ) -> ResumeFrame {
        ResumeFrame::MultiPickStep(MultiPickState {
            prov: test_provenance(),
            of_player,
            selecting_player,
            previous_phase: GamePhase::Main,
            zone,
            range_start,
            min: 0,
            max: 8,
            is_optional_zero: true,
            distinct_by: None,
            candidate_indices,
            accum: Vec::new(),
            bind_as: None,
            inner_tail: Arc::new(vec![CompiledStep::GainMemory(0)]),
            bindings: Bindings::new(),
            runtime: StepRuntime::default(),
            trigger_context: None,
            outer_conts: Vec::new(),
        })
    }

    fn seed_trash(runner: &mut DebugRunner, player: crate::enums::PlayerId, ids: &[&str]) {
        for id in ids {
            runner.inject_trash(player, id);
        }
        assert_eq!(
            runner.trash_size(player),
            ids.len(),
            "trash must hold exactly the seeded cards, or the index math below is off"
        );
    }

    #[test]
    fn count_capped_trash_pick_resolves_by_card_identity() {
        // The measured EX12-076 shape: an [Assembly] trash multi-pick.
        let mut runner = runner_with_cards(&["CC-A", "CC-B", "CC-C"]);
        seed_trash(&mut runner, 0, &["CC-A", "CC-B", "CC-C"]);
        let valid = vec![
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
            TRASH_EFFECT_START + 2,
        ];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid.clone(),
            Some(non_dsl_frame(0, CountCappedZone::Trash, valid)),
        );

        for (i, id) in ["CC-A", "CC-B", "CC-C"].iter().enumerate() {
            assert_eq!(
                resolve_next(&runner.game, &card_row(id), 0),
                Ok(Some(TRASH_EFFECT_START + i as u16)),
                "trash pick {id}"
            );
        }
    }

    #[test]
    fn count_capped_duplicate_ids_resolve_by_occurrence_order() {
        // Interchangeable copies: trash is DUP, X, DUP. Like the Hand / Trash
        // / Reveal / Material arms, the FIRST accepted occurrence wins.
        let mut runner = runner_with_cards(&["CC-DUP", "CC-X"]);
        seed_trash(&mut runner, 0, &["CC-DUP", "CC-X", "CC-DUP"]);
        let valid = vec![
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
            TRASH_EFFECT_START + 2,
        ];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid.clone(),
            Some(non_dsl_frame(0, CountCappedZone::Trash, valid)),
        );

        assert_eq!(
            resolve_next(&runner.game, &card_row("CC-DUP"), 0),
            Ok(Some(TRASH_EFFECT_START))
        );
    }

    #[test]
    fn count_capped_duplicate_skips_a_copy_the_prompt_does_not_accept() {
        // Same trash, but index 0 is no longer offered — a multi-pick drops
        // each taken index from `valid_action_ids` before re-parking, so on
        // the second step the bottom copy is gone. The scan must continue to
        // the next occurrence instead of returning an id the engine rejects.
        let mut runner = runner_with_cards(&["CC-DUP", "CC-X"]);
        seed_trash(&mut runner, 0, &["CC-DUP", "CC-X", "CC-DUP"]);
        let valid = vec![TRASH_EFFECT_START + 1, TRASH_EFFECT_START + 2];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid.clone(),
            Some(non_dsl_frame(0, CountCappedZone::Trash, valid)),
        );

        assert_eq!(
            resolve_next(&runner.game, &card_row("CC-DUP"), 0),
            Ok(Some(TRASH_EFFECT_START + 2))
        );
    }

    #[test]
    fn count_capped_reads_the_zone_owner_from_the_frame_not_the_selecting_player() {
        // The load-bearing case, and the reason band-arithmetic alone is not
        // enough: `zone_owner` is `None` on every count-capped prompt, so the
        // resolver's `unwrap_or(selecting_player)` fallback reads the WRONG
        // player's trash whenever a card picks out of the opponent's — which
        // is live in the pool (BT18-019: `select_count_capped_multi
        // { of: opponent, zone: trash, max: 7 }`).
        //
        // Both trashes hold CC-B at a DIFFERENT index, and both are two cards
        // long, so both action ids are in `valid` either way: a side mix-up
        // resolves CLEANLY to the other id rather than erroring. The two halves
        // below are each other's control — if the owner were being ignored they
        // would return the same id, and one of the assertions would fail.
        let mut runner = runner_with_cards(&["CC-B", "CC-X"]);
        seed_trash(&mut runner, 0, &["CC-X", "CC-B"]); // p0: CC-B at index 1
        seed_trash(&mut runner, 1, &["CC-B", "CC-X"]); // p1: CC-B at index 0
        let valid = vec![TRASH_EFFECT_START, TRASH_EFFECT_START + 1];

        // Selecting player 0, but the zone belongs to player 1.
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid.clone(),
            Some(non_dsl_frame(1, CountCappedZone::Trash, valid.clone())),
        );
        assert_eq!(
            resolve_next(&runner.game, &card_row("CC-B"), 0),
            Ok(Some(TRASH_EFFECT_START)),
            "of_player 1 must read player 1's trash (CC-B at index 0)"
        );

        // Sibling case: same prompt, same selecting player, owner 0 instead —
        // proving the assertion above is about the frame, not about a constant.
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid.clone(),
            Some(non_dsl_frame(0, CountCappedZone::Trash, valid)),
        );
        assert_eq!(
            resolve_next(&runner.game, &card_row("CC-B"), 0),
            Ok(Some(TRASH_EFFECT_START + 1)),
            "of_player 0 must read player 0's trash (CC-B at index 1)"
        );
    }

    #[test]
    fn count_capped_hand_pick_resolves_by_card_identity_via_the_dsl_frame() {
        // The other frame variant (`MultiPickStep`, the DSL `count_capped`
        // family) and the other card band (`PLAY_HAND_START`).
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("CC-H1", "CC-H1"))
            .add_card(make_test_card("CC-H2", "CC-H2"))
            .hand(0, &["CC-H1", "CC-H2"])
            .memory(0)
            .start();
        // Pin the hand geometry the ids below assume (`start_game` pushes any
        // turn-1 draw onto the END of the hand, so 0/1 stay put — assert it
        // rather than trust it).
        let hand_ids: Vec<String> = runner.game.player(0).hand[..2]
            .iter()
            .map(|c| c.card_id(&runner.game.card_data).to_string())
            .collect();
        assert_eq!(hand_ids, vec!["CC-H1".to_string(), "CC-H2".to_string()]);
        let valid = vec![PLAY_HAND_START, PLAY_HAND_START + 1];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid,
            Some(multipick_frame(
                0,
                0,
                CountCappedZone::Hand,
                PLAY_HAND_START,
                vec![0, 1],
            )),
        );

        assert_eq!(
            resolve_next(&runner.game, &card_row("CC-H2"), 0),
            Ok(Some(PLAY_HAND_START + 1))
        );
    }

    #[test]
    fn count_capped_material_pick_resolves_by_card_identity() {
        // `CountCappedZone::Material` shares the source-band geometry with
        // `SelectionKind::Material`, but the KIND here is
        // `CountCappedMultiSelect`, so the `Material` arm never fires for it.
        let mut runner = runner_with_cards(&["CC-M1", "CC-M2", "CC-MTOP"]);
        let carrier = runner.place_stack(0, &["CC-M1", "CC-M2", "CC-MTOP"]);
        let base = material_range_start(&runner.game, carrier);
        let valid = vec![base, base + 1];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid,
            Some(multipick_frame(
                0,
                0,
                CountCappedZone::Material(carrier),
                base,
                vec![0, 1],
            )),
        );

        assert_eq!(
            resolve_next(&runner.game, &card_row("CC-M2"), 0),
            Ok(Some(base + 1))
        );
        // The top card is not a selectable source (stack_len - 1 candidates).
        assert!(resolve_next(&runner.game, &card_row("CC-MTOP"), 0).is_err());
    }

    #[test]
    fn count_capped_without_a_data_frame_is_an_honest_err_not_a_guess() {
        // The legacy closure-only installer (`install_count_capped_step`)
        // parks no resume frame, so neither zone nor owner is knowable. A
        // band-arithmetic guess would resolve cleanly against whichever trash
        // the selecting player happens to own and silently take a wrong card,
        // so this must fail loudly instead.
        let mut runner = runner_with_cards(&["CC-A", "CC-B"]);
        seed_trash(&mut runner, 0, &["CC-A", "CC-B"]);
        park_count_capped_prompt(
            &mut runner.game,
            0,
            vec![TRASH_EFFECT_START, TRASH_EFFECT_START + 1],
            None,
        );

        let err = resolve_next(&runner.game, &card_row("CC-A"), 0)
            .expect_err("a frameless count-capped prompt must not resolve to an action id");
        assert!(err.contains("CC-A"), "error must name the pick: {err}");
    }

    #[test]
    fn count_capped_rejects_a_frame_whose_band_does_not_match_the_prompt() {
        // Cross-check: the frame claims Trash, but the live prompt's ids are
        // in the hand band. That is a stale/foreign frame, not a trash pick —
        // decoding it would index the trash with hand offsets.
        let mut runner = runner_with_cards(&["CC-A", "CC-B"]);
        seed_trash(&mut runner, 0, &["CC-A", "CC-B"]);
        let hand_band = vec![PLAY_HAND_START, PLAY_HAND_START + 1];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            hand_band,
            Some(non_dsl_frame(
                0,
                CountCappedZone::Trash,
                vec![TRASH_EFFECT_START, TRASH_EFFECT_START + 1],
            )),
        );

        assert!(
            resolve_next(&runner.game, &card_row("CC-A"), 0).is_err(),
            "a frame whose zone band does not contain the prompt's ids must be rejected"
        );
    }

    #[test]
    fn count_capped_single_card_payload_still_emits_the_trailing_pass() {
        // CHARACTERIZATION, not new behaviour — this held before the
        // `CountCappedMultiSelect` arm existed and is unchanged by it. Pinned
        // here because it is the NEXT blocker on the scenario that motivated
        // the arm, and the two are easy to confuse.
        //
        // `resolve_next`'s trailing-PASS rule (module header) fires once a
        // payload is exhausted against a multiselect-ish prompt where PASS is
        // legal. `exam/adapter.rs`'s `SelectPayload::Materials` loop answers an
        // [Assembly] declaration ONE id per `advance_through_selection` call,
        // and that helper loops `resolve_next` to `Ok(None)` — so each element
        // is followed by a PASS that COMMITS the multi-pick at one material,
        // and the next element finds no prompt parked ("`materials:` declares
        // 8 cards but our engine stopped asking after 1").
        //
        // That is an adapter-side cardinality question, not a resolution one:
        // the pick itself now resolves by identity (the tests above). Left for
        // whoever owns `exam/adapter.rs`.
        let mut runner = runner_with_cards(&["CC-A", "CC-B"]);
        seed_trash(&mut runner, 0, &["CC-A", "CC-B"]);
        let valid = vec![TRASH_EFFECT_START, TRASH_EFFECT_START + 1];
        park_count_capped_prompt(
            &mut runner.game,
            0,
            valid.clone(),
            Some(non_dsl_frame(0, CountCappedZone::Trash, valid)),
        );

        let row = card_row("CC-A");
        assert_eq!(payload_pick_count(&row), 1);
        assert_eq!(
            resolve_next(&runner.game, &row, 0),
            Ok(Some(TRASH_EFFECT_START)),
            "pick 0 resolves by identity"
        );
        assert_eq!(
            resolve_next(&runner.game, &row, 1),
            Ok(Some(PASS)),
            "a one-card payload against an 8-card multi-pick still stops the prompt"
        );
    }

    #[test]
    fn ordinary_in_range_frame_still_resolves_absolute() {
        let mut runner = DebugRunner::new();
        // Regression guard: frame 1 on player 1's target must still resolve
        // via encode_attack(1, 1) exactly as before.
        let absolute = ATTACK_START + TARGETS_PER_ATTACKER + 1;
        park_attack_target_selection(&mut runner.game, vec![absolute]);

        let payload = attack_target_row(1, 1);
        let result = resolve_next(&runner.game, &payload, 0);

        assert_eq!(result, Ok(Some(absolute)));
    }
}
