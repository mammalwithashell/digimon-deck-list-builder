from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class AD1_011(CardScript):
    """AD1-011 Paildramon | Lv.5 Blue/Green Digimon | DP 8000 | Play Cost 8

    <Partition (Blue Lv.4 & Green Lv.4)>
    [When Digivolving] Until your opponent's turn ends, this Digimon can't be
      deleted in battle. Then, if DNA digivolving, this Digimon's attack target
      can't change for the turn.
    [When Attacking] This Digimon may digivolve into a Digimon card with
      [Imperialdramon] in its name in the hand with the digivolution cost
      reduced by 2.

    Inherited Effect:
      <Partition (Blue Lv.4 & Green Lv.4)>

    Alt-Digi: Lv.4 w/[Free] attribute or [Hero] trait: Cost 3
    DNA: Blue Lv.4 + Green Lv.4
    Standard Evo: Blue Lv.4, cost 4
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alt-Digi 1: Lv.4 with [Free] attribute, cost 3 ─────────────
        # C# uses EqualsTraits("Free") — in the engine data, "Free" is in
        # attribute_eng (not type_eng/card_traits), so we need a condition_fn.
        effect_alt_free = ICardEffect()
        effect_alt_free.set_effect_name("AD1-011 Alt-digi: Free attribute")
        effect_alt_free.set_effect_description("Lv.4 w/[Free] attribute: Cost 3")
        effect_alt_free._alt_digi_cost = 3
        effect_alt_free._alt_digi_level = 4
        effect_alt_free._alt_digi_condition_fn = lambda perm: (
            'Free' in (getattr(perm.top_card.c_entity_base, 'attribute_eng', []) or [])
            if perm.top_card and perm.top_card.c_entity_base else False
        )

        def cond_alt_free(ctx: Dict[str, Any]) -> bool:
            return True
        effect_alt_free.set_can_use_condition(cond_alt_free)
        effects.append(effect_alt_free)

        # ── Alt-Digi 2: Lv.4 with [Hero] trait, cost 3 ─────────────────
        # "Hero" is in type_eng/card_traits, so _alt_digi_trait works.
        effect_alt_hero = ICardEffect()
        effect_alt_hero.set_effect_name("AD1-011 Alt-digi: Hero trait")
        effect_alt_hero.set_effect_description("Lv.4 w/[Hero] trait: Cost 3")
        effect_alt_hero._alt_digi_cost = 3
        effect_alt_hero._alt_digi_level = 4
        effect_alt_hero._alt_digi_trait = "Hero"

        def cond_alt_hero(ctx: Dict[str, Any]) -> bool:
            return True
        effect_alt_hero.set_can_use_condition(cond_alt_hero)
        effects.append(effect_alt_hero)

        # ── Partition (non-inherited) ───────────────────────────────────
        effect_partition = ICardEffect()
        effect_partition.set_effect_name("AD1-011 Partition")
        effect_partition.set_effect_description(
            "Partition (Blue Lv.4 & Green Lv.4)")
        effect_partition._is_partition = True

        def cond_partition(ctx: Dict[str, Any]) -> bool:
            return True
        effect_partition.set_can_use_condition(cond_partition)
        effects.append(effect_partition)

        # ── Partition (inherited) ───────────────────────────────────────
        effect_partition_inh = ICardEffect()
        effect_partition_inh.set_effect_name("AD1-011 Partition (inherited)")
        effect_partition_inh.set_effect_description(
            "Partition (Blue Lv.4 & Green Lv.4)")
        effect_partition_inh.is_inherited_effect = True
        effect_partition_inh._is_partition = True

        def cond_partition_inh(ctx: Dict[str, Any]) -> bool:
            return True
        effect_partition_inh.set_can_use_condition(cond_partition_inh)
        effects.append(effect_partition_inh)

        # ── [When Digivolving] ──────────────────────────────────────────
        # Until your opponent's turn ends, this Digimon can't be deleted
        # in battle. Then, if DNA digivolving, this Digimon's attack target
        # can't change for the turn.
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name(
            "AD1-011 WD: Can't be deleted in battle + DNA target lock")
        effect_wd.set_effect_description(
            "[When Digivolving] Until your opponent's turn ends, this Digimon "
            "can't be deleted in battle. Then, if DNA digivolving, this "
            "Digimon's attack target can't change for the turn."
        )
        effect_wd.is_when_digivolving = True

        def cond_wd(ctx: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(cond_wd)

        def process_wd(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            from digimon_gym.engine.interfaces.modifiers import ModifierType

            # Step 1: Can't be deleted in battle until opponent's turn ends
            game.register_modifier(
                perm, ModifierType.CANNOT_BE_DESTROYED_BY_BATTLE,
                condition=lambda target, mod_ctx: target is perm,
                value_fn=lambda: True,
                expiry='end_of_opponent_turn',
            )

            # Step 2: If DNA digivolving, attack target can't change for turn
            is_dna = ctx.get('is_dna_digivolve', False)
            if is_dna:
                game.register_modifier(
                    perm, ModifierType.CANNOT_SWITCH_ATTACK_TARGET,
                    condition=lambda target, mod_ctx: target is perm,
                    value_fn=lambda: True,
                    expiry='end_of_turn',
                )

        effect_wd.set_on_process_callback(process_wd)
        effects.append(effect_wd)

        # ── [When Attacking] ────────────────────────────────────────────
        # This Digimon may digivolve into a Digimon card with [Imperialdramon]
        # in its name in the hand with the digivolution cost reduced by 2.
        effect_wa = ICardEffect()
        effect_wa.set_timing(EffectTiming.OnUseAttack)
        effect_wa.set_effect_name("AD1-011 WA: Digivolve into Imperialdramon")
        effect_wa.set_effect_description(
            "[When Attacking] This Digimon may digivolve into a Digimon card "
            "with [Imperialdramon] in its name in the hand with the "
            "digivolution cost reduced by 2."
        )
        effect_wa.is_on_attack = True
        effect_wa.is_optional = True

        def cond_wa(ctx: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wa.set_can_use_condition(cond_wa)

        def process_wa(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            perm = card.permanent_of_this_card() if card else None
            if not (player and game and perm):
                return

            def imperialdramon_filter(c):
                """Filter for Digimon cards with [Imperialdramon] in name."""
                if not getattr(c, 'is_digimon', False):
                    return False
                return c.contains_card_name('Imperialdramon')

            game.effect_digivolve_from_hand(
                player, perm, imperialdramon_filter,
                cost_reduction=2, is_optional=True,
                prompt="Select a Digimon with [Imperialdramon] in its name "
                       "to digivolve into (cost reduced by 2).")

        effect_wa.set_on_process_callback(process_wa)
        effects.append(effect_wa)

        return effects
