# BT19 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 103
- Scripts with effects: 103
- Total effects: 345
- Factory effects: 116
- Activate effects: 229

## Per-Card Breakdown

```
BT19_005: 1 effects
  [factory] reboot
BT19_055: 2 effects
  [EffectTiming.OnDestroyedAnyone] add_to_hand, reveal_and_select
  [factory] reboot
BT19_056: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] dp_modifier
BT19_057: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnAllyAttack] play_card
  [factory] save
  [factory] collision
BT19_058: 2 effects
  [factory] blocker
  [factory] save
BT19_059: 3 effects
  [factory] retaliation
  [factory] save
  [factory] reboot
BT19_060: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] dp_modifier
BT19_061: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] collision
BT19_062: 6 effects
  [factory] alt_digivolve_req
  [factory] rush
  [factory] collision
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.OnEndTurn] force_attack (descriptive-tagged)
  [factory] collision
BT19_063: 7 effects
  [EffectTiming.None] no-action
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT19_064: 6 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] unsuspend (1/turn)
  [EffectTiming.OnAllyAttack] unsuspend (1/turn)
BT19_065: 5 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] redirect_attack (inherited) (1/turn)
BT19_086: 3 effects
  [EffectTiming.OnStartMainPhase] draw
  [EffectTiming.OnDeclaration] suspend, play_card
  [factory] security_play
BT19_087: 3 effects
  [factory] set_memory_3
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
BT19_002: 1 effects
  [EffectTiming.OnAllyAttack] bounce (inherited)
BT19_016: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnDestroyedAnyone] draw
BT19_017: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnEndAttack] gain_memory (inherited) (1/turn)
BT19_018: 2 effects
  [factory] evade
  [factory] jamming
BT19_019: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEndAttack] gain_memory (inherited) (1/turn)
BT19_020: 3 effects
  [factory] rush
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] reboot
BT19_021: 3 effects
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [factory] jamming
BT19_022: 3 effects
  [factory] blocker
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] blocker
BT19_023: 4 effects
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_be_deleted_by_battle
  [EffectTiming.None] target_lock (inherited)
BT19_024: 4 effects
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.OnEndAttack] play_card (inherited) (1/turn)
BT19_025: 6 effects
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_rush
  [EffectTiming.OnAllyAttack] play_card, de_digivolve
  [EffectTiming.None] no-action
  [EffectTiming.OnEndAttack] play_card (inherited) (1/turn)
BT19_026: 4 effects
  [EffectTiming.OnEnterFieldAnyone] bounce, de_digivolve
  [EffectTiming.OnEnterFieldAnyone] bounce, de_digivolve
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] dp_modifier
BT19_027: 3 effects
  [factory] decode
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEndTurn] bounce (1/turn)
BT19_028: 3 effects
  [factory] blocker
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] gain_memory, unsuspend
BT19_081: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
BT19_082: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] suspend
  [factory] security_play
BT19_092: 2 effects
  [EffectTiming.OptionSkill] bounce
  [factory] security_play
BT19_004: 1 effects
  [factory] dp_modifier
BT19_044: 2 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT19_045: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.BeforePayCost] cost_reduction
  [factory] dp_modifier
  [factory] dp_modifier_all
BT19_046: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, grant_cannot_unsuspend
BT19_047: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] save
  [factory] blocker
BT19_048: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenRemoveField] put_to_security (descriptive-tagged)
  [factory] dp_modifier
  [factory] dp_modifier_all
BT19_049: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] suspend (inherited) (1/turn)
BT19_050: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend
  [factory] dp_modifier
BT19_051: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck, grant_bounce_immunity (1/turn)
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_cannot_return_to_hand, gain_keyword_cannot_return_to_deck, grant_bounce_immunity (1/turn)
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] blocker
BT19_052: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEndBattle] destroy_security (inherited) (1/turn)
  [factory] blocker
BT19_053: 4 effects
  [factory] alt_digivolve_req
  [factory] alliance
  [EffectTiming.OnAllyAttack] play_card, cost_reduction (1/turn)
  [EffectTiming.WhenRemoveField] put_to_security (descriptive-tagged)
BT19_054: 3 effects
  [factory] security_attack_plus
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] bounce
BT19_084: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] suspend, play_card
  [factory] security_play
BT19_085: 3 effects
  [EffectTiming.OnEnterFieldAnyone] suspend
  [EffectTiming.OnStartMainPhase] gain_memory
  [factory] security_play
BT19_095: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] change_dp, gain_keyword_piercing
  [EffectTiming.OptionSkill] change_dp, gain_keyword_piercing
  [EffectTiming.SecuritySkill] add_to_hand, suspend
BT19_096: 2 effects
  [EffectTiming.OptionSkill] delete, add_to_security
  [factory] security_play
BT19_006: 1 effects
  [EffectTiming.OnDestroyedAnyone] add_to_hand (inherited)
BT19_066: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
  [factory] blocker
BT19_067: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] retaliation
BT19_068: 3 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnDestroyedAnyone] play_card
BT19_069: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnDestroyedAnyone] delete, trash_from_hand
  [factory] blocker
BT19_070: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] security_attack_plus
BT19_071: 3 effects
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, mill
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_blocker, mill
  [EffectTiming.OnDiscardLibrary] delete (1/turn)
BT19_072: 3 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] redirect_attack (1/turn)
BT19_073: 6 effects
  [factory] alt_digivolve_req
  [factory] collision
  [EffectTiming.OnEnterFieldAnyone] de_digivolve, effect_immunity
  [factory] alliance
  [factory] dp_modifier_all
  [EffectTiming.None] grant_skill
BT19_074: 5 effects
  [factory] alt_digivolve_req
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAllyAttack] destroy_security (1/turn)
BT19_075: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.OnEnterFieldAnyone] delete, trash_from_hand
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnDestroyedAnyone] destroy_security (1/turn)
BT19_088: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnDeclaration] suspend, digivolve
  [factory] security_play
BT19_097: 4 effects
  [EffectTiming.OnDiscardLibrary] no-action
  [EffectTiming.OptionSkill] mill
  [factory] delay
  [EffectTiming.OnStartTurn] play_card
BT19_098: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OptionSkill] no-action
  [EffectTiming.SecuritySkill] add_to_hand
BT19_099: 3 effects
  [EffectTiming.OptionSkill] play_card, cost_reduction
  [factory] delay
  [EffectTiming.WhenRemoveField] play_card
BT19_001: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT19_007: 2 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.None] no-action (inherited)
BT19_008: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnDestroyedAnyone] play_card
  [factory] rush
BT19_009: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.None] no-action (inherited)
BT19_010: 2 effects
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.None] no-action
BT19_011: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.None] no-action (inherited)
BT19_012: 6 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete (1/turn)
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete (1/turn)
  [EffectTiming.OnDestroyedAnyone] no-action
  [factory] rush
BT19_013: 3 effects
  [EffectTiming.WhenRemoveField] no-action
  [EffectTiming.OnDestroyedAnyone] play_card (1/turn)
  [EffectTiming.None] no-action
BT19_014: 7 effects
  [factory] alliance
  [factory] reboot
  [factory] save
  [factory] material_save
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnAllyAttack] delete
  [EffectTiming.None] no-action
BT19_015: 2 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_piercing
  [EffectTiming.OnDestroyedAnyone] gain_memory (1/turn)
BT19_079: 3 effects
  [factory] set_memory_3
  [EffectTiming.BeforePayCost] suspend
  [factory] security_play
BT19_080: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_raid, force_attack
  [factory] security_play
BT19_089: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] gain_keyword_immune_dp_minus, effect_immunity
  [EffectTiming.SecuritySkill] add_to_hand
BT19_090: 2 effects
  [EffectTiming.OptionSkill] play_card, unsuspend, force_attack
  [EffectTiming.SecuritySkill] play_card
BT19_091: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] gain_keyword_alliance, play_token, force_attack
  [EffectTiming.SecuritySkill] play_card
BT19_101: 7 effects
  [factory] alt_digivolve_req
  [factory] overclock
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] bounce
  [EffectTiming.None] effect_immunity
  [EffectTiming.None] no-action
BT19_076: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] play_card, add_to_hand, reveal_and_select
  [factory] save
BT19_077: 3 effects
  [EffectTiming.OnDeclaration] suspend, digivolve
  [EffectTiming.OnDestroyedAnyone] add_to_security
  [EffectTiming.SecuritySkill] play_card
BT19_078: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp
  [EffectTiming.OnDeclaration] no-action (1/turn)
  [EffectTiming.OnAllyAttack] play_card, redirect_attack (inherited)
BT19_100: 3 effects
  [EffectTiming.OnAllyAttack] change_dp
  [EffectTiming.OptionSkill] add_to_security, destroy_security
  [EffectTiming.SecuritySkill] play_card
BT19_102: 6 effects
  [factory] alt_digivolve_req
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] delete, play_card, effect_immunity
  [EffectTiming.OnDestroyedAnyone] play_card
BT19_003: 1 effects
  [EffectTiming.OnEndTurn] add_to_hand (inherited) (1/turn)
BT19_029: 2 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, destroy_security
  [EffectTiming.WhenRemoveField] destroy_security (inherited) (1/turn)
BT19_030: 2 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnUseOption] change_dp (inherited) (1/turn)
BT19_031: 4 effects
  [factory] alt_digivolve_req
  [factory] decoy
  [EffectTiming.OnDestroyedAnyone] play_card
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT19_032: 2 effects
  [EffectTiming.OnDestroyedAnyone] recovery, change_security_attack
  [factory] barrier
BT19_033: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [factory] save
BT19_034: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card
  [EffectTiming.OnUseOption] change_dp (inherited) (1/turn)
BT19_035: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] change_dp, change_security_attack (1/turn)
  [EffectTiming.OnDestroyedAnyone] no-action
  [EffectTiming.OnAllyAttack] change_dp (inherited) (1/turn)
BT19_036: 4 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, add_to_security, destroy_security
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, add_to_security, destroy_security
  [EffectTiming.WhenRemoveField] destroy_security (inherited) (1/turn)
BT19_037: 4 effects
  [factory] blast_digivolve
  [EffectTiming.OnEnterFieldAnyone] change_security_attack, disable_effect, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] change_security_attack, disable_effect, effect_immunity
  [EffectTiming.OnAllyAttack] change_dp (inherited)
BT19_038: 5 effects
  [factory] alt_digivolve_req
  [EffectTiming.None] no-action
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, disable_effect, grant_cannot_unsuspend, effect_immunity (1/turn)
  [EffectTiming.OnEnterFieldAnyone] suspend, gain_keyword_cannot_unsuspend, disable_effect, grant_cannot_unsuspend, effect_immunity (1/turn)
  [EffectTiming.OnDestroyedAnyone] no-action
BT19_039: 4 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory, delete, destroy_security
  [EffectTiming.OnEnterFieldAnyone] gain_memory, delete, destroy_security
  [EffectTiming.OnDestroyedAnyone] recovery
  [EffectTiming.OnLoseSecurity] unsuspend (inherited) (1/turn)
BT19_040: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] draw
  [EffectTiming.OnUseOption] no-action (1/turn)
BT19_040_token: 1 effects
  [factory] blocker
BT19_041: 3 effects
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_blocker, destroy_security
  [EffectTiming.OnEnterFieldAnyone] change_dp, gain_keyword_blocker, destroy_security
  [EffectTiming.WhenRemoveField] recovery (1/turn)
BT19_042: 6 effects
  [factory] alt_digivolve_req
  [factory] raid
  [factory] blocker
  [EffectTiming.OnEnterFieldAnyone] change_dp, destroy_security (1/turn)
  [EffectTiming.OnAllyAttack] change_dp, destroy_security (1/turn)
  [EffectTiming.OnEndTurn] recovery
BT19_043: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.WhenRemoveField] destroy_security (1/turn)
  [EffectTiming.OnEndTurn] recovery, delete, destroy_security (1/turn)
BT19_083: 3 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [EffectTiming.OnUseOption] gain_memory, suspend
  [factory] security_play
BT19_093: 4 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OnDestroyedAnyone] change_dp, disable_effect, effect_immunity
  [EffectTiming.OptionSkill] change_dp, disable_effect, effect_immunity
  [EffectTiming.SecuritySkill] add_to_hand, change_security_attack
BT19_094: 3 effects
  [EffectTiming.OnEnterFieldAnyone] recovery, destroy_security, return_to_deck
  [EffectTiming.OptionSkill] recovery
  [EffectTiming.SecuritySkill] play_card
```


## Cross-Validation Results

Checked 102 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
BT19-003: API has 'bounce' but script missing implementation
BT19-006: API has 'bounce' but script missing implementation
BT19-008: API has 'reveal_top' but script missing implementation
BT19-008: API has 'save' but script missing implementation
BT19-011: API has 'memory_gain' but script missing implementation
BT19-014: API has 'dp_modification' but script missing implementation
BT19-020: API has 'save' but script missing implementation
BT19-022: API has 'save' but script missing implementation
BT19-023: API has 'destruction_immunity' but script missing implementation
BT19-025: API has 'digivolve_into' but script missing implementation
BT19-026: API has 'save' but script missing implementation
BT19-028: API has 'suspend_target' but script missing implementation
BT19-029: API has 'mill' but script missing implementation
BT19-033: API has 'digivolve_into' but script missing implementation
BT19-033: API has 'piercing' but script missing implementation
BT19-038: API has 'piercing' but script missing implementation
BT19-040: API has 'play' but script missing implementation
BT19-040: API has 'token_play' but script missing implementation
BT19-042: API has 'mill' but script missing implementation
BT19-043: API has 'mill' but script missing implementation
BT19-047: API has 'digivolve_into' but script missing implementation
BT19-048: API has 'once_per_turn' but script missing implementation
BT19-068: API has 'save' but script missing implementation
BT19-073: API has 'piercing' but script missing implementation
BT19-077: API has 'attack_prevention' but script missing implementation
BT19-080: API has 'digivolve_into' but script missing implementation
BT19-085: API has 'digivolve_into' but script missing implementation
BT19-094: API has 'digivolve_into' but script missing implementation
BT19-094: API has 'mill' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
BT19-014: script has '_is_material_save' but API text doesn't mention it
BT19-014: script has '_is_save' but API text doesn't mention it
BT19-024: script has '_is_decode' but API text doesn't mention it
BT19-025: script has '_is_material_save' but API text doesn't mention it
BT19-025: script has '_is_save' but API text doesn't mention it
BT19-027: script has '_is_decode' but API text doesn't mention it
BT19-031: script has '_is_decoy' but API text doesn't mention it
BT19-063: script has '_is_material_save' but API text doesn't mention it
BT19-063: script has '_is_save' but API text doesn't mention it
BT19-101: script has '_is_overclock' but API text doesn't mention it
```

### Timing Mismatches

```
BT19-033: has inherited effect text but no is_inherited_effect flag
BT19-038: has inherited effect text but no is_inherited_effect flag
BT19-045: timing 'Security' -> is_security_effect not found
BT19-048: [Once Per Turn] in API but no set_max_count_per_turn
BT19-048: timing 'Security' -> is_security_effect not found
BT19-052: timing 'Security' -> is_security_effect not found
BT19-058: has inherited effect text but no is_inherited_effect flag
BT19-064: has inherited effect text but no is_inherited_effect flag
BT19-074: has inherited effect text but no is_inherited_effect flag
BT19-097: timing 'Security' -> is_security_effect not found
BT19-099: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
BT19-033: API has inherited effect but script has no is_inherited_effect
BT19-038: API has inherited effect but script has no is_inherited_effect
BT19-058: API has inherited effect but script has no is_inherited_effect
BT19-064: API has inherited effect but script has no is_inherited_effect
BT19-074: API has inherited effect but script has no is_inherited_effect
BT19-097: API has security effect but script has no is_security_effect
BT19-099: API has security effect but script has no is_security_effect
```

