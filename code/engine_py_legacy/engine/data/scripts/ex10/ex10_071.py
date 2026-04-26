from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_071(CardScript):
    """EX10-071 Paradise Lost | Option

    While you have a Digimon with [Lucemon] in its name on the field, you can
        ignore this card's color requirements.
    [Trash] [End of Your Turn] If you have a Digimon with [Lucemon] in its name,
        by returning this card to the bottom of the deck, trash your top security
        card and 1 of your Digimon attacks without suspending.
    [Main] Until your opponent's turn ends, 1 of your Digimon with [Lucemon] in
        its name gains Raid, Piercing, Blocker and +3000 DP.
    [Security] You may play 1 [Lucemon] from your trash without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Ignore color requirements while you have Lucemon
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-071 Ignore color requirements")
        effect0.set_effect_description(
            "While you have a Digimon with [Lucemon] in its name on the field, "
            "you can ignore this card's color requirements."
        )
        effect0._ignore_color_requirements = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [Main] Give 1 Lucemon Raid/Piercing/Blocker/+3000 DP until opponent's turn ends
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("EX10-071 Main: Lucemon gains Raid/Piercing/Blocker/+3000")
        effect1.set_effect_description(
            "[Main] Until your opponent's turn ends, 1 of your Digimon with "
            "[Lucemon] in its name gains Raid, Piercing, Blocker and +3000 DP."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            from ....interfaces.modifiers import ModifierType

            def target_filter(p):
                return p.is_digimon and p.contains_card_name('Lucemon')

            def on_grant(target_perm):
                target_perm.grant_keyword('_is_raid')
                target_perm.grant_keyword('_is_piercing')
                target_perm.grant_keyword('_is_blocker')
                game.register_modifier(
                    target_perm, ModifierType.CHANGE_DP,
                    value_fn=lambda: 3000,
                    expiry='end_of_opponent_turn',
                )

            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # [Trash] [End of Your Turn] If you have Lucemon, return this card to
        # deck bottom, trash own top security, 1 Digimon attacks without suspending
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.EndOfTurn)
        effect2.set_effect_name("EX10-071 Trash trigger: trash security, attack")
        effect2.set_effect_description(
            "[Trash] [End of Your Turn] If you have a Digimon with [Lucemon] in "
            "its name, by returning this card to the bottom of the deck, trash "
            "your top security card and 1 of your Digimon attacks without suspending."
        )
        effect2.is_optional = True
        effect2._is_trash_trigger = True

        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            player = card.owner
            # Must be in trash
            if card not in player.trash_cards:
                return False
            # Must have Lucemon on field
            has_lucemon = any(
                p.is_digimon and p.contains_card_name('Lucemon')
                for p in player.battle_area
            )
            if not has_lucemon:
                return False
            if not player.security_cards:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Cost: return this card to bottom of deck
            if card in player.trash_cards:
                player.trash_cards.remove(card)
                player.library_cards.append(card)
            else:
                return
            # Trash own top security card
            if player.security_cards:
                trashed = player.security_cards.pop(0)
                player.trash_cards.append(trashed)
            # 1 of your Digimon attacks without suspending
            # (engine doesn't fully support forced attacks from effects; descriptive)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # [Security] Play 1 [Lucemon] from trash
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX10-071 Security: play Lucemon from trash")
        effect3.set_effect_description(
            "[Security] You may play 1 [Lucemon] from your trash without paying the cost."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
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

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
