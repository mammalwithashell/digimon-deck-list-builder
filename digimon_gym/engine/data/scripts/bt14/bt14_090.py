from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_090(CardScript):
    """BT14-090 Dragon of Courage"""

    @staticmethod
    def _name_of(card: Any) -> str:
        return str(getattr(card, 'card_name', '') or getattr(card, 'name', '') or '').lower()

    @staticmethod
    def _has_named_tamer(player: Any, needle: str) -> bool:
        if not player:
            return False
        needle_l = needle.lower()
        for zone_name in ('tamers', 'tamer_cards', 'battle_tamers', 'field_tamers'):
            zone = getattr(player, zone_name, None)
            if not zone:
                continue
            for c in zone:
                if needle_l in BT14_090._name_of(c):
                    return True
        return False

    @staticmethod
    def _count_in_trash(player: Any, needle: str) -> int:
        if not player:
            return 0
        trash = getattr(player, 'trash_cards', None) or []
        needle_l = needle.lower()
        return sum(1 for c in trash if needle_l in BT14_090._name_of(c))

    @staticmethod
    def _has_in_hand(player: Any, needle: str) -> bool:
        if not player:
            return False
        hand = getattr(player, 'hand_cards', None) or []
        needle_l = needle.lower()
        return any(needle_l in BT14_090._name_of(c) for c in hand)

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # While you have a Tamer with [Tai Kamiya] in its name, you can ignore this card's color requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-090 Ignore color requirements")
        effect0.set_effect_description("While you have a Tamer with [Tai Kamiya] in its name, you can ignore this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            return self._has_named_tamer(player, 'tai kamiya')

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Engine-level color requirement bypass is handled by this passive effect tag.
            return

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Main] ... place 1 [Greymon] and 1 [MetalGreymon] from trash ... 1 of your [Agumon] may digivolve into [WarGreymon] ...
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-090 Digivolve")
        effect1.set_effect_description("[Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as the bottom digivolution cards of 1 of your [Agumon], it may digivolve into a [WarGreymon] from hand ignoring its digivolution requirements and without paying its cost.")

        def condition1(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            perm = context.get('permanent')
            if not (player and perm):
                return False
            perm_name = self._name_of(perm)
            return (
                'agumon' in perm_name
                and self._count_in_trash(player, 'greymon') > 0
                and self._count_in_trash(player, 'metalgreymon') > 0
                and self._has_in_hand(player, 'wargreymon')
            )

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def digi_filter(c):
                return 'wargreymon' in self._name_of(c)

            game.effect_digivolve_from_hand(player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to the hand.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-090 Security Play Agumon")
        effect2.set_effect_description("[Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to your hand.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                return 'agumon' in self._name_of(c)

            game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=True)
            game.effect_play_from_zone(player, 'trash', play_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
