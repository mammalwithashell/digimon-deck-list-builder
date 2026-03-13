from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_095(CardScript):
    """BT17-095 Brave Tornado | Option Red (Cost 4)

    [Main] You may play 1 [Agumon]/[Gabumon] from your hand or trash without
        paying the cost. Then, place this card in the battle area.
    [All Turns] When one of your level 6 Digimon with [Greymon]/[Garurumon]
        in its name would leave the battle area outside of a battle, <Delay>.
        That Digimon and a card in the hand may DNA digivolve into a Digimon
        card with [Omnimon] in its name in the hand.
    [Security] You may play 1 card with [Tai Kamiya]/[Matt Ishida] in its
        name from your hand or trash without paying the cost. Then, add this
        card to the hand.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Play [Agumon]/[Gabumon] from hand or trash ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name("BT17-095 Play Agumon/Gabumon from hand or trash")
        effect0.set_effect_description(
            "[Main] You may play 1 [Agumon]/[Gabumon] from your hand or trash "
            "without paying the cost. Then, place this card in the battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def play_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                names = getattr(c, 'card_names', []) or []
                return any(n == 'Agumon' for n in names) or any(n == 'Gabumon' for n in names)

            hand_ok = any(play_filter(c) for c in player.hand_cards)
            trash_ok = any(play_filter(c) for c in player.trash_cards)

            if hand_ok or trash_ok:
                if hand_ok and trash_ok:
                    zone = 'hand_or_trash'
                elif hand_ok:
                    zone = 'hand'
                else:
                    zone = 'trash'
                game.effect_play_from_zone(
                    player, zone, play_filter,
                    free=True, is_optional=True)

            # "Then, place this card in the battle area."
            # The delay marker effect handles this via _is_delay flag.

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        effect1 = ICardEffect()
        effect1.set_effect_name("BT17-095 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] Delay - When Lv.6 Greymon/Garurumon would
        #     leave outside of battle, DNA digivolve into Omnimon ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect2.set_effect_name("BT17-095 Delay: DNA digivolve into Omnimon")
        effect2.set_effect_description(
            "[All Turns] When one of your level 6 Digimon with [Greymon]/"
            "[Garurumon] in its name would leave the battle area outside of "
            "a battle, <Delay>. That Digimon and a card in the hand may DNA "
            "digivolve into a Digimon card with [Omnimon] in its name."
        )
        effect2.is_optional = True
        effect2.set_hash_string("DNAOmnimon_BT17_095")

        def condition2(context: Dict[str, Any]) -> bool:
            # This card must be in battle area (delay active)
            if card and card.permanent_of_this_card() is None:
                return False
            # Must be triggered by non-battle removal
            is_battle = context.get('is_battle', False)
            if is_battle:
                return False
            # The permanent being removed must be our Lv.6 Greymon/Garurumon
            event_perm = context.get('permanent')
            if event_perm is None:
                return False
            if not event_perm.is_digimon:
                return False
            if not (card and card.owner):
                return False
            if event_perm not in card.owner.battle_area:
                return False
            top = event_perm.top_card
            if top is None:
                return False
            level = getattr(top, 'level', None)
            if level != 6:
                return False
            if not (top.contains_card_name('Greymon') or
                    top.contains_card_name('Garurumon')):
                return False
            # Need an Omnimon card in hand with DNA conditions
            has_omnimon = any(
                getattr(c, 'is_digimon', False) and
                c.contains_card_name('Omnimon') and
                getattr(c, 'level', None) == 7 and
                c.c_entity_base and c.c_entity_base.dna_costs
                for c in card.owner.hand_cards
            )
            return has_omnimon

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash this delay card; DNA digivolve the leaving Digimon
            with a hand card into an Omnimon from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: trash this option card from battle area
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm is not None:
                if my_perm in player.battle_area:
                    player.battle_area.remove(my_perm)
                    if hasattr(game, 'cleanup_modifiers_for_permanent'):
                        game.cleanup_modifiers_for_permanent(my_perm)
                    player.trash_cards.extend(my_perm.card_sources)

            # DNA digivolve into Omnimon from hand
            def omnimon_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not c.contains_card_name('Omnimon'):
                    return False
                if getattr(c, 'level', None) != 7:
                    return False
                if not (c.c_entity_base and c.c_entity_base.dna_costs):
                    return False
                return True

            game.effect_dna_digivolve_from_hand(
                player, omnimon_filter, is_optional=True)

            # Prevent the pending deletion
            event_perm = ctx.get('permanent')
            if event_perm and event_perm in player.battle_area:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    event_perm, ModifierType.CANNOT_BE_DESTROYED,
                    condition=lambda perm, c: perm is event_perm,
                    expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play Tai Kamiya/Matt Ishida from hand or trash ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name("BT17-095 Security: Play Tai Kamiya/Matt Ishida")
        effect3.set_effect_description(
            "[Security] You may play 1 card with [Tai Kamiya]/[Matt Ishida] "
            "from your hand or trash without paying the cost. Then, add this "
            "card to the hand."
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

            def tamer_filter(c):
                if not getattr(c, 'is_tamer', False):
                    return False
                return (c.contains_card_name('Tai Kamiya') or
                        c.contains_card_name('TaiKamiya') or
                        c.contains_card_name('Matt Ishida') or
                        c.contains_card_name('MattIshida'))

            hand_ok = any(tamer_filter(c) for c in player.hand_cards)
            trash_ok = any(tamer_filter(c) for c in player.trash_cards)

            if hand_ok or trash_ok:
                if hand_ok and trash_ok:
                    zone = 'hand_or_trash'
                elif hand_ok:
                    zone = 'hand'
                else:
                    zone = 'trash'
                game.effect_play_from_zone(
                    player, zone, tamer_filter,
                    free=True, is_optional=True)

            # "Then, add this card to the hand."
            if card and player:
                # Card may be in security discard, move to hand
                if card in player.trash_cards:
                    player.trash_cards.remove(card)
                player.hand_cards.append(card)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
