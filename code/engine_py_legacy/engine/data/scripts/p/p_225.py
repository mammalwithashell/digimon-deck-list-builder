from __future__ import annotations
from typing import TYPE_CHECKING, List, Dict, Any
from ....core.card_script import CardScript
from ....interfaces.card_effect import ICardEffect
from ....data.enums import EffectTiming

if TYPE_CHECKING:
    from ....core.card_source import CardSource


class P_225(CardScript):
    """P-225 DigiLab | Option White Cost 2 CS

    While you have [CS] trait Digimon or Tamer, you can ignore this
        card's color requirements.
    [Main] <Draw 1> (Draw 1 card from your deck.) Then, place this card
        in the battle area.
    [Main] <Delay> (By trashing this card after the placing turn,
        activate the effect below.)
        - By placing the top stacked card of any of your level 4 or
          higher [CS] Digimon as its bottom digivolution card, gain 2
          memory.
    [Security] Place this card in the battle area.

    C# reference: DCGO/Assets/Scripts/CardEffect/P/White/P_225.cs
    """

    def get_card_effects(self, card: 'CardSource') -> List['ICardEffect']:
        effects = []

        # ─────────────────────────────────────────────────────────────
        # C1: Dynamic color bypass while own CS Digimon/Tamer on field
        # ─────────────────────────────────────────────────────────────
        # DCGO: IgnoreColorConditionClass with HasMatchConditionPermanent
        # checking (IsDigimon || IsTamer) && HasCSTraits for owner.
        def _owner_has_cs_permanent() -> bool:
            owner = card.owner if card else None
            if not owner:
                return False
            for p in owner.battle_area:
                if not (p.is_digimon or p.is_tamer):
                    continue
                if not p.top_card:
                    continue
                traits = getattr(p.top_card, 'card_traits', []) or []
                if any('CS' in t for t in traits):
                    return True
            return False

        def _match_color_req() -> bool:
            # Return True = enforce color requirement normally
            # Return False = bypass color requirement
            return not _owner_has_cs_permanent()

        card._match_color_requirement_fn = _match_color_req

        effect0 = ICardEffect()
        effect0.set_effect_name("P-225 Ignore color requirements")
        effect0.set_effect_description(
            "While you have [CS] trait Digimon or Tamer, you can ignore "
            "this card's color requirements."
        )
        effect0.is_declarative = True

        def condition0(context: Dict[str, Any]) -> bool:
            return _owner_has_cs_permanent()
        effect0.set_can_use_condition(condition0)
        effects.append(effect0)

        # ─────────────────────────────────────────────────────────────
        # C2: [Main] <Draw 1>. Then, place this card in the battle area.
        # ─────────────────────────────────────────────────────────────
        # DCGO: ActivateClass -> DrawClass(drawCount: 1) then
        # CardEffectCommons.PlaceDelayOptionCards(card).
        # In the Python engine, Option cards whose effect list contains
        # a _is_delay marker stay on field automatically
        # (Game._option_stays_on_field). So we only need to draw here.
        effect1 = ICardEffect()
        effect1.set_timing(EffectTiming.OptionSkill)
        effect1.set_effect_name("P-225 Draw 1 then place in battle area")
        effect1.set_effect_description(
            "[Main] <Draw 1> (Draw 1 card from your deck.) Then, place "
            "this card in the battle area."
        )

        def condition1(context: Dict[str, Any]) -> bool:
            return True
        effect1.set_can_use_condition(condition1)

        def process1(ctx: Dict[str, Any]):
            player = ctx.get('player')
            if player:
                player.draw_cards(1)
        effect1.set_on_process_callback(process1)
        effects.append(effect1)

        # ─────────────────────────────────────────────────────────────
        # C3a: Delay marker — flags this Option as "stays on field"
        # ─────────────────────────────────────────────────────────────
        effect2 = ICardEffect()
        effect2.set_effect_name("P-225 Delay")
        effect2.set_effect_description("Delay")
        effect2._is_delay = True

        def condition2(context: Dict[str, Any]) -> bool:
            return True
        effect2.set_can_use_condition(condition2)
        effects.append(effect2)

        # ─────────────────────────────────────────────────────────────
        # C3b: [Main] <Delay> — Place top stacked card as bottom +2 mem
        # ─────────────────────────────────────────────────────────────
        # DCGO: After DeletePeremanentAndProcessAccordingToResult trashes
        # the option card itself, SelectPermanentEffect filters by
        # IsPermanentExistsOnOwnerBattleAreaDigimon && HasCSTraits &&
        # HasLevel && Level >= 4 && DigivolutionCards.Count >= 1.
        # The selected permanent's TopCard is then added to the BOTTOM
        # of its own DigivolutionCards via AddDigivolutionCardsBottom,
        # after which the owner gains 2 memory.
        #
        # IMPORTANT: "top stacked card" = the CURRENT top card of the
        # permanent (card_sources[-1]). DCGO moves TopCard to the bottom
        # of DigivolutionCards via AddDigivolutionCardsBottom(TopCard),
        # which soft de-digivolves the permanent: the previous top becomes
        # a buried digivolution card and the card that was underneath
        # becomes the new top. DCGO's DigivolutionCards.Count >= 1 check
        # maps to Python's len(card_sources) >= 2.
        effect3 = ICardEffect()
        effect3.set_timing(EffectTiming.OnDeclaration)
        effect3.set_effect_name(
            "P-225 Delay: place top stacked card as bottom digi card, +2 memory"
        )
        effect3.set_effect_description(
            "[Main] <Delay> By placing the top stacked card of any of your "
            "level 4 or higher [CS] Digimon as its bottom digivolution "
            "card, gain 2 memory."
        )

        def condition3(context: Dict[str, Any]) -> bool:
            # NOTE: do NOT gate on card.permanent_of_this_card() — by the
            # time the engine invokes the delay callback, the option has
            # already been trashed (see Game._execute_delay). The action
            # mask gates availability via _has_delay_effect + turn_played,
            # so we just need to authorise execution here.
            return True
        effect3.set_can_use_condition(condition3)

        def _is_valid_target(p) -> bool:
            if not p.is_digimon or not p.top_card:
                return False
            lvl = getattr(p.top_card, 'level', None)
            if lvl is None or lvl < 4:
                return False
            traits = getattr(p.top_card, 'card_traits', []) or []
            if not any('CS' in t for t in traits):
                return False
            # DCGO: DigivolutionCards.Count >= 1 (excludes top). In
            # Python card_sources includes top, so need size >= 2.
            if len(p.card_sources) < 2:
                return False
            return True

        def process3(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            if not (player and game):
                return

            # Early out if no valid target exists at all.
            if not any(_is_valid_target(p) for p in player.battle_area):
                return

            def on_select(target_perm):
                if target_perm is None:
                    return
                if not _is_valid_target(target_perm):
                    return
                # "Top stacked card" = the current top (card_sources[-1]).
                # Pop it and insert at index 0 (bottom of the stack). This
                # soft de-digivolves the permanent: previous top goes to
                # the bottom, and the card that was underneath becomes
                # the new top. Matches DCGO AddDigivolutionCardsBottom.
                if len(target_perm.card_sources) < 2:
                    return
                top_stacked = target_perm.card_sources.pop()
                target_perm.card_sources.insert(0, top_stacked)
                # On success, owner gains 2 memory.
                player.add_memory(2)

            game.effect_select_own_permanent(
                player, on_select,
                filter_fn=_is_valid_target,
                is_optional=False,
                prompt="Select 1 of your Lv.4+ [CS] Digimon with a "
                       "stacked card to rearrange.")

        effect3.set_on_process_callback(process3)
        effects.append(effect3)

        # ─────────────────────────────────────────────────────────────
        # C4: [Security] Place this card in the battle area.
        # ─────────────────────────────────────────────────────────────
        # DCGO: CardEffectFactory.PlaceSelfDelayOptionSecurityEffect(card)
        # which plays the option from security into the battle area. In
        # Python, we use game.effect_play_from_security which sets
        # _security_played on the card so security_attack() won't trash
        # it after the security check resolves.
        effect4 = ICardEffect()
        effect4.set_timing(EffectTiming.SecuritySkill)
        effect4.set_effect_name("P-225 Security: place this card in battle area")
        effect4.set_effect_description(
            "[Security] Place this card in the battle area."
        )
        effect4.is_security_effect = True

        def condition4(context: Dict[str, Any]) -> bool:
            return True
        effect4.set_can_use_condition(condition4)

        def process4(ctx: Dict[str, Any]):
            player = ctx.get('player')
            game = ctx.get('game')
            sec_card = ctx.get('card')
            if not (player and game and sec_card):
                return
            game.effect_play_from_security(player, sec_card)

        effect4.set_on_process_callback(process4)
        effects.append(effect4)

        return effects
