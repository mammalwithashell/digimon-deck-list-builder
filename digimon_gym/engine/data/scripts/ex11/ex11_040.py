from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_040(CardScript):
    """EX11-040 Mulemon | Lv.4

    Alt digivolution: from [Maquinamon] for cost 2.

    [On Play] [When Digivolving] You may link 1 [Maquinamon] from your hand
    or this Digimon's digivolution cards to 1 of your Digimon without paying
    the cost.
    [Your Turn] [Once Per Turn] When this Digimon gets linked, if you have 1
    or fewer Tamers, you may play 1 [Unchained] from your hand or trash
    without paying the cost.

    --- Inherited ---
    <Reboot>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _is_maquinamon(c):
            names = getattr(c, 'card_names', []) or []
            return any('Maquinamon' in n for n in names)

        def _link_maquinamon(ctx: Dict[str, Any]):
            """Link 1 [Maquinamon] from hand or this Digimon's digivolution
            cards to 1 of your Digimon."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            hand_candidates = [c for c in player.hand_cards if _is_maquinamon(c)]
            digi_candidates = []
            if perm:
                for cs in list(perm.card_sources):
                    if cs is perm.top_card:
                        continue
                    if _is_maquinamon(cs):
                        digi_candidates.append(cs)

            has_hand = len(hand_candidates) > 0
            has_digi = len(digi_candidates) > 0

            if not has_hand and not has_digi:
                return

            def _do_link_from_hand():
                def on_hand_selected(selected_card):
                    if selected_card and selected_card in player.hand_cards:
                        player.hand_cards.remove(selected_card)
                        game.effect_link_to_permanent(player, selected_card, is_optional=True,
                                                      prompt="Select a Digimon to link Maquinamon to.")

                game.effect_select_hand_card(player, _is_maquinamon, on_hand_selected,
                                             is_optional=True,
                                             prompt="Select a [Maquinamon] from hand to link.")

            def _do_link_from_digi():
                if not perm:
                    return
                from ....game.constants import SEL_EFFECT_CHOICE_START
                from ....data.enums import GamePhase
                valid = []
                for i, cs in enumerate(perm.card_sources):
                    if cs is perm.top_card:
                        continue
                    if _is_maquinamon(cs):
                        valid.append(SEL_EFFECT_CHOICE_START + i)

                if not valid:
                    return

                def on_select_digi(action_id: int):
                    idx = action_id - SEL_EFFECT_CHOICE_START
                    if idx < 0 or idx >= len(perm.card_sources):
                        return
                    cs = perm.card_sources[idx]
                    if cs in perm.card_sources:
                        perm.card_sources.remove(cs)
                    game.effect_link_to_permanent(player, cs, is_optional=True,
                                                  prompt="Select a Digimon to link Maquinamon to.")

                game.request_selection(
                    GamePhase.SelectEffectChoice, player, on_select_digi, valid,
                    is_optional=True,
                    prompt="Select a [Maquinamon] from digivolution cards to link.")

            if has_hand and has_digi:
                def on_zone_choice(branch: int):
                    if branch == 0:
                        _do_link_from_hand()
                    elif branch == 1:
                        _do_link_from_digi()

                game.effect_choose_branch(
                    player, 2, on_zone_choice,
                    prompt="Link [Maquinamon] from hand or digivolution cards?",
                    branch_labels=["From hand", "From digivolution cards"])
            elif has_hand:
                _do_link_from_hand()
            elif has_digi:
                _do_link_from_digi()

        # --- Alt digivolution: from [Maquinamon] for cost 2 ---
        effect0 = ICardEffect()
        effect0.set_effect_name("EX11-040 Alternate digivolution requirement")
        effect0.set_effect_description("Alternate digivolution requirement")
        effect0._alt_digi_cost = 2
        effect0._alt_digi_name = "Maquinamon"

        def condition0(context: Dict[str, Any]) -> bool:
            permanent = card.permanent_of_this_card() if card else None
            if not (permanent and (permanent.contains_card_name('Maquinamon'))):
                return False
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- [On Play]: link 1 [Maquinamon] ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name("EX11-040 On Play: Link 1 [Maquinamon]")
        effect1.set_effect_description(
            "[On Play] You may link 1 [Maquinamon] from your hand or this "
            "Digimon's digivolution cards to 1 of your Digimon without "
            "paying the cost.")
        effect1.is_on_play = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_link_maquinamon)
        effects.append(effect1)

        # --- [When Digivolving]: link 1 [Maquinamon] ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect2.set_effect_name("EX11-040 When Digivolving: Link 1 [Maquinamon]")
        effect2.set_effect_description(
            "[When Digivolving] You may link 1 [Maquinamon] from your hand "
            "or this Digimon's digivolution cards to 1 of your Digimon "
            "without paying the cost.")
        effect2.is_when_digivolving = True

        def condition2(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect2.set_can_use_condition(condition2)
        effect2.set_on_process_callback(_link_maquinamon)
        effects.append(effect2)

        # --- WhenLinked: play [Unchained] if 1 or fewer Tamers ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.WhenLinked)
        effect3.set_effect_name("EX11-040 Play 1 [Unchained] from hand or trash")
        effect3.set_effect_description(
            "[Your Turn] [Once Per Turn] When this Digimon gets linked, if "
            "you have 1 or fewer Tamers, you may play 1 [Unchained] from "
            "your hand or trash without paying the cost.")
        effect3.is_optional = True
        effect3.set_max_count_per_turn(1)
        effect3.set_hash_string("EX11_040_YT")

        def condition3(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            owner = card.owner
            tamer_count = sum(1 for p in owner.battle_area if getattr(p, 'is_tamer', False))
            if tamer_count > 1:
                return False
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Play 1 [Unchained] from hand or trash without paying the cost."""
            player = ctx.get('player')
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

        # --- Inherited: Reboot ---
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
