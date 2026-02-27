# BT22 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 102
- Scripts with effects: 102
- Total effects: 398
- Factory effects: 156
- Activate effects: 242

## Per-Card Breakdown

```
BT22_005: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw (inherited) (1/turn)
BT22_053: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT22_054: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAddDigivolutionCards] no-action (1/turn)
  [EffectTiming.OnDeclaration] draw (inherited) (1/turn)
BT22_055: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [factory] blocker
BT22_056: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [factory] dp_modifier
BT22_057: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT22_058: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] gain_keyword_cannot_return_to_deck, gain_keyword_cannot_return_to_hand, grant_bounce_immunity (1/turn)
  [EffectTiming.WhenLinked] de_digivolve
BT22_059: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_immune_dp_minus, gain_keyword_cannot_return_to_deck, gain_keyword_cannot_return_to_hand, grant_bounce_immunity
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_immune_dp_minus, gain_keyword_cannot_return_to_deck, gain_keyword_cannot_return_to_hand, grant_bounce_immunity
  [EffectTiming.OnDestroyedAnyone] play_token (descriptive-tagged) (inherited) (1/turn)
BT22_060: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged) (inherited) (1/turn)
BT22_061: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [factory] fragment
  [EffectTiming.OnEnterFieldAnyone] bounce, trash_digivolution_cards, de_digivolve (1/turn)
  [EffectTiming.OnAllyAttack] bounce, trash_digivolution_cards, de_digivolve (1/turn)
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
BT22_062: 4 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged) (inherited) (1/turn)
BT22_063: 8 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
BT22_064: 5 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnAllyAttack] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] delete (1/turn)
BT22_065: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnDestroyedAnyone] digivolve (1/turn)
BT22_066: 7 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [factory] blocker
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] unsuspend, suspend
  [EffectTiming.OnEnterFieldAnyone] unsuspend, suspend
  [EffectTiming.OnTappedAnyone] de_digivolve (1/turn)
BT22_067: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [factory] raid
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] force_attack (descriptive-tagged)
  [EffectTiming.OnAllyAttack] play_card, reveal_and_select (1/turn)
BT22_090: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEndTurn] digivolve (1/turn)
  [factory] security_play
BT22_091: 4 effects
  [factory] security_play
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] suspend, redirect_attack
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
BT22_099: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] add_to_hand, reveal_and_select
  [factory] delay
BT22_100: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [factory] dp_modifier_all
  [EffectTiming.SecuritySkill] play_card
BT22_101: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnDestroyedAnyone] suspend, add_to_hand
  [EffectTiming.OnUnTappedAnyone] digivolve (1/turn)
  [factory] security_play
BT22_001: 1 effects
  [EffectTiming.OnAddDigivolutionCards] draw (inherited) (1/turn)
BT22_016: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenLinked] trash_digivolution_cards
BT22_017: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndTurn] play_card (inherited)
BT22_018: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, gain_keyword_cannot_be_deleted_by_battle
  [factory] jamming
BT22_019: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.WhenRemoveField] suspend (inherited) (1/turn)
BT22_020: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] draw (1/turn)
  [factory] jamming
BT22_021: 4 effects
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [factory] jamming
BT22_022: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.WhenRemoveField] suspend (inherited) (1/turn)
BT22_023: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEndTurn] unsuspend (1/turn)
  [EffectTiming.OnTappedAnyone] unsuspend (inherited) (1/turn)
BT22_024: 3 effects
  [factory] decode
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnEndAttack] play_card (inherited) (1/turn)
BT22_025: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] play_card, bounce
  [EffectTiming.OnEnterFieldAnyone] play_card, bounce
  [EffectTiming.OnAllyAttack] unsuspend (1/turn)
BT22_026: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnEnterFieldAnyone] bounce, digivolve
  [EffectTiming.OnAllyAttack] unsuspend (inherited) (1/turn)
BT22_027: 4 effects
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAddDigivolutionCards] bounce (1/turn)
BT22_028: 5 effects
  [factory] decode
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] bounce, unsuspend (1/turn)
  [EffectTiming.OnAllyAttack] bounce, unsuspend (1/turn)
BT22_085: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnAllyAttack] gain_keyword_jamming
  [factory] security_play
BT22_086: 3 effects
  [EffectTiming.OnStartMainPhase] play_card, bounce
  [EffectTiming.OnAddDigivolutionCards] draw, suspend
  [factory] security_play
BT22_096: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnTappedAnyone] digivolve
BT22_004: 1 effects
  [EffectTiming.OnAddDigivolutionCards] digivolve (inherited) (1/turn)
BT22_043: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAddDigivolutionCards] play_card (1/turn)
  [EffectTiming.OnDeclaration] draw (inherited) (1/turn)
BT22_044: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAddDigivolutionCards] gain_memory (1/turn)
  [EffectTiming.OnDeclaration] draw (inherited) (1/turn)
BT22_045: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_blocker
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_blocker
BT22_046: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier
BT22_047: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT22_048: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_piercing
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_piercing
  [factory] dp_modifier
BT22_049: 2 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEndTurn] digivolve (1/turn)
BT22_050: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.WhenLinked] suspend
BT22_051: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce, suspend
  [EffectTiming.OnEnterFieldAnyone] bounce, suspend
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
BT22_052: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.WhenRemoveField] gain_memory (1/turn)
BT22_006: 1 effects
  [EffectTiming.OnAddDigivolutionCards] draw, trash_from_hand (inherited) (1/turn)
BT22_068: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT22_069: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDeclaration] draw (inherited) (1/turn)
BT22_070: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] digivolve
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
BT22_071: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] add_to_hand (inherited)
BT22_072: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.WhenPermanentWouldBeDeleted] trash_digivolution_cards (inherited) (1/turn)
BT22_073: 4 effects
  [factory] alt_digivolve_req
  [factory] jamming
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand, effect_immunity
  [EffectTiming.WhenPermanentWouldBeDeleted] trash_digivolution_cards (inherited) (1/turn)
BT22_074: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] force_attack (descriptive-tagged) (1/turn)
  [EffectTiming.OnDestroyedAnyone] draw, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT22_075: 6 effects
  [factory] alt_digivolve_req
  [factory] retaliation
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenRemoveField] play_card (1/turn)
  [factory] scapegoat
BT22_076: 6 effects
  [factory] alt_digivolve_req
  [factory] change_digi_cost
  [factory] security_attack_plus
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, put_to_security (1/turn)
  [EffectTiming.OnAllyAttack] trash_digivolution_cards, put_to_security (1/turn)
BT22_077: 5 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, bounce
  [EffectTiming.OnEndTurn] unsuspend (1/turn)
  [EffectTiming.OnEndTurn] unsuspend (inherited) (1/turn)
BT22_078: 5 effects
  [factory] alt_digivolve_req
  [factory] rush
  [EffectTiming.None] grant_skill
  [EffectTiming.OnAllyAttack] delete (1/turn)
  [EffectTiming.None] no-action
BT22_092: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
  [factory] security_play
BT22_102: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnAllyAttack] suspend, digivolve
  [factory] security_play
BT22_008: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand
  [EffectTiming.OnEndTurn] play_card (inherited)
BT22_009: 5 effects
  [factory] alt_digivolve_req
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenLinked] delete
BT22_010: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] gain_keyword_raid, gain_keyword_piercing, force_attack (1/turn)
  [factory] dp_modifier
BT22_011: 4 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnDeclaration] play_card, force_attack (1/turn)
  [factory] alliance
BT22_012: 4 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] security_attack_plus
BT22_013: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, digivolve
  [EffectTiming.OnAllyAttack] destroy_security (inherited) (1/turn)
BT22_014: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [factory] raid
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] unsuspend, force_attack
  [EffectTiming.OnAttackTargetChanged] change_dp, gain_keyword_piercing (1/turn)
BT22_015: 8 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] jogress_condition
  [factory] blocker
  [factory] decode
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnEnterFieldAnyone] bounce, force_attack
BT22_083: 4 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAttackTargetChanged] suspend, effect_immunity
  [EffectTiming.OnAttackTargetChanged] no-action (inherited) (1/turn)
BT22_084: 5 effects
  [factory] set_memory_3
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier_all
  [factory] security_play
BT22_007: 4 effects
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
  [EffectTiming.OnRemovedField] no-action
BT22_079: 3 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.None] cost_reduction
BT22_080: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnSecurityCheck] play_card (1/turn)
  [EffectTiming.None] cost_reduction
BT22_081: 5 effects
  [factory] alt_digivolve_req
  [factory] raid
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.OnEnterFieldAnyone] effect_immunity
  [EffectTiming.WhenRemoveField] play_card
BT22_082: 5 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.WhenRemoveField] play_card
BT22_093: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, digivolve
  [factory] security_play
BT22_094: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.BeforePayCost] cost_reduction
  [factory] security_play
BT22_095: 6 effects
  [factory] security_play
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, suspend
  [EffectTiming.OnDeclaration] no-action
  [factory] rush
  [factory] alliance
  [factory] scapegoat
BT22_002: 1 effects
  [EffectTiming.OnDestroyedAnyone] draw (inherited) (1/turn)
BT22_003: 1 effects
  [EffectTiming.WhenLinked] change_dp (inherited) (1/turn)
BT22_029: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker
  [EffectTiming.OnDestroyedAnyone] gain_keyword_blocker
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT22_030: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenLinked] play_card (1/turn)
  [EffectTiming.OnAllyAttack] change_dp (1/turn)
BT22_031: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.None] cost_reduction (inherited)
BT22_032: 2 effects
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT22_033: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] play_card (1/turn)
  [EffectTiming.OnAllyAttack] play_card
BT22_034: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardSecurity] play_card
  [EffectTiming.OnEnterFieldAnyone] destroy_security
  [EffectTiming.OnEnterFieldAnyone] destroy_security
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT22_035: 6 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] play_card (1/turn)
  [EffectTiming.WhenLinked] no-action
BT22_036: 3 effects
  [factory] overclock
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
BT22_037: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDiscardSecurity] no-action
  [EffectTiming.OnEnterFieldAnyone] digivolve, destroy_security
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT22_038: 7 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [EffectTiming.WhenPermanentWouldBeDeleted] change_dp, trash_digivolution_cards, disable_effect, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_digivolution_cards, disable_effect, effect_immunity (1/turn)
  [EffectTiming.OnAllyAttack] change_dp, trash_digivolution_cards, disable_effect, effect_immunity (1/turn)
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT22_039: 6 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.None] play_card (1/turn)
  [EffectTiming.OnEnterFieldAnyone] play_card (1/turn)
  [EffectTiming.OnAllyAttack] play_card (1/turn)
  [EffectTiming.OnEnterFieldAnyone] no-action (1/turn)
BT22_040: 4 effects
  [factory] overclock
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] play_token (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] no-action (1/turn)
BT22_041: 7 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] cost_reduction
  [factory] blocker
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] add_to_security
  [EffectTiming.OnEnterFieldAnyone] add_to_security
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
BT22_042: 4 effects
  [factory] alt_digivolve_req
  [factory] overclock
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] no-action (1/turn)
BT22_087: 3 effects
  [factory] gain_memory_tamer
  [EffectTiming.WhenLinked] change_dp, suspend, play_card
  [factory] security_play
BT22_088: 3 effects
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnEnterFieldAnyone] draw, suspend
  [factory] security_play
BT22_089: 3 effects
  [EffectTiming.OnStartMainPhase] play_card
  [EffectTiming.OnEnterFieldAnyone] draw
  [factory] security_play
BT22_097: 5 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] draw
  [factory] delay
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.SecuritySkill] no-action
BT22_098: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnTappedAnyone] digivolve
```


## Cross-Validation Results

Checked 102 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
BT22-006: API has 'mill' but script missing implementation
BT22-008: API has 'bounce' but script missing implementation
BT22-008: API has 'digivolve_into' but script missing implementation
BT22-014: API has 'suspend_target' but script missing implementation
BT22-017: API has 'digivolve_into' but script missing implementation
BT22-018: API has 'destruction_immunity' but script missing implementation
BT22-019: API has 'digivolve_into' but script missing implementation
BT22-031: API has 'once_per_turn' but script missing implementation
BT22-033: API has 'dp_modification' but script missing implementation
BT22-038: API has 'armor_purge' but script missing implementation
BT22-038: API has 'digivolve_into' but script missing implementation
BT22-040: API has 'play' but script missing implementation
BT22-045: API has 'piercing' but script missing implementation
BT22-049: API has 'piercing' but script missing implementation
BT22-051: API has 'fortitude' but script missing implementation
BT22-052: API has 'blocker' but script missing implementation
BT22-054: API has 'dp_modification' but script missing implementation
BT22-058: API has 'bounce' but script missing implementation
BT22-059: API has 'bounce' but script missing implementation
BT22-059: API has 'play' but script missing implementation
BT22-060: API has 'de_digivolve' but script missing implementation
BT22-060: API has 'dp_modification' but script missing implementation
BT22-061: API has 'digivolve_into' but script missing implementation
BT22-062: API has 'dp_modification' but script missing implementation
BT22-063: API has 'dp_modification' but script missing implementation
BT22-064: API has 'play' but script missing implementation
BT22-065: API has 'dp_modification' but script missing implementation
BT22-067: API has 'dp_modification' but script missing implementation
BT22-068: API has 'bounce' but script missing implementation
BT22-071: API has 'bounce' but script missing implementation
BT22-072: API has 'destruction_immunity' but script missing implementation
BT22-073: API has 'destruction_immunity' but script missing implementation
BT22-074: API has 'delete_opponent' but script missing implementation
BT22-076: API has 'digivolve_into' but script missing implementation
BT22-079: API has 'once_per_turn' but script missing implementation
BT22-083: API has 'dp_modification' but script missing implementation
BT22-087: API has 'memory_gain' but script missing implementation
BT22-090: API has 'memory_gain' but script missing implementation
BT22-099: API has 'memory_gain' but script missing implementation
BT22-101: API has 'bounce' but script missing implementation
BT22-102: API has 'memory_gain' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
BT22-015: script has '_is_decode' but API text doesn't mention it
BT22-021: script has '_is_decode' but API text doesn't mention it
BT22-024: script has '_is_decode' but API text doesn't mention it
BT22-027: script has '_is_decode' but API text doesn't mention it
BT22-028: script has '_is_decode' but API text doesn't mention it
BT22-036: script has '_is_overclock' but API text doesn't mention it
BT22-040: script has '_is_overclock' but API text doesn't mention it
BT22-042: script has '_is_overclock' but API text doesn't mention it
BT22-055: script has '_is_blocker' but API text doesn't mention it
BT22-061: script has '_is_fragment' but API text doesn't mention it
BT22-075: script has '_is_scapegoat' but API text doesn't mention it
```

### Timing Mismatches

```
BT22-009: has inherited effect text but no is_inherited_effect flag
BT22-016: has inherited effect text but no is_inherited_effect flag
BT22-025: has inherited effect text but no is_inherited_effect flag
BT22-030: has inherited effect text but no is_inherited_effect flag
BT22-031: [Once Per Turn] in API but no set_max_count_per_turn
BT22-033: has inherited effect text but no is_inherited_effect flag
BT22-035: has inherited effect text but no is_inherited_effect flag
BT22-045: has inherited effect text but no is_inherited_effect flag
BT22-049: has inherited effect text but no is_inherited_effect flag
BT22-050: has inherited effect text but no is_inherited_effect flag
BT22-052: has inherited effect text but no is_inherited_effect flag
BT22-055: has inherited effect text but no is_inherited_effect flag
BT22-058: has inherited effect text but no is_inherited_effect flag
BT22-066: has inherited effect text but no is_inherited_effect flag
BT22-075: has inherited effect text but no is_inherited_effect flag
BT22-079: has inherited effect text but no is_inherited_effect flag
BT22-080: has inherited effect text but no is_inherited_effect flag
BT22-096: timing 'Security' -> is_security_effect not found
BT22-098: timing 'Security' -> is_security_effect not found
BT22-099: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
BT22-009: API has inherited effect but script has no is_inherited_effect
BT22-016: API has inherited effect but script has no is_inherited_effect
BT22-025: API has inherited effect but script has no is_inherited_effect
BT22-030: API has inherited effect but script has no is_inherited_effect
BT22-033: API has inherited effect but script has no is_inherited_effect
BT22-035: API has inherited effect but script has no is_inherited_effect
BT22-045: API has inherited effect but script has no is_inherited_effect
BT22-049: API has inherited effect but script has no is_inherited_effect
BT22-050: API has inherited effect but script has no is_inherited_effect
BT22-052: API has inherited effect but script has no is_inherited_effect
BT22-055: API has inherited effect but script has no is_inherited_effect
BT22-058: API has inherited effect but script has no is_inherited_effect
BT22-066: API has inherited effect but script has no is_inherited_effect
BT22-075: API has inherited effect but script has no is_inherited_effect
BT22-079: API has inherited effect but script has no is_inherited_effect
BT22-080: API has inherited effect but script has no is_inherited_effect
BT22-096: API has security effect but script has no is_security_effect
BT22-098: API has security effect but script has no is_security_effect
BT22-099: API has security effect but script has no is_security_effect
```

