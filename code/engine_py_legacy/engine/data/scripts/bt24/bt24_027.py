from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_027(CardScript):
    """BT24-027 Lanamon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: from [Calmaramon] for cost 0
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-027 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 0
        effect0._alt_digi_name = "Calmaramon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Calmaramon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.3 with [TS] trait for cost 2
        effect1 = ICardEffect()
        effect1.set_effect_name("BT24-027 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        effect1._alt_digi_cost = 2
        effect1._alt_digi_level = 3
        effect1._alt_digi_trait = "TS"

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Factory effect: decode
        # Decode ([Calmaramon])
        effect2 = ICardEffect()
        effect2.set_effect_name("BT24-027 Decode")
        effect2.set_effect_description("Decode")
        effect2._is_decode = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # [On Play] [When Digivolving] By placing 1 level 4 or lower blue [TS] trait
        # Digimon card from your hand as this Digimon's bottom digivolution card,
        # 1 of your blue [TS] trait Digimon can't be deleted in battle until your
        # opponent's turn ends.
        for is_play in [True, False]:
            effect_n = ICardEffect()
            effect_n.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect_n.set_effect_name("BT24-027 By tucking, 1 blue TS Digimon can't be deleted by battle")
            effect_n.set_effect_description(
                "[On Play] [When Digivolving] By placing 1 level 4 or lower blue [TS] trait "
                "Digimon card from your hand as this Digimon's bottom digivolution card, "
                "1 of your blue [TS] trait Digimon can't be deleted in battle until your "
                "opponent's turn ends."
            )
            if is_play:
                effect_n.is_on_play = True
            else:
                effect_n.is_when_digivolving = True
            effect_n.is_optional = True

            def make_condition(is_play_flag):
                def condition_n(context: Dict[str, Any]) -> bool:
                    perm = card.permanent_of_this_card() if card else None
                    if perm is None:
                        return False
                    player = perm.owner
                    if not player:
                        return False
                    # Check hand has valid tuck target
                    for c in player.hand_cards:
                        if (getattr(c, 'is_digimon', False)
                                and (c.level or 99) <= 4
                                and CardColor.Blue in (getattr(c, 'card_colors', []) or [])
                                and any('TS' in t for t in (c.card_traits or []))):
                            return True
                    return False
                return condition_n

            effect_n.set_can_use_condition(make_condition(is_play))

            def make_process(is_play_flag):
                def process_n(ctx: Dict[str, Any]):
                    player = ctx.get('player')
                    game = ctx.get('game')
                    if not (player and game):
                        return
                    perm = card.permanent_of_this_card() if card else None
                    if not perm:
                        return

                    def tuck_filter(c):
                        return (getattr(c, 'is_digimon', False)
                                and (c.level or 99) <= 4
                                and CardColor.Blue in (getattr(c, 'card_colors', []) or [])
                                and any('TS' in t for t in (c.card_traits or [])))

                    def on_tuck(selected):
                        if selected in player.hand_cards:
                            player.hand_cards.remove(selected)
                            perm.add_card_source_bottom(selected)
                            game.logger.log(
                                f"[Effect] {player.player_name} placed "
                                f"{game._card_ref(selected)} under "
                                f"{game._perm_ref(perm)} as bottom digivolution card")

                            # Grant can't be deleted by battle to 1 blue TS Digimon
                            def target_filter(p):
                                return (p.is_digimon
                                        and CardColor.Blue in (getattr(p.top_card, 'card_colors', []) or [])
                                        and any('TS' in t for t in (p.top_card.card_traits or [])))

                            def on_grant(target_perm):
                                target_perm.grant_keyword('_is_cannot_be_deleted_by_battle',
                                                          duration=game.turn_count + 1)

                            game.effect_select_own_permanent(
                                player, on_grant, filter_fn=target_filter,
                                is_optional=False,
                                prompt="Select 1 of your blue [TS] trait Digimon to grant "
                                       "can't be deleted in battle.")

                    game.effect_select_hand_card(
                        player, tuck_filter, on_tuck, is_optional=True,
                        prompt="Place 1 lv.4 or lower blue [TS] Digimon from hand as "
                               "bottom digivolution card.")

                return process_n

            effect_n.set_on_process_callback(make_process(is_play))
            effects.append(effect_n)

        # Inherited: [When Attacking][Once Per Turn] If you have 7 or fewer cards
        # in your hand, <Draw 1>.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnUseAttack)
        effect5.set_effect_name("BT24-027 Draw 1 card")
        effect5.set_effect_description(
            "[When Attacking][Once Per Turn] If you have 7 or fewer cards in your hand, "
            "<Draw 1>."
        )
        effect5.is_inherited_effect = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("Draw_BT24_027")
        effect5.is_on_attack = True

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            perm = card.permanent_of_this_card()
            if perm and perm.owner:
                return len(perm.owner.hand_cards) <= 7
            return False

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
