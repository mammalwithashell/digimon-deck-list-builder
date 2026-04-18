from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class BT23_067(CardScript):
    """BT23-067 LadyDevimon | Lv.5

    Effect text:
      When this card would be played from the hand, if you have [Angewomon] or
      [Mirei Mikagura], reduce the play cost by 3.
      <Blocker>
      [On Play] [When Digivolving] Delete 1 of your opponent's level 4 or
      lower Digimon.

    Inherited:
      <Scapegoat>

    Alt-digi: Lv.4 w/[CS] trait for cost 3 (no color requirement).
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alternate digivolution requirement ────────────────────────
        # Lv.4 with [CS] trait for cost 3. NO color restriction in the C#
        # PermanentCondition (matches DCGO's HasLevel && IsLevel4 && HasCSTraits).
        effect0 = ICardEffect()
        effect0.set_effect_name("BT23-067 Alternate digivolution requirement")
        effect0.set_effect_description(
            "[Digivolve] Lv.4 w/[CS] trait: Cost 3"
        )
        effect0._alt_digi_cost = 3
        effect0._alt_digi_level = 4
        effect0._alt_digi_trait = "CS"

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── BeforePayCost: -3 if you have [Angewomon] or [Mirei Mikagura]
        # C# uses EqualsCardName (exact match), so "Angewomon (X Antibody)"
        # does NOT qualify. Uses an exact-name comparison against each
        # friendly battle-area permanent's top card card_names list.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.BeforePayCost)
        effect1.set_effect_name("BT23-067 Play cost reduction -3")
        effect1.set_effect_description(
            "When this card would be played from the hand, if you have "
            "[Angewomon] or [Mirei Mikagura], reduce the play cost by 3."
        )
        effect1.set_hash_string("BT23_067_ReducePlayCost")
        effect1.cost_reduction = 3

        def _has_angewomon_or_mirei(owner) -> bool:
            target_names = ("Angewomon", "Mirei Mikagura")
            for p in owner.battle_area:
                top = p.top_card
                if top is None:
                    continue
                names = getattr(top, 'card_names', None) or []
                for n in names:
                    if n in target_names:
                        return True
            return False

        def condition1(context: Dict[str, Any]) -> bool:
            # Leak guard — only reduce cost for THIS card.
            if context.get('card_source') is not card:
                return False
            owner = getattr(card, 'owner', None)
            if not owner:
                return False
            # C# CanActivateCondition also checks card is in hand.
            if card not in owner.hand_cards:
                return False
            return _has_angewomon_or_mirei(owner)

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            # Cost reduction is applied via `cost_reduction` attribute
            # by calculate_play_cost(). No imperative action needed.
            pass

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Blocker keyword (self static) ─────────────────────────────
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("BT23-067 Blocker")
        effect_blocker.set_effect_description("<Blocker>")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # ── [On Play] Delete 1 opponent's Lv.4 or lower Digimon ──────
        def _build_delete_effect(is_digivolving: bool) -> 'ICardEffect':
            eff = ICardEffect()
            eff.set_timing(EffectTiming.OnEnterFieldAnyone)
            label = "[When Digivolving]" if is_digivolving else "[On Play]"
            eff.set_effect_name(
                "BT23-067 Delete 1 of your opponent's level 4 or lower Digimon"
            )
            eff.set_effect_description(
                f"{label} Delete 1 of your opponent's level 4 or lower Digimon."
            )
            if is_digivolving:
                eff.is_when_digivolving = True
            else:
                eff.is_on_play = True

            def can_use(context: Dict[str, Any]) -> bool:
                # Timing gating (CanTriggerOnPlay / CanTriggerWhenDigivolving)
                # is enforced by the engine's dispatcher; this effect must
                # also still be on the field when activating.
                return True
            eff.set_can_use_condition(can_use)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return

                def target_filter(p):
                    top = p.top_card
                    if top is None:
                        return False
                    if not p.is_digimon:
                        return False
                    lvl = getattr(top, 'level', None)
                    if lvl is None:
                        return False
                    return lvl <= 4

                def on_delete(target_perm):
                    enemy = player.enemy if player else None
                    if enemy:
                        enemy.delete_permanent(target_perm)

                game.effect_select_opponent_permanent(
                    player,
                    on_delete,
                    filter_fn=target_filter,
                    is_optional=False,
                    prompt="Select 1 Digimon to delete.",
                )

            eff.set_on_process_callback(process)
            return eff

        effects.append(_build_delete_effect(is_digivolving=False))
        effects.append(_build_delete_effect(is_digivolving=True))

        # ── Inherited <Scapegoat> ─────────────────────────────────────
        # Passive inherited keyword — granted to the Digimon above this
        # card in a digivolution stack.
        effect_scape = ICardEffect()
        effect_scape.set_effect_name("BT23-067 <Scapegoat>")
        effect_scape.set_effect_description(
            "<Scapegoat> (When this Digimon would be deleted other than by "
            "your effects, by deleting 1 of your other Digimon, prevent "
            "that deletion.)"
        )
        effect_scape.is_inherited_effect = True
        effect_scape._is_scapegoat = True

        def condition_scape(context: Dict[str, Any]) -> bool:
            return True
        effect_scape.set_can_use_condition(condition_scape)
        effects.append(effect_scape)

        return effects
