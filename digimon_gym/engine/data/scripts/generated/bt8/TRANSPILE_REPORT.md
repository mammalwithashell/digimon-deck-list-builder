# BT8 Transpilation Report

Generated from DCGO C# card scripts.

- Total scripts: 94
- Scripts with effects: 94
- Total effects: 198
- Factory effects: 56
- Activate effects: 142

## Per-Card Breakdown

```
BT8_005: 1 effects
  [EffectTiming.OnAddDigivolutionCards] change_dp (inherited) (1/turn)
BT8_058: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT8_059: 1 effects
  [EffectTiming.None] no-action
BT8_060: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
  [factory] decoy
BT8_061: 1 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
BT8_062: 3 effects
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.None] also_treated_as (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_jamming, gain_keyword_blocker
BT8_063: 1 effects
  [factory] blocker
BT8_064: 1 effects
  [factory] blocker
BT8_065: 1 effects
  [EffectTiming.OnEnterFieldAnyone] de_digivolve
BT8_066: 2 effects
  [EffectTiming.OnAddDigivolutionCards] digivolve
  [factory] reboot
BT8_067: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, de_digivolve
  [EffectTiming.None] attack_unsuspended (inherited)
BT8_068: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, reveal_and_select
  [factory] security_attack_plus
BT8_069: 3 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnAddDigivolutionCards] change_dp (1/turn)
  [EffectTiming.OnEndAttack] unsuspend (inherited) (1/turn)
BT8_070: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete
  [EffectTiming.OnDestroyedAnyone] unsuspend (1/turn)
BT8_092: 3 effects
  [EffectTiming.OnMove] draw, gain_memory
  [EffectTiming.OnAllyAttack] suspend
  [factory] security_play
BT8_104: 2 effects
  [EffectTiming.OptionSkill] delete, de_digivolve
  [EffectTiming.SecuritySkill] delete, de_digivolve
BT8_105: 2 effects
  [EffectTiming.OptionSkill] delete
  [EffectTiming.SecuritySkill] delete
BT8_106: 2 effects
  [EffectTiming.OptionSkill] delete, play_card, reveal_and_select
  [factory] security_play
BT8_002: 1 effects
  [factory] dp_modifier
BT8_020: 1 effects
  [EffectTiming.OnEndTurn] play_card (inherited)
BT8_021: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT8_022: 1 effects
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, effect_immunity
BT8_023: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] change_dp, trash_digivolution_cards
BT8_024: 2 effects
  [EffectTiming.BeforePayCost] recovery
  [EffectTiming.OnAllyAttack] bounce (inherited)
BT8_026: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] delete
BT8_028: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw, gain_memory
BT8_029: 2 effects
  [factory] blocker
  [EffectTiming.OnDigivolutionCardDiscarded] bounce (inherited) (1/turn)
BT8_031: 3 effects
  [EffectTiming.OnEnterFieldAnyone] bounce, trash_digivolution_cards, effect_immunity
  [EffectTiming.None] grant_skill, effect_immunity
  [EffectTiming.None] trash_digivolution_cards
BT8_032: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnAllyAttack] unsuspend, suspend (1/turn)
BT8_087: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] draw, suspend
  [factory] security_play
BT8_088: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnEnterFieldAnyone] suspend, unsuspend
  [factory] security_play
BT8_098: 2 effects
  [EffectTiming.OptionSkill] trash_digivolution_cards, gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block, effect_immunity
  [factory] security_play
BT8_099: 2 effects
  [EffectTiming.OptionSkill] suspend, effect_immunity
  [EffectTiming.SecuritySkill] suspend, bounce, effect_immunity
BT8_004: 1 effects
  [factory] dp_modifier
BT8_046: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT8_047: 1 effects
  [factory] dp_modifier
BT8_048: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] gain_keyword_cannot_attack, gain_keyword_cannot_block, grant_cannot_block
BT8_049: 1 effects
  [EffectTiming.OnDeclaration] suspend, add_to_hand, reveal_and_select
BT8_050: 2 effects
  [EffectTiming.OnEnterFieldAnyone] suspend, effect_immunity
  [factory] dp_modifier
BT8_051: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] change_dp
BT8_053: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] suspend
BT8_054: 2 effects
  [EffectTiming.BeforePayCost] cost_reduction, suspend, effect_immunity
  [factory] dp_modifier
BT8_055: 2 effects
  [EffectTiming.OnEnterFieldAnyone] bounce
  [EffectTiming.OnUnTappedAnyone] suspend (inherited)
BT8_057: 2 effects
  [EffectTiming.None] no-action
  [EffectTiming.OnUnTappedAnyone] destroy_security
BT8_091: 3 effects
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.BeforePayCost] suspend, cost_reduction
  [factory] security_play
BT8_102: 2 effects
  [EffectTiming.OptionSkill] suspend, gain_keyword_cannot_unsuspend, effect_immunity
  [EffectTiming.SecuritySkill] suspend
BT8_103: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp, gain_keyword_piercing
  [EffectTiming.SecuritySkill] suspend
BT8_006: 1 effects
  [EffectTiming.OnDiscardLibrary] draw (inherited) (1/turn)
BT8_071: 1 effects
  [EffectTiming.None] no-action
BT8_072: 1 effects
  [EffectTiming.OnEnterFieldAnyone] trash_from_hand, add_to_hand, reveal_and_select
BT8_074: 1 effects
  [EffectTiming.OnDiscardLibrary] gain_memory (inherited) (1/turn)
BT8_077: 2 effects
  [factory] rush
  [factory] retaliation
BT8_079: 2 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, mill
  [EffectTiming.OnDiscardLibrary] gain_memory (inherited) (1/turn)
BT8_080: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, mill
  [EffectTiming.OnDestroyedAnyone] play_card (inherited)
BT8_081: 3 effects
  [EffectTiming.OnEndAttack] digivolve
  [EffectTiming.OnEndTurn] destroy_security
  [EffectTiming.OnDigivolutionCardDiscarded] change_dp, unsuspend (inherited)
BT8_082: 3 effects
  [factory] alt_digivolve_req
  [EffectTiming.OnEnterFieldAnyone] recovery, delete
  [EffectTiming.OnDestroyedAnyone] play_card
BT8_083: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, destroy_security
  [EffectTiming.OnEnterFieldAnyone] gain_memory, mill
BT8_093: 3 effects
  [EffectTiming.OnDestroyedAnyone] gain_memory, suspend
  [EffectTiming.OnEndTurn] play_card
  [factory] security_play
BT8_107: 2 effects
  [EffectTiming.OptionSkill] delete
  [EffectTiming.SecuritySkill] delete
BT8_108: 3 effects
  [EffectTiming.OptionSkill] draw, mill
  [factory] delay
  [EffectTiming.OnDeclaration] gain_memory
BT8_109: 2 effects
  [EffectTiming.OptionSkill] change_dp, play_card
  [factory] security_play
BT8_111: 2 effects
  [EffectTiming.OnEnterFieldAnyone] play_card, mill
  [EffectTiming.OnAllyAttack] mill (1/turn)
BT8_001: 1 effects
  [EffectTiming.OnAllyAttack] draw (inherited) (1/turn)
BT8_008: 2 effects
  [EffectTiming.OnEnterFieldAnyone] draw (1/turn)
  [EffectTiming.OnAllyAttack] delete (inherited)
BT8_009: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT8_010: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnAllyAttack] delete (inherited)
BT8_012: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnAllyAttack] change_dp
BT8_015: 3 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] change_dp, delete
  [EffectTiming.OnAllyAttack] delete (inherited)
BT8_016: 1 effects
  [factory] security_attack_plus
BT8_018: 1 effects
  [EffectTiming.None] attack_unsuspended
BT8_019: 2 effects
  [EffectTiming.OnEnterFieldAnyone] delete, effect_immunity
  [EffectTiming.OnDestroyedAnyone] no-action
BT8_085: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAllyAttack] delete, suspend
  [factory] security_play
BT8_086: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAllyAttack] change_dp, suspend
  [factory] security_play
BT8_095: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_security_attack (descriptive-tagged)
  [EffectTiming.SecuritySkill] delete
BT8_096: 2 effects
  [EffectTiming.OptionSkill] delete
  [factory] security_play
BT8_097: 3 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OptionSkill] delete, play_restriction, effect_immunity
  [factory] security_play
BT8_084: 4 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] no-action
  [EffectTiming.None] no-action
  [factory] dp_modifier
BT8_094: 3 effects
  [EffectTiming.OnDestroyedAnyone] draw, suspend
  [EffectTiming.OnMove] gain_memory
  [factory] security_play
BT8_110: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] digivolve, unsuspend
  [EffectTiming.SecuritySkill] play_card
BT8_112: 3 effects
  [EffectTiming.BeforePayCost] cost_reduction, return_to_deck
  [EffectTiming.OnEnterFieldAnyone] trash_digivolution_cards, effect_immunity
  [EffectTiming.OnAllyAttack] trash_digivolution_cards, effect_immunity
BT8_003: 1 effects
  [factory] dp_modifier
BT8_033: 1 effects
  [EffectTiming.OnEnterFieldAnyone] add_to_hand, reveal_and_select
BT8_035: 1 effects
  [EffectTiming.OnEnterFieldAnyone] gain_memory (inherited) (1/turn)
BT8_036: 2 effects
  [EffectTiming.None] cost_reduction
  [EffectTiming.OnAllyAttack] change_dp (inherited)
BT8_038: 4 effects
  [factory] alt_digivolve_req
  [factory] blocker
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] unsuspend
BT8_039: 3 effects
  [factory] alt_digivolve_req
  [factory] armor_purge
  [EffectTiming.OnEnterFieldAnyone] change_dp, suspend
BT8_040: 1 effects
  [EffectTiming.OnEnterFieldAnyone] draw, trash_from_hand
BT8_042: 3 effects
  [EffectTiming.None] jogress_condition
  [EffectTiming.OnEnterFieldAnyone] recovery, bounce
  [EffectTiming.OnAllyAttack] change_dp (inherited)
BT8_043: 4 effects
  [EffectTiming.BeforePayCost] cost_reduction, effect_immunity
  [EffectTiming.None] cost_reduction, effect_immunity
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
  [EffectTiming.OnEnterFieldAnyone] change_security_attack (descriptive-tagged)
BT8_044: 2 effects
  [EffectTiming.OnAllyAttack] gain_memory, destroy_security
  [EffectTiming.OnEnterFieldAnyone] unsuspend (1/turn)
BT8_089: 3 effects
  [EffectTiming.OnStartMainPhase] gain_memory
  [EffectTiming.OnAllyAttack] change_dp, suspend
  [factory] security_play
BT8_090: 3 effects
  [factory] set_memory_3
  [EffectTiming.OnAddSecurity] gain_memory, suspend
  [factory] security_play
BT8_100: 2 effects
  [EffectTiming.OptionSkill] no-action
  [factory] security_play
BT8_101: 3 effects
  [EffectTiming.None] ignore_color_req (descriptive-tagged)
  [EffectTiming.OptionSkill] change_dp
  [factory] security_play
```


## Cross-Validation Results

Checked 94 cards against digimoncard.io effect text.

### Forward Mismatches (API mentions X, script missing)

```
BT8-019: API has 'memory_gain' but script missing implementation
BT8-020: API has 'digivolve_into' but script missing implementation
BT8-029: API has 'attack_prevention' but script missing implementation
BT8-038: API has 'dp_modification' but script missing implementation
BT8-048: API has 'blocker' but script missing implementation
BT8-065: API has 'bounce' but script missing implementation
BT8-069: API has 'destruction_immunity' but script missing implementation
BT8-079: API has 'bounce' but script missing implementation
BT8-081: API has 'mill' but script missing implementation
BT8-081: API has 'suspend_target' but script missing implementation
BT8-091: API has 'digivolve_into' but script missing implementation
BT8-095: API has 'blocker' but script missing implementation
BT8-100: API has 'dp_modification' but script missing implementation
BT8-110: API has 'mill' but script missing implementation
BT8-111: API has 'dp_modification' but script missing implementation
BT8-112: API has 'bounce' but script missing implementation
BT8-112: API has 'digivolve_into' but script missing implementation
```

### Reverse Mismatches (Script claims X, API doesn't mention)

```
BT8-060: script has '_is_decoy' but API text doesn't mention it
```

### Timing Mismatches

```
BT8-108: timing 'Security' -> is_security_effect not found
```

### Structural Warnings

```
BT8-108: API has security effect but script has no is_security_effect
```

