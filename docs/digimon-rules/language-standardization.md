# Name containment: Bandai's Language Standardization Rules

> **Decision (2026-10-05):** the English game follows Bandai's **Language
> Standardization Rules** (dated 2026-05-29). For the cards Bandai lists, they
> override a literal reading of `general_rule.pdf` 2-3-1-3 ("with [XX] in its
> name"). Example: **BurningGreymon, DoruGreymon and DexDoruGreymon are not
> treated as having [Greymon] in their names. KendoGarurumon is not treated as
> having [Garurumon].** This holds whether or not the referencing card prints an
> "(other than …)" clause.
>
> **Engine status: not implemented.** `name_contains` and its siblings are still
> literal, case-insensitive substring checks. The only card that applies any of
> these exclusions is `cards/bt5/BT5-092.yaml`, through its own `none_of`. The
> recommended fix is one substrate helper that every name-containment check goes
> through (see "Engine implications"). It is pending sign-off because it changes
> how the whole engine behaves.

## Sources (official, Bandai)

| Source | What it settles |
|---|---|
| `https://world.digimoncard.com/rule/lang-standardization-rules/`: "Language Standardization Rules (May 29, 2026)" | The rule itself. It applies "in any language", with BurningGreymon (JP: Vritramon) as the worked example. A printed (Rule) still takes priority, e.g. Panjyamon's "(Rule) Name: treated as having [Leomon]". |
| `https://world.digimoncard.com/rule/pdf/lang-standardization-01.pdf`: "[cards including XX] Reference List", 13 pp. | The list of affected cards for every language (the table below). |
| `https://world.digimoncard.com/rule/pdf/lang-standardization-02.pdf`: "[The same name List]" | A separate rule for different Digimon that share one name in some language: Huckmon/Hackmon, Callismon/Charismon, Tactimon/Sakusimon, Clockmon/Timemon. An [Appmon]-trait card and a non-[Appmon] card are treated as having different names. Not covered by this doc. |
| Comprehensive Rules **Ver.4.3** (2026-09-18), §2-3-1-3 and §2-3-4-4 | 2-3-1-3 is still literal. 2-3-4-4 defines "(Rule) Name: Also treated as having [XX]": the card counts for "[XX] in its name" but is not the name [XX]. The manual does not mention the standardization rules; they are a separate official document. |

All four were retrieved 2026-10-05. The base-repo `general_rule.pdf` is still
Ver.3.6. Neither version contradicts the standardization rules: 2-3-1-3 says
what "include" means in general, and the standardization list says which
names Bandai does not treat as including a fragment.

Why Bandai needed the rule: names are localised. ドルグレモン (DoruGreymon),
ヴリトラモン (BurningGreymon) and ガルムモン (KendoGarurumon) don't contain
グレイモン / ガルルモン in Japanese, but their English names do contain
"Greymon" / "Garurumon". Early English printings patched this card by card with
"(other than [DoruGreymon], [BurningGreymon], or [DexDoruGreymon])". Nine
BT4/BT5 cards carry it: BT4-092, BT4-099, BT4-113, BT5-001, BT5-007, BT5-010,
BT5-015, BT5-016 and BT5-095, plus the original BT5-092 printing. Later cards
stopped printing it. No card ever printed a KendoGarurumon exclusion. The
standardization rules now make the exclusion global.

## The English rows

The engine runs on English card names, so only these rows apply. Every other
row in the reference list concerns Japanese, Simplified Chinese or Korean
names: 巨龙兽, V, 猿猴兽, 加奥/가오, 暴龙兽, 死X, 龙兽/드라몬, 骑士兽, 콩알몬,
スターモン/星兽/스타몬, 恶魔兽, 狮子兽/레오몬, チューモン/吱吱兽, レアモン/레어몬,
워매몬.

| "[XX] in its name" | Ruling | Card name(s) | Our pool | Bandai's example referencing card |
|---|---|---|---|---|
| Agumon | not treated as including | Pagumon | BT2-007, BT6-005, BT9-006, BT19-006, BT25-005, BT26-004, EX9-006, EX10-005, ST6-01 | BT1-011 Agumon Expert |
| Vee | not treated as including | DemiVeemon | BT2-002, BT3-002, BT12-002, BT16-002, BT20-001, EX13-002, P-188, ST8-01 | BT2-086 Rina Shinomiya |
| Veemon | not treated as including | DemiVeemon | (as above) | BT11-112 Rina Shinomiya |
| Garurumon | not treated as including | KendoGarurumon | BT4-027, BT7-022, BT17-023 | P-007 Garurumon |
| Greymon | not treated as including | BurningGreymon | BT4-013, BT7-011, BT12-013, BT17-012, BT21-014 | P-009 Agumon |
| Greymon | not treated as including | DoruGreymon | BT7-064, BT13-072, BT16-061 | P-009 Agumon |
| Greymon | not treated as including | DexDoruGreymon | BT9-078, BT17-067 | P-009 Agumon |
| Dramon | not treated as including | Indramon | EX5-009 | BT4-099 Heir of Dragons |
| Starmon | not treated as including | BeelStarmon, BeelStarmon (X Antibody) | BT6-112, BT25-085, EX7-059, ST14-09; EX7-073 | BT5-098 Meteor Shower |
| Impmon | not treated as including | Blimpmon, MasterBlimpmon | BT4-069, BT20-049, BT24-058; BT24-062 | BT12-085 Beelzemon (X Antibody) |
| Kimeramon | **treated as including** | MarineChimairamon | BT4-031 | P-205 Insane Synthetic Monster |

## How to read the table

1. **It is a closed list.** The page opens with a general sentence (a card whose
   name contains part of another Digimon's name is not treated as that card).
   The only thing it lets anyone apply, though, is the affected-cards list.
   Implement the list and update it whenever Bandai does.
2. **The table applies to name strings.** A name in the table does not count for
   that fragment, whether it is the printed name or an "also treated as [N]"
   alias. A card that is also treated as [DoruGreymon] still doesn't count for
   [Greymon]. A grant of "having [XX] in its name" (a (Rule) or an effect) still
   counts, because Bandai says the (Rule) wins.
3. **Inclusion rows work like 2-3-4-4.** MarineChimairamon counts for
   "[Kimeramon] in its name". It does not count as the name [Kimeramon] (2-3-1-2).
4. **Matching ignores case.** Bandai lists Indramon under "Dramon". All the
   Dramon-family names spell it with a lowercase "dramon", so a case-sensitive
   match would never hit them. Birdramon, Hydramon and the rest are not listed
   for English, so they still count for [Dramon].
5. **Exact-name references are unaffected.** "[Greymon]" with no "in its name"
   (2-3-1-2) is an exact match and never hit BurningGreymon in the first place.
6. **BT5-092's newest printing drops the parenthetical.** That is the block-02
   reprint with modernised text, now "[Your Turn] … by suspending this Tamer".
   Under 2-1 the newest text governs, and the standardization rules supply the
   exclusion. Its behaviour is therefore the same as the original printing.
7. **Open question: "[XX] in its text".** These scans (the AD1-007-style Q&A:
   name, traits, all text, requirements) may or may not apply the table to the
   name part. Bandai's page covers "[with (XX) in its name] or some version of
   [including (XX)]" and gives no in-its-text example. The engine's
   `in_text_contains` stays literal until there's a ruling.

## Cross-check: DCGO

DCGO's `CardSource.HasGreymonName` / `HasGarurumonName` / `HasAgumonName` /
`HasDramonName` / `HasImpmonName` implement most of the English table: the
three Greymon names, KendoGarurumon, Pagumon, Indramon and Blimpmon. They are
used in 61 / 39 / 28 / 14 / 2 card scripts respectively. They do not cover
MasterBlimpmon, DemiVeemon (for Vee/Veemon), BeelStarmon (for Starmon) or the
MarineChimairamon inclusion. Also, 22 DCGO scripts call the plain
`ContainsCardName("Greymon")` substring check and skip the helper. When a DCGO
exam diverges on one of these names, the official table decides who is right,
not DCGO.

## Engine implications (recommended, not yet done)

Put the table and one helper in a single place, for example
`printed_name_includes(name, fragment) -> bool`: a case-insensitive substring
match that also consults the exclusion and inclusion rows. Then send every
name-containment check through that helper:

- `dsl_cards/predicate.rs`: `name_contains` (plus its synth-identity overlay
  arm), `source_name_contains`, `event_target_name_contains`, and the name part
  of `card_in_text_contains` (pending item 7 above);
- `effect_context/mod.rs` `event_card_name_contains`;
- `card_source.rs` `contains_card_name`, which backs
  `self_digivolution_contains_name` / `self_digivolution_sources_contain_name`;
- `permanent.rs` `contains_card_name_for_rules`;
- requirement matching in `dna_digivolve.rs` (alt-path / DNA `name_contains`) and
  `digixros.rs` (material filters).

Blast radius on 2026-10-05: 147 YAML predicate sites in 59 implemented cards
reference an affected fragment. That is Greymon 56 sites / 35 cards, Garurumon
50 / 30, Agumon 28 / 20, Veemon 8 / 6 and Kimeramon 5 / 3, plus BT12-059's
`self_digivolution_contains_name: "Greymon"`. No implemented YAML uses Vee,
Dramon, Starmon or Impmon as a fragment yet. Once the helper lands, BT5-092's
`none_of` becomes redundant and can be removed.

## Change log

- 2026-10-05: found the official rule while triaging BT5-092. It reverses the
  earlier same-day conclusion that DCGO's name exclusions were a Japanese-only
  reading that 2-3-1-3 doesn't support. `cards.json` keeps the printed text
  (`code/tools/archive/fix_card_names_drift_2026_10.py`): which parenthetical a
  card prints is printed-text data, and the rule above is what decides behaviour.
