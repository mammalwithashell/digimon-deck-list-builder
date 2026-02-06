# BT24 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 102
- Scripts with effects: 101
- Total effects: 320
- Factory effects: 50
- Activate effects: 270

## Per-Card Breakdown

```
BT24_005: 1 effects
  [EffectTiming.OnAddDigivolutionCards] reveal_and_select (inherited) (1/turn)
BT24_052: 3 effects
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT24_053: 2 effects
  [factory] blocker
  [factory] blocker
BT24_054: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnTappedAnyone] no-action (inherited) (1/turn)
BT24_055: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] no-action (inherited) (1/turn)
BT24_056: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] delete
BT24_057: 4 effects
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] de_digivolve
BT24_058: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] reboot
BT24_059: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card, reveal_and_select
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_060: 3 effects
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select
  [EffectTiming.OnAddDigivolutionCards] no-action
  [EffectTiming.WhenRemoveField] play_card (inherited) (1/turn)
BT24_061: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] de_digivolve (inherited) (1/turn)
BT24_062: 6 effects
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.None] no-action
  [EffectTiming.OnEndAttack] no-action
  [EffectTiming.OnEndTurn] no-action
  [EffectTiming.None] no-action (inherited)
BT24_063: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT24_064: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnTappedAnyone] de_digivolve (1/turn)
BT24_065: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.WhenRemoveField] play_card, trash_from_hand (1/turn)
BT24_086: 7 effects
  [EffectTiming.None] no-action
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] mind_link
  [factory] reboot
  [factory] alliance
  [EffectTiming.OnEndTurn] play_card (inherited)
BT24_002: 1 effects
  [EffectTiming.OnEndTurn] no-action (inherited) (1/turn)
BT24_019: 1 effects
  [factory] jamming
BT24_020: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnUnTappedAnyone] draw (inherited) (1/turn)
BT24_021: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_022: 4 effects
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUnTappedAnyone] draw (inherited) (1/turn)
BT24_023: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] jamming
BT24_024: 2 effects
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand, cost_reduction (1/turn)
BT24_025: 3 effects
  [EffectTiming.OnUnTappedAnyone] digivolve
  [EffectTiming.OnEndTurn] no-action (1/turn)
  [factory] jamming
BT24_026: 4 effects
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_027: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT24_028: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnUnTappedAnyone] digivolve
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
BT24_029: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndAttack] play_card (1/turn)
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
BT24_030: 6 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] no-action (1/turn)
  [EffectTiming.WhenRemoveField] suspend
BT24_090: 5 effects
  [EffectTiming.None] no-action
  [factory] alliance
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT24_091: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] bounce (1/turn)
BT24_004: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT24_042: 1 effects
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_043: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_044: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, add_to_hand, reveal_and_select
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT24_045: 4 effects
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_046: 4 effects
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_047: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT24_048: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action (inherited) (1/turn)
BT24_049: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndBattle] no-action (inherited) (1/turn)
BT24_050: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] play_card, trash_from_hand (inherited) (1/turn)
BT24_051: 7 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.None] no-action
BT24_085: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEndTurn] suspend, trash_from_hand
  [factory] security_play
BT24_094: 5 effects
  [EffectTiming.None] no-action
  [factory] alliance
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT24_095: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] bounce (1/turn)
BT24_006: 1 effects
  [EffectTiming.WhenLinked] draw, trash_from_hand (inherited) (1/turn)
BT24_007: 1 effects
  [EffectTiming.OnDiscardHand] play_card (inherited) (1/turn)
BT24_066: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_067: 1 effects
  [EffectTiming.WhenLinked] play_card, trash_from_hand (1/turn)
BT24_068: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_069: 5 effects
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] blocker
  [factory] dp_modifier
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_070: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_071: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
BT24_072: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] security_attack_plus
BT24_073: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_074: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT24_075: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] security_attack_plus
BT24_076: 4 effects
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT24_077: 5 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.WhenLinked] delete
BT24_078: 2 effects
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
BT24_079: 3 effects
  [EffectTiming.None] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] no-action (1/turn)
BT24_080: 6 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEndTurn] digivolve
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
BT24_081: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDestroyedAnyone] play_card
BT24_087: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.WhenLinked] draw, suspend, play_card, trash_from_hand
  [factory] security_play
BT24_088: 3 effects
  [EffectTiming.OnStartTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [factory] security_play
BT24_096: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OptionSkill] no-action
BT24_097: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] delete
  [EffectTiming.OnAllyAttack] delete (1/turn)
BT24_098: 3 effects
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.SecuritySkill] play_card, trash_from_hand, add_to_hand
BT24_099: 4 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.SecuritySkill] no-action
BT24_001: 1 effects
  [EffectTiming.OnLoseSecurity] delete (inherited) (1/turn)
BT24_008: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT24_009: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_010: 3 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] de_digivolve
  [factory] raid
BT24_011: 2 effects
  [factory] raid
  [factory] raid
BT24_012: 3 effects
  [factory] blocker
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
BT24_013: 4 effects
  [EffectTiming.OnDiscardHand] draw
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnDiscardHand] digivolve (inherited) (1/turn)
BT24_014: 2 effects
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
BT24_015: 4 effects
  [factory] blocker
  [EffectTiming.SecuritySkill] play_card
  [EffectTiming.OnAttackTargetChanged] delete (1/turn)
  [EffectTiming.OnAllyAttack] delete (inherited) (1/turn)
BT24_016: 4 effects
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] play_card, trash_from_hand (inherited) (1/turn)
BT24_017: 2 effects
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
BT24_018: 5 effects
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] delete (1/turn)
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT24_082: 3 effects
  [EffectTiming.OnStartMainPhase] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
  [factory] security_play
BT24_083: 3 effects
  [EffectTiming.OnStartTurn] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] security_play
BT24_089: 2 effects
  [EffectTiming.OptionSkill] play_card, trash_from_hand
  [EffectTiming.OnTappedAnyone] digivolve
BT24_100: 2 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
BT24_102: 3 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory, suspend
  [EffectTiming.OnEndTurn] suspend
  [factory] security_play
BT24_003: 1 effects
  [EffectTiming.OnLoseSecurity] digivolve (inherited) (1/turn)
BT24_031: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] recovery, add_to_hand (inherited) (1/turn)
BT24_032: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenLinked] no-action
BT24_033: 0 effects
BT24_034: 3 effects
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT24_035: 2 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
BT24_036: 4 effects
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] change_dp
BT24_037: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] play_card (1/turn)
  [EffectTiming.WhenRemoveField] play_card (inherited) (1/turn)
BT24_038: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] change_dp (1/turn)
  [EffectTiming.WhenLinked] change_dp
BT24_039: 3 effects
  [EffectTiming.SecuritySkill] play_card
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] recovery (inherited)
BT24_040: 5 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT24_041: 5 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] no-action
BT24_084: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnLoseSecurity] suspend, digivolve
  [factory] security_play
BT24_092: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT24_093: 3 effects
  [EffectTiming.OptionSkill] recovery, add_to_hand
  [EffectTiming.OnLoseSecurity] add_to_security
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT24_101: 4 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnLoseSecurity] no-action (1/turn)
  [EffectTiming.WhenRemoveField] no-action (1/turn)
```
