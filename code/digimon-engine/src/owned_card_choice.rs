//! Complete physical-card choices. The pending prompt owns its page and pins.
use crate::{
    card_choice_page::{
        CardChoicePage, CardChoiceResult, CardChoiceToken, SourceChoiceArea, SourceChoicePin,
    },
    card_source::CardHandle,
    enums::PlayerId,
    game::Game,
    permanent::PermanentHandle,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardChoicePin {
    Source(SourceChoicePin),
    BattleTop { owner: PlayerId, carrier_identity: u64, card: CardHandle },
    Security { owner: PlayerId, card: CardHandle },
    Hand { owner: PlayerId, card: CardHandle },
    Trash { owner: PlayerId, card: CardHandle },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedCardChoice {
    Source {
        host: PermanentHandle,
        index: usize,
        card: CardHandle,
    },
    Security {
        owner: PlayerId,
        index: usize,
        card: CardHandle,
    },
    BattleTop { host: PermanentHandle, card: CardHandle },
    Hand { owner: PlayerId, index: usize, card: CardHandle },
    Trash { owner: PlayerId, index: usize, card: CardHandle },
}

impl ResolvedCardChoice {
    pub fn digixros_origin(self) -> Option<crate::digixros::DigiXrosMaterialOrigin> {
        use crate::digixros::DigiXrosMaterialOrigin as Origin;
        match self {
            Self::Source { host, index, card } => Some(Origin::UnderTamer { tamer: host, source_index: index, card }),
            Self::BattleTop { host, card } => Some(Origin::BattleArea { permanent: host, card }),
            Self::Hand { owner, index, card } => Some(Origin::Hand { player: owner, index, card }),
            Self::Trash { owner, index, card } => Some(Origin::Trash { player: owner, index, card }),
            Self::Security { .. } => None,
        }
    }

    pub fn source_ref(self) -> Option<crate::selection::SourceSelectionRef> {
        match self {
            Self::Source { host, index, card } => Some(crate::selection::SourceSelectionRef {
                permanent: host, field_index: host.index, source_index: index, card,
            }),
            _ => None,
        }
    }

    pub fn union_ref(self) -> Option<(PlayerId, CardHandle, crate::selection::UnionZoneOrigin)> {
        use crate::selection::UnionZoneOrigin;
        match self {
            Self::Source { host, index, card } => Some((host.player, card,
                UnionZoneOrigin::Material { carrier: host, source_index: index })),
            Self::Hand { owner, card, .. } => Some((owner, card, UnionZoneOrigin::Hand)),
            Self::Trash { owner, card, .. } => Some((owner, card, UnionZoneOrigin::Trash)),
            Self::Security { .. } | Self::BattleTop { .. } => None,
        }
    }

}

/// Read-only raw predicates are shareable; one-shot callbacks never live here.
#[derive(Clone)]
pub struct RawSecurityFilter(std::sync::Arc<dyn Fn(&Game, usize) -> bool + Send + Sync>);
impl std::fmt::Debug for RawSecurityFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("RawSecurityFilter") }
}
impl PartialEq for RawSecurityFilter {
    fn eq(&self, other: &Self) -> bool { std::sync::Arc::ptr_eq(&self.0, &other.0) }
}
impl Eq for RawSecurityFilter {}

#[derive(Clone)]
struct SourceCompletionFilter(std::sync::Arc<dyn Fn(&Game, Option<CardChoicePin>) -> bool + Send + Sync>);
impl std::fmt::Debug for SourceCompletionFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("SourceCompletionFilter") }
}
impl PartialEq for SourceCompletionFilter {
    fn eq(&self, other: &Self) -> bool { std::sync::Arc::ptr_eq(&self.0, &other.0) }
}
impl Eq for SourceCompletionFilter {}

#[derive(Debug, Clone)]
enum SecurityEligibilityData {
    Raw(RawSecurityFilter),
    Dsl {
        filter: digimon_dsl::compiled::CompiledPredicate,
        bindings: crate::dsl_cards::bindings::Bindings,
        prov: crate::resume::ResumeProvenance,
    },
}
#[derive(Debug, Clone)]
struct SecurityEligibility(std::sync::Arc<SecurityEligibilityData>);
impl PartialEq for SecurityEligibility {
    fn eq(&self, other: &Self) -> bool { std::sync::Arc::ptr_eq(&self.0, &other.0) }
}
impl Eq for SecurityEligibility {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequiredSourcePrefix {
    floor: usize,
    picked: Vec<SourceChoicePin>,
    allow_zero: bool,
    distinct: Option<crate::effect_context::selections::DistinctByMode>,
    named: Option<crate::resume::NamedSourceGroups>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardChoiceRecovery { Current, RefreshSecurity, Impossible }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedCardChoice {
    page: CardChoicePage<CardChoicePin>,
    security_order: Option<(PlayerId, u64, Vec<CardHandle>)>,
    /// Inspection may present the whole zone while only this subset is legal.
    selectable_pins: Option<Vec<CardChoicePin>>,
    security_eligibility: Option<SecurityEligibility>,
    source_prefix: Option<RequiredSourcePrefix>,
    source_completion_filter: Option<SourceCompletionFilter>,
    digixros_menu: bool,
    /// An inspection permission is tied to a seat, not to zone ownership.
    pub inspection_player: Option<PlayerId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardChoiceOperation {
    CommitCard,
    InspectCard,
    PreviousPage,
    NextPage,
    Decline,
    RefreshPage,
    FinishImpossible,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardChoiceZone {
    BattleSource,
    BattleTop,
    BreedingSource,
    Security,
    Hand,
    Trash,
}

#[derive(Debug, Clone, Serialize)]
pub struct CardChoiceRowView {
    pub action_id: u16,
    pub operation: CardChoiceOperation,
    pub zone: Option<CardChoiceZone>,
    pub owner: Option<PlayerId>,
    pub carrier_slot: Option<u8>,
    pub carrier_identity: Option<String>,
    pub position: Option<String>,
    pub card_id: Option<String>,
    pub card_registry_index: Option<usize>,
    pub identity_visible: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CardChoiceView {
    pub prompt_id: String,
    pub page_generation: String,
    pub page_index: String,
    pub page_count: String,
    pub inspection_only: bool,
    pub rows: Vec<CardChoiceRowView>,
}

impl OwnedCardChoice {
    /// Temporary private inspection is part of the viewer's information set.
    pub(crate) fn inspected_security_owner(&self, observer: PlayerId) -> Option<PlayerId> {
        (self.inspection_player == Some(observer)).then(|| self.security_order.as_ref().map(|s| s.0)).flatten()
    }

    /// A sampled world substitutes unknown cards at the SAME visible positions.
    /// Rebind security pins to those sampled instances; treating the hypothetical
    /// substitution as a real shuffle wrongly invalidates the root menu.
    pub(crate) fn rebind_sampled_security(&mut self, source: &Game, world: &Game) {
        let Some((owner, generation, order)) = &self.security_order else { return; };
        if !self.security_is_current(source) { return; }
        let owner = *owner;
        let generation = *generation;
        let sampled: Vec<_> = world.player(owner).security.iter().map(|c| c.handle()).collect();
        assert_eq!(order.len(), sampled.len(), "determinization preserves security counts");
        let remap = |pin: &CardChoicePin| match *pin {
            CardChoicePin::Security { owner: p, card } if p == owner => {
                order.iter().position(|h| *h == card).map_or(*pin,
                    |slot| CardChoicePin::Security { owner, card: sampled[slot] })
            }
            _ => *pin,
        };
        self.page.map_candidates(remap);
        if let Some(selectable) = &mut self.selectable_pins {
            *selectable = selectable.iter().map(remap).collect();
        }
        self.security_order = Some((owner, generation, sampled));
    }

    pub fn material(game: &mut Game, host: PermanentHandle, indices: &[usize], optional: bool) -> Option<Self> {
        let cards: Vec<_> = crate::effect_context::selections::material_zone_slice(game, host)?
            .iter().enumerate().filter(|(index, _)| indices.contains(index))
            .map(|(_, source)| source.handle()).collect();
        let pins = cards.into_iter().filter_map(|card| SourceChoicePin::capture(game, host, card)).collect();
        Self::sources(game, pins, optional)
    }

    pub fn sources(game: &mut Game, pins: Vec<SourceChoicePin>, optional: bool) -> Option<Self> {
        Self::new(
            game,
            pins.into_iter().map(CardChoicePin::Source).collect(),
            optional,
            None,
        )
    }

    /// One complete physical menu across all union zones; no legacy ID bands.
    pub fn union(game: &mut Game, owner: PlayerId,
        candidates: &[(CardHandle, crate::selection::UnionZoneOrigin)], optional: bool) -> Option<Self> {
        use crate::selection::UnionZoneOrigin;
        let pins = candidates.iter().filter_map(|(card, origin)| match origin {
            UnionZoneOrigin::Hand => Some(CardChoicePin::Hand { owner, card: *card }),
            UnionZoneOrigin::Trash => Some(CardChoicePin::Trash { owner, card: *card }),
            UnionZoneOrigin::Material { carrier, .. } => SourceChoicePin::capture(game, *carrier, *card)
                .map(CardChoicePin::Source),
        }).collect();
        Self::new(game, pins, optional, None)
    }

    pub fn digixros_pin(game: &mut Game, origin: crate::digixros::DigiXrosMaterialOrigin) -> Option<CardChoicePin> {
        use crate::digixros::DigiXrosMaterialOrigin as Origin;
        Some(match origin {
            Origin::Hand { player, card, .. } => CardChoicePin::Hand { owner: player, card },
            Origin::Trash { player, card, .. } => CardChoicePin::Trash { owner: player, card },
            Origin::UnderTamer { tamer, card, .. } => CardChoicePin::Source(SourceChoicePin::capture(game, tamer, card)?),
            Origin::BattleArea { permanent, card } => {
                if game.player(permanent.player).battle_area.get(permanent.index as usize)?.top_card().handle() != card { return None; }
                CardChoicePin::BattleTop { owner: permanent.player,
                    carrier_identity: game.capture_source_play_host_identity(permanent)?, card }
            }
        })
    }

    pub fn resolve_digixros_pin(game: &Game, pin: CardChoicePin) -> Option<crate::digixros::DigiXrosMaterialOrigin> {
        use crate::digixros::DigiXrosMaterialOrigin as Origin;
        match pin {
            CardChoicePin::Source(source) => source.resolve(game).map(|(tamer, source_index)| Origin::UnderTamer { tamer, source_index, card: source.card() }),
            CardChoicePin::Hand { owner, card } => game.player(owner).hand.iter().position(|c| c.handle() == card)
                .map(|index| Origin::Hand { player: owner, index, card }),
            CardChoicePin::Trash { owner, card } => game.player(owner).trash.iter().position(|c| c.handle() == card)
                .map(|index| Origin::Trash { player: owner, index, card }),
            CardChoicePin::BattleTop { owner, carrier_identity, card } => {
                let permanent = game.resolve_source_play_host_identity(PermanentHandle { player: owner, index: 0 }, carrier_identity)?;
                (game.player(owner).battle_area.get(permanent.index as usize)?.top_card().handle() == card)
                    .then_some(Origin::BattleArea { permanent, card })
            }
            CardChoicePin::Security { .. } => None,
        }
    }

    pub fn digixros(game: &mut Game, origins: &[crate::digixros::DigiXrosMaterialOrigin], optional: bool) -> Option<Self> {
        let pins = origins.iter().filter_map(|origin| Self::digixros_pin(game, *origin)).collect();
        let mut owner = Self::new(game, pins, optional, None)?;
        owner.digixros_menu = true;
        Some(owner)
    }

    pub fn security(
        game: &mut Game,
        owner: PlayerId,
        indices: Vec<usize>,
        optional: bool,
    ) -> Option<Self> {
        let order: Vec<_> = game
            .player(owner)
            .security
            .iter()
            .map(|c| c.handle())
            .collect();
        let pins = indices
            .into_iter()
            .filter_map(|i| order.get(i).copied())
            .map(|card| CardChoicePin::Security { owner, card })
            .collect();
        let generation = game.security_shuffle_generations[owner as usize];
        Self::new(game, pins, optional, Some((owner, generation, order)))
    }

    fn new(
        game: &mut Game,
        pins: Vec<CardChoicePin>,
        optional: bool,
        security_order: Option<(PlayerId, u64, Vec<CardHandle>)>,
    ) -> Option<Self> {
        if pins.is_empty() {
            return None;
        }
        let id = game.next_card_choice_id.checked_add(1)?;
        let page = CardChoicePage::new(id, pins, optional).ok()?;
        game.next_card_choice_id = id;
        Some(Self {
            page,
            security_order,
            selectable_pins: None,
            security_eligibility: None,
            source_prefix: None,
            source_completion_filter: None,
            digixros_menu: false,
            inspection_player: None,
        })
    }

    /// Internal complete physical source pool, including off-page candidates.
    pub fn source_pins(&self) -> Option<Vec<SourceChoicePin>> {
        self.page.candidates().iter().map(|pin| match pin {
            CardChoicePin::Source(source) => Some(*source),
            _ => None,
        }).collect()
    }

    pub fn token(&self) -> CardChoiceToken {
        self.page.token()
    }

    /// Search may collapse presentation routes only for a current multi-page
    /// prompt. Recovery operations are real transitions, not page navigation.
    pub(crate) fn search_page_shape(&self, game: &Game) -> Option<(usize, usize)> {
        (self.page.page_count() > 1 && self.recovery(game) == CardChoiceRecovery::Current)
            .then(|| (self.page.page_index(), self.page.page_count()))
    }

    pub fn grant_security_inspection(&mut self, observer: PlayerId, eligible_indices: &[usize]) {
        let Some((owner, _, order)) = &self.security_order else { return; };
        self.selectable_pins = Some(eligible_indices.iter().filter_map(|&index| order.get(index))
            .map(|&card| CardChoicePin::Security { owner: *owner, card }).collect());
        self.inspection_player = Some(observer);
    }

    fn selectable(&self, pin: CardChoicePin) -> bool {
        self.selectable_pins.as_ref().is_none_or(|pins| pins.contains(&pin))
    }

    pub(crate) fn set_security_filter<F>(&mut self, filter: F)
    where F: Fn(&Game, usize) -> bool + Send + Sync + 'static {
        self.security_eligibility = Some(SecurityEligibility(std::sync::Arc::new(
            SecurityEligibilityData::Raw(RawSecurityFilter(std::sync::Arc::new(filter))))));
    }

    pub(crate) fn set_security_dsl_filter(&mut self,
        filter: digimon_dsl::compiled::CompiledPredicate,
        bindings: crate::dsl_cards::bindings::Bindings,
        prov: crate::resume::ResumeProvenance) {
        self.security_eligibility = Some(SecurityEligibility(std::sync::Arc::new(
            SecurityEligibilityData::Dsl { filter, bindings, prov })));
    }

    pub(crate) fn require_source_prefix(&mut self, floor: usize, picked: Vec<SourceChoicePin>, allow_zero: bool) {
        self.source_prefix = Some(RequiredSourcePrefix { floor, picked, allow_zero, distinct: None, named: None });
    }

    pub(crate) fn require_source_constraints(&mut self,
        distinct: Option<crate::effect_context::selections::DistinctByMode>,
        named: Option<crate::resume::NamedSourceGroups>) {
        if let Some(prefix) = &mut self.source_prefix { prefix.distinct = distinct; prefix.named = named; }
    }

    pub(crate) fn set_source_completion_filter<F>(&mut self, filter: F)
    where F: Fn(&Game, Option<CardChoicePin>) -> bool + Send + Sync + 'static {
        self.source_completion_filter = Some(SourceCompletionFilter(std::sync::Arc::new(filter)));
    }

    fn live_security_pins(&self, game: &Game) -> Vec<CardChoicePin> {
        let Some((owner, _, _)) = &self.security_order else { return Vec::new(); };
        game.player(*owner).security.iter().enumerate().filter_map(|(index, card)| {
            let pin = CardChoicePin::Security { owner: *owner, card: card.handle() };
            let eligible = match self.security_eligibility.as_ref().map(|rule| rule.0.as_ref()) {
                Some(SecurityEligibilityData::Raw(filter)) => (filter.0)(game, index),
                Some(SecurityEligibilityData::Dsl { filter, bindings, prov }) => {
                    let read = crate::effect_context::EffectReadContext::new_with_source_kind(
                        game, prov.source_card, prov.source_permanent, prov.source_kind, prov.controller);
                    crate::dsl_cards::predicate::eval_predicate_with_bindings(filter, &read,
                        crate::dsl_cards::predicate::PredicateSubject::Card(card.handle()), Some(bindings))
                }
                None => self.page.candidates().contains(&pin) && self.selectable(pin),
            };
            eligible.then_some(pin)
        }).collect()
    }

    fn prefix_live_count(&self, game: &Game) -> usize {
        self.source_prefix.as_ref().map_or(0, |prefix|
            prefix.picked.iter().filter(|pin| pin.resolve(game).is_some()).count())
    }

    pub(crate) fn recovery(&self, game: &Game) -> CardChoiceRecovery {
        if self.source_completion_filter.as_ref().is_some_and(|filter| !(filter.0)(game, None)) {
            return CardChoiceRecovery::Impossible;
        }
        if self.security_order.is_some() {
            let live = self.live_security_pins(game);
            let frozen: Vec<_> = self.page.candidates().iter().copied().filter(|pin| self.selectable(*pin)).collect();
            if !self.security_is_current(game) || live != frozen {
                if !self.security_is_current(game) && self.page.allows_decline() { return CardChoiceRecovery::Current; }
                return if live.is_empty() { CardChoiceRecovery::Impossible } else { CardChoiceRecovery::RefreshSecurity };
            }
        }
        let live = self.page.candidates().iter().filter(|pin| self.selectable(**pin)
            && self.resolve_pin(game, **pin).is_some()).count();
        if let Some(prefix) = &self.source_prefix {
            let picked = self.prefix_live_count(game);
            if picked + live < prefix.floor && !(prefix.allow_zero && picked == 0) {
                return CardChoiceRecovery::Impossible;
            }
            if !(prefix.allow_zero && picked == 0) {
                let picked_cards: Vec<_> = prefix.picked.iter().filter(|pin| pin.resolve(game).is_some()).map(|pin| pin.card()).collect();
                let candidates: Vec<_> = self.page.candidates().iter().filter_map(|pin| match self.resolve_pin(game, *pin) {
                    Some(ResolvedCardChoice::Source { card, .. }) => Some(card), _ => None,
                }).collect();
                if let Some(groups) = &prefix.named {
                    let all: Vec<_> = picked_cards.iter().chain(candidates.iter()).copied().collect();
                    if !crate::dsl_cards::step::selections::named_source_groups_can_complete(groups, &all, &picked_cards, prefix.floor) {
                        return CardChoiceRecovery::Impossible;
                    }
                }
                if let Some(mode) = prefix.distinct {
                    let mut selected: Vec<_> = picked_cards.iter().filter_map(|&card| game.card_data_for_handle(card)).collect();
                    let pool: Vec<_> = candidates.iter().filter_map(|&card| game.card_data_for_handle(card)).collect();
                    if mode == crate::effect_context::selections::DistinctByMode::SameLevel && selected.is_empty() {
                        let max = pool.iter().filter_map(|card| card.level).map(|level|
                            pool.iter().filter(|card| card.level == Some(level)).count()).max().unwrap_or(0);
                        if max < prefix.floor { return CardChoiceRecovery::Impossible; }
                    } else {
                        for card in pool { if mode.admits(&selected, card) { selected.push(card); } }
                        if selected.len() < prefix.floor { return CardChoiceRecovery::Impossible; }
                    }
                }
            }
        } else if !self.page.allows_decline() && live == 0 {
            return CardChoiceRecovery::Impossible;
        }
        CardChoiceRecovery::Current
    }

    fn refresh_security(&mut self, game: &Game) -> Result<(), ()> {
        let (owner, _, _) = self.security_order.as_ref().ok_or(())?;
        let owner = *owner;
        let order: Vec<_> = game.player(owner).security.iter().map(|card| card.handle()).collect();
        let eligible = self.live_security_pins(game);
        let pins = if self.inspection_player.is_some() {
            order.iter().map(|&card| CardChoicePin::Security { owner, card }).collect()
        } else { eligible.clone() };
        self.page.refresh(pins).map_err(|_| ())?;
        self.security_order = Some((owner, game.security_shuffle_generations[owner as usize], order));
        if self.inspection_player.is_some() { self.selectable_pins = Some(eligible); }
        Ok(())
    }

    pub fn security_is_current(&self, game: &Game) -> bool {
        self.security_order
            .as_ref()
            .is_none_or(|(owner, generation, order)| {
                game.security_shuffle_generations[*owner as usize] == *generation
                    && game
                        .player(*owner)
                        .security
                        .iter()
                        .map(|c| c.handle())
                        .eq(order.iter().copied())
            })
    }

    pub fn resolve_pin(&self, game: &Game, pin: CardChoicePin) -> Option<ResolvedCardChoice> {
        if self.digixros_menu {
            let origin = Self::resolve_digixros_pin(game, pin)?;
            if !game.digixros_material_is_eligible(game.pending_digixros_transaction.as_ref()?, origin) { return None; }
        }
        match pin {
            CardChoicePin::Source(pin) => {
                pin.resolve(game)
                    .map(|(host, index)| ResolvedCardChoice::Source {
                        host,
                        index,
                        card: pin.card(),
                    })
            }
            CardChoicePin::BattleTop { owner, carrier_identity, card } => {
                let host = game.resolve_source_play_host_identity(PermanentHandle { player: owner, index: 0 }, carrier_identity)?;
                (game.player(owner).battle_area.get(host.index as usize)?.top_card().handle() == card)
                    .then_some(ResolvedCardChoice::BattleTop { host, card })
            }
            CardChoicePin::Hand { owner, card } => game.player(owner).hand.iter()
                .position(|c| c.handle() == card).map(|index| ResolvedCardChoice::Hand { owner, index, card }),
            CardChoicePin::Trash { owner, card } => game.player(owner).trash.iter()
                .position(|c| c.handle() == card).map(|index| ResolvedCardChoice::Trash { owner, index, card }),
            CardChoicePin::Security { owner, card } => {
                if !self.security_is_current(game) {
                    return None;
                }
                game.player(owner)
                    .security
                    .iter()
                    .position(|c| c.handle() == card)
                    .map(|index| ResolvedCardChoice::Security { owner, index, card })
            }
        }
    }

    pub fn valid_action_ids(&self, game: &Game) -> Vec<u16> {
        match self.recovery(game) {
            CardChoiceRecovery::RefreshSecurity => return if self.page.allows_decline() {
                vec![crate::card_choice_page::CARD_CHOICE_NEXT, crate::action::space::PASS]
            } else { vec![crate::card_choice_page::CARD_CHOICE_NEXT] },
            CardChoiceRecovery::Impossible => return vec![crate::action::space::PASS],
            CardChoiceRecovery::Current => {},
        }
        if !self.security_is_current(game) {
            return self
                .page
                .valid_action_ids()
                .into_iter()
                .filter(|id| *id == crate::action::space::PASS)
                .collect();
        }
        let rows = self.page.rows();
        self.page
            .valid_action_ids()
            .into_iter()
            .filter(|id| {
                if *id == crate::action::space::PASS {
                    if let Some(prefix) = &self.source_prefix {
                        let count = self.prefix_live_count(game);
                        return count >= prefix.floor || (prefix.allow_zero && count == 0);
                    }
                }
                rows.iter()
                    .find(|row| row.action_id == *id)
                    .is_none_or(|row| {
                        if !self.selectable(row.pin) || self.resolve_pin(game, row.pin).is_none() { return false; }
                        if self.source_completion_filter.as_ref().is_some_and(|filter| !(filter.0)(game, Some(row.pin))) { return false; }
                        if let Some(prefix) = &self.source_prefix {
                            if prefix.distinct == Some(crate::effect_context::selections::DistinctByMode::SameLevel)
                                && self.prefix_live_count(game) == 0 {
                                let Some(ResolvedCardChoice::Source { card, .. }) = self.resolve_pin(game, row.pin) else { return false; };
                                let Some(level) = game.card_data_for_handle(card).and_then(|data| data.level) else { return false; };
                                let count = self.page.candidates().iter().filter(|pin| match self.resolve_pin(game, **pin) {
                                    Some(ResolvedCardChoice::Source { card, .. }) => game.card_data_for_handle(card).is_some_and(|data| data.level == Some(level)),
                                    _ => false,
                                }).count();
                                if count < prefix.floor { return false; }
                            }
                        }
                        true
                    })
            })
            .collect()
    }

    pub fn resolve(
        &mut self,
        game: &Game,
        action: u16,
    ) -> Result<CardChoiceResult<ResolvedCardChoice>, ()> {
        if !self.valid_action_ids(game).contains(&action) {
            return Err(());
        }
        match self.recovery(game) {
            CardChoiceRecovery::RefreshSecurity => {
                if action == crate::action::space::PASS { return Ok(CardChoiceResult::Declined); }
                self.refresh_security(game)?;
                return Ok(CardChoiceResult::Navigated);
            }
            CardChoiceRecovery::Impossible => return Ok(CardChoiceResult::Declined),
            CardChoiceRecovery::Current => {},
        }
        match self
            .page
            .resolve(self.page.token(), action)
            .map_err(|_| ())?
        {
            CardChoiceResult::Selected(pin) => self
                .resolve_pin(game, pin)
                .map(CardChoiceResult::Selected)
                .ok_or(()),
            CardChoiceResult::Navigated => Ok(CardChoiceResult::Navigated),
            CardChoiceResult::Declined => Ok(CardChoiceResult::Declined),
        }
    }

    pub fn view(&self, game: &Game, observer: PlayerId) -> CardChoiceView {
        use crate::card_choice_page::{CARD_CHOICE_NEXT, CARD_CHOICE_PREVIOUS};
        let rows = self.page.rows();
        let recovery = self.recovery(game);
        let presented = if recovery == CardChoiceRecovery::Current && self.security_is_current(game) { self.page.valid_action_ids() }
            else { self.valid_action_ids(game) };
        let rows = presented
            .into_iter()
            .map(|action_id| {
                let mut row = CardChoiceRowView {
                    action_id,
                    operation: if recovery == CardChoiceRecovery::Impossible {
                        CardChoiceOperation::FinishImpossible
                    } else if recovery == CardChoiceRecovery::RefreshSecurity && action_id == CARD_CHOICE_NEXT {
                        CardChoiceOperation::RefreshPage
                    } else if action_id == CARD_CHOICE_PREVIOUS {
                        CardChoiceOperation::PreviousPage
                    } else if action_id == CARD_CHOICE_NEXT {
                        CardChoiceOperation::NextPage
                    } else {
                        CardChoiceOperation::Decline
                    },
                    zone: None,
                    owner: None,
                    carrier_slot: None,
                    carrier_identity: None,
                    position: None,
                    card_id: None,
                    card_registry_index: None,
                    identity_visible: false,
                };
                if let Some(candidate) = rows.iter().find(|r| r.action_id == action_id) {
                    if let Some(resolved) = self.resolve_pin(game, candidate.pin) {
                        row.operation = if self.selectable(candidate.pin) { CardChoiceOperation::CommitCard }
                            else { CardChoiceOperation::InspectCard };
                        let (card, visible) = match (candidate.pin, resolved) {
                            (
                                CardChoicePin::Source(pin),
                                ResolvedCardChoice::Source { host, index, card },
                            ) => {
                                row.zone = Some(match pin.area() {
                                    SourceChoiceArea::Battle => CardChoiceZone::BattleSource,
                                    SourceChoiceArea::Breeding => CardChoiceZone::BreedingSource,
                                });
                                row.owner = Some(host.player);
                                row.carrier_slot = Some(host.index);
                                row.carrier_identity = Some(pin.carrier_identity().to_string());
                                row.position = Some(index.to_string());
                                let visible =
                                    crate::effect_context::selections::material_zone_slice(
                                        game, host,
                                    )
                                    .and_then(|stack| stack.get(index))
                                    .is_some_and(|c| !c.face_down);
                                (card, visible)
                            }
                            (CardChoicePin::BattleTop { carrier_identity, .. }, ResolvedCardChoice::BattleTop { host, card }) => {
                                row.zone = Some(CardChoiceZone::BattleTop);
                                row.owner = Some(host.player);
                                row.carrier_slot = Some(host.index);
                                row.carrier_identity = Some(carrier_identity.to_string());
                                row.position = Some(host.index.to_string());
                                (card, true)
                            }
                            (_, ResolvedCardChoice::Hand { owner, index, card }) => {
                                row.zone = Some(CardChoiceZone::Hand);
                                row.owner = Some(owner);
                                row.position = Some(index.to_string());
                                (card, owner == observer)
                            }
                            (_, ResolvedCardChoice::Trash { owner, index, card }) => {
                                row.zone = Some(CardChoiceZone::Trash);
                                row.owner = Some(owner);
                                row.position = Some(index.to_string());
                                (card, true)
                            }
                            (_, ResolvedCardChoice::Security { owner, index, card }) => {
                                row.zone = Some(CardChoiceZone::Security);
                                row.owner = Some(owner);
                                row.position = Some(index.to_string());
                                (
                                    card,
                                    self.inspection_player == Some(observer)
                                        || game.player(owner).face_up_security.contains(&card.0),
                                )
                            }
                            _ => unreachable!(),
                        };
                        row.identity_visible = visible;
                        if visible {
                            if let Some(data) = game.card_data_for_handle(card) {
                                row.card_id = Some(data.card_id.clone());
                                row.card_registry_index = Some(data.index as usize);
                            }
                        }
                    }
                }
                row
            })
            .collect();
        CardChoiceView {
            prompt_id: self.token().prompt_id.to_string(),
            page_generation: self.token().page_generation.to_string(),
            page_index: self.page.page_index().to_string(),
            page_count: self.page.page_count().to_string(),
            inspection_only: self.selectable_pins.as_ref().is_some_and(|pins| pins.is_empty()),
            rows,
        }
    }
}

/// Navigation is dispatched before taking callbacks or running lifecycle hooks.
/// The selected pin is handed to exactly one typed/legacy selection handler.
pub(crate) fn prepare_resolution(
    game: &mut Game,
    action: u16,
) -> Result<bool, crate::selection::SelectionError> {
    use crate::{card_choice_page::CardChoiceResult, selection::SelectionError};
    let Some(mut owner) = game
        .pending_selection
        .as_ref()
        .and_then(|p| p.card_choice.clone())
    else {
        game.resolved_card_choice = None;
        return Ok(false);
    };
    let result = owner
        .resolve(game, action)
        .map_err(|_| SelectionError::InvalidAction)?;
    match result {
        CardChoiceResult::Navigated => {
            let valid = owner.valid_action_ids(game);
            let pending = game
                .pending_selection
                .as_mut()
                .ok_or(SelectionError::NoPendingSelection)?;
            pending.valid_action_ids = valid;
            pending.card_choice = Some(owner);
            Ok(true)
        }
        CardChoiceResult::Selected(pin) => {
            game.resolved_card_choice = Some(pin);
            Ok(false)
        }
        CardChoiceResult::Declined => {
            game.resolved_card_choice = None;
            Ok(false)
        }
    }
}
