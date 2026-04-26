from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_187(CardScript):
    """P-187 Mastemon | Lv.6 Purple/Yellow Angel

    [When Digivolving] <Recovery +1 (Deck)> (Place the top card of your deck
        on top of your security stack.) Then, if DNA digivolving, by placing
        1 other Digimon or Tamer as the top or bottom security card, trash
        your opponent's top security card.
    [When Digivolving] [When Attacking] [Once Per Turn] By trashing your top
        security card, you may play 1 purple or yellow Digimon card with
        6000 DP or less from your hand or trash without paying the cost.

    DNA Digivolve: Purple Lv.5 + Yellow Lv.5 : Cost 0 (parsed from xros_req).

    C# reference: DCGO/Assets/Scripts/CardEffect/P/Purple/P_187.cs
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ── Effect 0: DNA Digivolve condition placeholder ────────────────
        # The engine parses DNA requirements from cards.json (xros_req field)
        # into c_entity_base.dna_costs, which the digivolve validator uses.
        # This placeholder mirrors the C# AddJogressConditionClass.
        effect0 = ICardEffect()
        effect0.set_effect_name("P-187 Jogress Condition")
        effect0.set_effect_description("DNA Digivolution: Purple Lv.5 + Yellow Lv.5 : Cost 0")

        def condition0(context: Dict[str, Any]) -> bool:
            return True

        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ── Effect 1: [When Digivolving] Recovery +1 (Deck) and, if DNA, ──
        #    by placing 1 other Digimon/Tamer as top/bottom security,
        #    trash opponent's top security card.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OnEnterFieldAnyone)
        effect1.set_effect_name(
            "P-187 Recovery +1 (Deck); if DNA, place 1 other perm as security, "
            "trash opp's top security"
        )
        effect1.set_effect_description(
            "[When Digivolving] <Recovery +1 (Deck)> (Place the top card of your "
            "deck on top of your security stack.) Then, if DNA digivolving, by "
            "placing 1 other Digimon or Tamer as the top or bottom security card, "
            "trash your opponent's top security card."
        )
        effect1.is_when_digivolving = True

        def condition1(context: Dict[str, Any]) -> bool:
            # Field presence: only fires while Mastemon is on the battle area.
            if card and card.permanent_of_this_card() is None:
                return False
            return True

        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            perm = ctx.get('permanent')
            game = ctx.get('game')
            if not (player and game):
                return

            # Step 1: <Recovery +1 (Deck)> — always fires, even without DNA.
            player.recovery(1)

            # Step 2: "Then, if DNA digivolving, ..." — DNA-only placement.
            if not ctx.get('is_dna_digivolve'):
                return

            def target_filter(p):
                # "1 other Digimon or Tamer" — exclude Mastemon itself.
                if p is perm:
                    return False
                return bool(getattr(p, 'is_digimon', False)
                            or getattr(p, 'is_tamer', False))

            def on_target_selected(target_perm):
                # Cost already checked: player selected a valid permanent.
                # Step 3: choose TOP or BOTTOM of security stack.
                def on_branch(branch: int):
                    # branch 0 = top, branch 1 = bottom.
                    # Engine convention: security_cards[0] = TOP (revealed
                    # first by pop(0)); insert(0, ...) = TOP, append = BOTTOM.
                    if target_perm not in player.battle_area:
                        return

                    # Capture the top card of the selected permanent before
                    # moving it (put_permanent_to_security trashes digi-cards
                    # under the top card and places the top card at index -1).
                    top_card = target_perm.top_card

                    # Remove the permanent from the battle area and move its
                    # top card to security; digivolution cards go to trash.
                    player.put_permanent_to_security(target_perm)

                    # put_permanent_to_security uses .append (= BOTTOM per
                    # engine convention where index 0 is TOP). If the agent
                    # chose TOP, move the card from the end to index 0.
                    if branch == 0 and top_card is not None and top_card in player.security_cards:
                        player.security_cards.remove(top_card)
                        player.security_cards.insert(0, top_card)

                    # Step 4: placement succeeded → trash opponent's top
                    # security card (fires OnDiscardSecurity/OnLoseSecurity).
                    enemy = player.enemy
                    if enemy and enemy.security_cards:
                        opp_top = enemy.security_cards[0]
                        enemy.trash_security_card(opp_top)

                game.effect_choose_branch(
                    player, 2, on_branch,
                    prompt="Place on top or bottom of your security stack?",
                    branch_labels=["Top of security", "Bottom of security"],
                )

            # Placement uses "by placing ..." cost grammar — optional (the
            # player MAY decline to pay and skip the whole clause).
            game.effect_select_own_permanent(
                player, on_target_selected,
                filter_fn=target_filter,
                is_optional=True,
                prompt="Select 1 other Digimon or Tamer to place in security."
            )

        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ── Effect 2/3: [When Digivolving] [When Attacking] [Once Per Turn] ──
        #    By trashing your top security card, you may play 1 purple or
        #    yellow Digimon with 6000 DP or less from hand or trash for free.
        #
        #    Shared OPT hash so [Once Per Turn] is enforced ACROSS both
        #    trigger branches — matches C# SetHashString("PlayDigimon_P_187").

        OPT_HASH = "PlayDigimon_P_187"

        def play_filter(c):
            # Must be a Digimon card.
            if not getattr(c, 'is_digimon', False):
                return False
            # Must be purple or yellow.
            colors = [col.name for col in (getattr(c, 'card_colors', []) or [])]
            if 'Purple' not in colors and 'Yellow' not in colors:
                return False
            # Must have DP <= 6000. CardSource exposes base_dp (NOT .dp,
            # which is a Permanent-only property). Tamers/options/eggs have
            # base_dp=None and are already excluded by is_digimon.
            card_dp = getattr(c, 'base_dp', None)
            if card_dp is None:
                return False
            return card_dp <= 6000

        def _build_opt_play_effect(*, is_when_digivolving: bool, is_on_attack: bool):
            effect = ICardEffect()
            if is_when_digivolving:
                effect.set_timing(EffectTiming.OnEnterFieldAnyone)
                effect.is_when_digivolving = True
                desc = (
                    "[When Digivolving] [Once Per Turn] By trashing your top "
                    "security card, you may play 1 purple or yellow Digimon "
                    "card with 6000 DP or less from your hand or trash "
                    "without paying the cost."
                )
            else:
                # [When Attacking] uses OnUseAttack (28), NOT OnAllyAttack (32).
                effect.set_timing(EffectTiming.OnUseAttack)
                effect.is_on_attack = True
                desc = (
                    "[When Attacking] [Once Per Turn] By trashing your top "
                    "security card, you may play 1 purple or yellow Digimon "
                    "card with 6000 DP or less from your hand or trash "
                    "without paying the cost."
                )

            effect.set_effect_name("P-187 Trash top security to play Digimon")
            effect.set_effect_description(desc)
            effect.is_optional = True
            effect.set_max_count_per_turn(1)
            effect.set_hash_string(OPT_HASH)

            def condition(context: Dict[str, Any]) -> bool:
                # Field presence check.
                if card and card.permanent_of_this_card() is None:
                    return False
                owner = card.owner if card else None
                if not owner:
                    return False
                # Cost check: own security must be non-empty (the "by" cost
                # must be payable for the effect to activate at all).
                if not owner.security_cards:
                    return False
                # Additionally, there must be at least one valid play target
                # in hand or trash (the "you may play" only works if there
                # is something to play — matches C# HasMatchCondition...).
                has_target = any(play_filter(c) for c in owner.hand_cards) \
                    or any(play_filter(c) for c in owner.trash_cards)
                return has_target

            effect.set_can_use_condition(condition)

            def process(ctx: Dict[str, Any]):
                player = ctx.get('player')
                game = ctx.get('game')
                if not (player and game):
                    return
                if not player.security_cards:
                    return

                # Pay cost: trash OWN top security card. Use
                # trash_security_card so OnDiscardSecurity + OnLoseSecurity
                # timings fire (same as DCGO IDestroySecurity).
                top_sec = player.security_cards[0]
                player.trash_security_card(top_sec)

                # Play 1 purple or yellow Digimon with 6000 DP or less from
                # hand or trash, free. Optional at the player-selection
                # layer: agent can decline (but the cost was already paid
                # per the "by" grammar, matching DCGO behavior).
                game.effect_play_from_zone(
                    player, 'hand_or_trash', play_filter,
                    free=True, is_optional=True,
                    prompt="Select a purple or yellow Digimon with 6000 DP "
                           "or less to play for free."
                )

            effect.set_on_process_callback(process)
            return effect

        effects.append(_build_opt_play_effect(
            is_when_digivolving=True, is_on_attack=False))
        effects.append(_build_opt_play_effect(
            is_when_digivolving=False, is_on_attack=True))

        return effects
