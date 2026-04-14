from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT13_019(CardScript):
    """BT13-019 Gankoomon | Lv.6"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # Factory effect: blocker
        effect0 = ICardEffect()
        effect0.set_effect_name("BT13-019 Blocker")
        effect0.set_effect_description("Blocker")
        effect0._is_blocker = True

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        def _make_play_effect(is_when_digivolving: bool) -> 'ICardEffect':
            """
            [On Play] / [When Digivolving]
            You may play 1 Digimon card with [Sistermon] in its name from your trash
            OR 1 Digimon card with the [Royal Knight] trait from the digivolution cards
            of your Digimon in the breeding area without paying its cost.
            You can't play [Gankoomon] or [Omnimon] with this effect.
            """
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name("BT13-019 Play Sistermon from trash or Royal Knight from breeding sources")
            if is_when_digivolving:
                effect.set_effect_description(
                    "[When Digivolving] You may play 1 Digimon card with [Sistermon] in its name from your trash "
                    "or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your Digimon "
                    "in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon] with this effect.")
                effect.is_when_digivolving = True
            else:
                effect.set_effect_description(
                    "[On Play] You may play 1 Digimon card with [Sistermon] in its name from your trash "
                    "or 1 Digimon card with the [Royal Knight] trait from the digivolution cards of your Digimon "
                    "in the breeding area without paying its cost. You can't play [Gankoomon] or [Omnimon] with this effect.")
                effect.is_on_play = True
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def _is_excluded(c):
                    names = getattr(c, 'card_names', [])
                    return any(n == 'Gankoomon' or n == 'Omnimon' for n in names)

                def sistermon_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    names = getattr(c, 'card_names', [])
                    if not any('Sistermon' in n for n in names):
                        return False
                    return not _is_excluded(c)

                def rk_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    if not any('Royal Knight' in t for t in traits):
                        return False
                    return not _is_excluded(c)

                # Check available sources
                has_sistermon_in_trash = any(sistermon_filter(c) for c in player.trash_cards)

                # Check breeding area for Royal Knight digi sources
                breeding_perm = player.breeding_area
                rk_in_breeding_sources = []
                if breeding_perm is not None and breeding_perm.is_digimon:
                    # Digi sources are all card_sources except the top card (last in list)
                    digi_sources = breeding_perm.card_sources[:-1] if len(breeding_perm.card_sources) > 1 else []
                    rk_in_breeding_sources = [cs for cs in digi_sources if rk_filter(cs)]
                has_rk_in_breeding = len(rk_in_breeding_sources) > 0

                if not (has_sistermon_in_trash or has_rk_in_breeding):
                    return

                def play_sistermon_from_trash():
                    game.effect_play_from_zone(
                        player, 'trash', sistermon_filter, free=True, is_optional=True,
                        prompt="Play 1 Sistermon from trash without paying its cost.")

                def play_rk_from_breeding_sources():
                    # Use Pattern 11: select a digi source card, remove from stack, play it
                    # Re-fetch breeding perm and sources at execution time
                    bp = player.breeding_area
                    if bp is None or not bp.is_digimon:
                        return
                    digi_srcs = bp.card_sources[:-1] if len(bp.card_sources) > 1 else []
                    valid_srcs = [cs for cs in digi_srcs if rk_filter(cs)]
                    if not valid_srcs:
                        return

                    # Select the card to play (greedy: pick first valid, or let engine choose)
                    # Since there's no standard "select from digi-sources" helper, we manually
                    # offer a branch per valid card using effect_choose_branch if multiple exist,
                    # or just play the only one automatically.
                    def do_play(cs):
                        bp2 = player.breeding_area
                        if bp2 is None:
                            return
                        if cs in bp2.card_sources:
                            bp2.card_sources.remove(cs)
                        played = player.play_card_from_source(cs, pay_cost=False)
                        if played:
                            game.execute_effects(
                                EffectTiming.OnEnterFieldAnyone,
                                {"played_card": cs, "played_permanent": played, "event_player": player})

                    if len(valid_srcs) == 1:
                        do_play(valid_srcs[0])
                    else:
                        # Multiple valid choices: use branch selection
                        branch_labels = [
                            getattr(cs, 'card_name', f"Card {i}") for i, cs in enumerate(valid_srcs)
                        ]

                        def on_branch(branch: int):
                            if 0 <= branch < len(valid_srcs):
                                do_play(valid_srcs[branch])

                        game.effect_choose_branch(
                            player, len(valid_srcs), on_branch,
                            prompt="Select a [Royal Knight] card from breeding sources to play.",
                            branch_labels=branch_labels)

                if has_sistermon_in_trash and has_rk_in_breeding:
                    # Player chooses which source to play from
                    def on_branch(branch: int):
                        if branch == 0:
                            play_sistermon_from_trash()
                        else:
                            play_rk_from_breeding_sources()

                    game.effect_choose_branch(
                        player, 2, on_branch,
                        prompt="Play Sistermon from trash or Royal Knight from breeding sources?",
                        branch_labels=["Sistermon from trash", "Royal Knight from breeding sources"])
                elif has_sistermon_in_trash:
                    play_sistermon_from_trash()
                else:
                    play_rk_from_breeding_sources()

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_play_effect(is_when_digivolving=False))
        effects.append(_make_play_effect(is_when_digivolving=True))

        return effects
