from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase
from ....game.constants import (
    SEL_HAND_START, SEL_OPP_FIELD_START, ACTION_SPACE_SIZE,
)

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

        def _puppet_play_filter(c) -> bool:
            if not getattr(c, 'is_digimon', False):
                return False
            level = getattr(c, 'level', None)
            if level is None or level > 4:
                return False
            return _is_puppet_trait(c)

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

        # ── Helpers ───────────────────────────────────────────────────

        def _play_tokens(player, game):
            """Play 1 [Familiar] Token for each of opponent's Digimon."""
            enemy = player.enemy if player else None
            if not enemy:
                return
            opp_digimon_count = sum(1 for p in enemy.battle_area if p.is_digimon)
            if opp_digimon_count > 0:
                game.effect_play_token(player, 'familiar', count=opp_digimon_count)

        def _dp_reduction_select(player, game):
            """To 1 opponent's Digimon, give -3000 DP * own Digimon count."""
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

        def _play_puppet_then(player, game, after_fn=None):
            """Select and play 1 Puppet Lv.4- from hand, then call after_fn.

            Uses manual request_selection to chain selections properly.
            after_fn is called both when a puppet is played and when the
            player declines (since the play is optional).
            """
            valid = []
            for i, c in enumerate(player.hand_cards):
                if _puppet_play_filter(c) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    if not game._is_play_blocked_by_modifier(c) and not game._is_effect_play_blocked(c):
                        valid.append(SEL_HAND_START + i)

            def _after():
                if after_fn:
                    after_fn()

            if not valid:
                _after()
                return

            def on_puppet_selected(action_id):
                idx = action_id - SEL_HAND_START
                if 0 <= idx < len(player.hand_cards):
                    selected_card = player.hand_cards[idx]
                    played_perm = player.play_card_from_source(
                        selected_card, pay_cost=False)
                    game.logger.log(
                        f"[Effect] {player.player_name} played "
                        f"{game._card_ref(selected_card)} from hand (free)")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {
                            "played_card": selected_card,
                            "played_permanent": played_perm,
                            "event_player": player,
                        },
                    )
                    game._fire_play_observers(played_perm, player)
                # Chain: play tokens (and further effects) after puppet resolves
                _after()

            game.request_selection(
                GamePhase.SelectTarget, player, on_puppet_selected, valid,
                is_optional=True,
                prompt="You may play 1 level 4 or lower [Puppet] Digimon from hand.",
                on_decline=_after)

        # --- Effect 2: [On Play] Play Puppet + Tokens ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-024 Play Puppet + Familiar Tokens")
        effect2.set_effect_description(
            "[On Play] Play 1 level 4 or lower [Puppet] Digimon from hand. "
            "Play Familiar Tokens.")
        effect2.is_on_play = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _play_puppet_then(
                player, game,
                after_fn=lambda: _play_tokens(player, game))
        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [When Digivolving] Play Puppet + Tokens + DP Reduction ---
        # Combined WD effect: puppet play → tokens → DP reduction
        # Must be a single effect to avoid pending_selection overwrite.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect3.set_effect_name(
            "EX11-024 Play Puppet + Familiar Tokens + DP Reduction")
        effect3.set_effect_description(
            "[When Digivolving] Play 1 level 4 or lower [Puppet] Digimon from "
            "hand. Play Familiar Tokens. Then give -3000 DP per your Digimon.")
        effect3.is_when_digivolving = True

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def _after_puppet():
                # Step 2: play tokens
                _play_tokens(player, game)
                # Step 3: DP reduction
                _dp_reduction_select(player, game)

            # Step 1: play puppet from hand (optional selection)
            _play_puppet_then(player, game, after_fn=_after_puppet)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [When Attacking] -3000 DP per Digimon ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("EX11-024 -3000 DP per Digimon")
        effect4.set_effect_description(
            "[When Attacking] -3000 DP per your Digimon to 1 opponent's Digimon.")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _dp_reduction_select(player, game)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
