from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_048(CardScript):
    """EX10-048 Myotismon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Timing: EffectTiming.BeforePayCost
        # When this card would be played, by deleting 1 of your Digimon with [Myotismon] in its text, reduce the play cost by 4.
        effect0 = ICardEffect()
        effect0.set_effect_name("EX10-048 Delete 1 of your Myotis-in-text Digimon to get Play Cost -4")
        effect0.set_effect_description("When this card would be played, by deleting 1 of your Digimon with [Myotismon] in its text, reduce the play cost by 4.")
        effect0.is_optional = True

        effect = effect0  # alias for condition closure
        def condition0(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Myotismon' in text):
                    return False
            else:
                return False
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # Timing: EffectTiming.None
        # Effect Immunity
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-048 Play Cost -4")
        effect1.set_effect_description("Effect Immunity")

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            permanent = effect.effect_source_permanent if hasattr(effect, 'effect_source_permanent') else None
            if permanent and permanent.top_card:
                text = permanent.top_card.card_text
                if not ('Myotismon' in text):
                    return False
            else:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Action: Effect Immunity"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            # Cost reduction (variable amount) — handled via cost_reduction property
            pass  # descriptive-tagged: cost_reduction
            # Grant effect immunity via modifier system
            if perm and game:
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    ModifierType.CANNOT_BE_SELECTED_BY_EFFECT, perm,
                    value_fn=lambda: True, expiry='end_of_turn')

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] 1 of your purple Digimon gains <Blocker> and <Retaliation> until your opponent's turn ends.
        effect2 = ICardEffect()
        effect2.set_effect_name("EX10-048 Your 1 Digimon gains Retaliation and Blocker")
        effect2.set_effect_description("[On Play] 1 of your purple Digimon gains <Blocker> and <Retaliation> until your opponent's turn ends.")
        effect2.is_on_play = True
        effect2._is_blocker = True
        effect2._is_retaliation = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker, Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')
                target_perm.grant_keyword('_is_retaliation')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] 1 of your purple Digimon gains <Blocker> and <Retaliation> until your opponent's turn ends.
        effect3 = ICardEffect()
        effect3.set_effect_name("EX10-048 Your 1 Digimon gains Retaliation and Blocker")
        effect3.set_effect_description("[On Deletion] 1 of your purple Digimon gains <Blocker> and <Retaliation> until your opponent's turn ends.")
        effect3.is_optional = True
        effect3.is_on_deletion = True
        effect3._is_blocker = True
        effect3._is_retaliation = True

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain Keyword Blocker, Gain Keyword Retaliation"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def target_filter(p):
                return p.is_digimon
            def on_grant(target_perm):
                target_perm.grant_keyword('_is_blocker')
                target_perm.grant_keyword('_is_retaliation')
            game.effect_select_own_permanent(
                player, on_grant, filter_fn=target_filter, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] You may play 1 purple Tamer card from your trash suspended without paying the cost.
        effect4 = ICardEffect()
        effect4.set_effect_name("EX10-048 Play 1 purple Tamer from trash suspended")
        effect4.set_effect_description("[On Deletion] You may play 1 purple Tamer card from your trash suspended without paying the cost.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Action: Play Card"""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                if not ('Purple' in [col.name for col in getattr(c, 'card_colors', [])]):
                    return False
                return True
            game.effect_play_from_zone(
                player, 'trash', play_filter, free=True, is_optional=True)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
