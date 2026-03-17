from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT24_059(CardScript):
    """BT24-059 Sharkmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # C# PermanentCondition: IsLevel4 && (HasTSTraits || HasAquaTraits)
        # HasAquaTraits = ContainsTraits("Aqua") || ContainsTraits("Sea Animal")
        # Pipe format not supported by validator — use separate effects
        effect0a = ICardEffect()
        effect0a.set_effect_name("BT24-059 Alternate digivolution requirement (TS)")
        effect0a.set_effect_description("Alternate digivolution requirement: Lv.4 with [TS] trait: Cost 3")
        effect0a._alt_digi_cost = 3
        effect0a._alt_digi_level = 4
        effect0a._alt_digi_trait = "TS"

        def condition0a(context: Dict[str, Any]) -> bool:
            return True
        effect0a.set_can_use_condition(condition0a)
        effects.append(effect0a)

        effect0b = ICardEffect()
        effect0b.set_effect_name("BT24-059 Alternate digivolution requirement (Aqua)")
        effect0b.set_effect_description("Alternate digivolution requirement: Lv.4 with [Aqua] trait: Cost 3")
        effect0b._alt_digi_cost = 3
        effect0b._alt_digi_level = 4
        effect0b._alt_digi_trait = "Aqua"

        def condition0b(context: Dict[str, Any]) -> bool:
            return True
        effect0b.set_can_use_condition(condition0b)
        effects.append(effect0b)

        effect0c = ICardEffect()
        effect0c.set_effect_name("BT24-059 Alternate digivolution requirement (Sea Animal)")
        effect0c.set_effect_description("Alternate digivolution requirement: Lv.4 with [Sea Animal] trait: Cost 3")
        effect0c._alt_digi_cost = 3
        effect0c._alt_digi_level = 4
        effect0c._alt_digi_trait = "Sea Animal"

        def condition0c(context: Dict[str, Any]) -> bool:
            return True
        effect0c.set_can_use_condition(condition0c)
        effects.append(effect0c)

        # Shared On Play / When Digivolving: <De-Digivolve 1> an opponent's Digimon
        def _shared_de_digi_process(ctx: Dict[str, Any]):
            """<De-Digivolve 1> of your opponent's Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(1)
                enemy = player.enemy if player else None
                if enemy:
                    enemy.trash_cards.extend(removed)
            game.effect_select_opponent_permanent(
                player, on_de_digivolve,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [On Play] <De-Digivolve 1> of your opponent's Digimon.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("BT24-059 De Digivolve")
        effect1.set_effect_description("[On Play] <De-Digivolve 1> of your opponent's Digimon.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_shared_de_digi_process)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] <De-Digivolve 1> of your opponent's Digimon.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("BT24-059 De Digivolve")
        effect2.set_effect_description("[When Digivolving] <De-Digivolve 1> of your opponent's Digimon.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_shared_de_digi_process)
        effects.append(effect2)

        # Timing: EffectTiming.OnDestroyedAnyone
        # [On Deletion] Reveal the top 3 cards of your deck. You may play 1 cost 7 or lower [TS] trait card suspended among them without paying the cost. Trash the rest.
        # C#: SimplifiedRevealDeckTopCardsAndSelect(revealCount:3, ..., remainingCardsPlace: Trash, isTapped: true)
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDestroyedAnyone)
        effect3.set_effect_name("BT24-059 Reveal 3, maybe play 7 cost [TS] trait, trash rest.")
        effect3.set_effect_description("[On Deletion] Reveal the top 3 cards of your deck. You may play 1 cost 7 or lower [TS] trait card suspended among them without paying the cost. Trash the rest.")
        effect3.is_on_deletion = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Reveal top 3 from deck; play 1 cost≤7 [TS] card (suspended) for free; trash the rest."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not (getattr(c, 'is_digimon', False) or getattr(c, 'is_tamer', False)):
                    return False
                cost = getattr(c, 'get_cost_itself', None)
                if cost is None:
                    cost = getattr(c, 'play_cost', 0)
                if cost > 7:
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('TS' in t for t in traits)

            def on_revealed(selected, remaining):
                # Play selected card suspended (isTapped: true per C#)
                if selected is not None:
                    played = player.play_card_from_source(selected, pay_cost=False, is_suspended=True)
                    if played:
                        game.execute_effects(
                            EffectTiming.OnEnterFieldAnyone,
                            {"played_card": selected, "played_permanent": played,
                             "event_player": player},
                        )
                # Remaining cards go to trash
                for c in remaining:
                    player.trash_cards.append(c)

            game.effect_reveal_and_select(
                player, 3, play_filter, on_revealed, is_optional=True,
                prompt="Select 1 play cost 7 or lower [TS] trait card to play.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Timing: EffectTiming.OnUseAttack (ESS / inherited)
        # [When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it (this Digimon) unsuspends.
        # C#: select another own Digimon permanent (not self), place its top card as bottom digi-source, then unsuspend self if it was placed.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnUseAttack)
        effect4.set_effect_name("BT24-059 Place 1 of your other Digimon as this Digimon's bottom digivolution card to unsuspend this Digimon.")
        effect4.set_effect_description("[When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this Digimon's bottom digivolution card, it unsuspends.")
        effect4.is_inherited_effect = True
        effect4.is_optional = True
        effect4.set_max_count_per_turn(1)
        effect4.set_hash_string("BT24_059_Inherited")
        effect4.is_on_attack = True

        def condition4(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            my_perm = card.permanent_of_this_card()
            player = card.owner if card else None
            if not player:
                return False
            # Need at least one other own Digimon
            return any(
                p is not my_perm and p.is_digimon
                for p in player.battle_area
            )

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Select another own Digimon, place ALL its cards as bottom digi-sources, unsuspend self.
            C#: IPlacePermanentToDigivolutionCards places the entire permanent into digivolution cards."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            def target_filter(p):
                return p is not perm and p.is_digimon

            def on_selected(target_perm):
                if target_perm is None:
                    return
                # C#: IPlacePermanentToDigivolutionCards — place ALL card_sources
                # of the sacrificed permanent as bottom digi-sources of self
                all_cards = list(target_perm.card_sources)
                if not all_cards:
                    return
                # Remove the other permanent from field
                player.remove_permanent(target_perm)
                # Insert all cards at bottom of self's card_sources
                for i, cs in enumerate(all_cards):
                    perm.card_sources.insert(i, cs)
                # Unsuspend self if any card was placed
                # C#: checks if placedCard (top card) is in DigivolutionCards
                if all_cards[-1] in perm.card_sources:
                    perm.unsuspend()

            game.effect_select_own_permanent(
                player, on_selected,
                filter_fn=target_filter,
                is_optional=False)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
