from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_071(CardScript):
    """EX10-071 Paradise Lost"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.None
        # Ignore Color Req
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-071 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Ignore Color Req"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Ignores color requirement for playing Options — not modeled in engine
            pass  # descriptive-tagged

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.OptionSkill
        # [Main] Until your opponent's turn ends, 1 of your Digimon with [Lucemon] in its name in its name gains <Raid>, <Piercing>, <Blocker> and +3000 DP.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-071 1 Digimon gains <Raid>, <Piercing>, <Blocker>, +3000 DP")
        effect1.set_effect_description("[Main] Until your opponent's turn ends, 1 of your Digimon with [Lucemon] in its name in its name gains <Raid>, <Piercing>, <Blocker> and +3000 DP.")
        effect1._is_raid = True
        effect1._is_piercing = True
        effect1._is_blocker = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            # Option main effect — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: DP +3000, Gain Keyword Raid, Gain Keyword Piercing, Gain Keyword Blocker"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if perm:
                perm.change_dp(3000)
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Lucemon')):
                    return False
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_raid')
                target_perm.grant_keyword('_is_piercing')
                target_perm.grant_keyword('_is_blocker')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEndTurn
        # [Trash] [End of Your Turn] If you have a Digimon with [Lucemon] in its name, by returning this card to the bottom of the deck, trash your top security card and 1 of your Digimon attacks without suspending.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-071 Trash top security, Then one of your Digimon may attack")
        effect2.set_effect_description("[Trash] [End of Your Turn] If you have a Digimon with [Lucemon] in its name, by returning this card to the bottom of the deck, trash your top security card and 1 of your Digimon attacks without suspending.")
        effect2.is_optional = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Destroy Security, Return To Deck, Force Attack"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Trash opponent's top security card(s)
            enemy = player.enemy if player else None
            if enemy:
                for _ in range(1):
                    if enemy.security_cards:
                        trashed = enemy.security_cards.pop(0)
                        enemy.trash_cards.append(trashed)
            if not (player and game):
                return
            def target_filter(p):
                if not (p.contains_card_name('Lucemon')):
                    return False
                return True
            def on_return(target_perm):
                enemy = player.enemy if player else None
                if enemy:
                    enemy.return_permanent_to_deck_bottom(target_perm)
            game.effect_select_opponent_permanent(
                player, on_return, filter_fn=target_filter, is_optional=True)
            # Force attack — target Digimon may attack (requires engine SelectAttack)
            pass  # descriptive-tagged: force_attack

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.SecuritySkill
        # [Security] You may play 1 [Lucemon] from your trash without paying the cost.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-071 Play 1 [Lucemon] from trash")
        effect3.set_effect_description("[Security] You may play 1 [Lucemon] from your trash without paying the cost.")
        effect3.is_optional = True
        effect3.is_security_effect = True
        effect3.is_security_effect = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Security effect — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not (any('Lucemon' in _n for _n in getattr(c, 'card_names', []))):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
