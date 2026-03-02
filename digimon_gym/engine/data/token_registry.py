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


# Map token type -> effect builder
_EFFECT_BUILDERS = {
    'petrification': _build_petrification_effects,
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

    builder = _EFFECT_BUILDERS.get(token_type)
    if builder:
        card_source._cached_effects = builder(card_source)
    else:
        card_source._cached_effects = []

    return card_source
