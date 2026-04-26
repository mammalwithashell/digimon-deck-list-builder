from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_012(CardScript):
    """AD1-012 CresGarurumon | Lv.6 Blue/Purple Digimon | 12000 DP | Play Cost 12

    <Alliance>
    <Evade> (When this Digimon would be deleted, you may suspend it to prevent that deletion.)
    [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may return 1 of your
      opponent's lowest level Digimon to the hand. Then, this Digimon and 1 of your Digimon
      with [Greymon] in its name may unsuspend.
    [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, 2 of your
      Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, you may change the
      attack target to 1 of your Digimon.

    Inherited: [Your Turn] This Digimon's attack target can't change.

    Alt-Digi: Lv.5 w/[Garurumon] in name or w/[ADVENTURE] trait: Cost 3
    Standard Evo: Blue Lv.5, cost 4
    Assembly: WereGarurumon: Sagittarius Mode + Garurumon, reduce cost 4 (BLOCKED)
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-Digi: Lv.5 w/[Garurumon] in name, cost 3 ──────────────
        # C# uses ContainsCardName("Garurumon") OR EqualsTraits("ADVENTURE")
        # The validator checks name AND trait as sequential filters on each effect,
        # so we need TWO separate alt-digi effects for the OR condition.

        effect_alt1 = ICardEffect()
        effect_alt1.set_effect_name("AD1-012 Alt digi: Lv.5 [Garurumon] name, cost 3")
        effect_alt1.set_effect_description("Alternate digivolution: Lv.5 with [Garurumon] in name, cost 3")
        effect_alt1._alt_digi_cost = 3
        effect_alt1._alt_digi_level = 5
        effect_alt1._alt_digi_name = "Garurumon"

        def condition_alt1(context: Dict[str, Any]) -> bool:
            return True
        effect_alt1.set_can_use_condition(condition_alt1)
        effects.append(effect_alt1)

        # Alt-Digi: Lv.5 w/[ADVENTURE] trait, cost 3
        effect_alt2 = ICardEffect()
        effect_alt2.set_effect_name("AD1-012 Alt digi: Lv.5 [ADVENTURE] trait, cost 3")
        effect_alt2.set_effect_description("Alternate digivolution: Lv.5 with [ADVENTURE] trait, cost 3")
        effect_alt2._alt_digi_cost = 3
        effect_alt2._alt_digi_level = 5
        effect_alt2._alt_digi_trait = "ADVENTURE"

        def condition_alt2(context: Dict[str, Any]) -> bool:
            return True
        effect_alt2.set_can_use_condition(condition_alt2)
        effects.append(effect_alt2)

        # ── Alliance ────────────────────────────────────────────────────
        eff_alliance = ICardEffect()
        eff_alliance._is_alliance = True
        eff_alliance.set_effect_name("AD1-012 Alliance")

        def cond_alliance(ctx: Dict[str, Any]) -> bool:
            return True
        eff_alliance.set_can_use_condition(cond_alliance)
        effects.append(eff_alliance)

        # ── Evade ──────────────────────────────────────────────────────
        eff_evade = ICardEffect()
        eff_evade._is_evade = True
        eff_evade.set_effect_name("AD1-012 Evade")

        def cond_evade(ctx: Dict[str, Any]) -> bool:
            return True
        eff_evade.set_can_use_condition(cond_evade)
        effects.append(eff_evade)

        # ── Shared OP / WD / WA: Bounce lowest level + unsuspend ──────
        # C# hash: "AD1_012_OP_WD_WA", OPT (once per turn shared)
        # Step 1: Optional - bounce 1 opponent's lowest level Digimon to hand
        # Step 2: Yes/No - unsuspend self + select 1 own suspended [Greymon] to unsuspend

        SHARED_HASH = "AD1_012_OP_WD_WA"

        def _shared_process(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            perm = card.permanent_of_this_card()
            enemy = player.enemy
            if not enemy:
                return

            # Step 1: Bounce opponent's lowest level Digimon (optional)
            opp_digimon = [p for p in enemy.battle_area
                           if p.is_digimon and p.level is not None]
            if opp_digimon:
                min_level = min(p.level for p in opp_digimon)

                def opp_filter(p):
                    return (p.is_digimon and p.level is not None
                            and p.level == min_level)

                def on_bounce(target_perm):
                    enemy.bounce_permanent_to_hand(target_perm)
                    # After bounce, proceed to unsuspend step
                    _do_unsuspend_step(player, game, perm)

                def on_skip_bounce():
                    # Player declined to bounce, still proceed to unsuspend
                    _do_unsuspend_step(player, game, perm)

                game.effect_select_opponent_permanent(
                    player, on_bounce, filter_fn=opp_filter, is_optional=True)
            else:
                # No opponent Digimon to bounce, proceed to unsuspend
                _do_unsuspend_step(player, game, perm)

        def _do_unsuspend_step(player, game, perm):
            """Step 2: 'this Digimon and 1 of your [Greymon] may unsuspend'.

            The overall effect is already optional (is_optional=True), so the
            player can decline the entire effect. Once activated, unsuspend self
            unconditionally and offer Greymon selection.
            """
            if perm and perm in player.battle_area:
                perm.unsuspend()

            greymons = [p for p in player.battle_area
                        if p.is_digimon
                        and p.contains_card_name('Greymon')
                        and p.is_suspended]
            if greymons:
                def greymon_filter(p):
                    return (p.is_digimon
                            and p.contains_card_name('Greymon')
                            and p.is_suspended)

                def on_greymon(target_perm):
                    target_perm.unsuspend()

                game.effect_select_own_permanent(
                    player, on_greymon, filter_fn=greymon_filter,
                    is_optional=True)

        # [On Play] instance
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("AD1-012 Bounce lowest + unsuspend (On Play)")
        effect_op.set_effect_description(
            "[On Play] [Once Per Turn] You may return 1 of your opponent's "
            "lowest level Digimon to the hand. Then, this Digimon and 1 of "
            "your Digimon with [Greymon] in its name may unsuspend."
        )
        effect_op.is_on_play = True
        effect_op.is_optional = True
        effect_op.set_max_count_per_turn(1)
        effect_op.set_hash_string(SHARED_HASH)

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_shared_process)
        effects.append(effect_op)

        # [When Digivolving] instance
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("AD1-012 Bounce lowest + unsuspend (When Digivolving)")
        effect_wd.set_effect_description(
            "[When Digivolving] [Once Per Turn] You may return 1 of your opponent's "
            "lowest level Digimon to the hand. Then, this Digimon and 1 of "
            "your Digimon with [Greymon] in its name may unsuspend."
        )
        effect_wd.is_when_digivolving = True
        effect_wd.is_optional = True
        effect_wd.set_max_count_per_turn(1)
        effect_wd.set_hash_string(SHARED_HASH)

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_shared_process)
        effects.append(effect_wd)

        # [When Attacking] instance
        effect_wa = ICardEffect()
        effect_wa.set_timing(EffectTiming.OnUseAttack)
        effect_wa.set_effect_name("AD1-012 Bounce lowest + unsuspend (When Attacking)")
        effect_wa.set_effect_description(
            "[When Attacking] [Once Per Turn] You may return 1 of your opponent's "
            "lowest level Digimon to the hand. Then, this Digimon and 1 of "
            "your Digimon with [Greymon] in its name may unsuspend."
        )
        effect_wa.is_on_attack = True
        effect_wa.is_optional = True
        effect_wa.set_max_count_per_turn(1)
        effect_wa.set_hash_string(SHARED_HASH)

        def condition_wa(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wa.set_can_use_condition(condition_wa)
        effect_wa.set_on_process_callback(_shared_process)
        effects.append(effect_wa)

        # ── [Opponent's Turn] [OPT] DNA digivolve + redirect ──────────
        # C# hash: "Redirect_AD1_012"
        # Fires when opponent's Digimon attacks during opponent's turn.
        # Step 1: DNA digivolve 2 own Digimon into [Omnimon Alter-S] from hand (optional)
        # Step 2: Change attack target to 1 own Digimon (optional)
        effect_opp = ICardEffect()
        effect_opp.set_effect_name("AD1-012 DNA into Omnimon Alter-S + redirect")
        effect_opp.set_effect_description(
            "[Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon "
            "attacks, 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in "
            "the hand. Then, you may change the attack target to 1 of your Digimon."
        )
        effect_opp.is_optional = True
        effect_opp.set_max_count_per_turn(1)
        effect_opp.set_hash_string("Redirect_AD1_012")
        effect_opp._is_when_attacked_observer = True

        def condition_opp(context: Dict[str, Any]) -> bool:
            # Must be on field as Digimon
            if card and card.permanent_of_this_card() is None:
                return False
            owner = card.owner
            if not owner:
                return False
            # Only on opponent's turn
            if owner.is_my_turn:
                return False
            return True
        effect_opp.set_can_use_condition(condition_opp)

        def process_opp(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: DNA digivolve into Omnimon Alter-S from hand
            def dna_filter(c):
                return (getattr(c, 'is_digimon', False)
                        and c.contains_card_name('Omnimon Alter-S'))

            game.effect_dna_digivolve_from_hand(
                player, dna_filter, is_optional=True,
                prompt="Select [Omnimon Alter-S] from hand to DNA digivolve.")

            # Step 2: Redirect attack to 1 own Digimon
            def redirect_filter(p):
                return p.is_digimon

            def on_redirect_selected(target_perm):
                game.switch_attack_target(target_perm)

            game.effect_select_own_permanent(
                player, on_redirect_selected, filter_fn=redirect_filter,
                is_optional=True,
                prompt="Select 1 of your Digimon to change the attack target to.")

        effect_opp.set_on_process_callback(process_opp)
        effects.append(effect_opp)

        # ── Inherited: [Your Turn] Attack target can't change ──────────
        # C# uses CanNotSwitchAttackTargetClass with PermanentCondition matching self.
        # In the engine, this should register a CANNOT_SWITCH_ATTACK_TARGET modifier
        # on the permanent. Since inherited effects with NoTiming are declarative/static,
        # we follow the existing BT20-026 pattern.
        effect_inh = ICardEffect()
        effect_inh.set_effect_name("AD1-012 Inherited: Attack target can't change")
        effect_inh.set_effect_description(
            "[Your Turn] This Digimon's attack target can't change."
        )
        effect_inh.is_inherited_effect = True

        def condition_inh(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect_inh.set_can_use_condition(condition_inh)

        def process_inh(ctx: Dict[str, Any]):
            """Register CANNOT_SWITCH_ATTACK_TARGET modifier on self."""
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card()
            if not (player and game and perm):
                return
            from engine_py_legacy.engine.interfaces.modifiers import ModifierType
            game.register_modifier(
                perm,
                ModifierType.CANNOT_SWITCH_ATTACK_TARGET,
                condition=lambda target, ctx_: target is perm,
                value_fn=lambda: True,
                expiry='permanent',
            )
        effect_inh.set_on_process_callback(process_inh)
        effects.append(effect_inh)

        return effects
