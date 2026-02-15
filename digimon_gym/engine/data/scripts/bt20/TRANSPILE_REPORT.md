# BT20 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 103
- Scripts with effects: 103
- Total effects: 391
- Factory effects: 138
- Activate effects: 253

## Per-Card Breakdown

```
BT20_005: 1 effects
  [EffectTiming.OnSecurityCheck] gain_keyword_jamming (inherited)
BT20_046: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [factory] dp_modifier
BT20_047: 2 effects
  [factory] blocker
  [factory] reboot
BT20_048: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT20_049: 3 effects
  [EffectTiming.OnEnterFieldAnyone] restrict_attack, gain_keyword_cannot_attack_player
  [EffectTiming.OnEnterFieldAnyone] restrict_attack, gain_keyword_cannot_attack_player
  [factory] reboot
BT20_050: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] flip_security
  [EffectTiming.OnEndAttack] draw (1/turn)
  [factory] dp_modifier
BT20_051: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [factory] dp_modifier
BT20_052: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] flip_security
  [EffectTiming.OnSecurityCheck] add_to_security
  [EffectTiming.None] target_lock (inherited)
BT20_053: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card, trash_from_hand
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT20_054: 3 effects
  [factory] blocker
  [EffectTiming.WhenRemoveField] play_card
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT20_055: 4 effects
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve, flip_security
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve, flip_security
  [EffectTiming.OnSecurityCheck] add_to_security
BT20_056: 5 effects
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] recovery, digivolve
  [EffectTiming.OnEnterFieldAnyone] recovery, digivolve
  [EffectTiming.OnLoseSecurity] change_dp (1/turn)
  [EffectTiming.WhenRemoveField] destroy_security (inherited) (1/turn)
BT20_057: 6 effects
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
BT20_058: 4 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] play_card
  [EffectTiming.None] no-action
BT20_059: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [factory] blocker
  [factory] reboot
  [factory] blocker
  [factory] reboot
BT20_060: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] change_dp, destroy_security
  [EffectTiming.OnEnterFieldAnyone] change_dp, destroy_security
  [EffectTiming.OnLoseSecurity] gain_memory (1/turn)
BT20_086: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnStartMainPhase] trash_from_hand, flip_security
  [factory] security_play
BT20_087: 4 effects
  [EffectTiming.None] no-action
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] suspend, digivolve
  [factory] security_play
BT20_095: 4 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.OnDestroyedAnyone] digivolve
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT20_002: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT20_022: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT20_023: 4 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [factory] dp_modifier
BT20_024: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT20_025: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [factory] security_attack_plus
BT20_026: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.None] target_lock (inherited)
BT20_027: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnLoseSecurity] unsuspend (1/turn)
  [EffectTiming.WhenRemoveField] suspend (inherited) (1/turn)
BT20_028: 7 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [factory] reboot
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] play_card (1/turn)
  [EffectTiming.OnAllyAttack] play_card (1/turn)
  [EffectTiming.OnEnterFieldAnyone] de_digivolve (1/turn)
BT20_102: 6 effects
  [factory] alt_digivolve_req
  [factory] raid
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete, bounce
  [EffectTiming.OnEnterFieldAnyone] delete, bounce
  [EffectTiming.OnEndTurn] gain_keyword_rush, force_attack (1/turn)
BT20_004: 1 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve (inherited) (1/turn)
BT20_038: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
BT20_039: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
BT20_040: 4 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [factory] dp_modifier
BT20_041: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT20_042: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT20_043: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend, force_attack
  [EffectTiming.OnEndTurn] play_card, trash_from_hand, force_attack
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT20_044: 6 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] suspend, force_attack
  [EffectTiming.OnEnterFieldAnyone] suspend, force_attack
  [EffectTiming.OnEndBattle] delete (1/turn)
  [EffectTiming.OnEndBattle] delete (inherited) (1/turn)
BT20_045: 6 effects
  [EffectTiming.None] jogress_condition
  [factory] raid
  [factory] blocker
  [factory] evade
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
BT20_085: 3 effects
  [EffectTiming.OnStartMainPhase] play_card, trash_from_hand
  [EffectTiming.OnEndTurn] change_dp, suspend
  [factory] security_play
BT20_101: 7 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [factory] vortex
  [factory] blocker
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
  [EffectTiming.OnEnterFieldAnyone] suspend, bounce
  [EffectTiming.OnEnterFieldAnyone] suspend, bounce
BT20_006: 1 effects
  [EffectTiming.OnDestroyedAnyone] add_to_hand (inherited)
BT20_061: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT20_062: 2 effects
  [factory] retaliation
  [EffectTiming.OnDestroyedAnyone] delete, trash_from_hand (inherited)
BT20_063: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT20_064: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT20_065: 3 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] add_temp_effect (descriptive-tagged)
  [factory] retaliation
BT20_066: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, trash_from_hand
  [factory] retaliation
BT20_067: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] delete, trash_from_hand (inherited)
BT20_068: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
BT20_069: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, gain_keyword_blocker, gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, gain_keyword_blocker, gain_keyword_retaliation
  [factory] dp_modifier
BT20_070: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand
  [factory] dp_modifier
BT20_071: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand, gain_keyword_raid
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand, gain_keyword_raid
  [EffectTiming.OnAddDigivolutionCards] delete
  [EffectTiming.None] disable_effect (descriptive-tagged) (inherited)
BT20_072: 3 effects
  [factory] execute
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT20_073: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] de_digivolve (inherited)
BT20_074: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.None] disable_effect (descriptive-tagged) (inherited)
BT20_075: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand, gain_keyword_raid, gain_keyword_piercing
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_from_hand, gain_keyword_raid, gain_keyword_piercing
BT20_076: 3 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] delete, digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, digivolve
BT20_077: 6 effects
  [factory] blast_digivolve
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] play_card, trash_from_hand
  [factory] blocker
  [factory] dp_modifier_all
BT20_078: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] de_digivolve (1/turn)
  [EffectTiming.OnDestroyedAnyone] delete
BT20_079: 6 effects
  [factory] security_attack_plus
  [factory] execute
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
BT20_080: 5 effects
  [factory] alt_digivolve_req
  [factory] scapegoat
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAddDigivolutionCards] force_attack (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] destroy_security (inherited) (1/turn)
BT20_081: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
  [EffectTiming.OnAllyAttack] destroy_security
BT20_082: 5 effects
  [factory] blocker
  [factory] reboot
  [factory] security_attack_plus
  [EffectTiming.WhenRemoveField] return_to_deck
  [EffectTiming.OnEndTurn] delete (1/turn)
BT20_088: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDestroyedAnyone] suspend, digivolve
  [factory] security_play
BT20_089: 7 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] mind_link
  [factory] alliance
  [factory] barrier
  [EffectTiming.OnEndTurn] play_card (inherited)
  [factory] security_play
BT20_090: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEndTurn] suspend, force_attack
  [factory] security_play
BT20_096: 3 effects
  [EffectTiming.OnDeclaration] delete, return_to_deck
  [EffectTiming.OptionSkill] delete, trash_from_hand
  [EffectTiming.SecuritySkill] delete
BT20_097: 4 effects
  [EffectTiming.OptionSkill] digivolve
  [factory] delay
  [EffectTiming.WhenRemoveField] play_card, add_to_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand, add_to_hand
BT20_098: 2 effects
  [EffectTiming.OptionSkill] play_card, return_to_deck, gain_keyword_rush, gain_keyword_blocker
  [EffectTiming.SecuritySkill] play_card
BT20_001: 1 effects
  [factory] dp_modifier
BT20_007: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [factory] dp_modifier
BT20_008: 2 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [factory] dp_modifier_all
BT20_009: 2 effects
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [factory] dp_modifier
BT20_010: 3 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [factory] dp_modifier
BT20_011: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, trash_from_hand
  [factory] dp_modifier
BT20_012: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] digivolve
  [factory] dp_modifier
BT20_013: 2 effects
  [EffectTiming.OnDeclaration] play_card, trash_from_hand, cost_reduction (1/turn)
  [factory] dp_modifier_all
BT20_014: 4 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndTurn] digivolve, suspend
  [factory] alliance
BT20_015: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card, trash_from_hand, change_security_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, play_card, trash_from_hand, change_security_attack
  [EffectTiming.None] disable_effect (descriptive-tagged) (inherited)
BT20_016: 5 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_piercing, force_attack
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_piercing, force_attack
  [EffectTiming.WhenPermanentWouldBeDeleted] play_card, trash_from_hand
  [factory] security_attack_plus
BT20_017: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] delete, force_attack (1/turn)
BT20_017_token: 3 effects
  [factory] reboot
  [factory] blocker
  [factory] decoy
BT20_018: 4 effects
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, digivolve
  [EffectTiming.OnLoseSecurity] delete (1/turn)
  [EffectTiming.OnAllyAttack] destroy_security (inherited) (1/turn)
BT20_019: 6 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action (inherited)
BT20_020: 4 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] destroy_security, play_restriction
  [EffectTiming.OnLoseSecurity] delete (1/turn)
BT20_021: 6 effects
  [EffectTiming.None] jogress_condition
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand (1/turn)
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand (1/turn)
  [EffectTiming.OnAllyAttack] trash_from_hand (1/turn)
  [EffectTiming.OnAllyAttack] unsuspend (1/turn)
BT20_093: 4 effects
  [EffectTiming.OptionSkill] play_card, trash_from_hand, cost_reduction
  [factory] delay
  [EffectTiming.WhenRemoveField] play_card, trash_from_hand
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT20_094: 4 effects
  [EffectTiming.OptionSkill] play_card, cost_reduction
  [factory] delay
  [EffectTiming.OnLoseSecurity] play_card
  [EffectTiming.SecuritySkill] play_card, trash_from_hand, add_to_hand
BT20_083: 5 effects
  [EffectTiming.None] no-action
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnLoseSecurity] suspend, play_card (inherited)
BT20_084: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] add_to_security
BT20_091: 3 effects
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, suspend
  [EffectTiming.WhenRemoveField] play_card, trash_from_hand (1/turn)
  [factory] security_play
BT20_092: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [EffectTiming.OnStartMainPhase] delete, play_card
  [factory] security_play
BT20_099: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] play_card, trash_from_hand, cost_reduction
  [EffectTiming.SecuritySkill] gain_memory, add_to_hand
  [EffectTiming.OnEndTurn] change_dp, destroy_security (inherited)
BT20_100: 4 effects
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.SecuritySkill] play_card, trash_from_hand
BT20_003: 1 effects
  [EffectTiming.OnEndTurn] no-action (inherited) (1/turn)
BT20_029: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT20_030: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT20_031: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnEnterFieldAnyone] change_dp
BT20_032: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] recovery, add_to_hand, destroy_security
  [EffectTiming.OnEnterFieldAnyone] recovery, add_to_hand, destroy_security
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT20_033: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, disable_effect
  [EffectTiming.OnEnterFieldAnyone] change_dp, disable_effect
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT20_034: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAddDigivolutionCards] disable_effect (descriptive-tagged)
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT20_035: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnAddDigivolutionCards] force_attack (descriptive-tagged)
  [EffectTiming.OnLoseSecurity] recovery (inherited) (1/turn)
BT20_036: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnEnterFieldAnyone] change_dp, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] change_dp, de_digivolve
  [EffectTiming.OnEndTurn] play_card, trash_from_hand, force_attack
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
BT20_037: 4 effects
  [EffectTiming.None] jogress_condition
  [factory] partition
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend_player, disable_effect
```
