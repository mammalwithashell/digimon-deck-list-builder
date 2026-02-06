# BT14 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 94
- Scripts with effects: 91
- Total effects: 198
- Factory effects: 31
- Activate effects: 167

## Per-Card Breakdown

```
BT14_005: 1 effects
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT14_056: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT14_057: 1 effects
  [factory] blocker
BT14_058: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [factory] blocker
BT14_059: 1 effects
  [factory] blocker
BT14_060: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT14_061: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory
  [EffectTiming.OnEnterFieldAnyone] gain_memory
BT14_062: 0 effects
BT14_063: 2 effects
  [EffectTiming.OnDestroyedAnyone] play_card, add_to_hand, reveal_and_select
  [factory] blocker
BT14_064: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] play_card, reveal_and_select (inherited) (1/turn)
BT14_065: 2 effects
  [EffectTiming.OnEnterFieldAnyone] reveal_and_select, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] reveal_and_select, de_digivolve
BT14_066: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card, trash_from_hand
BT14_067: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] delete, reveal_and_select
BT14_068: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndTurn] play_card, reveal_and_select (1/turn)
BT14_086: 6 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] mind_link
  [factory] jamming
  [factory] reboot
  [EffectTiming.OnEndTurn] play_card (inherited)
BT14_097: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] digivolve
  [EffectTiming.SecuritySkill] no-action
BT14_098: 1 effects
  [EffectTiming.OptionSkill] delete, de_digivolve
BT14_002: 1 effects
  [factory] jamming
BT14_019: 1 effects
  [EffectTiming.OnAllyAttack] trash_digivolution_cards (inherited) (1/turn)
BT14_020: 2 effects
  [EffectTiming.OnStartMainPhase] trash_digivolution_cards
  [EffectTiming.WhenPermanentWouldBeDeleted] play_card (inherited)
BT14_021: 0 effects
BT14_022: 1 effects
  [EffectTiming.OnAllyAttack] bounce, trash_digivolution_cards
BT14_023: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnAllyAttack] no-action (1/turn)
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT14_026: 3 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] bounce, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] bounce, trash_digivolution_cards
BT14_027: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT14_028: 2 effects
  [factory] blocker
  [EffectTiming.OnDigivolutionCardDiscarded] no-action (1/turn)
BT14_029: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT14_030: 3 effects
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnPermamemtReturnedToHand] recovery (1/turn)
BT14_083: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory, suspend
  [factory] security_play
BT14_091: 1 effects
  [EffectTiming.OptionSkill] trash_digivolution_cards
BT14_092: 2 effects
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.SecuritySkill] add_to_hand
BT14_004: 1 effects
  [EffectTiming.OnTappedAnyone] change_dp (inherited) (1/turn)
BT14_042: 1 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, add_to_hand, reveal_and_select
BT14_043: 1 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
BT14_044: 3 effects
  [EffectTiming.OnStartMainPhase] no-action
  [EffectTiming.OnStartMainPhase] no-action
  [EffectTiming.None] cost_reduction (inherited)
BT14_046: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.None] cost_reduction (inherited)
BT14_047: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT14_048: 2 effects
  [EffectTiming.OnAllyAttack] digivolve
  [factory] dp_modifier
BT14_049: 3 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT14_050: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT14_051: 1 effects
  [EffectTiming.OnEndTurn] suspend, add_to_hand, reveal_and_select (1/turn)
BT14_052: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] dp_modifier
BT14_053: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnTappedAnyone] no-action (1/turn)
BT14_054: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] no-action
BT14_085: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnTappedAnyone] gain_memory, suspend
  [factory] security_play
BT14_095: 2 effects
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OptionSkill] no-action
BT14_096: 1 effects
  [EffectTiming.OptionSkill] no-action
BT14_006: 1 effects
  [EffectTiming.OnDiscardHand] digivolve (inherited)
BT14_070: 1 effects
  [EffectTiming.OnDiscardHand] gain_memory (inherited) (1/turn)
BT14_071: 2 effects
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT14_072: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnAllyAttack] trash_from_hand, add_to_hand
BT14_073: 2 effects
  [EffectTiming.OnDiscardHand] gain_memory (1/turn)
  [EffectTiming.OnDiscardHand] gain_memory (inherited) (1/turn)
BT14_074: 2 effects
  [EffectTiming.OnAllyAttack] draw, gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT14_075: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [factory] dp_modifier
  [EffectTiming.OnDestroyedAnyone] trash_from_hand
BT14_076: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card
BT14_077: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDiscardLibrary] gain_memory (1/turn)
BT14_078: 2 effects
  [EffectTiming.OnEndTurn] draw, delete, add_to_hand
  [EffectTiming.OnDestroyedAnyone] delete, trash_from_hand
BT14_079: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] no-action (inherited) (1/turn)
BT14_080: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
  [EffectTiming.OnAllyAttack] no-action (1/turn)
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT14_081: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] no-action (1/turn)
  [EffectTiming.None] no-action
BT14_087: 6 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] mind_link
  [factory] blocker
  [factory] alliance
  [EffectTiming.OnEndTurn] play_card (inherited)
BT14_099: 1 effects
  [EffectTiming.OptionSkill] no-action
BT14_100: 2 effects
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OptionSkill] delete
BT14_001: 1 effects
  [EffectTiming.OnLoseSecurity] draw (inherited) (1/turn)
BT14_007: 2 effects
  [EffectTiming.OnStartMainPhase] digivolve
  [factory] dp_modifier
BT14_008: 1 effects
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT14_012: 2 effects
  [EffectTiming.OnAllyAttack] gain_memory, change_dp
  [factory] dp_modifier
BT14_013: 2 effects
  [EffectTiming.OnStartMainPhase] cost_reduction
  [EffectTiming.OnEndTurn] no-action (inherited) (1/turn)
BT14_014: 3 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
BT14_015: 1 effects
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT14_016: 1 effects
  [factory] raid
BT14_017: 2 effects
  [EffectTiming.None] no-action
  [factory] dp_modifier
BT14_018: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.BeforePayCost] recovery, delete
  [EffectTiming.WhenRemoveField] recovery, delete
BT14_082: 3 effects
  [EffectTiming.OnStartMainPhase] change_dp
  [EffectTiming.OnLoseSecurity] gain_memory, suspend
  [factory] security_play
BT14_089: 1 effects
  [EffectTiming.OptionSkill] delete
BT14_090: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] digivolve
  [EffectTiming.SecuritySkill] play_card, trash_from_hand, add_to_hand
BT14_101: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
BT14_088: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] suspend
  [factory] security_play
BT14_003: 1 effects
  [EffectTiming.OnAddSecurity] draw (inherited) (1/turn)
BT14_031: 1 effects
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT14_032: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, add_to_security
  [EffectTiming.OnDestroyedAnyone] change_dp (inherited)
BT14_033: 2 effects
  [EffectTiming.OnStartMainPhase] play_card, trash_from_hand, add_to_security
  [EffectTiming.OnAddSecurity] gain_memory (inherited) (1/turn)
BT14_034: 2 effects
  [factory] security_play
  [EffectTiming.OnDestroyedAnyone] change_dp (inherited)
BT14_035: 0 effects
BT14_036: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT14_037: 3 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnEnterFieldAnyone] recovery
BT14_038: 3 effects
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.OnDestroyedAnyone] add_to_security (inherited)
BT14_039: 3 effects
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] gain_memory
  [factory] security_attack_plus
BT14_040: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_security
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_security
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand (1/turn)
BT14_041: 2 effects
  [EffectTiming.OnEnterFieldAnyone] recovery
  [EffectTiming.OnAddSecurity] change_dp (1/turn)
BT14_084: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, add_to_security
  [EffectTiming.OnAddSecurity] gain_memory, suspend
  [factory] security_play
BT14_093: 2 effects
  [EffectTiming.OptionSkill] recovery, play_card
  [EffectTiming.SecuritySkill] play_card, trash_from_hand, add_to_hand
BT14_094: 1 effects
  [EffectTiming.OptionSkill] change_dp
BT14_102: 3 effects
  [EffectTiming.OnAllyAttack] change_dp
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.OnDestroyedAnyone] trash_from_hand, add_to_security (inherited)
```
