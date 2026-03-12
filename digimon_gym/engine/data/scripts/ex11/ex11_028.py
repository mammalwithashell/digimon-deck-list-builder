from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# Own-field selection indices: 100-113, opponent field: 114-127
_SEL_MY_FIELD_START = 100
_SEL_OPP_FIELD_START = 114


class EX11_028(CardScript):
    """EX11-028 Galemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _make_suspend_effect(*, is_on_play=False, is_when_digivolving=False, name_tag):
            """
            [On Play] / [When Digivolving] You may suspend 1 Digimon (own or opponent).
            """
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            eff.is_optional = True
            if is_on_play:
                eff.is_on_play = True
            elif is_when_digivolving:
                eff.is_when_digivolving = True
            eff.set_effect_name(f"EX11-028 [{name_tag}] You may suspend 1 Digimon")
            eff.set_effect_description(
                f"[{name_tag}] You may suspend 1 Digimon."
            )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            eff.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                """Suspend any 1 Digimon (own or opponent), optionally."""
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                from ....data.enums import GamePhase

                opponent = player.enemy
                own_area = player.battle_area
                opp_area = opponent.battle_area if opponent else []

                valid = []
                for i, p in enumerate(own_area):
                    if p.is_digimon:
                        valid.append(_SEL_MY_FIELD_START + i)
                for i, p in enumerate(opp_area):
                    if p.is_digimon:
                        if game.modifiers.can_be_selected_by_effect(p):
                            valid.append(_SEL_OPP_FIELD_START + i)

                if not valid:
                    return

                def on_select(action_id: int):
                    target_perm = None
                    if _SEL_MY_FIELD_START <= action_id < _SEL_MY_FIELD_START + len(own_area):
                        idx = action_id - _SEL_MY_FIELD_START
                        if 0 <= idx < len(own_area):
                            target_perm = own_area[idx]
                    elif _SEL_OPP_FIELD_START <= action_id < _SEL_OPP_FIELD_START + len(opp_area):
                        idx = action_id - _SEL_OPP_FIELD_START
                        if 0 <= idx < len(opp_area):
                            target_perm = opp_area[idx]
                    if target_perm:
                        target_perm.suspend()

                game.request_selection(
                    GamePhase.SelectTarget, player, on_select, valid,
                    is_optional=True,
                    prompt="You may suspend 1 Digimon (own or opponent)."
                )

            eff.set_on_process_callback(process)
            return eff

        # Effect 0: [On Play] You may suspend 1 Digimon.
        effect0 = _make_suspend_effect(is_on_play=True, name_tag="On Play")
        effects.append(effect0)

        # Effect 1: [When Digivolving] You may suspend 1 Digimon.
        effect1 = _make_suspend_effect(is_when_digivolving=True, name_tag="When Digivolving")
        effects.append(effect1)

        # Effect 2: [All Turns] [Once Per Turn] When any of your Digimon suspend,
        # if you have 1 or fewer Tamers, you may play 1 [Shoto Kazama] from your hand
        # without paying the cost.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name(
            "EX11-028 [All Turns][OPT] Your Digimon suspends + <=1 Tamer -> Play Shoto Kazama"
        )
        effect2.set_effect_description(
            "[All Turns] [Once Per Turn] When any of your Digimon suspend, if you have "
            "1 or fewer Tamers, you may play 1 [Shoto Kazama] from your hand without "
            "paying the cost."
        )
        effect2.is_optional = True
        effect2.set_max_count_per_turn(1)
        effect2.set_hash_string("EX11_028_AT_SuspendShoto")

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The suspended permanent must be owned by this player
            player_owner = card.owner if card else None
            if player_owner is None:
                return False
            event_perm = context.get('event_permanent')
            if event_perm is None:
                return False
            # Must be your own Digimon that suspended
            if event_perm not in player_owner.battle_area:
                return False
            if not event_perm.is_digimon:
                return False
            # Must have 1 or fewer Tamers
            tamer_count = sum(1 for p in player_owner.battle_area if p.is_tamer)
            if tamer_count > 1:
                return False
            # Must have a Shoto Kazama in hand to play
            has_shoto = any(
                any('Shoto Kazama' in n for n in getattr(c, 'card_names', []))
                for c in player_owner.hand_cards
            )
            if not has_shoto:
                return False
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Action: Play 1 [Shoto Kazama] from hand without paying the cost."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def shoto_filter(c):
                return any('Shoto Kazama' in n for n in getattr(c, 'card_names', []))

            game.effect_play_from_zone(
                player, 'hand', shoto_filter, free=True, is_optional=True)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # Effect 3 (Inherited): [Your Turn] [Once Per Turn] When this Digimon wins a battle,
        # gain 1 memory.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEndBattle)
        effect3.set_effect_name("EX11-028 Inherited: Win battle -> Gain 1 memory")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon wins a battle, gain 1 memory."
        )
        effect3.is_inherited_effect = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_028_ESS_WinBattle_Mem")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Gain 1 memory"""
            player = ctx.get('player')
            if player:
                player.add_memory(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
