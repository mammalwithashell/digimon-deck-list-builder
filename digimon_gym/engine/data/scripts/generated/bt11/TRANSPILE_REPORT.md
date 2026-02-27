# BT11 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 102
- Scripts with effects: 101
- Total effects: 267
- Factory effects: 66
- Activate effects: 201

## Per-Card Breakdown

```
BT11_005: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw (inherited) (1/turn)
BT11_060: 0 effects
BT11_061: 2 effects
  [EffectTiming.OnDeclaration] suspend, add_to_hand, reveal_and_select
  [EffectTiming.None] cost_reduction (inherited)
BT11_062: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT11_063: 2 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
BT11_064: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
BT11_065: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnDigivolutionCardReturnToDeckBottom] unsuspend, gain_keyword_blocker (inherited) (1/turn)
BT11_067: 2 effects
  [factory] jamming
  [factory] reboot
BT11_068: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker (inherited) (1/turn)
BT11_069: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_immune_dp_minus
  [EffectTiming.OnUnTappedAnyone] destroy_security (inherited) (1/turn)
BT11_070: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
BT11_071: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnDestroyedAnyone] add_to_hand
  [EffectTiming.None] no-action
BT11_072: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] play_card, effect_immunity
BT11_073: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, gain_keyword_piercing
  [EffectTiming.OnAllyAttack] digivolve
BT11_074: 4 effects
  [factory] alt_digivolve_req
  [factory] reboot
  [EffectTiming.OnAllyAttack] redirect_attack (1/turn)
  [EffectTiming.OnUnTappedAnyone] delete (1/turn)
BT11_092: 3 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [EffectTiming.OnAllyAttack] suspend, redirect_attack
  [factory] security_play
BT11_093: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, effect_immunity
  [factory] security_play
BT11_105: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] digivolve
  [EffectTiming.SecuritySkill] play_card, reveal_and_select
BT11_106: 4 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] gain_keyword_cannot_be_blocked
  [EffectTiming.OptionSkill] gain_memory, add_temp_effect, effect_immunity
  [EffectTiming.SecuritySkill] play_card, reveal_and_select
BT11_107: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete, force_attack
  [EffectTiming.SecuritySkill] delete
BT11_108: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete, de_digivolve
BT11_111: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnStartMainPhase] destroy_security
BT11_002: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT11_020: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] bounce (inherited) (1/turn)
BT11_022: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw (1/turn)
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT11_023: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT11_024: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw
BT11_025: 2 effects
  [EffectTiming.OnAllyAttack] gain_memory (1/turn)
  [EffectTiming.OnAllyAttack] bounce (inherited) (1/turn)
BT11_027: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw (1/turn)
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT11_028: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker
  [EffectTiming.OnAddHand] unsuspend (inherited) (1/turn)
BT11_029: 2 effects
  [EffectTiming.OnDeclaration] suspend, add_to_hand, reveal_and_select (1/turn)
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT11_030: 5 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.None] no-action
BT11_031: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_memory, unsuspend
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] blocker
BT11_032: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnUnTappedAnyone] bounce (1/turn)
BT11_033: 2 effects
  [EffectTiming.OnEnterFieldAnyone] bounce, add_to_hand, destroy_security
  [EffectTiming.OnAddHand] no-action (1/turn)
BT11_090: 3 effects
  [EffectTiming.OnStartMainPhase] gain_keyword_jamming
  [EffectTiming.OnAddHand] gain_memory, suspend
  [factory] security_play
BT11_098: 1 effects
  [EffectTiming.OptionSkill] play_card, bounce
BT11_099: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] bounce, trash_digivolution_cards
BT11_112: 4 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, gain_keyword_evade
  [EffectTiming.OnTappedAnyone] suspend
  [EffectTiming.OnUnTappedAnyone] gain_memory (1/turn)
  [factory] security_play
BT11_004: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT11_046: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT11_047: 1 effects
  [EffectTiming.OnStartTurn] draw
BT11_049: 1 effects
  [EffectTiming.OnStartTurn] gain_memory
BT11_050: 1 effects
  [EffectTiming.OnEnterFieldAnyone] suspend (inherited) (1/turn)
BT11_052: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier
BT11_054: 3 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush (inherited) (1/turn)
BT11_055: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT11_056: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select (1/turn)
BT11_057: 1 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, suspend
BT11_058: 3 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] bounce
BT11_059: 2 effects
  [factory] change_digi_cost
  [EffectTiming.OnEndBattle] unsuspend (1/turn)
BT11_091: 3 effects
  [factory] dp_modifier_all
  [EffectTiming.BeforePayCost] suspend, cost_reduction
  [factory] security_play
BT11_102: 2 effects
  [EffectTiming.OptionSkill] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.SecuritySkill] suspend
BT11_103: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] grant_skill
  [EffectTiming.OptionSkill] no-action
BT11_104: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] change_dp, gain_keyword_rush, force_attack
  [EffectTiming.SecuritySkill] add_to_hand
BT11_006: 1 effects
  [EffectTiming.OnDiscardHand] change_dp (inherited) (1/turn)
BT11_076: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT11_077: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
BT11_078: 2 effects
  [factory] retaliation
  [factory] dp_modifier_all
BT11_079: 2 effects
  [factory] retaliation
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand
BT11_080: 2 effects
  [factory] rush
  [factory] retaliation
BT11_081: 4 effects
  [EffectTiming.OnAddHand] draw, trash_digivolution_cards (1/turn)
  [factory] save
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
  [EffectTiming.None] no-action
BT11_082: 4 effects
  [factory] alt_digivolve_req
  [factory] decoy
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory (inherited)
BT11_083: 4 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (1/turn)
  [factory] retaliation
  [EffectTiming.None] grant_skill (inherited)
BT11_084: 3 effects
  [factory] retaliation
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT11_085: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT11_086: 6 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] rush
  [factory] blocker
  [EffectTiming.None] no-action
BT11_087: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, mill
  [EffectTiming.OnMove] trash_digivolution_cards
  [EffectTiming.OnMove] effect_immunity
BT11_088: 4 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, destroy_security (1/turn)
  [EffectTiming.OnAddDigivolutionCards] trash_digivolution_cards, destroy_security (1/turn)
BT11_094: 3 effects
  [EffectTiming.OnStartTurn] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, play_card
  [factory] security_play
BT11_109: 1 effects
  [EffectTiming.OptionSkill] no-action
BT11_110: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete
BT11_001: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw (inherited)
BT11_007: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT11_008: 1 effects
  [EffectTiming.OnAttackTargetChanged] change_dp (inherited) (1/turn)
BT11_009: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
  [EffectTiming.None] no-action
BT11_010: 2 effects
  [factory] raid
  [EffectTiming.OnAttackTargetChanged] change_dp (inherited) (1/turn)
BT11_011: 2 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT11_012: 5 effects
  [factory] save
  [factory] material_save
  [EffectTiming.OnStartTurn] gain_memory
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.None] no-action
BT11_014: 2 effects
  [factory] raid
  [EffectTiming.OnAttackTargetChanged] destroy_security (inherited) (1/turn)
BT11_015: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] save
  [factory] security_attack_plus
BT11_016: 2 effects
  [EffectTiming.OnLoseSecurity] no-action (1/turn)
  [EffectTiming.OnDestroyedAnyone] play_card
BT11_017: 3 effects
  [factory] raid
  [factory] blitz
  [EffectTiming.OnAttackTargetChanged] unsuspend (1/turn)
BT11_018: 6 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_cannot_attack
  [EffectTiming.OnEndAttack] gain_memory
  [EffectTiming.None] no-action
BT11_019: 6 effects
  [factory] rush
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] dp_modifier
  [EffectTiming.None] no-action
BT11_089: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_rush
  [factory] security_play
BT11_096: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete
BT11_097: 1 effects
  [EffectTiming.OptionSkill] delete
BT11_095: 3 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
BT11_003: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT11_034: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
BT11_036: 2 effects
  [factory] change_digi_cost
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT11_038: 1 effects
  [EffectTiming.OnDestroyedAnyone] play_card
BT11_039: 1 effects
  [EffectTiming.OnEnterFieldAnyone] put_to_security (descriptive-tagged)
BT11_040: 2 effects
  [EffectTiming.OnDestroyedAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action (inherited)
BT11_041: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand, trash_digivolution_cards, change_security_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand, trash_digivolution_cards, change_security_attack
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action (inherited)
BT11_042: 3 effects
  [EffectTiming.OnEnterFieldAnyone] recovery, add_to_hand, destroy_security
  [EffectTiming.OnEnterFieldAnyone] gain_memory (1/turn)
  [factory] blocker
BT11_043: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action (inherited)
BT11_044: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
BT11_045: 2 effects
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnLoseSecurity] change_dp
BT11_100: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] change_dp
BT11_101: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] change_dp, change_security_attack
```
