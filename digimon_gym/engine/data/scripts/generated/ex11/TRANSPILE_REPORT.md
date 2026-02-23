# EX11 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 74
- Scripts with effects: 74
- Total effects: 293
- Factory effects: 103
- Activate effects: 190

## Per-Card Breakdown

```
EX11_004: 1 effects
  [EffectTiming.OnFaceUpSecurityIncreased] draw (inherited) (1/turn)
EX11_037: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] draw, gain_memory
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory
  [factory] jamming
EX11_038: 3 effects
  [EffectTiming.OnMove] draw, trash_from_hand, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand, trash_digivolution_cards
  [EffectTiming.OnDigivolutionCardDiscarded] draw (inherited)
EX11_039: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] jamming
EX11_040: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] play_card (1/turn)
  [factory] reboot
EX11_041: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, digivolve
  [EffectTiming.OnSecurityCheck] add_to_security
  [EffectTiming.None] target_lock (inherited)
EX11_042: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.WhenLinked] delete (1/turn)
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
EX11_043: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEndTurn] play_card
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnSecurityCheck] add_to_security
EX11_044: 6 effects
  [factory] reboot
  [factory] fragment
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] delete, trash_digivolution_cards
  [EffectTiming.OnAllyAttack] delete, trash_digivolution_cards
  [EffectTiming.OnDigivolutionCardDiscarded] no-action (1/turn)
EX11_045: 8 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
  [EffectTiming.OnAllyAttack] de_digivolve
  [EffectTiming.OnEndTurn] digivolve (1/turn)
  [EffectTiming.None] no-action
  [EffectTiming.OnAddDigivolutionCards] delete (inherited) (1/turn)
EX11_046: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_blocker, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] delete, gain_keyword_blocker, effect_immunity
  [EffectTiming.OnEndTurn] digivolve
  [EffectTiming.None] no-action
EX11_064: 4 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] suspend, digivolve
  [factory] security_play
EX11_065: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand, trash_digivolution_cards
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] security_play
EX11_066: 5 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] suspend, reveal_and_select
  [factory] security_play
EX11_002: 1 effects
  [EffectTiming.None] attack_unsuspended (inherited)
EX11_013: 3 effects
  [EffectTiming.OnMove] draw
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnEndAttack] gain_memory (inherited) (1/turn)
EX11_014: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] jamming
EX11_015: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] jamming
EX11_016: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, put_to_security
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, put_to_security
  [factory] security_attack_plus
EX11_017: 6 effects
  [factory] alt_digivolve_req
  [factory] barrier
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] play_card
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards (1/turn)
EX11_018: 6 effects
  [factory] evade
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnEnterFieldAnyone] unsuspend
  [EffectTiming.OnAllyAttack] unsuspend
  [EffectTiming.OnAddDigivolutionCards] bounce (1/turn)
EX11_057: 4 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards
  [EffectTiming.OnDigivolutionCardDiscarded] gain_memory, suspend
  [factory] security_play
EX11_058: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] draw, suspend
  [factory] security_play
EX11_003: 1 effects
  [EffectTiming.OnAddSecurity] draw (inherited) (1/turn)
EX11_025: 4 effects
  [factory] alt_digivolve_req
  [factory] reboot
  [EffectTiming.OnStartMainPhase] add_to_hand, destroy_security
  [factory] dp_modifier
EX11_026: 3 effects
  [EffectTiming.OnMove] change_dp, suspend
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
EX11_027: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.WhenRemoveField] no-action
EX11_028: 4 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnTappedAnyone] play_card (1/turn)
  [EffectTiming.OnEndBattle] gain_memory (inherited) (1/turn)
EX11_029: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.WhenLinked] play_card (1/turn)
EX11_030: 5 effects
  [factory] alt_digivolve_req
  [factory] reboot
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, destroy_security
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, destroy_security
  [factory] dp_modifier
EX11_031: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
  [factory] blocker
EX11_032: 3 effects
  [EffectTiming.OnDeclaration] digivolve
  [EffectTiming.OnEnterFieldAnyone] play_card, suspend
  [EffectTiming.OnEndBattle] unsuspend (inherited) (1/turn)
EX11_033: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.WhenLinked] suspend, gain_keyword_cannot_unsuspend (1/turn)
  [EffectTiming.OnEndBattle] unsuspend (inherited) (1/turn)
EX11_034: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_security
  [EffectTiming.OnEnterFieldAnyone] delete, add_to_security
  [EffectTiming.OnAllyAttack] delete, add_to_security
  [EffectTiming.OnEnterFieldAnyone] play_card, cost_reduction
  [EffectTiming.OnAllyAttack] play_card, cost_reduction
EX11_035: 4 effects
  [factory] vortex
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] unsuspend, suspend
  [EffectTiming.OnTappedAnyone] play_card (1/turn)
EX11_036: 8 effects
  [factory] alt_digivolve_req
  [factory] vortex
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnAllyAttack] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEndTurn] digivolve (1/turn)
  [EffectTiming.None] no-action
  [EffectTiming.WhenLinked] suspend, force_attack (inherited) (1/turn)
EX11_062: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnTappedAnyone] draw, change_dp, suspend
  [factory] security_play
EX11_063: 4 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, destroy_security
  [EffectTiming.OnEndTurn] suspend, gain_keyword_piercing, force_attack
  [factory] security_play
EX11_072: 3 effects
  [EffectTiming.OptionSkill] play_card
  [factory] delay
  [EffectTiming.OnTappedAnyone] digivolve
EX11_073: 6 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndTurn] bounce (1/turn)
EX11_074: 6 effects
  [factory] alt_digivolve_req
  [factory] vortex
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] suspend, effect_immunity
  [EffectTiming.OnAllyAttack] suspend, effect_immunity
  [EffectTiming.OnTappedAnyone] unsuspend (1/turn)
EX11_005: 1 effects
  [EffectTiming.OnStartMainPhase] trash_from_hand, digivolve (inherited)
EX11_047: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [factory] dp_modifier
EX11_048: 3 effects
  [EffectTiming.OnMove] gain_keyword_retaliation
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_retaliation
  [EffectTiming.OnDestroyedAnyone] gain_memory (inherited)
EX11_049: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] trash_from_hand, digivolve
  [factory] dp_modifier
EX11_050: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [factory] scapegoat
  [EffectTiming.None] grant_skill
EX11_051: 5 effects
  [factory] execute
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [EffectTiming.OnDestroyedAnyone] delete, play_card
  [EffectTiming.OnDestroyedAnyone] digivolve
EX11_052: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, trash_from_hand
  [EffectTiming.OnEndAttack] delete, play_card, trash_from_hand
  [EffectTiming.WhenRemoveField] destroy_security (1/turn)
EX11_067: 4 effects
  [factory] security_play
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_memory, suspend
EX11_068: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] draw, suspend, trash_from_hand, digivolve
  [factory] security_play
EX11_069: 5 effects
  [factory] security_play
  [EffectTiming.OnStartMainPhase] gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] gain_memory, trash_from_hand
  [EffectTiming.OnAllyAttack] digivolve (1/turn)
  [EffectTiming.OnEndTurn] suspend, add_to_hand
EX11_001: 1 effects
  [EffectTiming.OnAllyAttack] digivolve (inherited) (1/turn)
EX11_007: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnMove] gain_keyword_raid, gain_keyword_piercing
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_raid, gain_keyword_piercing
  [factory] dp_modifier
EX11_008: 3 effects
  [EffectTiming.OnMove] change_dp, gain_keyword_raid
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_raid
  [EffectTiming.OnLoseSecurity] gain_memory (inherited) (1/turn)
EX11_009: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier
EX11_010: 5 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnEnterFieldAnyone] suspend
  [factory] security_attack_plus
EX11_011: 4 effects
  [factory] alt_digivolve_req
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] delete, suspend
  [EffectTiming.OnEnterFieldAnyone] delete, suspend
EX11_012: 5 effects
  [factory] rush
  [factory] progress
  [EffectTiming.OnEnterFieldAnyone] delete, return_to_deck, play_token
  [EffectTiming.OnEndAttack] delete, return_to_deck, play_token
  [EffectTiming.WhenRemoveField] no-action
EX11_054: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] draw, suspend
  [factory] security_play
EX11_055: 4 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] suspend, play_card
  [factory] security_play
EX11_056: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] suspend, digivolve
  [factory] security_play
EX11_006: 1 effects
  [EffectTiming.OnAllyAttack] digivolve (inherited) (1/turn)
EX11_053: 3 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnDestroyedAnyone] play_card
EX11_070: 4 effects
  [factory] security_play
  [factory] set_memory_3
  [EffectTiming.OnEndTurn] mind_link
  [EffectTiming.None] no-action (inherited)
EX11_071: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDeclaration] play_card, cost_reduction
  [factory] security_play
EX11_019: 2 effects
  [EffectTiming.OnDestroyedAnyone] play_token (descriptive-tagged)
  [factory] barrier
EX11_020: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
EX11_021: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] no-action (inherited) (1/turn)
EX11_022: 5 effects
  [factory] alt_digivolve_req
  [factory] scapegoat
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [EffectTiming.OnEnterFieldAnyone] delete, play_card
  [EffectTiming.WhenRemoveField] no-action (inherited) (1/turn)
EX11_023: 6 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [factory] scapegoat
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndTurn] delete
  [EffectTiming.OnDestroyedAnyone] play_card (1/turn)
EX11_024: 6 effects
  [factory] alliance
  [factory] overclock
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnAllyAttack] no-action
EX11_059: 4 effects
  [EffectTiming.OnStartMainPhase] draw, gain_memory, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] suspend
  [factory] security_play
EX11_060: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnDestroyedAnyone] draw, suspend, play_card
  [factory] security_play
EX11_061: 4 effects
  [factory] gain_memory_tamer
  [EffectTiming.OnEnterFieldAnyone] suspend, play_card
  [EffectTiming.OnEnterFieldAnyone] delete
  [factory] security_play
```
