from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX6_074(CardScript):
    """EX6-074 Mirei Mikagura | Tamer Purple/Yellow | Cost 4

    [Your Turn] When one of your Digimon with the [Holy Beast], [Archangel] or
        [Fallen Angel] trait is played, by suspending this Tamer, gain 1
        memory. Then, 1 of your Digimon may digivolve into [Angewomon] or
        [LadyDevimon] in your trash with the cost reduced by 1.
    [End of Your Turn] [Once Per Turn] 2 of your Digimon may DNA digivolve into
        a Digimon card in your hand.
    [Security] Play this card without paying the cost.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Clause 1: [Your Turn] OnPlayed trait trigger ──────────────────
        # [Your Turn] When one of your Digimon with the [Holy Beast],
        # [Archangel] or [Fallen Angel] trait is played, by suspending this
        # Tamer, gain 1 memory. Then, 1 of your Digimon may digivolve into
        # [Angewomon] or [LadyDevimon] in your trash with the cost reduced
        # by 1.
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect0.set_effect_name(
            "EX6-074 Memory +1, Digivolve from trash for 1 cost reduction"
        )
        effect0.set_effect_description(
            "[Your Turn] When one of your Digimon with the [Holy Beast], "
            "[Archangel] or [Fallen Angel] trait is played, by suspending "
            "this Tamer, gain 1 memory. Then, 1 of your Digimon may "
            "digivolve into [Angewomon] or [LadyDevimon] in your trash with "
            "the cost reduced by 1."
        )
        effect0.is_optional = True

        def condition0(context: Dict[str, Any]) -> bool:
            # Must be on the field
            if card is None or card.permanent_of_this_card() is None:
                return False
            # [Your Turn] only
            owner = card.owner
            if not (owner and owner.is_my_turn):
                return False
            # Tamer must be unsuspended (suspend is the cost)
            own_perm = card.permanent_of_this_card()
            if own_perm is None or own_perm.is_suspended:
                return False
            # The played event must belong to the tamer's owner (own Digimon)
            if context.get('event_player') is not owner:
                return False
            # Must be a Digimon being played
            played_card = context.get('played_card')
            if played_card is None:
                return False
            if not getattr(played_card, 'is_digimon', False):
                return False
            # Must have Holy Beast / Archangel / Fallen Angel trait
            traits = getattr(played_card, 'card_traits', []) or []
            for trait in traits:
                if not trait:
                    continue
                if ('Holy Beast' in trait
                        or 'Archangel' in trait
                        or 'Fallen Angel' in trait):
                    return True
            return False

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Cost: suspend this Tamer. Gain 1 memory. Then, 1 of your
            Digimon may digivolve into Angewomon/LadyDevimon in trash with
            cost reduced by 1 (player picks both the target and the card)."""
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and perm and game):
                return

            # Pay the cost: suspend this Tamer (condition already checked
            # that the Tamer is unsuspended).
            perm.suspend()

            # Gain 1 memory.
            player.add_memory(1)

            # Filter for Angewomon / LadyDevimon cards (by name) in the
            # player's trash.
            def digi_card_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any(
                    ('Angewomon' in (n or '') or 'LadyDevimon' in (n or ''))
                    for n in names
                )

            # Only offer the digivolve if there is a valid card in trash
            # (short-circuit avoids a pointless selection prompt).
            if not any(digi_card_filter(c) for c in player.trash_cards):
                return

            # Target filter: any of the player's own Digimon (player chooses
            # which one to digivolve — never auto-select).
            def target_filter(p):
                return getattr(p, 'is_digimon', False)

            if not any(target_filter(p) for p in player.battle_area):
                return

            def on_target(target_perm):
                if target_perm is None:
                    return
                game.effect_digivolve_from_hand(
                    player,
                    target_perm,
                    digi_card_filter,
                    cost_reduction=1,
                    is_optional=True,
                    include_trash=True,
                    prompt=(
                        "Select a card from your trash to digivolve your "
                        "Digimon into (cost reduced by 1)."
                    ),
                )

            game.effect_select_own_permanent(
                player,
                on_target,
                filter_fn=target_filter,
                is_optional=True,
                prompt=(
                    "Select 1 of your Digimon to digivolve into "
                    "[Angewomon]/[LadyDevimon] from trash."
                ),
            )

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # ── Clause 2: [End of Your Turn] [OPT] DNA digivolve ──────────────
        # [End of Your Turn] [Once Per Turn] 2 of your Digimon may DNA
        # digivolve into a Digimon card in your hand.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEndTurn)
        effect1.set_effect_name("EX6-074 DNA digivolve into hand card")
        effect1.set_effect_description(
            "[End of Your Turn] [Once Per Turn] 2 of your Digimon may DNA "
            "digivolve into a Digimon card in your hand."
        )
        effect1.is_optional = True
        effect1.set_max_count_per_turn(1)
        effect1.set_hash_string("DNA_EX6_074")

        def condition1(context: Dict[str, Any]) -> bool:
            # Must be on the field
            if card is None or card.permanent_of_this_card() is None:
                return False
            # [End of Your Turn] implies owner's turn
            owner = card.owner
            if not (owner and owner.is_my_turn):
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Trigger a DNA digivolve from hand (optional).

            The engine helper `effect_dna_digivolve_from_hand` prompts the
            agent to pick a hand card that has dna_costs, then to pick the
            two field material Digimon.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            game.effect_dna_digivolve_from_hand(
                player,
                lambda c: getattr(c, 'is_digimon', False),
                is_optional=True,
                prompt="Select a Digimon card from hand to DNA digivolve into.",
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Clause 3: [Security] Play this card without paying the cost ──
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.SecuritySkill)
        effect2.set_effect_name("EX6-074 Security: Play this card")
        effect2.set_effect_description(
            "[Security] Play this card without paying the cost."
        )
        effect2.is_security_effect = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Play this Tamer from security without paying the cost."""
            player = ctx.get('player')
            if player and card:
                player.play_card_from_source(card, pay_cost=False)

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        return effects
