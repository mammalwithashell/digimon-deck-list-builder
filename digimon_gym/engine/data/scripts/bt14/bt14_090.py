from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT14_090(CardScript):
    """BT14-090 Dragon of Courage"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # While you have a Tamer with [Tai Kamiya] in its name, you can ignore this card's color requirements.
        effect0 = ICardEffect()
        effect0.set_effect_name("BT14-090 Ignore color requirements")
        effect0.set_effect_description("While you have a Tamer with [Tai Kamiya] in its name, ignore this card's color requirements.")

        def condition0(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            if not player:
                return False
            for t in getattr(player, 'tamers', []) or []:
                name = str(getattr(t, 'card_name', '') or getattr(t, 'name', '') or '')
                if 'tai kamiya' in name.lower():
                    return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            # Engine handles color requirement checks externally; this effect acts as a condition gate.
            return

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        effect1 = ICardEffect()
        effect1.set_effect_name("BT14-090 Digivolve")
        effect1.set_effect_description("[Main] By placing 1 [Greymon] and 1 [MetalGreymon] from your trash as the bottom digivolution cards of 1 of your [Agumon], it may digivolve into [WarGreymon] in your hand ignoring its digivolution requirements and without paying its cost.")

        def condition1(context: Dict[str, Any]) -> bool:
            player = context.get('player')
            perm = context.get('permanent')
            if not player or not perm:
                return False
            perm_name = str(getattr(perm, 'card_name', '') or getattr(perm, 'name', '') or '').lower()
            if 'agumon' not in perm_name:
                return False
            trash = getattr(player, 'trash_cards', []) or []
            has_greymon = any('greymon' in str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower() and 'metalgreymon' not in str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower() for c in trash)
            has_metalgreymon = any('metalgreymon' in str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower() for c in trash)
            return has_greymon and has_metalgreymon

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            perm_name = str(getattr(perm, 'card_name', '') or getattr(perm, 'name', '') or '').lower()
            if 'agumon' not in perm_name:
                return

            trash = getattr(player, 'trash_cards', None)
            if trash is None:
                return

            greymon_idx = None
            metalgreymon_idx = None
            for i, c in enumerate(trash):
                n = str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower()
                if metalgreymon_idx is None and 'metalgreymon' in n:
                    metalgreymon_idx = i
                elif greymon_idx is None and 'greymon' in n and 'metalgreymon' not in n:
                    greymon_idx = i

            if greymon_idx is None or metalgreymon_idx is None:
                return

            # Remove in descending index order to keep indices stable.
            take_indices = sorted([greymon_idx, metalgreymon_idx], reverse=True)
            taken = [trash.pop(i) for i in take_indices]

            evo_cards = getattr(perm, 'evolution_cards', None)
            if evo_cards is not None:
                # Put as bottom digivolution cards (preserve declared order: Greymon then MetalGreymon when possible)
                greymon_card = next((c for c in taken if 'metalgreymon' not in str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower()), None)
                metalgreymon_card = next((c for c in taken if 'metalgreymon' in str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower()), None)
                if metalgreymon_card is not None:
                    evo_cards.insert(0, metalgreymon_card)
                if greymon_card is not None:
                    evo_cards.insert(0, greymon_card)

            def digi_filter(c):
                n = str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower()
                return 'wargreymon' in n

            game.effect_digivolve_from_hand(player, perm, digi_filter, is_optional=True)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.SecuritySkill
        effect2 = ICardEffect()
        effect2.set_effect_name("BT14-090 Play Card, Add To Hand")
        effect2.set_effect_description("[Security] You may play 1 [Agumon] from your hand or trash without paying the cost. Then, add this card to your hand.")
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            source_card = ctx.get('card')
            if not (player and game):
                return

            def play_filter(c):
                n = str(getattr(c, 'card_name', '') or getattr(c, 'name', '') or '').lower()
                return 'agumon' in n

            game.effect_play_from_zone(player, 'hand', play_filter, free=True, is_optional=True)
            game.effect_play_from_zone(player, 'trash', play_filter, free=True, is_optional=True)

            if source_card is not None:
                security_zone = getattr(player, 'security_cards', None)
                if isinstance(security_zone, list) and source_card in security_zone:
                    security_zone.remove(source_card)
                hand = getattr(player, 'hand_cards', None)
                if isinstance(hand, list):
                    hand.append(source_card)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
