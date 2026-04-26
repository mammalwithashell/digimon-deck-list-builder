from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming, CardColor

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_040(CardScript):
    """BT24-040 Venusmon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution: Lv.5 with [TS] trait: Cost 3
        effect0 = ICardEffect()
        effect0.set_effect_name("BT24-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 5
        effect0._alt_digi_trait = "TS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # When this card would be played, if you have 3 or fewer security cards,
        # reduce the play cost by 5.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT24-040 Reduce play cost (5)")
        effect1.set_effect_description(
            "When this card would be played, if you have 3 or fewer security cards, "
            "reduce the play cost by 5."
        )
        effect1.cost_reduction = 5

        def condition1(context: Dict[str, Any]) -> bool:
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            return len(owner.security_cards) <= 3

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # [On Play] [When Digivolving] Trash all digivolution cards of 1 of your
        # opponent's Digimon. Then, until your opponent's turn ends, 2 of their
        # Digimon or Tamers can't suspend or activate [When Digivolving] effects.
        for is_play in [True, False]:
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            eff.set_effect_name("BT24-040 Trash digi cards, 2 opp can't suspend/digivolve")
            eff.set_effect_description(
                "[On Play] [When Digivolving] Trash all digivolution cards of 1 of your "
                "opponent's Digimon. Then, until your opponent's turn ends, 2 of their "
                "Digimon or Tamers can't suspend or activate [When Digivolving] effects."
            )
            if is_play:
                eff.is_on_play = True
            else:
                eff.is_when_digivolving = True

            def make_cond():
                def cond(context: Dict[str, Any]) -> bool:
                    if card and card.permanent_of_this_card() is None:
                        return False
                    return True
                return cond

            eff.set_can_use_condition(make_cond())

            def make_proc():
                def proc(ctx: Dict[str, Any]):
                    player = ctx.get('player')
                    game = ctx.get('game')
                    if not (player and game):
                        return
                    enemy = player.enemy
                    if not enemy:
                        return

                    # Trash all digivolution cards of 1 opponent Digimon
                    def opp_digi_filter(p):
                        return p.is_digimon

                    def on_select_opp(target_perm):
                        if not target_perm:
                            return
                        digi_count = max(0, len(target_perm.card_sources) - 1)
                        if digi_count > 0:
                            trashed = target_perm.trash_digivolution_cards(digi_count)
                            enemy.trash_cards.extend(trashed)
                            game.logger.log(
                                f"[Effect] Trashed {digi_count} digivolution cards from "
                                f"{game._perm_ref(target_perm)}")

                        # Then freeze 2 opponent Digimon/Tamers
                        def freeze_filter(p):
                            return p.is_digimon or p.is_tamer

                        _frozen = []

                        def on_freeze(t):
                            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
                            # CANNOT_SUSPEND scoped to this specific permanent
                            game.register_modifier(
                                t, ModifierType.CANNOT_SUSPEND,
                                condition=lambda target, ctx, fp=t: target is fp,
                                expiry='end_of_opponent_turn')
                            # Disable only [When Digivolving] effects
                            game.register_modifier(
                                t, ModifierType.DISABLE_EFFECT,
                                condition=lambda p, c, fp=t: (
                                    p is fp
                                    and c.get('effect') is not None
                                    and getattr(c['effect'], 'is_when_digivolving', False)
                                ),
                                expiry='end_of_opponent_turn')
                            game.logger.log(
                                f"[Effect] {game._perm_ref(t)} can't suspend or "
                                f"activate [When Digivolving] effects")
                            _frozen.append(t)
                            if len(_frozen) < 2:
                                def freeze_filter2(p):
                                    return (p.is_digimon or p.is_tamer) and p not in _frozen

                                if any(freeze_filter2(p) for p in enemy.battle_area):
                                    game.effect_select_opponent_permanent(
                                        player, on_freeze, filter_fn=freeze_filter2,
                                        is_optional=False,
                                        prompt="Select opponent's Digimon/Tamer that can't "
                                               "suspend (2nd).")

                        if any(freeze_filter(p) for p in enemy.battle_area):
                            game.effect_select_opponent_permanent(
                                player, on_freeze, filter_fn=freeze_filter,
                                is_optional=False,
                                prompt="Select opponent's Digimon/Tamer that can't "
                                       "suspend (1st).")

                    if any(opp_digi_filter(p) for p in enemy.battle_area):
                        game.effect_select_opponent_permanent(
                            player, on_select_opp, filter_fn=opp_digi_filter,
                            is_optional=False,
                            prompt="Select 1 of your opponent's Digimon to trash all "
                                   "digivolution cards.")
                return proc

            eff.set_on_process_callback(make_proc())
            effects.append(eff)

        # [All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave
        # the battle area other than by your effects, by placing 1 other Digimon with
        # no digivolution cards as the bottom security card, they don't leave.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.WhenRemoveField)
        effect5.set_effect_name("BT24-040 Protect TS Digimon by tucking to security")
        effect5.set_effect_description(
            "[All Turns] [Once Per Turn] When any of your [TS] trait Digimon would leave "
            "the battle area other than by your effects, by placing 1 other Digimon with "
            "no digivolution cards as the bottom security card, they don't leave."
        )
        effect5.is_optional = True
        effect5.set_max_count_per_turn(1)
        effect5.set_hash_string("BT24_040_AT_protect")

        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # The leaving permanent must be one of our TS Digimon
            leaving_perm = context.get('event_permanent') or context.get('permanent')
            if not leaving_perm or not leaving_perm.is_digimon:
                return False
            traits = leaving_perm.top_card.card_traits if leaving_perm.top_card else []
            if not any('TS' in t for t in (traits or [])):
                return False
            # Must not be by our own effects
            if context.get('is_own_effect', False):
                return False
            # Must have another Digimon with no digivolution cards to sacrifice
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            my_perm = card.permanent_of_this_card()
            for p in owner.battle_area:
                if p is leaving_perm or p is my_perm:
                    continue
                if p.is_digimon and len(p.card_sources) <= 1:
                    return True
            return False

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            leaving_perm = ctx.get('event_permanent') or ctx.get('permanent')
            my_perm = card.permanent_of_this_card() if card else None

            def sacrifice_filter(p):
                if p is leaving_perm or p is my_perm:
                    return False
                return p.is_digimon and len(p.card_sources) <= 1

            def on_sacrifice(target):
                # Place as bottom security card
                player.battle_area.remove(target)
                for cs in target.card_sources:
                    player.security_cards.append(cs)
                game.logger.log(
                    f"[Effect] Placed {game._perm_ref(target)} as bottom security card "
                    f"to protect {game._perm_ref(leaving_perm)}")
                # Prevent the leaving
                if leaving_perm:
                    leaving_perm.will_be_remove_field = False
                    if hasattr(leaving_perm, 'willBeRemoveField'):
                        leaving_perm.willBeRemoveField = False

            game.effect_select_own_permanent(
                player, on_sacrifice, filter_fn=sacrifice_filter,
                is_optional=False,
                prompt="Select a Digimon with no digivolution cards to place as "
                       "bottom security card.")

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        return effects
