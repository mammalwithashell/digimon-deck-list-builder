from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX10_069(CardScript):
    """EX10-069 Unique Emblem: Gravel Hearts | Option (Black, Cost 3)

    [Main] You may play 1 [Sunarizamon] or [Close] from your hand or trash
        without paying the cost. Then, place this card in the battle area.
    [Your Turn] When any of your [Close]s suspend, <Delay>
        (By trashing this card after the placing turn, activate the effect below.)
        ・1 of your [Mineral] or [Rock] trait Digimon may digivolve into a Digimon
          card with the [Mineral] trait and [LIBERATOR] trait in the hand with the
          digivolution cost reduced by 3.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        def _play_sunarizamon_or_close(player, game):
            """Shared play logic for [Main] and [Security]."""
            def play_filter(c):
                # Match [Sunarizamon] by name
                names = getattr(c, 'card_names', []) or []
                if any('Sunarizamon' in n for n in names):
                    return True
                # Match [Close] by name (Close is a card name, not a trait)
                if any('Close' in n for n in names):
                    return True
                return False
            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True)

        # --- Effect 0: [Main] ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "EX10-069 Play 1 [Sunarizamon] or [Close] from hand or trash, "
            "then place in battle area"
        )
        effect0.set_effect_description(
            "[Main] You may play 1 [Sunarizamon] or [Close] from your hand or "
            "trash without paying the cost. Then, place this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _play_sunarizamon_or_close(player, game)

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        # Marks this option card to remain in the battle area after use.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX10-069 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [Your Turn] When any of your [Close]s suspend, activate Delay ---
        # Timing: OnTappedAnyone fires when any permanent is suspended.
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.OnTappedAnyone)
        effect2.set_effect_name(
            "EX10-069 When your [Close] suspends, digivolve a Mineral/Rock Digimon "
            "into Mineral+LIBERATOR with cost -3"
        )
        effect2.set_effect_description(
            "[Your Turn] When any of your [Close]s suspend, <Delay> "
            "(By trashing this card after the placing turn, activate the effect below.)\n"
            "・1 of your [Mineral] or [Rock] trait Digimon may digivolve into a Digimon "
            "card with the [Mineral] trait and [LIBERATOR] trait in the hand with the "
            "digivolution cost reduced by 3."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            # This card must be in the battle area (placed after Main effect)
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # The suspended permanent must be one of our [Close] trait Digimon
            event_perm = context.get('event_permanent')
            if not event_perm:
                return False
            if event_perm not in owner.battle_area:
                return False
            if not event_perm.contains_card_name('Close'):
                return False
            return True
        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: trash this delay card from the battle area.
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm is not None:
                player.delete_permanent(my_perm)

            def base_filter(target_perm):
                return (
                    target_perm.is_digimon
                    and (target_perm.has_trait('Rock') or target_perm.has_trait('Mineral'))
                )

            def on_target(target_perm):
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = set(getattr(c, 'card_traits', []) or [])
                    return 'Mineral' in traits and 'LIBERATOR' in traits

                game.effect_digivolve_from_hand(
                    player,
                    target_perm,
                    digi_filter,
                    cost_reduction=3,
                    is_optional=True,
                )

            game.effect_select_own_permanent(
                player,
                on_target,
                filter_fn=base_filter,
                is_optional=True,
                prompt="Select 1 of your [Rock] or [Mineral] Digimon to digivolve.",
            )

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Activate this card's [Main] effects ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("EX10-069 Security: Activate Main effects")
        effect3.set_effect_description(
            "[Security] Activate this card's [Main] effects."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            _play_sunarizamon_or_close(player, game)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
