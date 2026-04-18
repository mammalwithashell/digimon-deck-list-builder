from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT22_099(CardScript):
    """BT22-099 Kuremi Detective Agency

    While you have a [CS] trait Digimon or Tamer on the field, you can ignore
    this card's color requirements.
    [Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait card among
    them to the hand. Return the rest to the bottom of the deck. Then, place
    this card in the battle area.
    [Main] <Delay> (By trashing this card after the placing turn, activate the
    effect below.)
      - Gain 2 memory.
    [Security] Place this card in the battle area.

    C# ref: DCGO/Assets/Scripts/CardEffect/BT22/Black/BT22_099.cs
      - None timing: IgnoreColorConditionClass (CS Digimon/Tamer on owner field)
      - OptionSkill: SimplifiedRevealDeckTopCardsAndSelect(3, CS filter)
                     + PlaceDelayOptionCards(card)
      - OnDeclaration: CardEffectFactory.Gain2MemoryOptionDelayEffect(card)
      - SecuritySkill: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Dynamic color bypass ---
        # While you have a [CS] trait Digimon or Tamer on the field, you can
        # ignore this card's color requirements.
        # Engine pattern: _match_color_requirement_fn callable (returns False
        # to bypass the color requirement, True to enforce it).
        def _has_cs_on_field():
            owner = card.owner if card else None
            if owner is None:
                return False
            for p in owner.battle_area:
                top = p.top_card
                if top is None:
                    continue
                if not (p.is_digimon or p.is_tamer):
                    continue
                traits = getattr(top, 'card_traits', []) or []
                # DCGO: HasCSTraits — exact trait list membership
                if 'CS' in traits:
                    return True
            return False

        def _check_cs_color_req():
            # Return False to bypass color requirement, True to enforce
            return not _has_cs_on_field()

        card._match_color_requirement_fn = _check_cs_color_req

        # --- Effect 1: [Main] Reveal top 3, add 1 CS to hand, rest to bottom ---
        # Delay option: the engine places this card in the battle area
        # automatically (via _option_stays_on_field → _is_delay marker) when
        # playing an Option from hand with a Delay effect.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name(
            "BT22-099 Reveal top 3, add 1 [CS] card to hand, rest to bottom")
        effect1.set_effect_description(
            "[Main] Reveal the top 3 cards of your deck. Add 1 [CS] trait "
            "card among them to the hand. Return the rest to the bottom of "
            "the deck. Then, place this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            """Reveal top 3, pick 1 [CS] card (optional) to hand, rest to bottom.

            Engine trivia: the option is already on the battle_area by the
            time OptionSkill resolves (via play_card). Because effect2 is a
            Delay marker, _option_stays_on_field returns True and the card
            remains in the battle area after OptionSkill completes.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def is_cs_card(c) -> bool:
                traits = getattr(c, 'card_traits', []) or []
                return 'CS' in traits

            # DCGO: SimplifiedRevealDeckTopCardsAndSelect with 1 pass
            # (AddHand, maxCount=1, CS filter). Optional selection.
            game.effect_reveal_and_select_multi(
                player, 3,
                passes=[(is_cs_card, 'hand')],
                remaining_placement='deck_bottom',
                is_optional=True,
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # --- Effect 2: Delay marker ---
        # Makes the engine treat this Option as "stays on field" after
        # OptionSkill resolves, and exposes the Delay activation action
        # (trash self to fire the next effect) in the action mask.
        effect2 = ICardEffect()
        effect2.set_effect_name("BT22-099 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            # Must be on the owner's battle area to be trashed via Delay.
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Main] <Delay> Gain 2 memory ---
        # Activated via Delay (effect 2). When the engine's _execute_delay
        # fires, the permanent has ALREADY been removed from the battle area
        # and its card sources appended to trash. The callback runs AFTER
        # trashing, so condition3 cannot require the perm to still be on
        # field.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3._is_field_main = True
        effect3.set_effect_name("BT22-099 Delay: Gain 2 memory")
        effect3.set_effect_description(
            "[Main] <Delay> (By trashing this card after the placing turn, "
            "activate the effect below.) - Gain 2 memory."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            # Always allow the delayed effect to fire once Delay is triggered
            # (engine's action_mask handles the "not same turn" guard and
            # the "Delay requires perm on field" guard).
            return True

        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            """Gain 2 memory. Engine has already trashed the card."""
            player = ctx.get('player')
            if player is None:
                return
            player.add_memory(2)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Place this card in the battle area ---
        # C# ref: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)
        # When BT22-099 is revealed from security by a security attack, it is
        # placed in its owner's battle area instead of being trashed.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("BT22-099 Security: Place in battle area")
        effect4.set_effect_description("[Security] Place this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True

        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            """Place this card in the owner's battle area (from security).

            The engine's security_attack() will normally append the revealed
            card to trash, but only if card._security_played is False.
            effect_play_from_security sets that flag and creates a Permanent
            in the owner's battle area.
            """
            player = ctx.get('player')
            game = ctx.get('game')
            if player is None or game is None or card is None:
                return
            game.effect_play_from_security(player, card)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
