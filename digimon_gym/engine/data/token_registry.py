"""Token definitions and factory for the Digimon TCG engine.

Tokens are synthetic Digimon permanents that cease to exist when leaving
the field by any means. Each token type defines card metadata and effects
that are pre-attached to the CardSource's _cached_effects.
"""
from __future__ import annotations
from typing import TYPE_CHECKING, Dict, Any, List

from ..core.entity_base import CEntity_Base
from ..core.card_source import CardSource
from ..interfaces.card_effect import ICardEffect
from .enums import CardColor, CardKind, EffectTiming

if TYPE_CHECKING:
    from ..core.player import Player


# ── Token Definitions ────────────────────────────────────────────────

TOKENS = {
    'petrification': {
        'card_id': 'TOKEN_PETRIFICATION',
        'card_name': 'Petrification Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.White],
        'dp': 3000,
        'level': None,
        'traits': [],
    },
    'diaboromon': {
        'card_id': 'TOKEN_DIABOROMON',
        'card_name': 'Diaboromon',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.White],
        'dp': 3000,
        'play_cost': 14,
        'level': 6,
        'traits': ['Unidentified'],
        'form': ['Mega'],
        'attribute': ['Unknown'],
    },
    'atho_rene_por': {
        'card_id': 'TOKEN_ATHO_RENE_POR',
        'card_name': 'Atho, Rene & Por Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.White],
        'dp': 6000,
        'level': None,
        'traits': [],
        'keywords': ['_is_reboot', '_is_blocker', '_is_decoy'],
    },
    'hinukamuy': {
        'card_id': 'TOKEN_HINUKAMUY',
        'card_name': 'Hinukamuy Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.White],
        'dp': 6000,
        'level': None,
        'traits': [],
        'keywords': ['_is_alliance', '_is_reboot', '_is_blocker'],
    },
    'familiar': {
        'card_id': 'TOKEN_FAMILIAR',
        'card_name': 'Familiar Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Yellow],
        'dp': 3000,
        'level': None,
        'traits': [],
    },
    'kohagurumon': {
        'card_id': 'TOKEN_KOHAGURUMON',
        'card_name': 'KoHagurumon Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Black],
        'dp': 3000,
        'level': None,
        'traits': ['Machine'],
    },
    'fujitsumon': {
        'card_id': 'TOKEN_FUJITSUMON',
        'card_name': 'Fujitsumon Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Purple],
        'dp': 3000,
        'level': None,
        'traits': [],
    },
    'wargrowlmon': {
        'card_id': 'TOKEN_WARGROWLMON',
        'card_name': 'WarGrowlmon Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Red],
        'dp': 6000,
        'level': None,
        'traits': [],
    },
    'taomon': {
        'card_id': 'TOKEN_TAOMON',
        'card_name': 'Taomon Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Yellow],
        'dp': 6000,
        'level': None,
        'traits': [],
    },
    'rapidmon': {
        'card_id': 'TOKEN_RAPIDMON',
        'card_name': 'Rapidmon Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Green],
        'dp': 6000,
        'level': None,
        'traits': [],
    },
    'amon': {
        'card_id': 'TOKEN_AMON',
        'card_name': 'Amon of Crimson Flame Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Red],
        'dp': 6000,
        'level': None,
        'traits': [],
        'keywords': ['_is_rush'],
    },
    'umon': {
        'card_id': 'TOKEN_UMON',
        'card_name': 'Umon of Blue Thunder Token',
        'card_kind': CardKind.Digimon,
        'colors': [CardColor.Yellow],
        'dp': 6000,
        'level': None,
        'traits': [],
        'keywords': ['_is_blocker'],
    },
}


def _build_petrification_effects(card_source: CardSource) -> List[ICardEffect]:
    """Build effects for a Petrification Token.

    [On Deletion] Trash the top card of this Digimon's owner's security stack.
    """
    effects = []

    effect0 = ICardEffect()
    effect0.set_timing(EffectTiming.OnDestroyedAnyone)
    effect0.set_effect_name("Petrification Token On Deletion")
    effect0.set_effect_description("[On Deletion] Trash the top card of this Digimon's owner's security stack.")
    effect0.is_on_deletion = True

    def condition0(context: Dict[str, Any]) -> bool:
        return True

    effect0.set_can_use_condition(condition0)

    def process0(ctx: Dict[str, Any]):
        owner = card_source.owner
        if owner and len(owner.security_cards) > 0:
            trashed = owner.security_cards.pop(0)
            owner.trash_cards.append(trashed)
            if hasattr(owner, '_log'):
                owner._log(f"[Petrification Token] Trashed top security card on deletion.")

    effect0.set_on_process_callback(process0)
    effects.append(effect0)

    return effects


def _build_keyword_effects(keyword_attrs: List[str], label: str) -> List[ICardEffect]:
    effect = ICardEffect()
    effect.set_effect_name(f"{label} Keywords")
    effect.set_effect_description(f"{label} token innate keywords")

    def condition0(context: Dict[str, Any]) -> bool:
        return True

    effect.set_can_use_condition(condition0)
    for keyword_attr in keyword_attrs:
        setattr(effect, keyword_attr, True)
    return [effect]


def _build_familiar_effects(card_source: CardSource) -> List[ICardEffect]:
    """Build effects for a Familiar Token.

    [On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn.
    """
    effects = []

    effect0 = ICardEffect()
    effect0.set_timing(EffectTiming.OnDestroyedAnyone)
    effect0.set_effect_name("Familiar Token On Deletion")
    effect0.set_effect_description(
        "[On Deletion] 1 of your opponent's Digimon gets -3000 DP for the turn."
    )
    effect0.is_on_deletion = True

    def condition0(context: Dict[str, Any]) -> bool:
        return True

    effect0.set_can_use_condition(condition0)

    def process0(ctx: Dict[str, Any]):
        game = ctx.get('game')
        player = ctx.get('player')
        if not (game and player):
            owner = card_source.owner
            if not owner:
                return
            player = owner
            game = getattr(player, 'game', None)
            if not game:
                return
        enemy = player.enemy if player else None
        if not enemy:
            return
        opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
        if not opp_digimon:
            return

        def on_select(target_perm):
            target_perm.change_dp(-3000)

        game.effect_select_opponent_permanent(
            player, on_select,
            filter_fn=lambda p: p.is_digimon,
            is_optional=False,
            prompt="Select 1 of your opponent's Digimon to get -3000 DP.",
        )

    effect0.set_on_process_callback(process0)
    effects.append(effect0)

    return effects


def _build_fujitsumon_effects(card_source: CardSource) -> List[ICardEffect]:
    """Build effects for a Fujitsumon Token.

    [All Turns] This Digimon doesn't unsuspend.
    [On Deletion] Trash 1 card in your hand.
    """
    effects = []

    # Doesn't unsuspend
    effect0 = ICardEffect()
    effect0.set_effect_name("Fujitsumon Token Cannot Unsuspend")
    effect0.set_effect_description("[All Turns] This Digimon doesn't unsuspend.")
    effect0._is_cannot_unsuspend = True

    def condition0(context: Dict[str, Any]) -> bool:
        return True
    effect0.set_can_use_condition(condition0)
    effects.append(effect0)

    # On Deletion: Trash 1 card in owner's hand
    effect1 = ICardEffect()
    effect1.set_timing(EffectTiming.OnDestroyedAnyone)
    effect1.set_effect_name("Fujitsumon Token On Deletion")
    effect1.set_effect_description("[On Deletion] Trash 1 card in your hand.")
    effect1.is_on_deletion = True

    def condition1(context: Dict[str, Any]) -> bool:
        return True
    effect1.set_can_use_condition(condition1)

    def process1(ctx: Dict[str, Any]):
        owner = card_source.owner
        if not owner:
            return
        game = ctx.get('game')
        if not game:
            return
        if len(owner.hand_cards) == 0:
            return

        def on_select(selected_card):
            if selected_card in owner.hand_cards:
                owner.hand_cards.remove(selected_card)
                owner.trash_cards.append(selected_card)

        game.effect_select_hand_card(
            owner, lambda c: True, on_select,
            is_optional=False,
            prompt="Trash 1 card from your hand (Fujitsumon).",
        )

    effect1.set_on_process_callback(process1)
    effects.append(effect1)

    return effects


# Map token type -> effect builder
_EFFECT_BUILDERS = {
    'petrification': _build_petrification_effects,
    'familiar': _build_familiar_effects,
    'fujitsumon': _build_fujitsumon_effects,
}


# ── Factory ──────────────────────────────────────────────────────────

def create_token_card_source(token_type: str, owner: 'Player') -> CardSource:
    """Create a CardSource for a token, with metadata and effects pre-attached.

    Args:
        token_type: Key into TOKENS dict (e.g., 'petrification').
        owner: The player who will control this token on the field.

    Returns:
        A CardSource with is_token=True and _cached_effects pre-populated.
    """
    template = TOKENS[token_type]

    entity = CEntity_Base()
    entity.card_id = template['card_id']
    entity.card_name_eng = template['card_name']
    entity.card_kind = template['card_kind']
    entity.card_colors = list(template['colors'])
    entity.dp = template['dp']
    entity.level = template['level']
    entity.type_eng = list(template['traits'])
    if 'play_cost' in template:
        entity.play_cost = template['play_cost']
    if 'form' in template:
        entity.form_eng = list(template['form'])
    if 'attribute' in template:
        entity.attribute_eng = list(template['attribute'])

    card_source = CardSource()
    card_source.set_base_data(entity, owner)
    card_source.is_token = True

    effects: List[ICardEffect] = []
    builder = _EFFECT_BUILDERS.get(token_type)
    if builder:
        effects.extend(builder(card_source))
    if template.get('keywords'):
        effects.extend(_build_keyword_effects(template['keywords'], template['card_name']))
    card_source._cached_effects = effects

    return card_source
