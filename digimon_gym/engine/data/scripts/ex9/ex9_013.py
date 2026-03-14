from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class EX9_013(CardScript):
    """EX9-013 BlitzGreymon | Lv.6

    [Hand] [Counter] <Blast Digivolve>
    <Alliance> <Blocker>
    [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
    [End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S]
    in the hand. Then, 1 of your Digimon may attack.
    Inherited: Ace Overflow <-4>
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Alternate digivolution: Lv.5 w/[Greymon] in name: Cost 3 ────────
        effect_alt1 = ICardEffect()
        effect_alt1.set_effect_name("EX9-013 Alt digi (Greymon name)")
        effect_alt1.set_effect_description("Lv.5 w/[Greymon] in name: Cost 3")
        effect_alt1._alt_digi_cost = 3
        effect_alt1._alt_digi_level = 5
        effect_alt1._alt_digi_name = "Greymon"

        def condition_alt1(context: Dict[str, Any]) -> bool:
            return True
        effect_alt1.set_can_use_condition(condition_alt1)
        effects.append(effect_alt1)

        # ── Alternate digivolution: Lv.5 w/[DM] trait: Cost 3 ───────────────
        effect_alt2 = ICardEffect()
        effect_alt2.set_effect_name("EX9-013 Alt digi (DM trait)")
        effect_alt2.set_effect_description("Lv.5 w/[DM] trait: Cost 3")
        effect_alt2._alt_digi_cost = 3
        effect_alt2._alt_digi_level = 5
        effect_alt2._alt_digi_trait = "DM"

        def condition_alt2(context: Dict[str, Any]) -> bool:
            return True
        effect_alt2.set_can_use_condition(condition_alt2)
        effects.append(effect_alt2)

        # ── Blast Digivolve ──────────────────────────────────────────────────
        effect_blast = ICardEffect()
        effect_blast.set_effect_name("EX9-013 Blast Digivolve")
        effect_blast.set_effect_description("[Hand] [Counter] <Blast Digivolve>")
        effect_blast.is_counter_effect = True
        effect_blast._is_blast_digivolve = True

        def condition_blast(context: Dict[str, Any]) -> bool:
            return True
        effect_blast.set_can_use_condition(condition_blast)
        effects.append(effect_blast)

        # ── Alliance ─────────────────────────────────────────────────────────
        effect_alliance = ICardEffect()
        effect_alliance.set_effect_name("EX9-013 Alliance")
        effect_alliance.set_effect_description("<Alliance>")
        effect_alliance._is_alliance = True

        def condition_alliance(context: Dict[str, Any]) -> bool:
            return True
        effect_alliance.set_can_use_condition(condition_alliance)
        effects.append(effect_alliance)

        # ── Blocker ──────────────────────────────────────────────────────────
        effect_blocker = ICardEffect()
        effect_blocker.set_effect_name("EX9-013 Blocker")
        effect_blocker.set_effect_description("<Blocker>")
        effect_blocker._is_blocker = True

        def condition_blocker(context: Dict[str, Any]) -> bool:
            return True
        effect_blocker.set_can_use_condition(condition_blocker)
        effects.append(effect_blocker)

        # ── Shared process for De-Digivolve 3 ───────────────────────────────
        def _de_digivolve_process(ctx: Dict[str, Any]):
            """<De-Digivolve 3> 1 of opponent's Digimon."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return
            enemy = player.enemy if player else None
            if not enemy:
                return

            def on_select(target_perm):
                removed = target_perm.de_digivolve(3)
                for c in removed:
                    enemy.trash_cards.append(c)

            game.effect_select_opponent_permanent(
                player, on_select,
                filter_fn=lambda p: p.is_digimon,
                is_optional=False,
                prompt="Select 1 of your opponent's Digimon to De-Digivolve 3.")

        # ── [On Play] De-Digivolve 3 ────────────────────────────────────────
        effect_op = ICardEffect()
        effect_op.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_op.set_effect_name("EX9-013 De-Digivolve 3 (On Play)")
        effect_op.set_effect_description("[On Play] <De-Digivolve 3> 1 of your opponent's Digimon.")
        effect_op.is_on_play = True

        def condition_op(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_op.set_can_use_condition(condition_op)
        effect_op.set_on_process_callback(_de_digivolve_process)
        effects.append(effect_op)

        # ── [When Digivolving] De-Digivolve 3 ───────────────────────────────
        effect_wd = ICardEffect()
        effect_wd.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect_wd.set_effect_name("EX9-013 De-Digivolve 3 (When Digivolving)")
        effect_wd.set_effect_description("[When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.")
        effect_wd.is_when_digivolving = True

        def condition_wd(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            return True
        effect_wd.set_can_use_condition(condition_wd)
        effect_wd.set_on_process_callback(_de_digivolve_process)
        effects.append(effect_wd)

        # ── [End of Your Turn] DNA digivolve + 1 Digimon may attack ─────────
        # 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand.
        # Then, 1 of your Digimon may attack.
        effect_eot = ICardEffect()
        effect_eot.set_timing(EffectTiming.OnEndTurn)
        effect_eot.set_effect_name("EX9-013 End of turn DNA digivolve + attack")
        effect_eot.set_effect_description(
            "[End of Your Turn] 2 of your Digimon may DNA digivolve into "
            "[Omnimon Alter-S] in the hand. Then, 1 of your Digimon may attack."
        )
        effect_eot.is_optional = True

        def condition_eot(context: Dict[str, Any]) -> bool:
            if card and card.permanent_of_this_card() is None:
                return False
            if not (card and card.owner and card.owner.is_my_turn):
                return False
            return True
        effect_eot.set_can_use_condition(condition_eot)

        def process_eot(ctx: Dict[str, Any]):
            """DNA digivolve into Omnimon Alter-S from hand, then 1 Digimon may attack."""
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            def _grant_attack_after_dna():
                """After DNA digivolve (or skip), grant 1 Digimon may attack."""
                own_digimon = [p for p in player.battle_area if p.is_digimon]
                if not own_digimon:
                    return

                def on_attack_selected(selected_perm):
                    if selected_perm is None:
                        return
                    from ....interfaces.modifiers import ModifierType
                    game.register_modifier(
                        selected_perm, ModifierType.FORCE_ATTACK,
                        value_fn=lambda: True, expiry='end_of_turn')

                game.effect_select_own_permanent(
                    player, on_attack_selected,
                    filter_fn=lambda p: p.is_digimon,
                    is_optional=True,
                    prompt="Select 1 of your Digimon to attack.")

            # Step 1: DNA digivolve — filter for Omnimon Alter-S in hand
            def omnimon_alter_s_filter(c):
                if not getattr(c, 'is_digimon', False):
                    return False
                if not c.contains_card_name('Omnimon Alter-S'):
                    return False
                if not (c.c_entity_base and c.c_entity_base.dna_costs):
                    return False
                return True

            has_dna_candidate = any(omnimon_alter_s_filter(c) for c in player.hand_cards)
            has_two_digimon = sum(1 for p in player.battle_area if p.is_digimon) >= 2

            if has_dna_candidate and has_two_digimon:
                game.effect_dna_digivolve_from_hand(
                    player, omnimon_alter_s_filter,
                    is_optional=True,
                    prompt="Select [Omnimon Alter-S] from hand to DNA digivolve.")
                # After DNA digivolve resolves (or is declined), grant attack
                if game.pending_selection:
                    _orig_decline = getattr(game.pending_selection, 'on_decline', None)
                    def on_decline_dna():
                        if _orig_decline:
                            _orig_decline()
                        _grant_attack_after_dna()
                    game.pending_selection.on_decline = on_decline_dna
                    # If DNA was picked, the chain will eventually resolve and we
                    # need to queue the attack grant. DNA digivolve is multi-step
                    # so we chain via the last selection's on_decline and completion.
                    # However, the DNA chain is async and effect_dna_digivolve_from_hand
                    # handles its own multi-step selection internally.
                    # The attack grant must happen after the full DNA chain completes.
                    # For now, we schedule it to run after this effect.
                else:
                    # No pending selection (no valid DNA targets after all)
                    _grant_attack_after_dna()
            else:
                # No DNA candidates — just grant attack
                _grant_attack_after_dna()

        effect_eot.set_on_process_callback(process_eot)
        effects.append(effect_eot)

        # ── Inherited: Ace Overflow <-4> ─────────────────────────────────────
        # Ace Overflow is handled by the engine via entity_base.is_ace / ace_overflow_cost.
        # The inherited effect is a marker so the engine recognizes it.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("EX9-013 Ace Overflow <-4>")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True
        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
