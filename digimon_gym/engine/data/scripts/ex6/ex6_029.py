from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


# Traits that qualify for the Angel play-from-hand-or-trash effect.
# Matches C# HasAngelTraits helper (Angel, Archangel, Fallen Angel).
_ANGEL_TRAITS = {"Angel", "Archangel", "Fallen Angel"}


def _is_angel_lv5_or_lower_digimon(c) -> bool:
    """Match condition used by the [On Play]/[When Digivolving] free play
    effect. Mirrors C# IsCardAngelCondition: IsDigimon + Level <= 5 +
    HasAngelTraits."""
    if not getattr(c, 'is_digimon', False):
        return False
    lvl = getattr(c, 'level', None)
    if lvl is None or lvl > 5:
        return False
    traits = getattr(c, 'card_traits', None) or []
    return any(t in _ANGEL_TRAITS for t in traits)


class EX6_029(CardScript):
    """EX6-029 Mastemon | Lv.6 | Yellow/Purple | Angel"""

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── [Hand][Counter] <Blast DNA Digivolve ([Angewomon] + [LadyDevimon])> ──
        # C# reference: EffectTiming.OnCounterTiming registers
        # CardEffectFactory.BlastDNADigivolveEffect with BlastDNAConditions
        # ["Angewomon", "LadyDevimon"].
        effect0 = ICardEffect()
        effect0.set_effect_name("EX6-029 Blast DNA Digivolve")
        effect0.set_effect_description(
            "[Hand] [Counter] <Blast DNA Digivolve ([Angewomon] + [LadyDevimon])>"
        )
        effect0.is_counter_effect = True
        effect0._is_blast_dna_digivolve = True
        # Blast DNA participant names (from BlastDNACondition list in C#).
        effect0._blast_dna_names = ["Angewomon", "LadyDevimon"]

        def condition0(context: Dict[str, Any]) -> bool:
            return True
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── DNA Digivolution Condition (Yellow Lv.5 + Purple Lv.5) ──
        # C# reference: EffectTiming.None AddJogressConditionClass with two
        # PermanentCondition predicates (Yellow Lv.5 and Purple Lv.5).
        # The Python engine enforces DNA requirements from the card entity's
        # dna_costs metadata, so this entry is a documentation-only marker.
        effect1 = ICardEffect()
        effect1.set_effect_name("EX6-029 DNA Digivolution Condition")
        effect1.set_effect_description(
            "DNA Digivolution: Yellow Lv.5 + Purple Lv.5"
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)
        effects.append(effect1)

        # ── Shared [On Play] / [When Digivolving] resolution ──
        # C# reference: both triggers point to the same ActivateCoroutine:
        #   1. Select 1 Angel/Archangel/Fallen Angel Lv.5 or lower Digimon
        #      from hand OR trash, play it for free.
        #   2. If DNA digivolving: pick 1 OTHER Digimon from ANY battle area
        #      (IsPermanentExistsOnBattleArea -> both players' fields),
        #      place it at the bottom of ITS OWNER'S security stack.
        #   3. If DNA digivolving AND opponent has >4 security cards, trash
        #      from the top of opponent's security until it has 4 left.
        #
        # The trash-opponent-security step happens unconditionally (no
        # dependency on step 2 succeeding), matching DCGO's sequential
        # coroutine. Step 2 and step 3 both only fire when DNA digivolving.

        def _make_enter_effect(is_wd: bool) -> ICardEffect:
            effect = ICardEffect()
            effect.set_timing(EffectTiming.OnEnterFieldAnyone)
            effect.set_effect_name(
                "EX6-029 Play Angel/Archangel/Fallen Angel free, if DNA "
                "digivolving place 1 other Digimon at bottom of its owner's "
                "security and trash opponent security to 4"
            )
            if is_wd:
                effect.is_when_digivolving = True
                effect.set_effect_description(
                    "[When Digivolving] You may play 1 level 5 or lower "
                    "Digimon card with the [Angel], [Archangel] or [Fallen "
                    "Angel] trait from your hand or trash without paying the "
                    "cost. Then, if DNA digivolving, place 1 other Digimon "
                    "at the bottom of its owner's security stack, and trash "
                    "cards from the top of your opponent's security stack "
                    "until it has 4 left."
                )
            else:
                effect.is_on_play = True
                effect.set_effect_description(
                    "[On Play] You may play 1 level 5 or lower Digimon card "
                    "with the [Angel], [Archangel] or [Fallen Angel] trait "
                    "from your hand or trash without paying the cost. Then, "
                    "if DNA digivolving, place 1 other Digimon at the bottom "
                    "of its owner's security stack, and trash cards from the "
                    "top of your opponent's security stack until it has 4 "
                    "left."
                )
            effect.is_optional = True

            def condition(context: Dict[str, Any]) -> bool:
                if card and card.permanent_of_this_card() is None:
                    return False
                return True
            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                """Shared resolution for On Play and When Digivolving."""
                player = ctx.get('player')
                perm = ctx.get('permanent')
                game = ctx.get('game')
                if not (player and game):
                    return

                # Step 1: optional free play of 1 Angel Lv5 or lower from
                # hand or trash.
                game.effect_play_from_zone(
                    player, 'hand_or_trash',
                    _is_angel_lv5_or_lower_digimon,
                    free=True, is_optional=True,
                    prompt=(
                        "Select 1 level 5 or lower Angel/Archangel/Fallen "
                        "Angel Digimon from hand or trash to play for free."
                    ),
                )

                # The security rider only fires while DNA digivolving.
                if not ctx.get('is_dna_digivolve'):
                    return

                enemy = player.enemy

                # Step 2: select 1 OTHER Digimon from either battle area
                # and place it at the bottom of its owner's security stack.
                # C# CanSelectPermanentSharedCondition:
                #   IsPermanentExistsOnBattleArea + IsDigimon + not self.
                from ....game.constants import (
                    SEL_MY_FIELD_START, SEL_OPP_FIELD_START, FIELD_SLOTS,
                )
                from ....data.enums import GamePhase

                def _is_valid_target(p) -> bool:
                    if p is perm:
                        return False
                    if not getattr(p, 'is_digimon', False):
                        return False
                    # Engine-level targeting protection
                    if not game.modifiers.can_be_selected_by_effect(p):
                        return False
                    return True

                valid = []
                for i, p in enumerate(player.battle_area):
                    if _is_valid_target(p):
                        valid.append(SEL_MY_FIELD_START + i)
                if enemy:
                    for i, p in enumerate(enemy.battle_area):
                        if _is_valid_target(p):
                            valid.append(SEL_OPP_FIELD_START + i)

                def _do_trash_opponent_security():
                    """Unconditional step 3: trim opponent security to 4."""
                    if not enemy:
                        return
                    while len(enemy.security_cards) > 4:
                        trashed = enemy.security_cards.pop(0)  # top = idx 0
                        enemy.trash_cards.append(trashed)

                if valid:
                    def on_select(action_id: int):
                        target_perm = None
                        target_owner = None
                        if (SEL_MY_FIELD_START <= action_id
                                < SEL_MY_FIELD_START + FIELD_SLOTS):
                            idx = action_id - SEL_MY_FIELD_START
                            if 0 <= idx < len(player.battle_area):
                                target_perm = player.battle_area[idx]
                                target_owner = player
                        elif (SEL_OPP_FIELD_START <= action_id
                                < SEL_OPP_FIELD_START + FIELD_SLOTS):
                            idx = action_id - SEL_OPP_FIELD_START
                            if enemy and 0 <= idx < len(enemy.battle_area):
                                target_perm = enemy.battle_area[idx]
                                target_owner = enemy
                        if target_perm is not None and target_owner is not None:
                            # put_permanent_to_security appends the top card
                            # to the end of security_cards, which is the
                            # BOTTOM of the stack (index 0 = top of stack).
                            target_owner.put_permanent_to_security(target_perm)
                        # Step 3 fires after step 2 (DCGO ordering).
                        _do_trash_opponent_security()

                    game.request_selection(
                        GamePhase.SelectTarget, player, on_select, valid,
                        is_optional=False,
                        prompt=(
                            "Select 1 other Digimon to place at the bottom "
                            "of its owner's security stack."
                        ),
                    )
                else:
                    # No valid 'other Digimon' target — still perform step 3
                    # unconditionally (DCGO sequential coroutine).
                    _do_trash_opponent_security()

            effect.set_on_process_callback(process)
            return effect

        effects.append(_make_enter_effect(is_wd=False))
        effects.append(_make_enter_effect(is_wd=True))

        # ── Ace Overflow <-4> (inherited descriptive marker) ──
        # Engine parses this from inherited_effect_description_eng and
        # applies memory loss automatically. This entry documents the
        # effect in the script for parity with BT20-060.
        effect_ace = ICardEffect()
        effect_ace.set_effect_name("EX6-029 Ace Overflow")
        effect_ace.set_effect_description("Ace Overflow <-4>")
        effect_ace.is_inherited_effect = True

        def condition_ace(context: Dict[str, Any]) -> bool:
            return True
        effect_ace.set_can_use_condition(condition_ace)
        effects.append(effect_ace)

        return effects
