from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT18_086(CardScript):
    """BT18-086 Lucemon: Larva | Lv.6

    [Security] You may play 1 [Lucemon] from your trash without paying the cost.
    [Breeding] [All Turns] When any of your [Lucemon: Satan Mode] would leave
        the battle area, by moving this Digimon to battle area, it doesn't leave.
    [All Turns] While you have a non-white Digimon with [Lucemon] in its name,
        all of your 0 DP Digimon can't be deleted.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # [Security] Play 1 [Lucemon] from trash
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.SecuritySkill)
        effect0.set_effect_name("BT18-086 Security: play Lucemon from trash")
        effect0.set_effect_description(
            "[Security] You may play 1 [Lucemon] from your trash without paying the cost."
        )
        effect0.is_security_effect = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any(n == 'Lucemon' for n in names)

            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Breeding] [All Turns] When [Lucemon: Satan Mode] would leave,
        # by moving this Digimon to battle area, it doesn't leave.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.WhenRemoveField)
        effect1.set_effect_name("BT18-086 Prevent Satan Mode from leaving")
        effect1.set_effect_description(
            "[Breeding] [All Turns] When any of your [Lucemon: Satan Mode] would "
            "leave the battle area, by moving this Digimon to battle area, it doesn't leave."
        )
        effect1.is_optional = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be in breeding area
            player = card.owner if card else None
            if not player:
                return False
            perm = card.permanent_of_this_card()
            if not perm:
                return False
            if perm not in (player.breeding_area or []):
                # Check if it's the breeding area permanent
                if player.breeding_area and perm is not player.breeding_area:
                    return False
            # Check that the leaving permanent is Lucemon: Satan Mode
            leaving_perm = context.get('permanent')
            if not leaving_perm:
                return False
            if not leaving_perm.contains_card_name('Lucemon: Satan Mode'):
                return False
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Move this Digimon from breeding to battle area
            player = ctx.get('player')
            if player:
                perm = card.permanent_of_this_card() if card else None
                if perm and player.breeding_area is perm:
                    player.move_from_breeding()
                    ctx['prevent_leave'] = True

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [All Turns] While you have a non-white Digimon with [Lucemon] in its name,
        # all of your 0 DP Digimon can't be deleted
        # descriptive-tagged: continuous deletion prevention based on board state
        effect2 = ICardEffect()
        effect2.set_effect_name("BT18-086 0 DP Digimon can't be deleted")
        effect2.set_effect_description(
            "[All Turns] While you have a non-white Digimon with [Lucemon] in "
            "its name, all of your 0 DP Digimon can't be deleted."
        )

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        return effects
