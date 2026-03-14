from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_040(CardScript):
    """EX11-040 Mulemon | Lv.4"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: alt_digivolve_req
        # Alternate digivolution requirement
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        # Alternate digivolution: from [Maquinamon] for cost 2
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Maquinamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Maquinamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # Effect
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-040 Effect")
        effect1.set_effect_description("Effect")
        effect1.is_on_play = True

        effect = effect1  # alias for condition closure
        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered on play — validated by engine timing
            return True

        effect1.set_can_use_condition(condition1)

        def _link_maquinamon(ctx: Dict[str, Any]):
            """Link 1 [Maquinamon] from hand or this Digimon's digi cards to 1 of your Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def maquinamon_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Maquinamon' in n for n in names)
            hand_candidates = [c for c in player.hand_cards if maquinamon_filter(c)]
            digi_candidates = []
            if perm:
                for cs in list(perm.card_sources):
                    if cs is perm.top_card:
                        continue
                    if maquinamon_filter(cs):
                        digi_candidates.append(cs)
            if hand_candidates:
                def on_hand_selected(selected_card):
                    if selected_card and selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                        game.effect_link_to_permanent(player, selected_card, is_optional=True)
                game.effect_select_hand_card(player, maquinamon_filter, on_hand_selected, is_optional=True)
            elif digi_candidates:
                cs = digi_candidates[0]
                if perm and cs in perm.card_sources:
                    perm.card_sources.remove(cs)
                game.effect_link_to_permanent(player, cs, is_optional=True)

        effect1.set_on_process_callback(_link_maquinamon)
        effects.append(effect1)

        # Timing: EffectTiming.OnEnterFieldAnyone
        # [When Digivolving] Link 1 [Maquinamon] from hand or digi cards to 1 of your Digimon
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-040 Link 1 [Maquinamon] from hand or digi cards")
        effect2.set_effect_description("[When Digivolving] You may link 1 [Maquinamon] from your hand or this Digimon's digivolution cards to 1 of your Digimon without paying the cost.")
        effect2.is_when_digivolving = True

        effect = effect2  # alias for condition closure
        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Triggered when digivolving — validated by engine timing
            return True

        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_link_maquinamon)
        effects.append(effect2)

        # Timing: EffectTiming.WhenLinked
        # Play Card
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenLinked)
        effect3.set_effect_name("EX11-040 Play 1 [Unchained] from hand or trash")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon gets linked, if you "
            "have 1 or fewer Tamers, you may play 1 [Unchained] from your hand "
            "or trash without paying the cost.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_040_YT")

        effect = effect3  # alias for condition closure
        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            # Your Turn only
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            # 1 or fewer Tamers
            owner = card.owner
            tamer_count = sum(1 for p in owner.battle_area if getattr(p, 'is_tamer', False))
            if tamer_count > 1:
                return False
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Action: Play 1 [Unchained] from hand or trash without paying the cost."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return
            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any('Unchained' in n for n in names)
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # Factory effect: reboot
        # Reboot
        effect4 = ICardEffect()
        effect4.set_effect_name("EX11-040 Reboot")
        effect4.set_effect_description("Reboot")
        effect4.is_inherited_effect = True
        effect4._is_reboot = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)
        effects.append(effect4)

        return effects
