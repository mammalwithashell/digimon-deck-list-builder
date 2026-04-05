from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, GamePhase
from ....game.constants import SEL_HAND_START, ACTION_SPACE_SIZE

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_061(CardScript):
    """EX11-061 Mirai Kinosaki"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: gain_memory_tamer
        # [Start of Main] Gain 1 memory if opponent has Digimon
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnStartMainPhase)
        effect0.set_effect_name("EX11-061 Gain 1 memory")
        effect0.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon in play, gain 1 memory.")

        def condition0(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Action: Gain 1 memory if opponent has Digimon"""
            player = ctx.get('player')
            game = ctx.get('game')
            if player and game:
                opponent = game.opponent_player
                if opponent and any(p.is_digimon for p in opponent.battle_area):
                    game.memory += 1
        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # [Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon,
        # by suspending this Tamer, you may play 1 level 3 [Puppet] trait Digimon card
        # from your hand without paying the cost. At turn end, delete the Digimon
        # this effect played.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX11-061 Suspend tamer, play lvl 3 [Puppet], delete it at end of turns.")
        effect1.set_effect_description("[Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon, by suspending this Tamer, you may play 1 level 3 [Puppet] trait Digimon card from your hand without paying the cost. At turn end, delete the Digimon this effect played.")
        effect1.is_optional = True
        effect1._is_digivolve_observer = True

        def _is_puppet_trait(c) -> bool:
            traits = getattr(c, 'card_traits', []) or []
            return any('Puppet' in t for t in traits)

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # Cost: suspend — cannot activate if already suspended
            tamer_perm = card.permanent_of_this_card()
            if tamer_perm and getattr(tamer_perm, 'is_suspended', False):
                return False
            # Check the digivolved permanent has Puppet trait
            trigger_perm = context.get('digivolved_permanent')
            if not trigger_perm:
                return False
            if trigger_perm.owner != card.owner:
                return False
            if not trigger_perm.is_digimon:
                return False
            if trigger_perm.top_card and _is_puppet_trait(trigger_perm.top_card):
                return True
            return False

        effect1.set_can_use_condition(condition1)

        def _schedule_eot_deletion(played_perm):
            """Attach an OnEndTurn effect to the played Digimon that deletes it."""
            eot_effect = ICardEffect()
            eot_effect.set_timing(EffectTiming.OnEndTurn)
            eot_effect.set_effect_name("EX11-061 Delete played Digimon at turn end")
            eot_effect.set_effect_description("Delete the Digimon played by this effect at end of turn.")
            _perm_ref = played_perm

            def eot_condition(context: Dict[str, Any], p=_perm_ref) -> bool:
                return p in (card.owner.battle_area if card and card.owner else [])
            eot_effect.set_can_use_condition(eot_condition)

            def eot_process(ctx: Dict[str, Any], p=_perm_ref):
                owner = card.owner if card else None
                if owner and p in owner.battle_area:
                    owner.delete_permanent(p)
            eot_effect.set_on_process_callback(eot_process)

            if played_perm.top_card:
                played_perm.top_card._cached_effects = played_perm.top_card._cached_effects or []
                played_perm.top_card._cached_effects.append(eot_effect)

        def process1(ctx: Dict[str, Any]):
            """Action: Suspend this Tamer, play level 3 Puppet from hand free, delete at EOT."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            tamer_perm = card.permanent_of_this_card() if card else None
            if not tamer_perm:
                return
            if tamer_perm.is_suspended:
                return
            tamer_perm.suspend()

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if getattr(c, 'level', None) != 3:
                    return False
                return _is_puppet_trait(c)

            # Build valid hand indices manually so we can hook into the play
            # callback to schedule EOT deletion
            valid = []
            for i, hand_card in enumerate(player.hand_cards):
                if play_filter(hand_card) and (SEL_HAND_START + i) < ACTION_SPACE_SIZE:
                    if not game._is_play_blocked_by_modifier(hand_card) and not game._is_effect_play_blocked(hand_card):
                        valid.append(SEL_HAND_START + i)
            if not valid:
                return

            def on_select(action_id: int):
                idx = action_id - SEL_HAND_START
                if 0 <= idx < len(player.hand_cards):
                    selected_card = player.hand_cards[idx]
                    played_perm = player.play_card_from_source(selected_card, pay_cost=False)
                    game.logger.log(f"[Effect] {player.player_name} played "
                                    f"{selected_card.c_entity_base.card_name_eng if selected_card.c_entity_base else selected_card.card_id} from hand")
                    game.execute_effects(
                        EffectTiming.OnEnterFieldAnyone,
                        {"played_card": selected_card, "played_permanent": played_perm, "event_player": player},
                    )
                    game._fire_play_observers(played_perm, player)
                    # Schedule end-of-turn deletion for the played Digimon
                    _schedule_eot_deletion(played_perm)

            game.request_selection(GamePhase.SelectTarget, player,
                                   on_select, valid, is_optional=True,
                                   prompt="You may play 1 level 3 [Puppet] trait Digimon card from your hand.")

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # Factory effect: security_play
        # Security: Play this card
        effect3 = ICardEffect()
        effect3.set_effect_name("EX11-061 Security: Play this card")
        effect3.set_effect_description("Security: Play this card")
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        return effects
