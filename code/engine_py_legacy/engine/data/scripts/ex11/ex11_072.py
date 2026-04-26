from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX11_072(CardScript):
    """EX11-072 Unique Emblem: Guardian Vortex | Option, Green, Vortex Warriors/LIBERATOR, Cost 3

    [Main] You may play 1 [Pteromon], [Muchomon] or [Shoto Kazama] from your hand or trash
        without paying the cost. Then, place this card in the battle area.
    [Your Turn] When any of your [Shoto Kazama]s suspend, <Delay> — 1 of your Digimon with
        [Avian] or [Bird] in any of its traits or the [Vortex Warriors] trait may digivolve
        into a Digimon card with the [Bird Dragon] and [LIBERATOR] trait in the hand with the
        digivolution cost reduced by 3.
    [Security] Activate this card's [Main] effects.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- [Main] Play 1 Pteromon/Muchomon/Shoto Kazama from hand or trash for free.
        #     Then, place this card in the battle area.
        #     Note: "_is_delay" on the factory effect causes _option_stays_on_field to return True,
        #     so the engine keeps this card in the battle area after the main effect resolves.
        effect_main = ICardEffect()
        effect_main.set_timing(EffectTiming.OptionSkill)
        effect_main.set_effect_name(
            "EX11-072 [Main] Play 1 [Pteromon]/[Muchomon]/[Shoto Kazama] from hand or trash, "
            "then place in battle area"
        )
        effect_main.set_effect_description(
            "[Main] You may play 1 [Pteromon], [Muchomon] or [Shoto Kazama] from your hand "
            "or trash without paying the cost. Then, place this card in the battle area."
        )

        def cond_main(context: Dict[str, Any]) -> bool:
            return True
        effect_main.set_can_use_condition(cond_main)

        def process_main(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                names = getattr(c, 'card_names', []) or []
                return any(
                    'Pteromon' in n or 'Muchomon' in n or 'Shoto Kazama' in n
                    for n in names
                )

            game.effect_play_from_zone(
                player, 'hand_or_trash', play_filter, free=True, is_optional=True,
                prompt="You may play 1 [Pteromon], [Muchomon], or [Shoto Kazama] from hand or trash for free.",
            )
            # Placement in battle area is handled by _option_stays_on_field via _is_delay.

        effect_main.set_on_process_callback(process_main)
        effects.append(effect_main)

        # --- Delay marker: keeps this option card in the battle area after resolution ---
        effect_delay = ICardEffect()
        effect_delay.set_effect_name("EX11-072 Delay")
        effect_delay.set_effect_description("Delay")
        effect_delay._is_delay = True

        def cond_delay(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_delay.set_can_use_condition(cond_delay)
        effects.append(effect_delay)

        # --- [Your Turn] When any of your [Shoto Kazama]s suspend, <Delay> — digivolve effect ---
        effect_trig = ICardEffect()
        effect_trig.set_timing(EffectTiming.OnTappedAnyone)
        effect_trig.set_effect_name(
            "EX11-072 [Your Turn] When your Shoto Kazama suspends, digivolve Avian/Bird/VW into Bird Dragon+LIBERATOR"
        )
        effect_trig.set_effect_description(
            "[Your Turn] When any of your [Shoto Kazama]s suspend, <Delay> — "
            "1 of your Digimon with [Avian] or [Bird] in any of its traits or the "
            "[Vortex Warriors] trait may digivolve into a Digimon card with the "
            "[Bird Dragon] and [LIBERATOR] trait in the hand with the digivolution cost reduced by 3."
        )
        effect_trig.is_optional = True
        effect_trig.is_linked_effect = True

        def cond_trig(context: Dict[str, Any]) -> bool:
            # This card must be in the battle area (delay is active)
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be the owner's turn
            owner = card.owner if card else None
            if not (owner and owner.is_my_turn):
                return False
            # The triggering permanent must be one of our Shoto Kazama permanents
            suspended_perm = context.get('permanent')
            if not suspended_perm:
                return False
            if suspended_perm not in list(owner.battle_area):
                return False
            if not suspended_perm.contains_card_name('Shoto Kazama'):
                return False
            return True

        effect_trig.set_can_use_condition(cond_trig)

        def process_trig(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Select 1 of your Digimon with Avian/Bird in traits or Vortex Warriors trait
            def source_filter(p):
                if not p.is_digimon or not p.top_card:
                    return False
                traits = getattr(p.top_card, 'card_traits', []) or []
                return (
                    any('Avian' in t or 'Bird' in t for t in traits)
                    or any('Vortex Warriors' in t for t in traits)
                )

            # Step 2: Digivolve that Digimon into a Bird Dragon + LIBERATOR card, cost reduced by 3
            def on_source_selected(source_perm):
                def digi_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    traits = getattr(c, 'card_traits', []) or []
                    has_bird_dragon = any('Bird Dragon' in t for t in traits)
                    has_liberator = any('LIBERATOR' in t for t in traits)
                    return has_bird_dragon and has_liberator

                game.effect_digivolve_from_hand(
                    player, source_perm, digi_filter,
                    cost_reduction=3, is_optional=True,
                    prompt="Select a Bird Dragon + LIBERATOR Digimon card to digivolve into (cost -3).",
                )

            game.effect_select_own_permanent(
                player, on_source_selected,
                filter_fn=source_filter,
                is_optional=True,
                prompt="Select 1 of your Avian/Bird or Vortex Warriors Digimon to digivolve.",
            )

        effect_trig.set_on_process_callback(process_trig)
        effects.append(effect_trig)

        # --- [Security] Activate this card's [Main] effects ---
        effect_sec = ICardEffect()
        effect_sec.set_effect_name("EX11-072 Security: Play this card")
        effect_sec.set_effect_description("Security: Activate this card's [Main] effects.")
        effect_sec.is_security_effect = True

        def cond_sec(context: Dict[str, Any]) -> bool:
            return True
        effect_sec.set_can_use_condition(cond_sec)
        effects.append(effect_sec)

        return effects
