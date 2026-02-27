# BT10 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 100
- Scripts with effects: 100
- Total effects: 254
- Factory effects: 81
- Activate effects: 173

## Per-Card Breakdown

```
BT10_005: 1 effects
  [factory] dp_modifier
BT10_058: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT10_059: 2 effects
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnAllyAttack] add_to_hand, reveal_and_select (inherited) (1/turn)
BT10_060: 4 effects
  [factory] alt_digivolve_req
  [factory] dp_modifier
  [factory] save
  [factory] reboot
BT10_061: 4 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_hand, reveal_and_select
  [EffectTiming.None] no-action
BT10_063: 1 effects
  [EffectTiming.None] no-action
BT10_066: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.WhenPermanentWouldBeDeleted] play_card, add_to_hand
  [EffectTiming.None] no-action
BT10_067: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_hand
  [EffectTiming.OnAllyAttack] digivolve
BT10_068: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card, grant_bounce_immunity
BT10_069: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_hand, unsuspend
  [EffectTiming.OnDestroyedAnyone] play_card
BT10_070: 3 effects
  [factory] rush
  [factory] blitz
  [EffectTiming.OnAllyAttack] delete, trash_digivolution_cards (1/turn)
BT10_092: 4 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] gain_memory
  [factory] security_play
BT10_104: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] play_card, mill
  [EffectTiming.SecuritySkill] add_to_hand
BT10_105: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] gain_keyword_blocker, gain_keyword_reboot
  [EffectTiming.SecuritySkill] play_card, add_to_hand, reveal_and_select
BT10_106: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete, play_card
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT10_002: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT10_018: 1 effects
  [EffectTiming.OnDestroyedAnyone] play_card
BT10_019: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] save
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT10_020: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] save
  [factory] dp_modifier
BT10_021: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_hand
  [factory] save
  [EffectTiming.OnAllyAttack] gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block (inherited) (1/turn)
BT10_023: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnAllyAttack] trash_from_hand, unsuspend (1/turn)
BT10_024: 4 effects
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block, gain_keyword_rush
  [EffectTiming.None] no-action
BT10_025: 2 effects
  [EffectTiming.OnDeclaration] unsuspend
  [factory] dp_modifier
BT10_026: 4 effects
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block
  [EffectTiming.None] no-action
BT10_027: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnAllyAttack] play_card
BT10_028: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnEndBattle] unsuspend (1/turn)
BT10_088: 3 effects
  [factory] set_memory_3
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
BT10_097: 3 effects
  [EffectTiming.OptionSkill] play_card, add_to_hand
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
BT10_098: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] bounce
  [EffectTiming.SecuritySkill] bounce
BT10_004: 1 effects
  [EffectTiming.OnTappedAnyone] change_dp (inherited) (1/turn)
BT10_044: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw (1/turn)
  [EffectTiming.OnTappedAnyone] draw (inherited) (1/turn)
BT10_045: 1 effects
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT10_046: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT10_048: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, play_card, effect_immunity
  [EffectTiming.OnTappedAnyone] draw (inherited) (1/turn)
BT10_049: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] save
BT10_050: 1 effects
  [factory] alt_digivolve_req
BT10_051: 1 effects
  [EffectTiming.OnTappedAnyone] gain_memory (inherited) (1/turn)
BT10_052: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction, suspend, effect_immunity
  [EffectTiming.OnAllyAttack] redirect_attack (1/turn)
BT10_053: 2 effects
  [EffectTiming.OnDeclaration] suspend, play_card, effect_immunity (1/turn)
  [EffectTiming.OnTappedAnyone] gain_memory (inherited) (1/turn)
BT10_054: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEndBattle] unsuspend (1/turn)
  [EffectTiming.OnAllyAttack] no-action
BT10_056: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.None] grant_skill
  [EffectTiming.None] gain_memory, add_to_hand
BT10_057: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend, gain_keyword_piercing
  [factory] dp_modifier
  [factory] security_attack_plus
BT10_090: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [factory] security_play
BT10_091: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] suspend
  [factory] security_play
BT10_102: 2 effects
  [EffectTiming.OptionSkill] gain_keyword_piercing, suspend
  [EffectTiming.SecuritySkill] add_to_hand, suspend
BT10_103: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] suspend, bounce
BT10_006: 1 effects
  [EffectTiming.OnDigivolutionCardDiscarded] draw (inherited)
BT10_071: 1 effects
  [factory] retaliation
BT10_072: 3 effects
  [EffectTiming.OnAllyAttack] draw
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
BT10_073: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
BT10_074: 1 effects
  [factory] armor_purge
BT10_075: 4 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
BT10_076: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_digivolution_cards (1/turn)
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
BT10_077: 4 effects
  [EffectTiming.OnAddHand] trash_from_hand, trash_digivolution_cards (1/turn)
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
  [EffectTiming.None] no-action
BT10_078: 3 effects
  [factory] alt_digivolve_req
  [factory] retaliation
  [EffectTiming.OnDestroyedAnyone] play_card
BT10_080: 3 effects
  [EffectTiming.OnDiscardHand] digivolve
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, add_temp_effect
BT10_081: 2 effects
  [EffectTiming.OnAllyAttack] mill
  [EffectTiming.OnDestroyedAnyone] play_card
BT10_082: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, mill
  [EffectTiming.OnEnterFieldAnyone] delete, mill
  [EffectTiming.OnEndAttack] no-action (inherited) (1/turn)
BT10_083: 3 effects
  [factory] retaliation
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
BT10_084: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, gain_keyword_blocker
  [EffectTiming.WhenWouldDigivolutionCardDiscarded] trash_from_hand
BT10_093: 3 effects
  [EffectTiming.OnAddDigivolutionCards] draw, gain_memory (1/turn)
  [EffectTiming.None] cost_reduction
  [factory] security_play
BT10_107: 2 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT10_108: 2 effects
  [EffectTiming.OnDiscardLibrary] add_to_hand
  [EffectTiming.OptionSkill] delete
BT10_001: 1 effects
  [factory] dp_modifier
BT10_007: 1 effects
  [factory] alt_digivolve_req
BT10_008: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] save
  [factory] rush
BT10_009: 5 effects
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnEndAttack] delete, unsuspend
  [EffectTiming.None] no-action
BT10_011: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnTappedAnyone] change_dp (1/turn)
  [EffectTiming.None] grant_skill
  [EffectTiming.None] grant_skill (inherited)
BT10_012: 4 effects
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.None] no-action
BT10_013: 5 effects
  [factory] security_attack_plus
  [factory] blocker
  [factory] save
  [factory] material_save
  [EffectTiming.None] no-action
BT10_014: 2 effects
  [factory] blitz
  [factory] dp_modifier
BT10_015: 5 effects
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.None] no-action
BT10_016: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, attack_unsuspended
BT10_087: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
BT10_094: 2 effects
  [EffectTiming.OptionSkill] draw, change_dp
  [EffectTiming.SecuritySkill] play_card
BT10_095: 2 effects
  [EffectTiming.OptionSkill] draw, change_security_attack
  [EffectTiming.SecuritySkill] add_to_hand
BT10_096: 2 effects
  [EffectTiming.OptionSkill] delete
  [EffectTiming.SecuritySkill] play_card, add_to_hand
BT10_111: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [factory] save
  [factory] material_save
  [EffectTiming.None] no-action
  [factory] dp_modifier
BT10_112: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] blocker
  [factory] security_attack_plus
BT10_085: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_memory (1/turn)
BT10_086: 5 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] destroy_security, return_to_deck (1/turn)
  [EffectTiming.OnAllyAttack] destroy_security, return_to_deck (1/turn)
BT10_109: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp
  [EffectTiming.SecuritySkill] gain_memory, add_to_hand
BT10_110: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] unsuspend
  [EffectTiming.SecuritySkill] add_to_hand
BT10_003: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited)
BT10_029: 3 effects
  [factory] alt_digivolve_req
  [factory] save
  [EffectTiming.OnAllyAttack] draw (inherited)
BT10_030: 1 effects
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
BT10_031: 2 effects
  [factory] alt_digivolve_req
  [factory] blocker
BT10_032: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnUseOption] change_dp (inherited) (1/turn)
BT10_034: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [factory] save
BT10_035: 1 effects
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited) (1/turn)
BT10_036: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand, add_to_hand
BT10_038: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnAllyAttack] change_security_attack (descriptive-tagged) (inherited) (1/turn)
BT10_039: 1 effects
  [EffectTiming.OnEnterFieldAnyone] ignore_color_req (descriptive-tagged)
BT10_040: 2 effects
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnAllyAttack] gain_memory, change_dp (1/turn)
BT10_041: 2 effects
  [EffectTiming.OnEnterFieldAnyone] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnAllyAttack] digivolve
BT10_042: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.None] disable_effect, effect_immunity
BT10_089: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] draw, suspend
  [factory] security_play
BT10_099: 1 effects
  [EffectTiming.OptionSkill] change_security_attack (descriptive-tagged)
BT10_100: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
BT10_101: 2 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp, put_to_security
```
