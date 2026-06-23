# make-engine-cloneable — remaining DSL/step-file selection installers (flip plan)

Produced 2026-06-23 by the remaining-dsl-installer-flip-specs workflow (6 agents). Raw per-installer specs + full recipes: see workflow output (this is the synthesis).

## Recommended order
- zone_moves.rs:66 MayAddTopSecurityToHand (atomic Security park â€” pending: confirm add_top_security_to_hand fires no draining trigger)
- combat.rs:152 select_redirect_attack_target (lands new AttackTarget variant; atomic substitution)
- install_trash_bottom_face_down_source_under_tamer selections.rs:3405 (FieldPermanent post-action enum extension; nested-park threads via park_pending_selection_tail, no bespoke channel)
- install_select_dna_pair selections.rs:3849 (new DnaPairRight left-frame; re-invokes already-flipped install_select_any_permanent for right)
- install_use_option_from_hand selections.rs:2972 (new UseOptionFromHandStep frame; drain_or_rewrap channel; outer_conts ordering risk)
- install_trash_n_bottom_face_down_sources_under_tamers selections.rs:3504 (new TrashUnderTamersStep mutation-per-pick frame; after_selection_resume_hooks per-pick defer)
- combat.rs:93/102/124 may_attack_now_*/force_opponent_attack (reuse AttackTarget; begin_attack_open sub-machine defers via channel)
- link_cards.rs 248/394/396/459/492/613/766 (new LinkPickState recursive frame; OnLink/absorb mid-callback nested installs via channel)

## Needs continuation channel (after_selection_resume_hooks / drain_or_rewrap)
- install_use_option_from_hand (selections.rs:2972) â€” drain_or_rewrap_pending_tail / wrap_pending_selection_with_tail; outer_conts ordering across the option-play park is the single highest-risk detail
- install_trash_n_bottom_face_down_sources_under_tamers (selections.rs:3504) â€” after_selection_resume_hooks to defer the next pick when a per-pick trash observer parks; likely needs run_after_selections_drain re-arm for the 3-deep P-169 chain
- combat.rs:93/102 may_attack_now_* and combat.rs:124 force_opponent_attack â€” begin_attack_open starts the attack sub-machine (counter/block/alliance interrupts); defer inner_tail onto after_selection_resume_hooks, never run inline
- link_cards.rs 394/396/459/613 â€” link_chosen_card_into_host fires OnLink + maybe_drain mid-callback; defer the next recursive pick via the channel
- link_cards.rs 766 â€” absorb_standing_digimon_as_link fires OnDigivolutionCardTrashed + OnLinkedCardTrashed + OnLink with 3 queue drains; multiple mid-callback nested-install points requiring the channel

## Full plan

# Implementation Plan â€” Flip the Remaining Closure-Based DSL Selection Installers onto the Resumable VM

## Grounding (verified against source)

I confirmed the load-bearing infrastructure the specs rely on, in this worktree:

- `ResumeFrame` (resume.rs:213-274) and `ResumeSelectKind` (resume.rs:109-159) are the variant homes. `Hand`, `Trash`, `FieldPermanent`, `Security`, `Reveal{route}`, `AnyPermanent{candidates}` already exist and are flipped.
- The `FieldPermanent` run_resume arm (selections.rs:352-399) decodes `(action_id - ATTACK_START) % TARGETS_PER_ATTACKER`, pushes an effect-target, scopes `effect_source_player = Some(prov.controller)`, binds via `bind_as`, then runs `inner_tail` **inline** via `run_tail_preserving_trigger_context`. This confirms the spec's claim that the existing RunTail arms run the tail inline and so cannot host a parking post-action without modification.
- `wrap_pending_selection_with_tail` (mod.rs:248-279) composes an outer tail **as data** onto the active frame when `pending_selection_resume.is_some()`, with an **exhaustive** frame match (mod.rs:263-275) ending in `None => unreachable!`. Every new `ResumeFrame` variant MUST be added here or a nested park hits `unreachable!`.
- `after_selection_resume_hooks` drains in effect_queue.rs:3613-3616 **after** `run_resume` returns, and a hook that re-installs a resume-driven selection re-arms by pushing onto the (now-emptied) channel for the next resolution. This is the exact mechanism the mutation-per-pick cases need.

The specs are accurate; nothing below contradicts the code I read.

---

## 1. RECOMMENDED ORDER (easiest/lowest-risk first)

The order is chosen to (a) land the atomic/no-nesting flips first to de-risk the new-variant plumbing, (b) group the two `FieldPermanent`-extension installers so they share one enum change + one gate, and (c) defer the genuinely adversarial nested-park cases until the continuation-channel pattern is exercised by a simpler card.

**Batch A â€” atomic single-pick flips (LOW risk, no nested selection):**
1. **zone_moves.rs:66 â€” MayAddTopSecurityToHand** (MODERATE per spec, but truly the cheapest): bespoke Security park â†’ `ResumeSelectKind::Security{of_player}` + `ResumeDecline::None`. Atomic post-action. *Gated by open question Q on whether `add_top_security_to_hand` fires a draining trigger â€” resolve first; if it drains, it moves to Batch D.*
2. **combat.rs:152 â€” select_redirect_attack_target**: the ONE atomic combat site. New `ResumeSelectKind::AttackTarget{attacker}` + new dispatcher arm; post-action `apply_attack_target_substitution_with_reason` is a pure synchronous substitution (no queue drain). `ResumeDecline::None`. Lands the new combat variant cheaply so the HARD combat sites reuse it.

**Batch B â€” FieldPermanent post-action extension (MODERATE, additive enum field):**
3. **install_trash_bottom_face_down_source_under_tamer (selections.rs:3405)**: extend `ResumeSelectKind::FieldPermanent{of_player}` â†’ `FieldPermanent{of_player, post: Option<FieldPermanentPostAction>}` with `FieldPermanentPostAction::TrashBottomFaceDownSource`. All existing FieldPermanent installers pass `post: None` (purely additive). Cost-gated tail. **This one DOES carry nested-selection risk** (the trash fires `OnDigivolutionCardTrashed` + synchronous drain), but per the spec it threads automatically via `park_pending_selection_tail` inside `run_tail_preserving_trigger_context` (the tail's first step parks the remainder onto the nested select) â€” so it needs NO bespoke channel wiring, only the FieldPermanent post-action marker + the nested-park test. Land it here because the enum change is small and it's the template for #6.

**Batch C â€” DNA pair (MODERATE, composes two already-flipped installers):**
4. **install_select_dna_pair (selections.rs:3849)**: flip ONLY the LEFT pick into a new `ResumeSelectKind::DnaPairRight{candidates, right_filter, bind_right_as, right_prompt, optional}`; the LEFT resume arm re-invokes the **already-flipped** `install_select_any_permanent(excluded=Some(left))` for the RIGHT pick, which parks its own `AnyPermanent` frame. `ResumeDecline::None` on the left (declining the pair runs nothing). The only real subtlety is `outer_conts` ordering across the right-install (see Â§3).

**Batch D â€” mutation-per-pick + sub-machine flips (HARD, need the continuation channel):**
5. **install_use_option_from_hand (selections.rs:2972)**: new `ResumeFrame::UseOptionFromHandStep`. Hand decode â†’ `use_option_from_hand_without_paying_cost` (can park) â†’ `drain_or_rewrap_pending_tail` for the tail. Decline RUNS the same tail (continue-tail). The hardest ordering interaction in the single-installer set (outer_conts vs the option's nested park).
6. **install_trash_n_bottom_face_down_sources_under_tamers (selections.rs:3504)**: new `ResumeFrame::TrashUnderTamersStep` â€” the ONLY multi-pick that mutates game state on every pick. Each pick trashes (fires triggers + synchronous drain) then re-parks the next pick; nested observer parks must defer the next pick via `after_selection_resume_hooks`. No existing exemplar.
7. **combat.rs:93/102/124 â€” may_attack_now_* / force_opponent_attack**: reuse the Batch-A `AttackTarget` variant but the post-action `begin_attack_open` spins up the attack sub-machine (its own interrupt selections). Tail/follow-up MUST defer onto the continuation channel, never run inline.
8. **link_cards.rs (248, 394, 396, 459, 492, 613, 766)**: the recursive link pick-loop. Introduce a dedicated `LinkPickState` frame (mirroring `SourceMultiState`/`MultiPickState`) carrying the `LinkCardsSpec` (Arc, pure), `pick_index`, accumulated `bind_as` list, and the zone-choiceâ†’card-selectâ†’host-select sub-flow. `link_chosen_card_into_host` (OnLink + maybe_drain) and `absorb_standing_digimon_as_link` (3 trigger families + 3 drains) are mid-callback nested installs requiring the channel. Land LAST â€” largest surface, most nested-install points.

**Rationale for grouping:** #3 and #6 are the two `select_own_permanent` callers that build on `FieldPermanent`; doing #3 first establishes the `FieldPermanentPostAction` marker that #6's spec explicitly says should not be precluded. #2 lands the `AttackTarget` variant that #7 reuses. #4 (DNA) is sequenced before the HARD batch because it reuses an already-flipped installer (lowest new-machinery cost despite touching two picks).

---

## 2. PER-INSTALLER IMPLEMENTATION RECIPES

### #1 zone_moves.rs:66 â€” MayAddTopSecurityToHand
- **resume.rs**: no new variant (`Security{of_player}` exists). Verify the `Security` decode tolerates a *pinned* single valid_action_id (the top-security slot) â€” the install currently sets `valid_action_ids` to one base+top_index id; the decode is `action_id - base`, which resolves the same slot, so it's fine.
- **run_resume**: reuse the existing `Security` arm; post-action `add_top_security_to_hand(of_player)` runs in that arm. **PREREQ**: confirm it is atomic (open question Q). If it fires no draining trigger, no channel needed.
- **installer edit**: replace the bespoke `pending_selection = Some(PendingSelection{kind:Security, callback})` with `ctx.select_security(...)`-style install + park `ResumeStack{frames:vec![RunTail{select_kind:Security{of_player:target}, bind_as:None, inner_tail:Arc::new(tail), decline:ResumeDecline::None, ...}]}`.
- **clobber guard**: install returns early when `security_len==0` (L45-47); guard the park `if ctx.game.pending_selection.is_some()`.
- **decline**: `ResumeDecline::None` (is_optional=true, no-op on_decline â†’ PASS does nothing).
- **gate tests**: `tests/dsl/zone_movement_verbs.rs`, `tests/zone_manipulation.rs`; add a resume.rs unit test (clone-independence + pinned-slot decode).

### #2 combat.rs:152 â€” select_redirect_attack_target (lands the new combat variant)
- **resume.rs**: add `ResumeSelectKind::AttackTarget{attacker: PermanentHandle}` (Copy). Add it to `wrap_pending_selection_with_tail` if it's a RunTail select_kind (it is â€” no frame-match change needed since the match is on `ResumeFrame`, not `select_kind`; only NEW `ResumeFrame` variants need a match arm).
- **run_resume**: new arm â€” decode via `decode_attack(action_id)`, validate redirect target, call `apply_attack_target_substitution_with_reason(target, EffectRedirect(Some(prov.source_card)))`, then run `inner_tail` + outer_conts as normal RunTail (atomic, no defer).
- **installer edit**: replace the closure callback with park of `RunTail{select_kind:AttackTarget{attacker}, ...}` guarded by `if ctx.game.pending_selection.is_some()`; keep the coexistence closure.
- **clobber guard**: returns `Ok(())` on empty valid_action_ids (combat.rs:130-132) â€” nothing installed.
- **decline**: `ResumeDecline::None` (no-op on_decline when optional).
- **gate tests**: `tests/cards_behavioral/bt23/bt23_013.rs`, `tests/cards_behavioral/ad1/ad1_012.rs` (post-DNA redirect); add a resume.rs `AttackTarget` arm test.

### #3 install_trash_bottom_face_down_source_under_tamer (selections.rs:3405)
- **resume.rs**: change `FieldPermanent{of_player}` â†’ `FieldPermanent{of_player, post: Option<FieldPermanentPostAction>}`; new `enum FieldPermanentPostAction{TrashBottomFaceDownSource}`. All other FieldPermanent installers pass `post:None`.
- **run_resume (FieldPermanent arm, selections.rs:352)**: after decode + `effect_source_player` scoping + ctx build, `match post { None => run_tail (existing); Some(TrashBottomFaceDownSource) => { let trashed = ctx.trash_bottom_face_down_source(h); debug_assert!(trashed); if trashed { run_tail_preserving_trigger_context(...inner_tail...) } } }`. The trash's synchronous drain may install a nested selection BEFORE the tail; the tail's first step then re-parks the remainder via `park_pending_selection_tail` (step/mod.rs:536-537) â€” identical to the closure path, NO bespoke channel.
- **installer edit**: keep `ctx.select_own_permanent(...)`, drop the real logic from the closure (coexistence stub), park `FieldPermanent{of_player:target, post:Some(TrashBottomFaceDownSource)}` with `inner_tail = Arc::new(tail.clone())` captured BEFORE the move.
- **clobber guard**: `if ctx.game.pending_selection.is_some()` (always true here â€” candidates non-empty guaranteed by the caller at :1782-1794 â€” but keep for pattern parity).
- **decline**: optional iff the step's `optional` (ST24-11). `optional â‡’ ResumeDecline::RunTail{tail: Arc::new(vec![]), aborts_clause:true}` (empty tail is clearer than reusing inner_tail; both correct â€” `aborts_clause:true` short-circuits via `dsl_clause_aborted`). Non-optional â‡’ `ResumeDecline::None`.
- **gate tests**: `tests/dsl/trash_bottom_face_down_source_under_tamer.rs` (incl. `#[should_panic]` desync), `tests/cards_behavioral/st24/` (ST24-11/12). **CRITICAL new test**: a resume.rs unit test seeding an `OnDigivolutionCardTrashed` observer that parks (EX10-036 shape) and asserting `inner_tail` re-parks via `park_pending_selection_tail` rather than running before the nested select resolves.

### #4 install_select_dna_pair (selections.rs:3849)
- **resume.rs**: add `ResumeSelectKind::DnaPairRight{candidates: Vec<(u16, PermanentHandle)>, right_filter: CompiledPredicate, bind_right_as: String, right_prompt: String, optional: bool}` (all Clone). The LEFT frame is a normal `RunTail` with `select_kind=DnaPairRight`, `bind_as=Some(bind_left_as)`, `inner_tail=Arc::new(post_dna_tail)`, `decline=ResumeDecline::None`.
- **run_resume (new DnaPairRight arm)**: linear-search `candidates` for `action_id` â†’ `left`; build cb_ctx; `b.insert_permanent(bind_left_as, left)`; call `install_select_any_permanent(&mut ctx, right_filter, Some(left), None, Some(bind_right_as), right_prompt, optional, (*inner_tail).clone(), b, runtime)` â€” which re-derives right candidates (minus `left`) and parks its OWN `AnyPermanent` frame. **Do NOT run inner_tail here** (it becomes the right pick's tail). Whether to `push_effect_target` for the left: the closure does NOT â€” match it (no push) to avoid drift.
- **installer edit**: keep the bespoke inline candidate loop + `AnyField` PendingSelection install for the LEFT (coexistence), park `RunTail{select_kind:DnaPairRight{...}}` guarded `if ctx.game.pending_selection.is_some()`.
- **clobber guard**: two layers â€” install bails `if candidates.is_empty()` before installing (selections.rs:3881); inside the LEFT arm, `install_select_any_permanent` ALSO bails on empty right candidates (selections.rs:3748) and runs nothing.
- **decline**: `ResumeDecline::None` on the left (matches the no-on_decline closure â€” declining the pair must NOT run the DNA verb). The whole-clause optionality lives one level up in the YAML `optional:` wrapper.
- **gate tests**: `tests/cards_behavioral/bt20/bt20_016.rs`, `ad1/ad1_009.rs`, `ad1/ad1_012.rs` (post-DNA redirect chain), EX9-013/BT24-035 if present; add a resume.rs two-pick test asserting the right install excludes the left handle and a Game::clone taken **between** the left and right pick replays identically.

### #5 install_use_option_from_hand (selections.rs:2972) â€” HARD
- **resume.rs**: add `ResumeFrame::UseOptionFromHandStep(UseOptionFromHandState)`. State: `prov`, `of_player`, `tail: Arc<Vec<CompiledStep>>` (shared accept/decline), `bindings`, `runtime`, `trigger_context: Option<TriggerContext>`, `outer_conts: Vec<OuterContinuation>`, `optional: bool`. **Add it to `wrap_pending_selection_with_tail`'s frame match (mod.rs:263-275): `Some(ResumeFrame::UseOptionFromHandStep(s)) => s.outer_conts.push(cont)`** â€” else a nested option play that parks THIS frame hits `unreachable!`.
- **run_resume (new arm)**: see pseudocode in spec. ACCEPT: decode `idx = action_id - PLAY_HAND_START`; push_effect_target; set trigger; `use_option_from_hand_without_paying_cost(of_player, idx)`; restore trigger; early-return on `OptionPlayResult::Invalid` (NO tail, NO outer_conts); else `drain_or_rewrap_pending_tail(...tail..., tail_context=trigger_context)`. DECLINE (optional): set trigger; `drain_or_rewrap_pending_tail` with the SAME tail (continue-tail, NO `dsl_clause_aborted`); restore trigger.
- **installer edit**: keep `ctx.select_hand(...)` + on_decline wiring (non-resume path); after, `if ctx.game.pending_selection.is_some() { pending_selection_resume = Some(ResumeStack{frames:vec![UseOptionFromHandStep(state)]}) }`.
- **clobber guard**: load-bearing â€” `select_hand` returns WITHOUT installing when no eligible option; guard the park `if ctx.game.pending_selection.is_some()`. The installed selection IS this hand-select (last thing the installer does), so no inner-step clobber risk.
- **decline**: continue-tail (RUN the same tail), NOT a cost-gated abort. The "cost" is paid one clause up (the option plays Free).
- **gate tests**: `tests/cards_behavioral/bt24/bt24_085.rs` (both existing tests), `tests/dsl/option_effect_use.rs`. **NEW test (highest-risk)**: an Option whose `OptionMain` installs a resume-driven select, wrapped by an outer clause that pushed an `OuterContinuation` onto this frame â€” asserts the outer cont runs AFTER this frame's tail even across the nested park (see Â§3 ordering).

### #6 install_trash_n_bottom_face_down_sources_under_tamers (selections.rs:3504) â€” HARD
- **resume.rs**: add `ResumeFrame::TrashUnderTamersStep(TrashUnderTamersState)` with `prov, of_player, selecting_player, previous_phase, remaining: u8, filter: CompiledPredicate, filter_bindings: Bindings, prompt: String, inner_tail: Arc<Vec<CompiledStep>>, bindings, runtime, trigger_context, outer_conts`. Add to `wrap_pending_selection_with_tail` frame match.
- **run_resume (new arm)**: see pseudocode in spec. Decode FieldPermanent handle â†’ `trash_bottom_face_down_source` (MUTATES + fires + drains) â†’ `debug_assert!(trashed)` â†’ if `!trashed` early-return â†’ `now = remaining-1`. **If `pending_selection.is_some()` after the trash** (nested observer parked): if `now==0` defer tail via `wrap_pending_selection_with_tail`; else re-park the next pick via `after_selection_resume_hooks` (push `install_trash_under_tamers_resume_step(state{remaining:now})`). Else (no nested park): `if now==0` run terminal tail else re-install next pick.
- **installer edit**: `install_trash_under_tamers_resume_step` builds the OwnField PendingSelection (`is_optional=false`) with a vestigial `Box::new(|_,_|{})` callback + parks the frame. Re-derive candidates each pick from the `CompiledPredicate` (data-pure).
- **clobber guard**: TWO concerns. (1) install-time empty-candidate is pre-gated by the dispatch arm (:1826), so the first install always parks. (2) NESTED-SELECTION clobber (the real one): before re-parking the next pick OR running the terminal, check `if pending_selection.is_some()` and DEFER â€” NEVER overwrite the live selection. This also fixes the latent closure-world clobber bug.
- **decline**: not optional at the frame level (`is_optional=false`); no PASS arm. Cost-gating is structural â€” the tail runs only when `remaining` hits 0 via `count` successful trashes.
- **gate tests**: `tests/cards_behavioral/bt25/bt25_035.rs` (distribution: 2-from-one vs 1-from-each), `st24/st24_06.rs`, `st24/st24_10.rs`, archetype combos. **NEW test (single most important new gate)**: a Tamer-host `OnDigivolutionCardTrashed` observer that parks; assert the next pick is deferred via `after_selection_resume_hooks` and NOT clobbered.

### #7 combat.rs:93/102/124 â€” may_attack_now_* / force_opponent_attack â€” HARD
- **resume.rs**: reuse `AttackTarget` from #2, OR (leaning toward) a dedicated `ResumeFrame::AttackOpenStep` state struct carrying `attacker, optional, without_suspending, ignore_summoning_sickness, cost_upgrade: Option<AttackCostUpgrade>` so `begin_attack_open` runs from data in the terminal. `force_opponent_attack` sets `override_pin=attacker.player`.
- **run_resume**: decode target â†’ `begin_attack_open(AttackOpen{...})`. begin_attack_open installs a nested interrupt selection â†’ **DEFER inner_tail onto `after_selection_resume_hooks`**, never run inline.
- **decline**: optional sites â†’ `ResumeDecline::None`; force_opponent_attack mandatory â†’ no decline.
- **PREREQ** (open question): confirm begin_attack_open installs its interrupt selection synchronously vs via the queue â€” determines whether the channel suffices or a dedicated attack-resume bridge is needed.
- **gate tests**: `tests/cards_behavioral/bt23/bt23_013.rs` and combat behavioral suite.

### #8 link_cards.rs (248/394/396/459/492/613/766) â€” HARD, largest
- **resume.rs**: introduce `ResumeFrame::LinkPickState` carrying `spec: Arc<LinkCardsSpec>` (pure), `pick_index: u8`, accumulated `Vec<CardHandle>` for `bind_as`, the captured step `inner_tail`, and the zone-choiceâ†’card-selectâ†’host-select sub-flow state. Reuse `SourceMultiState`-shaped data for the source-pick (:459). Add to `wrap_pending_selection_with_tail` frame match.
- **run_resume**: each `install_pick` re-park is a Hand/Trash/Security/FieldPermanent decode or SourceMultiStep; on_pick (`after_card_chosen â†’ link_chosen_card_into_host`) runs BEFORE re-parking the next pick. `link_chosen_card_into_host` (OnLink + maybe_drain) and `absorb_standing_digimon_as_link` (3 trigger families + 3 drains) are mid-callback nested installs â†’ route through `after_selection_resume_hooks`.
- **decline**: card-select (UpTo count) â†’ `ResumeDecline::RunTail{tail=captured_tail, aborts_clause:false}` (the :492 manual on_decline is exactly this); host selects (:613/:766) mandatory â†’ `None`; select_own_sources min=0 â†’ SourceMultiStep terminal.
- **clobber guards**: all sites already pre-check (any_host L599-611/L757-761, has_match, eligible-zone filter) and run the captured tail inline when empty â€” only park when THIS install left a pending_selection.
- **data purity**: replace all parked closures (candidate_at/on_pick/filter_fn/matches_source/host_filter) with re-derivation from the `CompiledPredicate` + `Bindings` (Clone). NO non-Clone capture.
- **gate tests**: `tests/cards_behavioral/bt25/bt25_052.rs`, `bt25_056.rs`, `ad1/ad1_005.rs`; add resume.rs LinkPickState tests incl. an OnLink-parks-mid-callback nested test.

---

## 3. CASES THAT NEED THE after_selection_resume_hooks CONTINUATION CHANNEL (the risky ones)

These have **nested-selection-installing post-actions** â€” a post-action that itself drains a trigger queue or starts a sub-machine that parks a new `pending_selection`. The naive "park a RunTail and run the tail inline" pattern would CLOBBER the nested selection. Two distinct mechanisms apply:

**Mechanism A â€” `drain_or_rewrap_pending_tail` / `wrap_pending_selection_with_tail` (compose the tail as data onto the nested frame):** used when there is a SINGLE tail to defer after one parking post-action.
- **#5 install_use_option_from_hand**: `use_option_from_hand_without_paying_cost` can park (dual-mode mode-select, OptionMain body drain, arts-digivolve, Link host-select). The ACCEPT arm calls `drain_or_rewrap_pending_tail(tail)`, which â€” when `pending_selection_resume.is_some()` â€” pushes the tail as an `OuterContinuation` onto the option's nested frame (mod.rs:248-279). **Highest-risk detail: outer_conts ordering.** When the option play parks, `drain_or_rewrap` DEFERS the tail; calling `run_outer_conts` synchronously afterward would invert order (outer clause's cont running before this frame's still-parked tail). RESOLUTION: thread `outer_conts` through the same deferral (push them onto the same nested frame, or fold into the deferred tail Arc) â€” only call `run_outer_conts` inline when `pending_selection.is_none()` after the drain. Needs a dedicated nested-park test.

**Mechanism B â€” `after_selection_resume_hooks` (defer the NEXT pick / a mid-callback continuation onto the post-resolution hook channel):** used when a MUTATION-PER-PICK loop must continue after a parking observer, mid-callback (not a following step).
- **#6 install_trash_n_bottom_face_down_sources_under_tamers**: each pick's `trash_bottom_face_down_source â†’ fire_digivolution_card_trashed â†’ synchronous drain_effect_queue` can install a nested selection (P-169 'Close' shape â€” a 3-deep chain). On a per-pick park: push `install_trash_under_tamers_resume_step(state{remaining-1})` onto `game.after_selection_resume_hooks.0`. The channel drains at effect_queue.rs:3613-3616 after `run_resume` returns; a hook re-installing a resume-driven selection re-arms for the next resolution. **OPEN ISSUE**: a 3-deep nested chain means "after the next single resolution" is wrong â€” the re-arm likely needs `run_after_selections_drain` semantics (fire only after the WHOLE nested chain drains). Pin down with a Tamer-host observer test.
- **#7 may_attack_now_* / force_opponent_attack**: `begin_attack_open` starts the attack state machine and installs interrupt selections (counter/block/alliance). DEFER `inner_tail` onto `after_selection_resume_hooks`, never inline.
- **#8 link_cards**: `link_chosen_card_into_host` (OnLink + maybe_drain) and `absorb_standing_digimon_as_link` (OnDigivolutionCardTrashed + OnLinkedCardTrashed + OnLink, 3 drains) are MID-CALLBACK nested installs â€” exactly what Mechanism B is for, since the link attach is a post-action inside the pick callback, not a following step.

**Notably NOT needing a bespoke channel: #3 (trash_bottom under tamer, single pick).** Its trash also drains and can park, BUT because the FieldPermanent arm runs the cost-gated `inner_tail` via `run_tail_preserving_trigger_context` AFTER the trash, the tail's first step sees `pending_selection.is_some()` and re-parks the remainder via `park_pending_selection_tail` (step/mod.rs:536-537) â€” identical to the closure path. This is the same inline posture as the existing `Reveal{route:Some}` arm. Still REQUIRES a nested-park test, but no new channel code.

**Open question to resolve before #1:** whether `add_top_security_to_hand` fires a draining OnSecurity trigger. If it does, #1 moves from Batch A (atomic) into the channel set and is no longer the cheapest flip.

> **RESOLVED 2026-06-23 (verified in source):** `add_top_security_to_hand` → `add_to_hand_from_security` → `fire_security_removed_observers` → `fire_effect_security_removal` (effect_queue.rs:3265) which `enqueue_triggered` + **`drain_effect_queue()` (line 3288)** and then checks `pending_selection.is_some()` (3300). So it is **NOT atomic** — a security-removal observer can park a nested selection. **#1 therefore moves OUT of Batch A into the channel set** (Mechanism A: the post-action can park, so the tail/outer_conts must compose onto the nested selection via `drain_or_rewrap`/`wrap_pending_selection_with_tail`, not run inline). Consequence: **there is no longer a truly-atomic cheapest flip** — `combat.rs:152 select_redirect_attack_target` (#2) is the only remaining atomic one but it requires a brand-new `AttackTarget` `ResumeSelectKind` + combat dispatcher arm (new machinery). So EVERY remaining installer needs either new-variant machinery or the continuation channel + a nested-park test. This is the plan's "harder half"; approach each deliberately, one flip + one full-suite gate at a time.

> **SECOND RESOLUTION 2026-06-23 — `combat.rs:152` is ALSO NOT atomic.** `select_redirect_attack_target`'s callback calls `apply_attack_target_substitution_with_reason` → `fire_attack_target_change_observers` (combat/mod.rs:1099) which `enqueue_triggered(OnAttackTargetChange)` + **`drain_effect_queue()` (line 1125)** → an `OnAttackTargetChange` observer can park a nested selection. So #2 is channel-needing too. **NET: there are ZERO truly-atomic flips left** — every remaining installer (#1 security-to-hand, #2 redirect-target, #3-#8) needs the continuation channel (`drain_or_rewrap`/`after_selection_resume_hooks`) + a nested-park test, OR a new multi-pick/state-machine frame. The workflow's "Batch A atomic" labeling was over-optimistic (it didn't trace the observer-firing); ALWAYS verify a post-action's atomicity in source (grep the action → `fire_*` → `drain_effect_queue`) before assuming a clean RunTail flip. Implication: the recommended order's "land the cheap atomic ones first to de-risk the plumbing" no longer applies — pick the flip with the SMALLEST new-machinery surface (likely #3 `trash_bottom_face_down` — extends the existing `FieldPermanent` kind, and the plan notes its nested-park threads automatically via `park_pending_selection_tail`, no bespoke channel) as the first to attempt, with a nested-park test.

---

## 4. WHAT REMAINS AFTER THESE DSL INSTALLERS (the non-DSL surface â€” a separate, larger effort)

Flipping these installers completes the **DSL card-scripting** selection surface. It does NOT make `Game: Clone` fully sound, because a large body of selections live OUTSIDE the DSL lowering path and still install bespoke closure callbacks (which clone to the panic-stub `PendingSelection::clone`). These are a separate, larger design effort because each owns its own state machine with no shared `ResumeFrame`/`ResumeSelectKind` vocabulary, and several are engine-core (combat, costs) rather than card-scripting:

1. **The raw `ctx.select_*` primitive API surface** (effect_context/selections.rs): `select_redirect_attack_target`, `may_attack_now_*`, `force_opponent_attack_with_upgrade`, `select_effect_choice` (raw), `select_own_sources`, `search_own_security_stack`, and the bespoke Security park â€” these are partly addressed by Batches A/D above where DSL steps call them, but the primitives themselves and any non-DSL caller remain closure-based until each gets a resume kind. The combat ones in particular (`SelectionKind::Target`) have NO resume arm at all â€” they need a brand-new `AttackTarget` decode family that #2/#7 only begin.

2. **Combat / keyword interrupt selections** â€” the attack state machine (`begin_attack_open`) installs counter/block/alliance interrupt selections through its own (non-DSL) selection path. These are sub-machines, not bind+tail; flipping them requires modeling the attack interrupt window as resumable state, a design task on its own (and the reason #7 is HARD â€” it can defer ONTO the window but does not flip the window itself).

3. **Digivolve cost-choice** (the `EffectChoice` prompt when a digivolve has >1 distinct cost, rule 17) â€” installed by engine-core digivolution, not the DSL step runner.

4. **Play-order selection** (`SelectPlayOrder`, BO3 between-games) â€” engine-driven via `Game::request_play_order_selection`, outside the DSL.

5. **Overclock, TriggerOrder, replacement-effect accept, Delay** â€” each is a distinct engine-level interactive point (keyword timing windows, ordering simultaneous triggers, "you may" replacement acceptance, delayed-effect resolution) with its own bespoke `pending_selection`. None flows through the DSL lowering, so none can reuse `ResumeFrame::RunTail` as-is; each needs either a new `ResumeSelectKind`/`ResumeFrame` variant or a parallel resumable model.

**Why separate:** the DSL installers in this plan share ONE substrate (`run_resume` + the `ResumeFrame`/`ResumeSelectKind` vocabulary + `wrap_pending_selection_with_tail` + `after_selection_resume_hooks`), so they batch and gate together against the same resume.rs unit-test harness and the `cards_behavioral` suite. The non-DSL surface above is heterogeneous engine-core machinery â€” combat sub-machines, BO3 match flow, keyword timing windows â€” each requiring its own resumable-state design, its own decode, and its own clone-safety proof. Until ALL of them are flipped, `Game: Clone` will still hit the panic-stub for those paths, so MCTS/AlphaZero search remains gated on a follow-on engine-core effort (the harder half of make-engine-cloneable) rather than this DSL-installer batch.

---

## Cross-cutting gate (every batch)
Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral` with `RUST_MIN_STACK=268435456` (per the cards_behavioral flaky-crash memory) and an isolated `CARGO_TARGET_DIR='D:\cargo-target-wt\elated-hopper-9f42c2'` (per the per-worktree-target memory) to avoid phantom compile errors. Every flipped installer MUST add: (a) a resume.rs `game_clone_is_independent_and_replays_identically` analog, and (b) for the channel cases, a nested-park test proving the tail/next-pick defers rather than clobbering the live `pending_selection`.