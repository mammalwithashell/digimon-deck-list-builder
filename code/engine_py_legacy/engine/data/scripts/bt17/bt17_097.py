from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT17_097(CardScript):
    """BT17-097 Return to the Primogenitor | Option (Blue/Green, Cost 2)

    [Main] 1 of your Digimon may digivolve into a level 5 or higher Digimon
        card with the [Free] trait in your hand with the digivolution cost
        reduced by 4. Then, place this card in the battle area.
    [All Turns] When one of your Digimon with the [Free] trait would be
        deleted other than by one of your effects, <Delay> (trash this card
        from battle area): By digivolving that Digimon into a Digimon card
        with [Imperialdramon] in its name in your hand without paying the
        cost, prevent that deletion.
    [Security] You may play 1 Tamer card with [Davis Motomiya] or
        [Ken Ichijoji] in its name from your hand or trash without paying the
        cost. Then, place this card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: [Main] Digivolve into Lv.5+ [Free] from hand, cost -4,
        #     then place this card in battle area ---
        effect0 = ICardEffect()
        effect0.set_timing(EffectTiming.OptionSkill)
        effect0.set_effect_name(
            "BT17-097 Digivolve 1 Digimon into Lv.5+ [Free] from hand, cost -4")
        effect0.set_effect_description(
            "[Main] 1 of your Digimon may digivolve into a level 5 or higher "
            "Digimon card with the [Free] trait in your hand with the "
            "digivolution cost reduced by 4. Then, place this card in the "
            "battle area."
        )

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)

        def process0(ctx: Dict[str, Any]):
            """Select a Digimon on field; digivolve it into Lv.5+ Free from hand.
            Then place this card in the battle area (handled by Delay marker)."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def hand_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if (getattr(c, 'level', None) or 0) < 5:
                    return False
                # [Free] is an attribute (Vaccine, Data, Virus, Free), not a type/trait
                attrs = c.c_entity_base.attribute_eng if c.c_entity_base else []
                return 'Free' in attrs

            def own_filter(p):
                if not p.is_digimon:
                    return False
                return any(hand_filter(c) for c in player.hand_cards)

            def on_select(target_perm):
                game.effect_digivolve_from_hand(
                    player, target_perm, hand_filter,
                    cost_reduction=4, is_optional=True)

            game.effect_select_own_permanent(
                player, on_select, filter_fn=own_filter, is_optional=True)

            # "Then, place this card in the battle area."
            # The _is_delay flag on the delay marker effect causes the engine
            # to keep this option card on the field after resolution
            # (via _option_stays_on_field), so no manual placement needed here.

        effect0.set_on_process_callback(process0)
        effects.append(effect0)

        # --- Effect 1: Delay marker ---
        # The _is_delay flag tells the engine to keep this option card in the
        # battle area after the main/security effect resolves.
        effect1 = ICardEffect()
        effect1.set_effect_name("BT17-097 Delay")
        effect1.set_effect_description("Delay")
        effect1._is_delay = True

        def condition1(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # --- Effect 2: [All Turns] When a [Free]-trait allied Digimon would be
        # deleted (not by own effects), <Delay> trash this card to digivolve it
        # into an Imperialdramon from hand without paying cost, preventing
        # the deletion. ---
        effect2 = ICardEffect()
        effect2.set_timing(EffectTiming.WhenPermanentWouldBeDeleted)
        effect2.set_effect_name(
            "BT17-097 Delay: Digivolve [Free] Digimon into Imperialdramon "
            "to prevent deletion")
        effect2.set_effect_description(
            "[All Turns] When one of your Digimon with the [Free] trait would "
            "be deleted other than by one of your effects, <Delay> — by "
            "digivolving that Digimon into a Digimon card with [Imperialdramon] "
            "in its name in your hand without paying the cost, prevent that "
            "deletion."
        )
        effect2.is_optional = True

        def condition2(context: Dict[str, Any]) -> bool:
            # This card must be in battle area (delay is active).
            if card and card.permanent_of_this_card() is None:
                return False
            # The permanent being deleted must be an allied [Free]-trait Digimon.
            event_perm = context.get('permanent')
            if event_perm is None:
                return False
            if not event_perm.is_digimon:
                return False
            top = getattr(event_perm, 'top_card', None)
            if top is None:
                return False
            # [Free] is an attribute (Vaccine, Data, Virus, Free), not a type/trait
            attrs = top.c_entity_base.attribute_eng if top.c_entity_base else []
            if 'Free' not in attrs:
                return False
            # Must be owned by us (on our battle area).
            owner = card.owner if card else None
            if owner is None:
                return False
            if event_perm not in (owner.battle_area or []):
                return False
            # "other than by one of your effects": the context has 'is_battle'
            # and 'player'. If the deletion is caused by this card's owner's
            # own effects (i.e. owner == the player performing the deletion and
            # it's not a battle), we should not trigger. However the engine
            # context doesn't pass the effect source. We approximate by checking
            # if the player performing the deletion is NOT the owner (opponent
            # effect or battle).
            ctx_player = context.get('player')
            is_battle = context.get('is_battle', False)
            if ctx_player is owner and not is_battle:
                # Deletion by own player's effect — do not trigger.
                return False
            # We need an Imperialdramon card in hand to digivolve into.
            has_imperialdramon = any(
                getattr(c, 'is_digimon', False) and
                c.contains_card_name('Imperialdramon')
                for c in owner.hand_cards
            )
            return has_imperialdramon

        effect2.set_can_use_condition(condition2)

        def process2(ctx: Dict[str, Any]):
            """Trash this delay card (cost); digivolve [Free] Digimon into
            Imperialdramon without paying cost; prevent the deletion."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Cost: trash this option card from the battle area.
            my_perm = card.permanent_of_this_card() if card else None
            if my_perm is not None:
                if my_perm in player.battle_area:
                    player.battle_area.remove(my_perm)
                    if hasattr(game, 'cleanup_modifiers_for_permanent'):
                        game.cleanup_modifiers_for_permanent(my_perm)
                    player.trash_cards.extend(my_perm.card_sources)

            # The target — the [Free]-trait Digimon that would be deleted.
            event_perm = ctx.get('permanent')
            if event_perm is None or event_perm not in player.battle_area:
                return

            def imper_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                return c.contains_card_name('Imperialdramon')

            # Digivolve into Imperialdramon without paying the cost.
            game.effect_digivolve_from_hand(
                player, event_perm, imper_filter,
                cost_override=0, is_optional=False)

            # Prevent the pending deletion: register CANNOT_BE_DESTROYED.
            # After WhenPermanentWouldBeDeleted timing completes, the engine
            # checks game.modifiers.can_be_destroyed() — this modifier will
            # block the deletion. Using 'end_of_turn' expiry so it clears
            # after this turn (the deletion is already prevented by then).
            if event_perm in player.battle_area:
                from ....interfaces.modifiers import ModifierType
                game.register_modifier(
                    event_perm, ModifierType.CANNOT_BE_DESTROYED,
                    condition=lambda perm, c: perm is event_perm,
                    expiry='end_of_turn')

        effect2.set_on_process_callback(process2)
        effects.append(effect2)

        # --- Effect 3: [Security] Play Davis/Ken Tamer from hand or trash,
        #     then place this card in battle area ---
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.SecuritySkill)
        effect3.set_effect_name(
            "BT17-097 Security: Play Davis Motomiya/Ken Ichijoji Tamer "
            "from hand or trash; place this card in battle area")
        effect3.set_effect_description(
            "[Security] You may play 1 Tamer card with [Davis Motomiya] or "
            "[Ken Ichijoji] in its name from your hand or trash without paying "
            "the cost. Then, place this card in the battle area."
        )
        effect3.is_security_effect = True

        def condition3(context: Dict[str, Any]) -> bool:
            return True

        effect3.set_can_use_condition(condition3)

        def _is_davis_or_ken(c):
            """True if c is a Davis Motomiya or Ken Ichijoji Tamer."""
            if not getattr(c, 'is_tamer', False):
                return False
            return (c.contains_card_name('Davis Motomiya') or
                    c.contains_card_name('DavisMotomiya') or
                    c.contains_card_name('Ken Ichijoji') or
                    c.contains_card_name('KenIchijoji'))

        def process3(ctx: Dict[str, Any]):
            """Play Davis/Ken Tamer from hand or trash; place card in battle area."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            hand_ok = any(_is_davis_or_ken(c) for c in player.hand_cards)
            trash_ok = any(_is_davis_or_ken(c) for c in player.trash_cards)

            if hand_ok or trash_ok:
                if hand_ok and trash_ok:
                    zone = 'hand_or_trash'
                elif hand_ok:
                    zone = 'hand'
                else:
                    zone = 'trash'

                game.effect_play_from_zone(
                    player, zone, _is_davis_or_ken,
                    free=True, is_optional=True)

            # "Then, place this card in the battle area."
            # Create a Permanent from this card and add to battle area.
            from ....core.permanent import Permanent
            new_perm = Permanent([card])
            if game:
                new_perm.turn_played = game.turn_count
                new_perm._owner_game = game
            player.battle_area.append(new_perm)

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        return effects
