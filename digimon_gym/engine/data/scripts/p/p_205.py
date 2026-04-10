from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_205(CardScript):
    """P-205 Insane Synthetic Monster | Option

    While you have a Digimon or Tamer with the [DM] trait on the field,
        you can ignore this card's color requirements.
    [Main] <Draw 2> and trash 2 cards in your hand. Then, place this card
        in the battle area.
    [Main] <Delay>. By deleting 1 of your play cost 7 or lower Digimon,
        you may play 1 Digimon card with [Kimeramon] or [Millenniummon] in
        its name from your trash with the play cost reduced by 3.
    [Security] <Draw 2> and trash 2 cards in your hand. Then, place this
        card in the battle area.
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # --- Effect 0: Ignore color requirements if [DM] trait on field ---
        # DCGO: IgnoreColorConditionClass with HasMatchConditionPermanent
        # checking for (IsTamer || IsDigimon) && HasDMTraits
        # Set card._match_color_requirement_fn so the action mask dynamically
        # checks whether a DM Digimon/Tamer is on the field.
        def _check_color_req() -> bool:
            owner = card.owner if card else None
            if not owner:
                return True  # enforce color requirement if no owner
            for p in owner.battle_area:
                if not (p.is_digimon or p.is_tamer):
                    continue
                if not p.top_card:
                    continue
                traits = getattr(p.top_card, 'card_traits', []) or []
                if any('DM' in tr for tr in traits):
                    return False  # bypass color requirement
            return True  # enforce color requirement
        card._match_color_requirement_fn = _check_color_req

        effect0 = ICardEffect()
        effect0.set_effect_name("P-205 Ignore color requirements")
        effect0.set_effect_description("Ignore Color Req")
        effect0.is_declarative = True

        def condition0(context: Dict[str, Any]) -> bool:
            return not _check_color_req()
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # --- Shared Main/Security draw+trash logic ---
        def _draw_and_trash(ctx: Dict[str, Any]):
            """Draw 2, then trash 2 from hand."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Draw 2
            player.draw_cards(2)

            # Trash 2 from hand
            _trash_count = [0]

            def _trash_one():
                if _trash_count[0] >= 2 or not player.hand_cards:
                    return

                def on_trashed(selected):
                    if selected in player.hand_cards:
                        player.hand_cards.remove(selected)
                        player.trash_cards.append(selected)
                    _trash_count[0] += 1
                    _trash_one()

                game.effect_select_hand_card(
                    player, lambda c: True, on_trashed,
                    is_optional=False,
                    prompt=f"Trash card {_trash_count[0]+1}/2 from your hand.")

            _trash_one()

        # --- Effect 1: [Main] Draw 2, trash 2, place in battle area ---
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-205 Draw 2, Trash 2")
        effect1.set_effect_description(
            "[Main] <Draw 2> and trash 2 cards in your hand. Then, place "
            "this card in the battle area.")

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effect1.set_on_process_callback(_draw_and_trash)
        effects.append(effect1)

        # --- Effect 2: Delay ---
        effect2 = ICardEffect()
        effect2.set_effect_name("P-205 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True
        effect2.is_on_deletion = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # --- Effect 3: [Main] <Delay> Delete own Digimon (cost 7 or less),
        #     play [Kimeramon]/[Millenniummon] from trash with cost -3 ---
        # DCGO: First deletes the delay option card itself, then on success
        # selects 1 of your Digimon with play cost <= 7 to delete, then on
        # success plays from trash with fixedCost = BasePlayCost - 3.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3.set_effect_name(
            "P-205 Delay: delete own Digimon, play Kimeramon/Millenniummon from trash")
        effect3.set_effect_description(
            "[Main] <Delay>. By deleting 1 of your play cost 7 or lower Digimon, "
            "you may play 1 Digimon card with [Kimeramon] or [Millenniummon] in "
            "its name from your trash with the play cost reduced by 3.")

        def condition3(context: Dict[str, Any]) -> bool:
            return True
        effect3.set_can_use_condition(condition3)

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: Select 1 of your Digimon with play cost 7 or lower to delete
            # DCGO: CanSelectPermanentCondition checks IsPermanentExistsOnOwnerBattleAreaDigimon
            # and BasePlayCostFromEntity <= 7
            def own_digi_filter(p):
                if not p.is_digimon or not p.top_card:
                    return False
                cost = p.top_card.get_cost_itself
                return cost is not None and cost <= 7

            own_qualifying = [p for p in player.battle_area if own_digi_filter(p)]
            if not own_qualifying:
                return

            def on_own_deleted(own_perm):
                if own_perm is None:
                    return
                player.delete_permanent(own_perm)

                # Step 2: Play 1 [Kimeramon]/[Millenniummon] from trash with cost -3
                # DCGO: CanSelectCardCondition checks IsDigimon &&
                # (ContainsCardName("Kimeramon") || ContainsCardName("Millenniummon"))
                def kim_filter(c):
                    if not getattr(c, 'is_digimon', False):
                        return False
                    names = getattr(c, 'card_names', []) or []
                    return any('Kimeramon' in n or 'Millenniummon' in n for n in names)

                # DCGO: payCost=true, root=Trash, fixedCost=BasePlayCost-3
                game.effect_play_from_zone(
                    player, 'trash', kim_filter,
                    free=False, manual_reduction=3, is_optional=True,
                    prompt="Play 1 [Kimeramon]/[Millenniummon] from trash (cost -3).")

            game.effect_select_own_permanent(
                player, on_own_deleted,
                filter_fn=own_digi_filter,
                is_optional=True,
                prompt="Delete 1 of your Digimon (play cost 7 or lower).")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # --- Effect 4: [Security] Draw 2, trash 2, place in battle area ---
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("P-205 Security: Draw 2, Trash 2")
        effect4.set_effect_description(
            "[Security] <Draw 2> and trash 2 cards in your hand. Then, place "
            "this card in the battle area.")
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def _security_process(ctx: Dict[str, Any]):
            """Security: Draw 2, trash 2, then place this card in battle area."""
            _draw_and_trash(ctx)
            # Place this card in the battle area (PlaceDelayOptionCards)
            player = ctx.get('player')
            game = ctx.get('game')
            if player and card:
                from ....core.permanent import Permanent
                perm = Permanent([card])
                perm.turn_played = game.turn_count if game else 0
                player.battle_area.append(perm)
                # Prevent the engine from trashing this card after security
                card._security_played = True

        effect4.set_on_process_callback(_security_process)
        effects.append(effect4)

        return effects
