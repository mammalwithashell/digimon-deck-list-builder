from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_009(CardScript):
    """AD1-009 BlitzGreymon | Lv.6 Red/Purple Digimon | Cyborg, ADVENTURE

    <Alliance> <Piercing> <Blocker>
    [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
    Then, until your opponent's turn ends, their Digimon's effects don't affect
    this Digimon and 1 of your Digimon with [Garurumon] in its name.
    [End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S]
    in the hand. Then, 1 of your Digimon may attack.
    Inherited: <Security A. +1>

    Alt-Digi: Lv.5 w/[Greymon] in name or w/[ADVENTURE] trait: Cost 3
    Standard evo: Red Lv.5 for cost 4
    Assembly: MetalGreymon: Alterous Mode + Greymon, reduce cost 4 (BLOCKED)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        from ....interfaces.modifiers import ModifierType

        effects = []

        # ─── Alt-Digi Effect A: Lv.5 w/[Greymon] in name for cost 3 ────
        # The C# uses permanentCondition: ContainsCardName("Greymon") OR EqualsTraits("ADVENTURE")
        # Since the validator checks _alt_digi_name AND _alt_digi_trait as AND conditions,
        # we need TWO separate alt-digi effects for the OR logic.
        effect_alt_a = ICardEffect()
        effect_alt_a.set_effect_name("AD1-009 Alt-digi: Lv.5 w/Greymon in name")
        effect_alt_a.set_effect_description("Alternate digivolution: Lv.5 w/[Greymon] in name for cost 3")
        effect_alt_a._alt_digi_cost = 3
        effect_alt_a._alt_digi_level = 5
        effect_alt_a._alt_digi_name = "Greymon"

        def condition_alt_a(context: Dict[str, Any]) -> bool:
            return True
        effect_alt_a.set_can_use_condition(condition_alt_a)
        effects.append(effect_alt_a)

        # ─── Alt-Digi Effect B: Lv.5 w/[ADVENTURE] trait for cost 3 ────
        effect_alt_b = ICardEffect()
        effect_alt_b.set_effect_name("AD1-009 Alt-digi: Lv.5 w/ADVENTURE trait")
        effect_alt_b.set_effect_description("Alternate digivolution: Lv.5 w/[ADVENTURE] trait for cost 3")
        effect_alt_b._alt_digi_cost = 3
        effect_alt_b._alt_digi_level = 5
        effect_alt_b._alt_digi_trait = "ADVENTURE"

        def condition_alt_b(context: Dict[str, Any]) -> bool:
            return True
        effect_alt_b.set_can_use_condition(condition_alt_b)
        effects.append(effect_alt_b)

        # ─── Alliance ───────────────────────────────────────────────────
        effect_alliance = ICardEffect()
        effect_alliance.set_effect_name("AD1-009 Alliance")
        effect_alliance.set_effect_description("Alliance")
        effect_alliance.is_on_attack = True
        effect_alliance._is_alliance = True

        def condition_alliance(context: Dict[str, Any]) -> bool:
            return True
        effect_alliance.set_can_use_condition(condition_alliance)
        effects.append(effect_alliance)

        # ─── Piercing ───────────────────────────────────────────────────
        effect_piercing = ICardEffect()
        effect_piercing.set_effect_name("AD1-009 Piercing")
        effect_piercing.set_effect_description("Piercing")
        effect_piercing._is_piercing = True

        def condition_piercing(context: Dict[str, Any]) -> bool:
            return True
        effect_piercing.set_can_use_condition(condition_piercing)
        effects.append(effect_piercing)

        # ─── Blocker ────────────────────────────────────────────────────
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("AD1-009 Blocker")
        effect_blocker.set_effect_description("Blocker")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # ─── Shared OP/WD Process ───────────────────────────────────────
        # De-Digivolve 3 opponent Digimon, then immunity for self + Garurumon
        def _shared_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card() if card else None
            if not perm:
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            # Step 1: De-Digivolve 3 one opponent Digimon
            def on_de_digivolve(target_perm):
                removed = target_perm.de_digivolve(3)
                for c in removed:
                    enemy.trash_cards.append(c)
                _after_de_digivolve()

            opp_digimon = [p for p in enemy.battle_area if p.is_digimon]
            if opp_digimon:
                game.effect_select_opponent_permanent(
                    player, on_de_digivolve,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=False,
                    prompt="Select 1 of your opponent's Digimon to De-Digivolve 3.")
            else:
                _after_de_digivolve()

            def _after_de_digivolve():
                # Step 2: Select 1 own [Garurumon] for immunity (mandatory if available)
                def is_own_garurumon(p):
                    return (p.is_digimon and
                            p.contains_card_name("Garurumon") and
                            p in player.battle_area)

                own_garurumon = [p for p in player.battle_area if is_own_garurumon(p)]
                if own_garurumon:
                    def on_garurumon_select(target_perm):
                        game.register_modifier(
                            target_perm, ModifierType.CANNOT_BE_AFFECTED,
                            expiry='end_of_opponent_turn')
                        _grant_self_immunity()

                    game.effect_select_own_permanent(
                        player, on_garurumon_select,
                        filter_fn=is_own_garurumon,
                        is_optional=False,
                        prompt="Select 1 of your Digimon with [Garurumon] in its name for immunity.")
                else:
                    _grant_self_immunity()

            def _grant_self_immunity():
                # Step 3: Grant THIS permanent immunity (unconditionally)
                current_perm = card.permanent_of_this_card() if card else None
                if current_perm and current_perm in player.battle_area:
                    game.register_modifier(
                        current_perm, ModifierType.CANNOT_BE_AFFECTED,
                        expiry='end_of_opponent_turn')

        # ─── [On Play] ─────────────────────────────────────────────────
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("AD1-009 On Play: De-Digivolve 3 + immunity")
        effect_op.set_effect_description(
            "[On Play] <De-Digivolve 3> 1 of your opponent's Digimon. Then, until "
            "your opponent's turn ends, their Digimon's effects don't affect this "
            "Digimon and 1 of your Digimon with [Garurumon] in its name.")
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_shared_process)
        effects.append(effect_op)

        # ─── [When Digivolving] ─────────────────────────────────────────
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("AD1-009 When Digivolving: De-Digivolve 3 + immunity")
        effect_wd.set_effect_description(
            "[When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon. Then, "
            "until your opponent's turn ends, their Digimon's effects don't affect this "
            "Digimon and 1 of your Digimon with [Garurumon] in its name.")
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_shared_process)
        effects.append(effect_wd)

        # ─── [End of Your Turn] ─────────────────────────────────────────
        # 2 Digimon may DNA digivolve into [Omnimon Alter-S] in the hand.
        # Then, 1 of your Digimon may attack.
        effect_eot = ICardEffect()
        effect_eot.set_timing(EffectTiming.OnEndTurn)
        effect_eot.set_effect_name("AD1-009 End of Turn: DNA digivolve + may attack")
        effect_eot.set_effect_description(
            "[End of Your Turn] 2 of your Digimon may DNA digivolve into "
            "[Omnimon Alter-S] in the hand. Then, 1 of your Digimon may attack.")
        effect_eot.is_optional = True

        def condition_eot(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect_eot.set_can_use_condition(condition_eot)

        def process_eot(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: DNA digivolve into [Omnimon Alter-S] from hand (optional)
            def dna_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                c_names = getattr(c, 'card_names', []) or []
                return any('Omnimon Alter-S' in n for n in c_names)

            game.effect_dna_digivolve_from_hand(
                player, dna_filter, is_optional=True,
                prompt="Select [Omnimon Alter-S] from hand for DNA digivolve.")

            # Step 2: 1 of your Digimon may attack
            def _grant_may_attack():
                def own_digi_filter(p):
                    return p.is_digimon

                own_digimon = [p for p in player.battle_area if own_digi_filter(p)]
                if not own_digimon:
                    return

                def on_attack_select(target_perm):
                    if target_perm.is_suspended:
                        target_perm.unsuspend()
                    target_perm.grant_keyword('_is_rush')
                    game.register_modifier(
                        target_perm, ModifierType.MAY_ATTACK,
                        expiry='end_of_turn')

                game.effect_select_own_permanent(
                    player, on_attack_select,
                    filter_fn=own_digi_filter,
                    is_optional=True,
                    prompt="Select 1 of your Digimon that may attack.")

            _grant_may_attack()

        effect_eot.set_on_process_callback(process_eot)
        effects.append(effect_eot)

        # ─── Inherited: Security A. +1 ──────────────────────────────────
        effect_ess = ICardEffect()
        effect_ess.set_effect_name("AD1-009 Inherited: Security A. +1")
        effect_ess.set_effect_description("<Security A. +1>")
        effect_ess._security_attack_modifier = 1
        effect_ess.is_inherited_effect = True

        def condition_ess(context: Dict[str, Any]) -> bool:
            return True
        effect_ess.set_can_use_condition(condition_ess)
        effects.append(effect_ess)

        return effects
