use crate::card_source::CardHandle;
use crate::enums::{CardKind, PlayerId, Zone};
use crate::permanent::PermanentHandle;
use crate::selection::AttackTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackTargetChangeReason {
    Raid,
    Collision,
    Blocker,
    EffectRedirect(Option<CardHandle>),
    EffectForced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackTargetChange {
    pub attacker: PermanentHandle,
    pub old_target: AttackTarget,
    pub new_target: AttackTarget,
    pub reason: AttackTargetChangeReason,
    pub controller: PlayerId,
}

/// Stable token for effect-created objects or effect-initiated transitions.
/// Downstream cleanup/suppression should key off this token rather than a
/// battle-area index that can shift as permanents leave play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProvenanceToken(pub u64);

impl From<CardHandle> for ProvenanceToken {
    fn from(value: CardHandle) -> Self {
        Self(value.0 as u64)
    }
}

/// Broad cause categories carried by observer payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventCause {
    BattleDeletion,
    EffectDeletion,
    OwnEffect,
    OpponentEffect,
    Overclock,
    Return,
    DeckBottom,
    SecurityPlacement,
    SecurityRemoval,
    Cost,
    Rule,
}

impl From<crate::replacement::ReplacementCause> for EventCause {
    fn from(value: crate::replacement::ReplacementCause) -> Self {
        match value {
            crate::replacement::ReplacementCause::Battle => EventCause::BattleDeletion,
            crate::replacement::ReplacementCause::OwnEffect => EventCause::OwnEffect,
            crate::replacement::ReplacementCause::OpponentEffect => EventCause::OpponentEffect,
            crate::replacement::ReplacementCause::SecurityCheck => EventCause::SecurityRemoval,
            crate::replacement::ReplacementCause::Cost => EventCause::Cost,
            crate::replacement::ReplacementCause::Overclock => EventCause::Overclock,
            // DigiXros material consumption is the controller's own play action.
            crate::replacement::ReplacementCause::DigiXros => EventCause::OwnEffect,
        }
    }
}

/// The object that caused the event. This is intentionally distinct from the
/// observer card/permanent carrying the triggered effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSubject {
    Permanent(PermanentHandle),
    Card { card: CardHandle, zone: Zone },
    Player(PlayerId),
}

/// Effect/source attribution for an event. `source_permanent` is optional
/// because hand, trash, security, and rule effects may not have a live
/// permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectAttribution {
    pub controller: PlayerId,
    pub source_card: Option<CardHandle>,
    pub source_permanent: Option<PermanentHandle>,
}

/// A selected result made by an earlier pending-selection step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultBinding {
    pub name: &'static str,
    pub permanent: Option<PermanentHandle>,
    pub card: Option<CardHandle>,
}

/// A compact moved-card batch descriptor for observer predicates that care
/// which cards moved together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedCardSet {
    pub cards: Vec<CardHandle>,
    pub from: Option<Zone>,
    pub to: Option<Zone>,
}

/// Snapshot captured before a permanent leaves the board. Observers must use
/// this for deletion predicates after the live battle-area slot is gone.
///
/// The `*_just_before` fields mirror DCGO's `PermanentJustBeforeRemoveField` /
/// `DPJustBeforeRemoveField` / `LevelJustBeforeRemoveField` / etc. captured at
/// step 7 of `DestroyPermanentsClass.Destroy()`. They let OnDeletion handlers
/// answer "what was this Digimon's DP / level / cost / names / traits / source
/// list immediately before deletion?" after the permanent has been trashed.
#[derive(Clone, PartialEq, Eq)]
pub struct DeletedObjectSnapshot {
    pub former_controller: PlayerId,
    pub top_card: CardHandle,
    /// Last-known effective field kind; card-only queries use top_card metadata.
    pub card_kind: CardKind,
    pub retained_kind: Option<CardKind>,
    pub traits: Vec<String>,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub cause: EventCause,
    /// Pre-removal effective DP (modifier-aware). `None` when the permanent
    /// did not have a DP value (e.g. Tamer).
    pub dp_just_before: Option<i32>,
    /// Pre-removal level. `None` for non-Digimon kinds.
    pub level_just_before: Option<u8>,
    /// Pre-removal printed play cost of the top card. `None` when not
    /// applicable. Matches `CardData::play_cost` (u16).
    pub cost_just_before: Option<u16>,
    /// Pre-removal top-card card-names list.
    pub names_just_before: Vec<String>,
    /// Pre-removal top-card traits list. Duplicates `traits` for legacy
    /// callers; new code should prefer this field for clarity.
    pub traits_just_before: Vec<String>,
    /// Pre-removal source count BELOW the top (i.e. `card_sources.len() - 1`).
    /// 0 when the carrier had only its top card.
    pub source_count_just_before: usize,
    /// Pre-removal digivolution-card handles in stack order (bottom-most
    /// first), excluding the top card.
    pub digisources_just_before: Vec<CardHandle>,
    /// Whether the deleted carrier's top card was a token. DCGO
    /// `CardEffectCommons.CanActivateOnDeletion` returns `true` immediately
    /// for tokens (`if (card.IsToken) return true;`), so the OnDeletion bundle
    /// fires even though a trashed token leaves no card in trash.
    pub is_token: bool,
}

/// Immutable field identity at battle comparison time. Unlike a permanent
/// handle, this remains meaningful after mutual deletion and slot compaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleCombatantSnapshot {
    /// Stable identity of the compared Digimon; ordinary evolution retains it.
    pub battle_identity: u64,
    /// Captured after all battle-deletion replacements finish, before fanout.
    pub survived: bool,
    pub controller: PlayerId,
    pub top_card: CardHandle,
    pub data_index: usize,
    pub identity: crate::permanent::SynthIdentity,
    pub is_suspended: bool,
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct TriggerContext {
    /// This entry was captured at an actual security check, before Security
    /// processing. Leaving its source area permanently cancels activation.
    pub captured_security_check: bool,
    pub security_check_source_invalidated: bool,
    pub security_check_face_generation: Option<u64>,
    /// Physical members of one simultaneous play event. Each member has no nested group.
    pub simultaneous_play_members: Vec<TriggerContext>,
    /// Transient stamp: this entry was enqueued by the source-move event that
    /// is still open (`Game::source_move_event_open`). Cleared when the event
    /// closes and its entries are coalesced; never set between steps.
    pub(crate) in_source_move_event: bool,
    /// Captured physical eligible subjects for one paid deletion-prevention effect.
    pub replacement_deletion_members: Vec<(PlayerId, u16, u64)>,
    /// Immutable effect lookup retained while its physical payer leaves.
    pub(crate) replacement_paid_prevention: Option<(CardHandle, String, u8, Option<crate::enums::Keyword>)>,
    pub(crate) replacement_deletion_cause: Option<(crate::replacement::ReplacementCause, Option<PlayerId>)>,
    pub subject: Option<EventSubject>,
    pub target_permanent: Option<PermanentHandle>,
    pub target_card: Option<CardHandle>,
    pub event_permanent: Option<PermanentHandle>,
    /// Identity of the event Digimon, retained across ordinary evolution and
    /// compaction; DNA and leaving/replaying create a different permanent.
    pub event_permanent_identity: Option<(PlayerId, u64)>,
    pub event_card: Option<CardHandle>,
    pub event_source_card: Option<CardHandle>,
    pub event_host_card: Option<CardHandle>,
    pub event_host_permanent: Option<PermanentHandle>,
    pub affected_player: Option<PlayerId>,
    pub attack_target_change: Option<AttackTargetChange>,
    pub source_player: Option<PlayerId>,
    pub cause: Option<EventCause>,
    pub source_effect: Option<EffectAttribution>,
    pub selected_results: Vec<ResultBinding>,
    pub moved_card_sets: Vec<MovedCardSet>,
    pub effect_initiated: bool,
    pub dna_origin: bool,
    pub deleted_object: Option<DeletedObjectSnapshot>,
    pub old_attack_target: Option<crate::selection::AttackTarget>,
    pub new_attack_target: Option<crate::selection::AttackTarget>,
    pub provenance_token: Option<ProvenanceToken>,
    pub was_security_skill: bool,
    pub option_last_field_state: Option<crate::option_lifecycle::OptionFieldState>,
    /// The surviving permanent that won a battle by a strict comparison,
    /// including against a Security Digimon. Opponent deletion is not required.
    /// Set on the `EndOfBattle`
    /// firing so a BOARD-WIDE observer can react to "any of your [X] Digimon
    /// win a battle" (BT25-020 Marsmon; G-DSL-BATTLE-WINNER-BOARDWIDE). `None`
    /// on a tie — a tie has no winner — or when the winning Security Digimon
    /// has no battle-area permanent. A check without a security battle does
    /// not fire `EndOfBattle`. Distinct from the
    /// carrier-scoped `source_deleted_battle_opponent` idiom, which fires on
    /// the OnAnyDeletion path when the observer *is* the winner. Mirrors DCGO
    /// `CardEffectCommons.CanTriggerWhenWinBattle` reading the battle
    /// hashtable's `WinnerPermanents` while `WasTie == false`.
    pub battle_winner: Option<PermanentHandle>,
    /// Combatants whose opposing combatant was actually deleted by this
    /// battle. Both entries can exist on a tie. Observers themselves still
    /// require a live eligible source; these snapshots never grant liveness.
    pub battle_deleters: Vec<BattleCombatantSnapshot>,
    /// TOP CARD of the Digimon the deleted carrier was battling, captured when
    /// its `[On Deletion]` entries are ENQUEUED on a `Battle`-cause deletion
    /// (`Game::enqueue_batch_on_deletion`, while `pending_attack` is still
    /// live). general_rule.pdf 16-13: <Retaliation> "delete[s] the Digimon it
    /// battled" -- who that is, is a fact of the TRIGGER. Reading it from the
    /// live `pending_attack` at RESOLUTION time is wrong whenever resolution is
    /// deferred: a second [On Deletion] trigger on the same stack opens a
    /// TriggerOrder prompt (15-4), the battle unwinds while the prompt is
    /// parked, and the keyword then found no opponent and deleted nothing.
    /// A card identity rather than a `PermanentHandle` because handles are
    /// battle-area POSITIONS and shift when the loser's slot is compacted.
    /// `None` for every non-battle deletion and for direct player attacks.
    pub battle_opponent_card: Option<CardHandle>,
    /// The player whose HAND lost one or more cards to trash during an
    /// effect-caused discard batch — the "your hand is trashed from" subject.
    /// Set on the `OnDiscardHand` firing (G-ENGINE-ON-DISCARD-HAND). Mirrors
    /// DCGO's `DiscardedCards` list owner in `CanTriggerOnTrashHand`.
    pub discard_hand_player: Option<PlayerId>,
    /// The controller of the EFFECT that caused the hand discard. Only set on
    /// `OnDiscardHand`. Lets a card gate "when one of YOUR effects trashes a
    /// card in your hand" (ST16-14 Matt Ishida) vs the opponent's effect.
    /// Mirrors DCGO `CanTriggerOnTrashHand`'s `cardEffect.EffectSourceCard.Owner`.
    pub discard_cause_controller: Option<PlayerId>,
    /// The EFFECT that caused this event, for event timings that only exist
    /// because an effect did something (today: `OnAddDigivolutionCards`).
    /// Distinct from `source_effect`, which is auto-filled from whatever
    /// effect happens to be executing when the trigger is enqueued: this one
    /// is captured explicitly at the mutation site by the placing facade and
    /// survives the deferred batch flush. Mirrors the `CardEffect` DCGO puts
    /// in the `OnAddDigivolutionCards` hashtable (Permanent.cs:1124 / 1228),
    /// read by `CanTriggerOnAddDigivolutionCard`'s `cardEffectCondition`
    /// (BT7-056 "one of YOUR effects": `EffectSourceCard.Owner == card.Owner`).
    /// `event_caused_by_own_effect` reads it. G-ENGINE-ON-ADD-DIGIVOLUTION-CARDS.
    pub event_cause_effect: Option<EffectAttribution>,
    /// Last-known information of the event Digimon (`event_permanent`), taken
    /// when the effect triggered (`Game::trigger_context_for_source`), in the
    /// `deleted_object` shape. Read only by a clause-level gate whose event
    /// Digimon has since LEFT the field (`Game::gate_trigger_context`): the
    /// event happened and the references the trigger made keep their
    /// last-known values. Shared (`Arc`) by every context of one event.
    /// Derived from the trigger-time board (whose identity is already in
    /// `event_permanent_identity`), so it is left out of `Debug` and the
    /// verification digest built from it.
    pub(crate) event_snapshot: Option<std::sync::Arc<DeletedObjectSnapshot>>,
    /// Identity of the battle WINNER (`battle_winner`), recorded when the
    /// battle was compared (`PendingBattleResolution::winner_identity`) and
    /// carried on every `EndOfBattle` firing even when the winner has left
    /// the field before the event was emitted. The `event_winner_*` reads
    /// resolve the winner by it (never by the slot `battle_winner` named at
    /// enqueue, which a lower-index departure shifts) and, once it no longer
    /// resolves, read the winner's last-known information: the battle's
    /// `event_snapshot` (the winner is the event Digimon), exposed as
    /// `deleted_object` by `Game::gate_trigger_context` (§15-8-3-1,
    /// §15-8-3-8, §15-4-4-5; G-ENGINE-BATTLE-WINNER-QUALIFIER-STALE-INDEX).
    /// Equal to `event_permanent_identity` on these firings, so it is left
    /// out of `Debug` and the verification digest built from it.
    pub(crate) battle_winner_identity: Option<(PlayerId, u64)>,
}

impl TriggerContext {
    /// Parked observer payloads must not retain unentitled concealed identities.
    pub(crate) fn belief_safe(&self, allowed: &std::collections::HashSet<CardHandle>) -> bool {
        let snapshot_safe = |s: &DeletedObjectSnapshot| {
            (s.is_token || allowed.contains(&s.top_card))
                && s.digisources_just_before
                    .iter()
                    .all(|h| allowed.contains(h))
        };
        self.simultaneous_play_members
            .iter()
            .all(|s| s.belief_safe(allowed))
            && [
                self.target_card,
                self.event_card,
                self.event_source_card,
                self.event_host_card,
                self.battle_opponent_card,
                self.source_effect.and_then(|s| s.source_card),
                self.event_cause_effect.and_then(|s| s.source_card),
            ]
            .into_iter()
            .flatten()
            .all(|h| allowed.contains(&h))
            && self.subject.is_none_or(|s| match s {
                EventSubject::Card { card, .. } => allowed.contains(&card),
                _ => true,
            })
            && self
                .selected_results
                .iter()
                .filter_map(|s| s.card)
                .all(|h| allowed.contains(&h))
            && self
                .moved_card_sets
                .iter()
                .flat_map(|s| &s.cards)
                .all(|h| allowed.contains(h))
            && self.deleted_object.as_ref().is_none_or(snapshot_safe)
            && self
                .event_snapshot
                .as_ref()
                .is_none_or(|s| snapshot_safe(s))
            && self
                .battle_deleters
                .iter()
                .all(|s| allowed.contains(&s.top_card))
    }

    pub(crate) fn belief_missing_handles(
        &self,
        allowed: &std::collections::HashSet<CardHandle>,
    ) -> Vec<CardHandle> {
        let mut handles: Vec<_> = [
            self.target_card,
            self.event_card,
            self.event_source_card,
            self.event_host_card,
            self.battle_opponent_card,
            self.source_effect.and_then(|s| s.source_card),
            self.event_cause_effect.and_then(|s| s.source_card),
        ]
        .into_iter()
        .flatten()
        .collect();
        if let Some(EventSubject::Card { card, .. }) = self.subject {
            handles.push(card);
        }
        handles.extend(self.selected_results.iter().filter_map(|s| s.card));
        handles.extend(
            self.moved_card_sets
                .iter()
                .flat_map(|s| s.cards.iter().copied()),
        );
        for s in self
            .deleted_object
            .iter()
            .chain(self.event_snapshot.iter().map(|s| s.as_ref()))
        {
            if !s.is_token {
                handles.push(s.top_card);
            }
            handles.extend(s.digisources_just_before.iter().copied());
        }
        handles.extend(self.battle_deleters.iter().map(|s| s.top_card));
        handles.extend(
            self.simultaneous_play_members
                .iter()
                .flat_map(|s| s.belief_missing_handles(allowed)),
        );
        handles.retain(|h| !allowed.contains(h));
        handles.sort_by_key(|h| h.0);
        handles.dedup();
        handles
    }

    pub(crate) fn belief_snapshots_safe(
        &self,
        identities: &std::collections::HashSet<CardHandle>,
    ) -> bool {
        self.simultaneous_play_members
            .iter()
            .all(|c| c.belief_snapshots_safe(identities))
            && self
                .deleted_object
                .iter()
                .chain(self.event_snapshot.iter().map(|s| s.as_ref()))
                .all(|s|
                // Only the top has cached printed metadata. Source handles
                // are references, audited separately and sampled in the world.
                s.is_token || identities.contains(&s.top_card))
            && self
                .battle_deleters
                .iter()
                .all(|s| identities.contains(&s.top_card))
            && self
                .replacement_paid_prevention
                .as_ref()
                .is_none_or(|(h, _, _, _)| identities.contains(h))
    }

    /// Resolve the captured event object, never a later occupant of its old slot.
    /// Legacy contexts without a captured token retain their compatibility handle.
    pub fn resolve_event_permanent(&self, game: &crate::game::Game) -> Option<PermanentHandle> {
        match self.event_permanent_identity {
            Some(identity) => resolve_battle_identity(game, identity),
            None => self.event_permanent,
        }
    }

    /// The battle winner of an `EndOfBattle` firing while it is on the field:
    /// found by identity, so a lower-index departure that shifted its slot
    /// does not redirect the read to the Digimon that slid in. `None` once it
    /// has left the field (read `deleted_object` -- its last-known
    /// information -- then), on a tie, or on any other event. Legacy contexts
    /// without a captured identity keep the compatibility handle.
    pub fn resolve_battle_winner(&self, game: &crate::game::Game) -> Option<PermanentHandle> {
        match self.battle_winner_identity {
            Some(identity) => resolve_battle_identity(game, identity),
            None => self.battle_winner,
        }
    }

    /// Whether the battle winner carries `trait_name` (case-insensitive): read
    /// live by identity while it is on the field (§15-4-4-5), and from its
    /// last-known information once it has left -- the snapshot a clause-level
    /// gate exposes as `deleted_object` (`Game::gate_trigger_context`;
    /// §15-8-3-1 / §15-8-3-8). The one reader behind the DSL
    /// `event_winner_trait_has` and `EffectContext::battle_winner_has_trait`.
    /// G-ENGINE-BATTLE-WINNER-QUALIFIER-STALE-INDEX.
    ///
    /// Both branches read the winner's TOP card (with trait-changing effects),
    /// never its digivolution cards: the live read is
    /// `Permanent::has_trait_for_rules` (the DSL `trait_has` reading) and the
    /// snapshot's `traits` are its synthesized identity's
    /// (`Game::build_snapshot_for_handle`). §4-7-7; official Q&A Q3225; DCGO
    /// `BT25_020.cs` `permanent.TopCard.HasTSTraits`.
    /// G-ENGINE-BATTLE-WINNER-TRAIT-READS-DIGIVOLUTION-CARDS.
    pub fn battle_winner_has_trait(&self, game: &crate::game::Game, trait_name: &str) -> bool {
        match self.resolve_battle_winner(game) {
            Some(winner) => game
                .player(winner.player)
                .battle_area
                .get(winner.index as usize)
                .is_some_and(|perm| {
                    perm.has_trait_for_rules(trait_name, &game.card_data, &game.modifiers, winner)
                }),
            None => {
                self.battle_winner_identity.is_some()
                    && self.deleted_object.as_ref().is_some_and(|snapshot| {
                        snapshot
                            .traits
                            .iter()
                            .any(|t| t.eq_ignore_ascii_case(trait_name))
                    })
            }
        }
    }

    /// The controller of the battle winner (whether or not it is still on the
    /// field), or `None` when the event has no winner.
    pub fn battle_winner_controller(&self) -> Option<PlayerId> {
        self.battle_winner_identity
            .map(|(player, _)| player)
            .or_else(|| self.battle_winner.map(|winner| winner.player))
    }

    /// The cards an effect just trashed from the event player's hand
    /// (`OnDiscardHand` board-wide observer fan-out only; the trashed card's
    /// own `scope: trash` firing names itself via `event_card`). Empty on
    /// every other timing.
    /// G-ENGINE-DISCARDED-HAND-CARDS.
    pub fn discarded_hand_cards(&self) -> &[CardHandle] {
        if self.discard_hand_player.is_none() {
            return &[];
        }
        self.moved_card_sets
            .iter()
            .find(|set| set.from == Some(Zone::Hand) && set.to == Some(Zone::Trash))
            .map(|set| set.cards.as_slice())
            .unwrap_or(&[])
    }

    /// The cards an effect just placed into the event host's digivolution
    /// cards (`OnAddDigivolutionCards` only). Empty on every other timing.
    pub fn added_source_cards(&self) -> &[CardHandle] {
        if self.event_cause_effect.is_none() {
            return &[];
        }
        self.moved_card_sets
            .iter()
            .find(|set| set.from.is_none() && set.to == Some(Zone::BattleArea))
            .map(|set| set.cards.as_slice())
            .unwrap_or(&[])
    }
}

/// The battle-area permanent carrying `identity` (`Permanent::battle_identity`),
/// or `None` once it has left the field (or become a new Digimon by DNA).
fn resolve_battle_identity(
    game: &crate::game::Game,
    (player, identity): (PlayerId, u64),
) -> Option<PermanentHandle> {
    game.player(player)
        .battle_area
        .iter()
        .position(|permanent| permanent.battle_identity == Some(identity))
        .map(|index| PermanentHandle { player, index: index as u8 })
}

impl From<crate::option_lifecycle::OptionTrashCause> for EventCause {
    fn from(value: crate::option_lifecycle::OptionTrashCause) -> Self {
        match value {
            crate::option_lifecycle::OptionTrashCause::Effect => EventCause::EffectDeletion,
            crate::option_lifecycle::OptionTrashCause::LeaveField => EventCause::Rule,
            crate::option_lifecycle::OptionTrashCause::EndOfTurnDelayExpiry => EventCause::Cost,
            crate::option_lifecycle::OptionTrashCause::PlugInCarrierLoss => EventCause::Rule,
            crate::option_lifecycle::OptionTrashCause::SecurityActivation => {
                EventCause::SecurityRemoval
            }
            crate::option_lifecycle::OptionTrashCause::Resolution => EventCause::Cost,
        }
    }
}

impl std::fmt::Debug for DeletedObjectSnapshot {
 fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
 let mut d=f.debug_struct("DeletedObjectSnapshot");
 d.field("former_controller", &self.former_controller);
 d.field("top_card", &self.top_card);
 d.field("card_kind", &self.card_kind);
 d.field("traits", &self.traits);
 d.field("level", &self.level);
 d.field("dp", &self.dp);
 d.field("cause", &self.cause);
 d.field("dp_just_before", &self.dp_just_before);
 d.field("level_just_before", &self.level_just_before);
 d.field("cost_just_before", &self.cost_just_before);
 d.field("names_just_before", &self.names_just_before);
 d.field("traits_just_before", &self.traits_just_before);
 d.field("source_count_just_before", &self.source_count_just_before);
 d.field("digisources_just_before", &self.digisources_just_before);
 d.field("is_token", &self.is_token);
 if self.retained_kind.is_some() { d.field("retained_kind", &self.retained_kind); }
 d.finish()
 }
}

impl std::fmt::Debug for TriggerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("TriggerContext");
        d.field("subject", &self.subject);
        d.field("target_permanent", &self.target_permanent);
        d.field("target_card", &self.target_card);
        d.field("event_permanent", &self.event_permanent);
        d.field("event_permanent_identity", &self.event_permanent_identity);
        d.field("event_card", &self.event_card);
        d.field("event_source_card", &self.event_source_card);
        d.field("event_host_card", &self.event_host_card);
        d.field("event_host_permanent", &self.event_host_permanent);
        d.field("affected_player", &self.affected_player);
        d.field("attack_target_change", &self.attack_target_change);
        d.field("source_player", &self.source_player);
        d.field("cause", &self.cause);
        d.field("source_effect", &self.source_effect);
        d.field("selected_results", &self.selected_results);
        d.field("moved_card_sets", &self.moved_card_sets);
        d.field("effect_initiated", &self.effect_initiated);
        d.field("dna_origin", &self.dna_origin);
        d.field("deleted_object", &self.deleted_object);
        d.field("old_attack_target", &self.old_attack_target);
        d.field("new_attack_target", &self.new_attack_target);
        d.field("provenance_token", &self.provenance_token);
        d.field("was_security_skill", &self.was_security_skill);
        d.field("option_last_field_state", &self.option_last_field_state);
        d.field("battle_winner", &self.battle_winner);
        d.field("battle_deleters", &self.battle_deleters);
        d.field("battle_opponent_card", &self.battle_opponent_card);
        d.field("discard_hand_player", &self.discard_hand_player);
        d.field("discard_cause_controller", &self.discard_cause_controller);
        d.field("event_cause_effect", &self.event_cause_effect);
        if !self.replacement_deletion_members.is_empty() { d.field("replacement_deletion_members", &self.replacement_deletion_members); }
        if let Some(value)=&self.replacement_paid_prevention { d.field("replacement_paid_prevention", value); }
        if let Some(value)=&self.replacement_deletion_cause { d.field("replacement_deletion_cause", value); }
        if !self.simultaneous_play_members.is_empty() { d.field("simultaneous_play_members", &self.simultaneous_play_members); }
        if self.in_source_move_event { d.field("in_source_move_event", &self.in_source_move_event); }
        d.finish()
    }
}

#[cfg(test)] mod simultaneous_play_debug_tests {
 use super::*;
 #[test] fn ordinary_event_debug_retains_legacy_shape() {
  assert_eq!(format!("{:?}",TriggerContext::default()),"TriggerContext { subject: None, target_permanent: None, target_card: None, event_permanent: None, event_permanent_identity: None, event_card: None, event_source_card: None, event_host_card: None, event_host_permanent: None, affected_player: None, attack_target_change: None, source_player: None, cause: None, source_effect: None, selected_results: [], moved_card_sets: [], effect_initiated: false, dna_origin: false, deleted_object: None, old_attack_target: None, new_attack_target: None, provenance_token: None, was_security_skill: false, option_last_field_state: None, battle_winner: None, battle_deleters: [], battle_opponent_card: None, discard_hand_player: None, discard_cause_controller: None, event_cause_effect: None }");
 }
 #[test] fn grouped_event_debug_retains_all_physical_members() {
  let mut group=TriggerContext::default();
  group.simultaneous_play_members=vec![TriggerContext{event_card:Some(CardHandle(11)),..Default::default()},TriggerContext{event_card:Some(CardHandle(12)),..Default::default()}];
  let before=format!("{group:?}");assert_eq!(before,format!("{:?}",group.clone()));
  group.simultaneous_play_members[1].event_card=Some(CardHandle(13));assert_ne!(before,format!("{group:?}"));
 }
}
