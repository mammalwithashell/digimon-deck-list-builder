from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_024(CardScript):
    """EX11-024 Cendrillmon | Lv.6

    <Alliance>
    <Overclock ([Puppet] Trait)>

    [On Play][When Digivolving] You may play 1 level 4 or lower [Puppet] trait
        Digimon card from your hand without paying the cost. Then, you may play
        1 [Familiar] Token for each of your opponent's Digimon.

    [When Digivolving][When Attacking] To 1 of your opponent's Digimon, give
        -3000 DP for the turn for each of your Digimon.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        # --- Effect 0: Alliance ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-024 Alliance")
        effect0.set_effect_description("Alliance")
        effect0.is_on_attack = True
        effect0._is_alliance = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Effect 1: Overclock ---
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-024 Overclock")
        effect1.set_effect_description("Overclock")
        effect1._is_overclock = True

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Shared: Play Puppet + Familiar Tokens ---
        def _play_puppet_and_tokens(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            # Play 1 level 4 or lower Puppet Digimon from hand without paying cost
            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                level = getattr(c, 'level', None)
                if level is None or level > 4:
                    return False
                return _is_puppet_trait(c)
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=True, is_optional=True,
                prompt="You may play 1 level 4 or lower [Puppet] Digimon from hand.")
            # Then play 1 Familiar Token for each of opponent's Digimon
            enemy = player.enemy if player else None
            if enemy:
                opp_digimon_count = sum(1 for p in enemy.battle_area if p.is_digimon)
                if opp_digimon_count > 0:
                    game.effect_play_token(player, 'familiar', count=opp_digimon_count)

        # --- Effect 2: [On Play] Play Puppet + Tokens ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-024 Play Puppet + Familiar Tokens")
        effect2.set_effect_description("[On Play] Play 1 level 4 or lower [Puppet] Digimon from hand. Play Familiar Tokens.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_play_puppet_and_tokens)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] Play Puppet + Tokens ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name("EX11-024 Play Puppet + Familiar Tokens")
        effect3.set_effect_description("[When Digivolving] Play 1 level 4 or lower [Puppet] Digimon from hand. Play Familiar Tokens.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)
        effect3.set_on_process_callback(_play_puppet_and_tokens)
        effects.append(effect3)

        # --- Shared: -3000 DP per own Digimon ---
        def _minus_dp_per_digimon(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if not opp_digimon:
                return
            own_digimon_count = sum(1 for p in player.battle_area if p.is_digimon)
            dp_change = -3000 * own_digimon_count
            if dp_change == 0:
                return

            def target_filter(p):
                return p.is_digimon

            def on_select(target_perm):
                target_perm.change_dp(dp_change)

            game.effect_select_opponent_permanent(
                player, on_select, filter_fn=target_filter, is_optional=False,
                prompt=f"Select 1 opponent's Digimon to give {dp_change} DP.")

        # --- Effect 4: [When Digivolving] -3000 DP per Digimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect4.set_effect_name("EX11-024 -3000 DP per Digimon")
        effect4.set_effect_description("[When Digivolving] -3000 DP per your Digimon to 1 opponent's Digimon.")
        effect4.is_when_digivolving = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)
        effect4.set_on_process_callback(_minus_dp_per_digimon)
        effects.append(effect4)

        # --- Effect 5: [When Attacking] -3000 DP per Digimon ---
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("EX11-024 -3000 DP per Digimon")
        effect5.set_effect_description("[When Attacking] -3000 DP per your Digimon to 1 opponent's Digimon.")
        effect5.is_on_attack = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect5.set_can_use_condition(condition5)
        effect5.set_on_process_callback(_minus_dp_per_digimon)
        effects.append(effect5)

        return effects
