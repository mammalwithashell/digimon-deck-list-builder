from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT15_038(CardScript):
    """BT15-038 Angewomon | Lv.5

    [Hand] [Counter] <Blast Digivolve>
    [On Play] [When Digivolving] By trashing the top or bottom card of your
        security stack, 1 of your opponent's Digimon gets -6000 DP until the
        end of their turn.
    [All Turns] [Once Per Turn] When a card is removed from your security
        stack, if you have 3 or fewer security cards, Recovery +1 (Deck).
    Inherited: Ace Overflow <-3>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Blast Digivolve
        effect0 = ICardEffect()
        effect0.set_effect_name("BT15-038 Blast Digivolve")
        effect0.set_effect_description("Blast Digivolve")
        effect0.is_counter_effect = True
        effect0._is_blast_digivolve = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # [On Play] [When Digivolving] By trashing top/bottom security,
        # opponent Digimon gets -6000 DP until end of their turn
        def _build_dp_effect(is_on_play=False, is_when_digivolving=False):
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.is_on_play = is_on_play
            effect.is_when_digivolving = is_when_digivolving
            effect.is_optional = True
            effect.set_effect_name("BT15-038 Trash security, opponent -6000 DP")
            effect.set_effect_description(
                "By trashing the top or bottom card of your security stack, "
                "1 of your opponent's Digimon gets -6000 DP until the end of their turn."
            )

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                player = card.owner if card else None
                if not player or not player.security_cards:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                if not player.security_cards:
                    return

                from ....interfaces.modifiers import ModifierType

                def on_choice(choice):
                    if not player.security_cards:
                        return
                    if choice == 0:
                        trashed = player.security_cards.pop(0)
                    else:
                        trashed = player.security_cards.pop(-1)
                    player.trash_cards.append(trashed)

                    # Give 1 opponent Digimon -6000 DP until end of their turn.
                    # Card text is mandatory ("1 of your opponent's Digimon gets"),
                    # matching C# `canNoSelect=false`. Cost is paid regardless.
                    def target_filter(p):
                        return p.is_digimon

                    def on_target(target_perm):
                        # CHANGE_DP value_fn arity is (current, target, ctx) -> new.
                        game.register_modifier(
                            target_perm, ModifierType.CHANGE_DP,
                            value_fn=lambda cur, t, c: cur - 6000,
                            expiry='end_of_opponent_turn',
                        )

                    enemy = player.enemy
                    if enemy and any(p.is_digimon for p in enemy.battle_area):
                        game.effect_select_opponent_permanent(
                            player, on_target, filter_fn=target_filter, is_optional=False)

                game.effect_choose_branch(
                    player, 2, on_choice,
                    branch_labels=["Trash top security", "Trash bottom security"])

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_dp_effect(is_on_play=True))
        effects.append(_build_dp_effect(is_when_digivolving=True))

        # [All Turns] [Once Per Turn] When a card is removed from your security,
        # if you have 3 or fewer security cards, Recovery +1
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnLoseSecurity)
        effect3.set_effect_name("BT15-038 Recovery +1 when 3 or fewer security")
        effect3.set_effect_description(
            "[All Turns] [Once Per Turn] When a card is removed from your "
            "security stack, if you have 3 or fewer security cards, Recovery +1 (Deck)."
        )
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("Recovery1_BT15_038")

        def condition3(context: Dict[str, Any]) -> bool:
            # Must be on the field.
            if card and card.permanent_of_this_card() is None:
                return False
            player = card.owner if card else None
            if not player:
                return False
            # Only fire when OUR security is lost, not opponent's.
            # C# `CanTriggerWhenLoseSecurity(hashtable, player => player == card.Owner)`.
            event_player = context.get('event_player')
            if event_player is not None and event_player is not player:
                return False
            # "If you have 3 or fewer security cards"
            if len(player.security_cards) > 3:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.recovery(1)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Inherited: Ace Overflow <-3>
        # Marker effect only — engine handles ACE Overflow via
        # `Player._apply_ace_overflow()` driven by `card.c_entity_base.is_ace`
        # and `card.c_entity_base.ace_overflow_cost`.
        effect4 = ICardEffect()
        effect4.set_effect_name("BT15-038 Ace Overflow -3")
        effect4.set_effect_description("Ace Overflow <-3>")
        effect4.is_inherited_effect = True
        effect4._is_ace_overflow = True
        effect4._overflow_amount = -3

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
