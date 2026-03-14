from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_062(CardScript):
    """EX9-062 SkullGreymon | Lv.5 | Undead/DM/Ver.2
    This card is also treated as level 4 for [Kimeramon]'s assembly.
    [On Play][When Digivolving] For each face-down digi card, trash top card
    of deck. Then, return 1 [DM] trait Digimon from trash to hand.
    [On Deletion] Play 1 Lv.4 or lower [DM] trait Digimon from trash free.
    Inherited: [On Deletion] Play 1 Lv.4 or lower [DM] Digimon from trash free.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _mill_and_return(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return
            face_down_count = len(perm.digivolution_cards)
            mill_count = min(face_down_count, len(player.library_cards))
            trashed = player.library_cards[:mill_count]
            player.library_cards = player.library_cards[mill_count:]
            player.trash_cards.extend(trashed)

            # Return 1 DM Digimon from trash to hand
            for c in player.trash_cards:
                if getattr(c, 'is_digimon', False):
                    traits = getattr(c, 'card_traits', []) or []
                    if any('DM' in t for t in traits):
                        player.trash_cards.remove(c)
                        player.hand_cards.append(c)
                        break

        def _play_dm_lv4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def dm_lv4_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('DM' in t for t in traits)

            game.effect_play_from_zone(
                player, 'trash', dm_lv4_filter, free=True, is_optional=True,
                prompt="Play 1 Lv.4 or lower [DM] Digimon from trash.")

        # [On Play] Mill per face-down, return DM from trash
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name("EX9-062 On Play: Mill per face-down, return DM")
        effect0.set_effect_description("[On Play] Mill per face-down digi, return DM to hand.")
        effect0.is_on_play = True

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effect0.set_on_process_callback(_mill_and_return)
        effects.append(effect0)

        # [When Digivolving] Same
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX9-062 When Digivolving: Mill per face-down, return DM")
        effect1.set_effect_description("[When Digivolving] Mill per face-down, return DM.")
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_mill_and_return)
        effects.append(effect1)

        # [On Deletion] Play Lv.4 or lower DM from trash
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("EX9-062 On Deletion: Play DM Lv4 from trash")
        effect2.set_effect_description("[On Deletion] Play 1 Lv.4 or lower [DM] from trash.")
        effect2.is_on_deletion = True
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_dm_lv4)
        effects.append(effect2)

        # Inherited: [On Deletion] Play Lv.4 or lower DM from trash
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("EX9-062 Inherited: On Deletion Play DM Lv4 from trash")
        effect3.set_effect_description(
            "Inherited: [On Deletion] Play 1 Lv.4 or lower [DM] from trash."
        )
        effect3.is_inherited_effect = True
        effect3.is_on_deletion = True
        effect3.is_optional = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_play_dm_lv4)
        effects.append(effect3)

        return effects
