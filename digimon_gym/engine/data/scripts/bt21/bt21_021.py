from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT21_021(CardScript):
    """BT21-021 OmniShoutmon | Lv.5"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("BT21-021 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Shoutmon] for cost 4
        effect0._alt_digi_cost = 4
        effect0._alt_digi_name = "Shoutmon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Shoutmon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect1 = ICardEffect()
        effect1.set_effect_name("BT21-021 Alternate digivolution requirement")
        effect1.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: Lv.4 with [Xros Heart] or [Hero] trait for cost 3
        effect1._alt_digi_cost = 3
        effect1._alt_digi_level = 4

        def _xros_or_hero_check(base_perm) -> bool:
            traits = getattr(base_perm.top_card, 'card_traits', []) or []
            return any('Xros Heart' in t or 'Hero' in t for t in traits)
        effect1._alt_digi_condition_fn = _xros_or_hero_check

        def condition1(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])) or any('Hero' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # Timing: EffectTiming.None
        # Effect
        effect2 = ICardEffect()
        effect2.set_effect_name("BT21-021 Also treated as [Shoutmon] for a DigiXros")
        effect2.set_effect_description("Effect")

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # Timing: EffectTiming.None
        # Effect
        effect3 = ICardEffect()
        effect3.set_effect_name("BT21-021 Effect")
        effect3.set_effect_description("Effect")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)
        effects.append(effect3)

        # Timing: EffectTiming.OnDestroyedAnyone
        # Effect
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.OnDestroyedAnyone)
        effect4.set_effect_name("BT21-021 Place 1 [Xros]/[Blue Flare] digimon under tamer, then <Save>")
        effect4.set_effect_description("Effect")
        effect4.is_optional = True
        effect4.is_on_deletion = True

        effect = effect4  # alias for condition closure
        def condition4(context: Dict[str, Any]) -> bool:
            # Triggered on deletion — validated by engine timing
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """[On Deletion] Place 1 Xros Heart/Blue Flare Digimon from hand/trash under tamer, then Save."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Find eligible Digimon cards (Xros Heart or Blue Flare trait) in hand/trash
            def _is_xros_blue_flare_digimon(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                traits = getattr(c, 'card_traits', []) or []
                return any('Xros Heart' in t or 'Blue Flare' in t for t in traits)

            # Build combined hand+trash selection
            from ....data.enums import GamePhase
            _SEL_HAND_START = 100
            _SEL_TRASH_START = 130

            valid = []
            for i, c in enumerate(player.hand_cards):
                if _is_xros_blue_flare_digimon(c):
                    valid.append(_SEL_HAND_START + i)
            for i, c in enumerate(player.trash_cards):
                if _is_xros_blue_flare_digimon(c):
                    valid.append(_SEL_TRASH_START + i)

            if not valid:
                return

            def on_card_selected(action_id):
                # Determine which card was selected
                if action_id >= _SEL_TRASH_START:
                    idx = action_id - _SEL_TRASH_START
                    if idx < len(player.trash_cards):
                        selected = player.trash_cards[idx]
                        player.trash_cards.remove(selected)
                    else:
                        return
                elif action_id >= _SEL_HAND_START:
                    idx = action_id - _SEL_HAND_START
                    if idx < len(player.hand_cards):
                        selected = player.hand_cards[idx]
                        player.hand_cards.remove(selected)
                    else:
                        return
                else:
                    return

                # Select a tamer to place the card under
                def tamer_filter(p):
                    return p.is_tamer

                def on_tamer_selected(tamer_perm):
                    tamer_perm.add_card_source_bottom(selected)

                tamers = [p for p in player.battle_area if p.is_tamer]
                if tamers:
                    game.effect_select_own_permanent(
                        player, on_tamer_selected,
                        filter_fn=tamer_filter,
                        is_optional=False,
                        prompt="Select a Tamer to place the Digimon card under."
                    )

            game.request_selection(
                GamePhase.SelectHand, player, on_card_selected,
                valid, is_optional=True,
                prompt="Select 1 [Xros Heart] or [Blue Flare] Digimon from hand or trash to place under a Tamer.")

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        # Timing: EffectTiming.OnEndAttack
        # [End of Attack] You may play 1 card with the [Xros Heart]/[Blue Flare]/[Hero] trait from your hand with the play cost reduced by 5. If you did, delete this Digimon.
        effect5 = ICardEffect()
        effect5.set_timing(EffectTiming.OnEndAttack)
        effect5.set_effect_name("BT21-021 Play 1 [Xros Heart]/[Blue Flare]/[Hero] Digimon")
        effect5.set_effect_description("[End of Attack] You may play 1 card with the [Xros Heart]/[Blue Flare]/[Hero] trait from your hand with the play cost reduced by 5. If you did, delete this Digimon.")
        effect5.cost_reduction = 5

        effect = effect5  # alias for condition closure
        def condition5(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect5.set_can_use_condition(condition5)

        def process5(ctx: Dict[str, Any]):
            """[End of Attack] Play 1 Xros Heart/Blue Flare/Hero from hand cost -5, then delete this Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                # Card text: "play 1 CARD with [Xros Heart]/[Blue Flare]/[Hero]
                # trait" — any card type (Digimon/Tamer/Option) matching the trait.
                traits = getattr(c, 'card_traits', []) or []
                return any('Xros Heart' in t or 'Blue Flare' in t or 'Hero' in t for t in traits)

            def on_played(_played_card, _played_perm):
                # "If you did, delete this Digimon" — delete Gogmamon after
                # successfully playing the card.
                self_perm = card.permanent_of_this_card() if card else None
                if self_perm and player:
                    player.delete_permanent(self_perm, removal_cause='effect')

            # Play from hand with cost reduction of 5
            game.effect_play_from_zone(
                player, 'hand', play_filter, free=False, manual_reduction=5,
                is_optional=True, on_played=on_played)

        effect5.set_on_process_callback(process5)
        effects.append(effect5)

        # Factory effect: rush
        # Rush
        effect6 = ICardEffect()
        effect6.set_effect_name("BT21-021 Rush")
        effect6.set_effect_description("Rush")
        effect6.is_inherited_effect = True
        effect6._is_rush = True

        def condition6(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and permanent.top_card and (any('Xros Heart' in tr for tr in (getattr(permanent.top_card, 'card_traits', []) or [])))):
                return False
            return True
        effect6.set_can_use_condition(condition6)
        effects.append(effect6)

        return effects
