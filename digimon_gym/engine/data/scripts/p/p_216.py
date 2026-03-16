from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_216(CardScript):
    """P-216 WaruMonzaemon | Lv.5

    Blocker.
    [On Play] You may play 1 [Dark Masters] trait Digimon card from your
    hand without paying the cost. The Digimon this effect played can't
    digivolve and is deleted at turn end.
    [On Deletion] You may play 1 face-up [Dark Masters] trait Digimon card
    from your security stack without paying the cost. At the end of your
    turn, delete the Digimon this effect played.
    Inherited: Blocker.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Blocker ---
        effect0 = ICardEffect()
        effect0.set_effect_name("P-216 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: [On Play] Play a Dark Masters from hand, free ---
        # The Digimon played can't digivolve and is deleted at turn end.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("P-216 Play a Dark Master from Hand.")
        effect1.set_effect_description(
            "[On Play] You may play 1 [Dark Master] trait Digimon card from "
            "your hand without paying the cost. The Digimon this effect played "
            "can't digivolve and is deleted at turn end."
        )
        effect1.is_optional = True
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def _play_filter_hand(c):
            """Dark Masters trait Digimon from hand."""
            if not getattr(c, 'is_digimon', False):
                return False
            traits = getattr(c, 'card_traits', []) or []
            return any('Dark Masters' in t for t in traits)

        def process1(ctx: Dict[str, Any]):
            """Select and play a Dark Masters Digimon from hand, then apply
            CANNOT_DIGIVOLVE and end-of-turn deletion to the played Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_card_selected(selected_card):
                # Play the selected card without paying cost
                played_perm = player.play_card_from_source(
                    selected_card, pay_cost=False)
                if not played_perm:
                    return
                game.logger.log(
                    f"[Effect] {player.player_name} played "
                    f"{game._card_ref(selected_card)} from hand via P-216 On Play")

                # Fire On Enter Field effects for the played card
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": selected_card,
                     "played_permanent": played_perm,
                     "event_player": player})
                game._fire_play_observers(played_perm, player)

                # Apply CANNOT_DIGIVOLVE to the played Digimon
                # C# ref: CanNotDigivolveClass attached to the played permanent
                from digimon_gym.engine.interfaces.modifiers import ModifierType
                game.register_modifier(
                    played_perm, ModifierType.CANNOT_DIGIVOLVE,
                    value_fn=lambda: True, expiry='end_of_turn')

                # Register end-of-turn deletion for the played Digimon
                # C# ref: EffectDuration.UntilEachTurnEnd, OnEndTurn timing
                _played = played_perm  # capture reference

                def _eot_delete_condition(eot_ctx):
                    return (_played.top_card is not None
                            and _played in player.battle_area)

                def _eot_delete_process(eot_ctx):
                    if _played in player.battle_area:
                        game_ref = eot_ctx.get('game') or game
                        player.delete_permanent(_played)

                eot_effect = ICardEffect()
                eot_effect.set_timing(EffectTiming.OnEndTurn)
                eot_effect.set_effect_name("P-216 Delete played Digimon at turn end")
                eot_effect.set_can_use_condition(_eot_delete_condition)
                eot_effect.set_on_process_callback(_eot_delete_process)
                played_perm.grant_temp_effect(eot_effect)

            game.effect_select_hand_card(
                player, _play_filter_hand, on_card_selected,
                is_optional=True,
                prompt="Select 1 [Dark Masters] trait Digimon from hand to play.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: [On Deletion] Play a Dark Masters from security ---
        # At the end of your turn, delete the Digimon this effect played.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnDestroyedAnyone)
        effect2.set_effect_name("P-216 Play a Dark Master from Security")
        effect2.set_effect_description(
            "[On Deletion] You may play 1 face-up [Dark Masters] trait Digimon "
            "card from your security stack without paying the cost. At the end "
            "of your turn, delete the Digimon this effect played."
        )
        effect2.is_optional = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def _security_filter(c):
            """Dark Masters trait Digimon, face-up in security."""
            if not getattr(c, 'is_digimon', False):
                return False
            if getattr(c, 'is_flipped', False):
                return False
            traits = getattr(c, 'card_traits', []) or []
            return any('Dark Masters' in t for t in traits)

        def process2(ctx: Dict[str, Any]):
            """Select and play a Dark Masters Digimon from security, then
            register end-of-owner-turn deletion."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def on_security_selected(selected_card):
                # Remove from security and play onto the field
                if selected_card in player.security_cards:
                    player.security_cards.remove(selected_card)
                played_perm = player.play_card_from_source(
                    selected_card, pay_cost=False)
                if not played_perm:
                    return
                game.logger.log(
                    f"[Effect] {player.player_name} played "
                    f"{game._card_ref(selected_card)} from security via P-216 On Deletion")

                # Fire On Enter Field effects for the played card
                game.execute_effects(
                    EffectTiming.OnEnterFieldAnyone,
                    {"played_card": selected_card,
                     "played_permanent": played_perm,
                     "event_player": player})
                game._fire_play_observers(played_perm, player)

                # Register end-of-owner-turn deletion
                # C# ref: EffectDuration.UntilOwnerTurnEnd, OnEndTurn timing
                _played = played_perm

                def _eot_delete_condition(eot_ctx):
                    if _played.top_card is None:
                        return False
                    if not player.is_my_turn:
                        return False
                    return _played in player.battle_area

                def _eot_delete_process(eot_ctx):
                    if _played in player.battle_area:
                        player.delete_permanent(_played)

                eot_effect = ICardEffect()
                eot_effect.set_timing(EffectTiming.OnEndTurn)
                eot_effect.set_effect_name("P-216 Delete played Digimon at owner turn end")
                eot_effect.set_can_use_condition(_eot_delete_condition)
                eot_effect.set_on_process_callback(_eot_delete_process)
                played_perm.grant_temp_effect(eot_effect)

            game.effect_select_own_security(
                player, _security_filter, on_security_selected,
                is_optional=True,
                prompt="Select 1 [Dark Masters] trait Digimon from security to play.")

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: Inherited Blocker ---
        effect3 = ICardEffect()
        effect3.set_effect_name("P-216 Blocker")
        effect3.set_effect_description("Blocker")
        effect3.is_inherited_effect = True
        effect3._is_blocker = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
