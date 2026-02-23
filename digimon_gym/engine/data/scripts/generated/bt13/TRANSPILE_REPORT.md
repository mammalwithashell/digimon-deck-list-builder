# BT13 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 109
- Scripts with effects: 109
- Total effects: 283
- Factory effects: 70
- Activate effects: 213

## Per-Card Breakdown

```
BT13_005: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited)
BT13_061: 2 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] add_to_hand, reveal_and_select
BT13_062: 2 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT13_063: 1 effects
  [factory] dp_modifier
BT13_064: 2 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card
BT13_065: 2 effects
  [EffectTiming.OnDestroyedAnyone] de_digivolve
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action (inherited)
BT13_067: 2 effects
  [factory] jamming
  [factory] reboot
BT13_068: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card
BT13_069: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] play_card
  [EffectTiming.WhenPermanentWouldBeDeleted] no-action (inherited)
BT13_070: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] play_card
BT13_071: 2 effects
  [factory] blocker
  [EffectTiming.OnTappedAnyone] destroy_security (inherited) (1/turn)
BT13_072: 2 effects
  [EffectTiming.OnEnterFieldAnyone] reveal_and_select, gain_keyword_immune_dp_minus
  [EffectTiming.OnEndTurn] no-action (inherited) (1/turn)
BT13_073: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] unsuspend
BT13_074: 4 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [factory] jamming
  [factory] reboot
BT13_075: 3 effects
  [EffectTiming.OnEnterFieldAnyone] restrict_attack, gain_keyword_cannot_attack_player
  [EffectTiming.OnEnterFieldAnyone] restrict_attack, gain_keyword_cannot_attack_player
  [EffectTiming.WhenRemoveField] no-action (1/turn)
BT13_076: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] change_dp, change_security_attack (1/turn)
  [factory] blocker
BT13_077: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged)
BT13_108: 3 effects
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.OptionSkill] delete, effect_immunity
  [EffectTiming.SecuritySkill] delete
BT13_002: 1 effects
  [factory] dp_modifier
BT13_021: 2 effects
  [EffectTiming.OnAllyAttack] draw (1/turn)
  [factory] dp_modifier
BT13_023: 2 effects
  [factory] evade
  [EffectTiming.OnAllyAttack] trash_digivolution_cards (inherited)
BT13_025: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier
BT13_026: 2 effects
  [EffectTiming.OnAllyAttack] draw
  [EffectTiming.OnAllyAttack] trash_digivolution_cards (inherited)
BT13_027: 2 effects
  [factory] blocker
  [EffectTiming.OnAllyAttack] play_card
BT13_028: 2 effects
  [EffectTiming.OnDeclaration] play_card
  [EffectTiming.OnEndAttack] unsuspend, return_to_deck (inherited) (1/turn)
BT13_029: 2 effects
  [EffectTiming.OnAllyAttack] target_lock
  [EffectTiming.OnAddHand] unsuspend (inherited) (1/turn)
BT13_030: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] bounce (1/turn)
BT13_031: 3 effects
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAddHand] play_card (1/turn)
BT13_032: 2 effects
  [factory] blocker
  [EffectTiming.OnAllyAttack] play_card
BT13_033: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] unsuspend, flip_security, return_to_deck
BT13_096: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] security_play
BT13_097: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] draw, suspend
  [factory] security_play
BT13_105: 2 effects
  [EffectTiming.OptionSkill] bounce
  [EffectTiming.SecuritySkill] bounce
BT13_004: 1 effects
  [factory] dp_modifier
BT13_047: 2 effects
  [factory] blocker
  [factory] dp_modifier
BT13_048: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT13_049: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.None] cost_reduction (inherited)
BT13_050: 2 effects
  [EffectTiming.OnDeclaration] suspend, digivolve
  [EffectTiming.None] cost_reduction (inherited)
BT13_051: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_piercing
  [factory] dp_modifier
BT13_052: 2 effects
  [factory] jamming
  [factory] dp_modifier
BT13_053: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.None] cost_reduction (inherited)
BT13_054: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] security_attack_plus
BT13_055: 2 effects
  [EffectTiming.OnDeclaration] play_card
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT13_056: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, cost_reduction (1/turn)
  [EffectTiming.OnDeclaration] play_card, cost_reduction (1/turn)
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
BT13_057: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend
  [EffectTiming.OnTappedAnyone] suspend (1/turn)
BT13_058: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnAllyAttack] suspend, unsuspend
  [EffectTiming.OnEndTurn] unsuspend
BT13_059: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnTappedAnyone] suspend, unsuspend (1/turn)
BT13_060: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend_player
  [EffectTiming.OnAllyAttack] no-action
BT13_100: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [factory] security_play
BT13_107: 2 effects
  [EffectTiming.OptionSkill] bounce, add_to_hand, unsuspend
  [EffectTiming.SecuritySkill] add_to_hand, suspend
BT13_006: 1 effects
  [EffectTiming.OnDestroyedAnyone] delete, trash_from_hand (inherited)
BT13_078: 2 effects
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand
  [EffectTiming.OnEndTurn] draw, trash_from_hand (inherited) (1/turn)
BT13_079: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] trash_from_hand (inherited)
BT13_080: 4 effects
  [EffectTiming.BeforePayCost] play_card, cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card, return_to_deck
BT13_081: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] delete
  [EffectTiming.OnEndTurn] draw, trash_from_hand (inherited) (1/turn)
BT13_082: 2 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] trash_from_hand (inherited)
BT13_083: 4 effects
  [EffectTiming.BeforePayCost] play_card, cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card, return_to_deck
BT13_084: 3 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnDiscardHand] play_card (inherited) (1/turn)
BT13_085: 2 effects
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT13_086: 5 effects
  [EffectTiming.BeforePayCost] play_card, cost_reduction
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
BT13_087: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] delete
BT13_088: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, effect_immunity
  [EffectTiming.OnAllyAttack] trash_from_hand (1/turn)
BT13_089: 3 effects
  [EffectTiming.OnEndTurn] no-action
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
BT13_090: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnAllyAttack] no-action (1/turn)
BT13_091: 3 effects
  [EffectTiming.OnStartMainPhase] change_dp, delete
  [EffectTiming.OnEndAttack] unsuspend (1/turn)
  [EffectTiming.OnEndTurn] no-action (inherited)
BT13_092: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, destroy_security
  [EffectTiming.OnAllyAttack] delete, return_to_deck
BT13_102: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [factory] security_play
BT13_103: 4 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEndTurn] draw, delete, trash_from_hand (1/turn)
  [factory] security_play
BT13_109: 2 effects
  [EffectTiming.OptionSkill] delete, digivolve
  [EffectTiming.SecuritySkill] delete, trash_from_hand
BT13_001: 1 effects
  [EffectTiming.OnDestroyedAnyone] delete (inherited)
BT13_008: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] no-action (1/turn)
  [EffectTiming.OnTappedAnyone] delete (inherited) (1/turn)
BT13_009: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT13_010: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnDestroyedAnyone] draw (inherited)
BT13_011: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] draw (inherited)
BT13_012: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] recovery, play_card, destroy_security
  [EffectTiming.OnTappedAnyone] delete (inherited) (1/turn)
BT13_013: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT13_014: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] delete (inherited)
BT13_015: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] add_to_security (1/turn)
  [EffectTiming.OnDestroyedAnyone] add_to_security (inherited) (1/turn)
BT13_016: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnAllyAttack] play_card (inherited) (1/turn)
BT13_017: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] dp_modifier_all
BT13_018: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] gain_keyword_blocker
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker
  [EffectTiming.OnTappedAnyone] change_dp (1/turn)
BT13_019: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
BT13_020: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] play_card, gain_keyword_rush
  [EffectTiming.OnTappedAnyone] destroy_security (1/turn)
BT13_094: 4 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] play_card, add_temp_effect
  [factory] security_play
BT13_095: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnTappedAnyone] gain_memory, change_dp
  [factory] security_play
BT13_104: 1 effects
  [EffectTiming.OptionSkill] change_dp, play_card
BT13_111: 5 effects
  [EffectTiming.None] cost_reduction
  [factory] rush
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
BT13_007: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnStartMainPhase] no-action
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT13_093: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnDestroyedAnyone] no-action
BT13_110: 3 effects
  [EffectTiming.OptionSkill] draw
  [factory] delay
  [EffectTiming.OnDeclaration] play_card, gain_keyword_rush
BT13_112: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
BT13_003: 1 effects
  [EffectTiming.OnLoseSecurity] gain_keyword_jamming (inherited) (1/turn)
BT13_034: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT13_035: 2 effects
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] reboot
BT13_036: 2 effects
  [EffectTiming.OnLoseSecurity] gain_memory (1/turn)
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT13_037: 2 effects
  [EffectTiming.OnAllyAttack] change_dp, destroy_security
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT13_038: 2 effects
  [EffectTiming.OnAllyAttack] change_security_attack, destroy_security
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT13_039: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] reboot
BT13_040: 3 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.WhenRemoveField] draw, play_card
BT13_041: 2 effects
  [factory] barrier
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT13_042: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] reboot
BT13_043: 2 effects
  [factory] barrier
  [factory] barrier
BT13_044: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] change_dp, destroy_security
  [EffectTiming.OnLoseSecurity] play_card (1/turn)
BT13_045: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
BT13_046: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, add_to_security
  [EffectTiming.OnEnterFieldAnyone] gain_memory, add_to_security
  [EffectTiming.OnAllyAttack] change_dp, destroy_security, unsuspend (1/turn)
BT13_098: 4 effects
  [EffectTiming.OnDiscardSecurity] play_card
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] suspend, digivolve
  [factory] security_play
BT13_099: 3 effects
  [EffectTiming.OnTappedAnyone] change_dp (1/turn)
  [EffectTiming.OnEndTurn] gain_keyword_blocker (1/turn)
  [factory] security_play
BT13_101: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, suspend
  [factory] security_play
BT13_106: 2 effects
  [EffectTiming.OnDiscardSecurity] no-action
  [EffectTiming.OptionSkill] change_dp
```
