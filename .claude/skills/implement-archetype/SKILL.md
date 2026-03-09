---
name: implement-archetype
description: Implement all cards for a Digimon TCG archetype by scraping tournament decklists, analyzing missing cards, ingesting metadata, transpiling scripts, and promoting to frozen lane. Use when asked to implement an archetype, make an archetype playable, or implement cards for a competitive deck.
argument-hint: <ARCHETYPE_NAME> [--scrape URL ...] [--min-meta-share 0.02] [--skip-scrape]
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, WebFetch, Agent, TodoWrite
---

# Implement a Digimon TCG Archetype

You are implementing all missing cards for archetype **$ARGUMENTS** so that competitive
decklists for this archetype become fully playable in the game engine.

Unlike `implement-set` (which processes an entire booster set), this skill targets
only the cards actually used in tournament decklists for a specific archetype.
This is more efficient — a typical archetype uses 25-40 unique cards spanning 5-10 sets.

## Quick Reference

- **CLAUDE.md** — project overview, architecture, common commands
- **RULES_CONTEXT.md** — official Digimon TCG rules (keyword mechanics, effect timing)
- **Official Rule PDFs** (for ambiguous rules):
  - Comprehensive Rules: https://world.digimoncard.com/rule/pdf/general_rule.pdf?20251225
  - Rule Manual: https://world.digimoncard.com/rule/pdf/manual.pdf?20250711
- **Card API**: `https://digimoncard.io/index.php/api-public/search?card=<CARD_ID>`
- For detailed review procedures, see [../implement-set/review-checklist.md](../implement-set/review-checklist.md)
- For common transpiler issues and fixes, see [../implement-set/transpiler-fixes.md](../implement-set/transpiler-fixes.md)

---

## Phase 0: Scrape Fresh Tournament Data (Optional)

**Goal**: Update deck_library.json with latest tournament results to ensure the
archetype definition reflects the current meta.

Skip this phase if `--skip-scrape` is passed or if the user confirms the library is current.

### 0a. Identify scrape sources

Search for recent tournament data for the archetype. The three supported sources are:

1. **DigimonMeta.com** — largest tournament database, searchable by archetype
2. **Egman Events** — North American tournament organizer
3. **DigimonCard.io** — tournament deck submissions

If the user provided `--scrape URL` arguments, use those directly. Otherwise, help
them find relevant tournament pages:

- DigimonMeta: search `https://digimonmeta.com` for the archetype name
- Egman: check `https://egmanevents.com` for recent event results
- DigimonCard.io: search tournaments at `https://digimoncard.io`

### 0b. Scrape decklists

```bash
# DigimonMeta (most common — excludes JP format by default)
python tools/meta_loader.py --scrape-digimonmeta "URL" --build

# Egman Events
python tools/meta_loader.py --scrape-egman "URL" --build

# DigimonCard.io
python tools/meta_loader.py --scrape-digimoncard-io "URL" --build
```

Multiple sources can be scraped in sequence — the tool merges incrementally and
deduplicates when `--build` is passed.

### 0c. Verify scrape results

```bash
python tools/meta_loader.py --report
```

Confirm the target archetype appears with reasonable deck counts. If the archetype
name doesn't match what's in the library, check for variant spellings.

---

## Phase 1: Analyze Archetype Card Pool

**Goal**: Identify every unique card used across all decklists for this archetype,
then determine which cards are already implemented and which are missing.

### 1a. Collect all unique cards

```bash
python tools/archetype_analyzer.py "$ARGUMENTS"
```

This script outputs:
- Total unique cards across all decklists for the archetype
- Cards already frozen (implemented)
- Cards missing from frozen manifest, grouped by set
- Per-card usage frequency (how many decklists include it)
- Estimated archetype coverage percentage
- Sets that need C# source files for transpilation

If `archetype_analyzer.py` is not available, run the analysis manually:

```python
import json
from pathlib import Path
from collections import Counter

lib = json.loads(Path('digimon_gym/engine/data/deck_library.json').read_text())
manifest = json.loads(Path('digimon_gym/engine/data/scripts/_frozen_manifest.json').read_text())
frozen_ids = set(manifest.get('cards', {}).keys())

archetype = lib['archetypes'].get('ARCHETYPE_NAME')
if not archetype:
    # Try fuzzy match
    for name in lib['archetypes']:
        if 'ARCHETYPE_NAME'.lower() in name.lower():
            archetype = lib['archetypes'][name]
            print(f'Matched: {name}')
            break

all_cards = Counter()
for dl in archetype.get('decklists', []):
    unique_in_deck = set(json.loads(dl['decklist']))
    for cid in unique_in_deck:
        all_cards[cid] += 1

implemented = {cid for cid in all_cards if cid in frozen_ids}
missing = {cid: count for cid, count in all_cards.items() if cid not in frozen_ids}

print(f'Total unique cards: {len(all_cards)}')
print(f'Already implemented: {len(implemented)}')
print(f'Missing: {len(missing)}')
print(f'Coverage: {len(implemented)/len(all_cards)*100:.1f}%')
print()

# Group missing by set
from collections import defaultdict
by_set = defaultdict(list)
for cid, freq in sorted(missing.items(), key=lambda x: -x[1]):
    set_id = cid.split('-')[0]
    by_set[set_id].append((cid, freq))

print('Missing cards by set:')
for set_id, cards in sorted(by_set.items()):
    print(f'  {set_id}: {len(cards)} cards')
    for cid, freq in cards:
        print(f'    {cid} (in {freq}/{len(archetype["decklists"])} decklists)')
```

### 1b. Prioritize cards

Cards should be implemented in priority order:
1. **Core cards** — appear in >75% of the archetype's decklists (archetype-defining)
2. **Common staples** — appear in 50-75% of decklists
3. **Tech choices** — appear in 25-50% of decklists
4. **Fringe includes** — appear in <25% of decklists

Focus on groups 1-3 first to make the majority of decklists playable.

### 1c. Check related archetypes

Some cards are shared across archetypes. Before implementing, check if the missing
cards appear in other high-meta-share archetypes — implementing them benefits
multiple archetypes at once.

```python
# Find which other archetypes use the missing cards
shared = {}
for cid in missing:
    users = []
    for name, arch in lib['archetypes'].items():
        for dl in arch.get('decklists', []):
            if cid in json.loads(dl['decklist']):
                users.append(name)
                break
    if len(users) > 1:
        shared[cid] = users
```

---

## Phase 2: Ingest Card Metadata

**Goal**: Ensure all missing cards exist in `cards.json`.

### 2a. Ingest by set

For each set that has missing cards:

```bash
python tools/ingest_cards.py --set SET_ID
```

If only a few cards are missing from a set that's mostly ingested, verify they
exist individually:

```bash
python -c "
from digimon_gym.engine.data.card_database import CardDatabase
db = CardDatabase()
for cid in ['CARD_ID_1', 'CARD_ID_2']:
    card = db.get_card(cid)
    if card:
        print(f'{cid}: {card.card_name_eng} (Lv.{card.level} {card.card_kind.name})')
    else:
        print(f'{cid}: NOT IN DATABASE')
"
```

### 2b. Verify card data

For each missing card, fetch official data to understand what needs to be implemented:

```bash
python -c "
from digimon_gym.engine.data.card_database import CardDatabase
db = CardDatabase()
card = db.get_card('CARD_ID')
if card:
    print(f'Name: {card.card_name_eng}')
    print(f'Kind: {card.card_kind.name}')
    print(f'Level: {card.level}')
    print(f'Play Cost: {card.play_cost}')
    print(f'DP: {card.dp}')
    print(f'Colors: {[c.name for c in card.card_colors]}')
    print(f'Effect: {card.effect_description_eng}')
    print(f'Inherited: {card.inherited_effect_description_eng}')
    print(f'Security: {card.security_effect_description_eng}')
    print(f'Evo Costs: {[(e.card_color.name, e.level, e.memory_cost) for e in card.evo_costs]}')
"
```

---

## Phase 3: Transpile Missing Card Scripts

**Goal**: Generate Python CardScript files for each missing card.

### 3a. Check for C# source availability

The transpiler converts DCGO C# CardEffect scripts to Python. Check if the C#
sources exist for each needed set:

```bash
ls DCGO/Assets/Scripts/CardEffect/ | grep -i SET_ID
```

### 3b. Transpile per-set

For each set with missing cards that has C# sources:

```bash
python tools/transpile_dcgo.py \
    DCGO/Assets/Scripts/CardEffect/SET_ID \
    digimon_gym/engine/data/scripts/generated/{set_lower}
```

**Important**: This transpiles the entire set, not individual cards. Cards that are
already frozen will not be affected (frozen scripts live in the non-generated directory).

### 3c. If no C# sources exist

For cards without C# sources (very new sets, promo cards), you must write the
CardScript manually. Use existing frozen scripts as templates:

```bash
# Find a similar card to use as template
grep -r "class.*CardScript" digimon_gym/engine/data/scripts/bt23/ | head -5
```

Write the script following the CardScript pattern from existing implementations.
Place it in `digimon_gym/engine/data/scripts/generated/{set_lower}/`.

### 3d. Validate transpiled scripts

Check that each missing card's script is importable and returns effects:

```python
import importlib, os, sys
sys.path.insert(0, '.')

missing_cards = [('SET_LOWER', 'CARD_ID'), ...]  # from Phase 1
errors = []

for set_lower, card_id in missing_cards:
    module_name = f'{set_lower}_{card_id.replace("-", "_").lower()}'
    full_module = f'digimon_gym.engine.data.scripts.generated.{set_lower}.{module_name}'
    try:
        mod = importlib.import_module(full_module)
        fn = getattr(mod, 'get_card_effects', None)
        if fn is None:
            errors.append((card_id, 'no get_card_effects function'))
    except Exception as e:
        errors.append((card_id, str(e)))

for card_id, err in errors:
    print(f'  {card_id}: {err}')
print(f'{len(errors)} errors out of {len(missing_cards)} cards')
```

---

## Phase 4: Review Card Scripts

**Goal**: Verify each transpiled script faithfully implements the card's official effects.

Follow the review procedures in [../implement-set/review-checklist.md](../implement-set/review-checklist.md).

### 4a. Review each missing card

For each card that was transpiled or written:

1. Fetch the official card text from the API or `cards.json`
2. Read the generated Python script
3. Compare every effect, timing, keyword, and condition
4. Check for the common issues listed in [../implement-set/transpiler-fixes.md](../implement-set/transpiler-fixes.md)

### 4b. Keyword and timing verification

Pay special attention to:
- **[On Play]** → `is_on_play = True`
- **[When Digivolving]** → `is_when_digivolving = True`
- **[When Attacking]** → timing at attack declaration
- **[On Deletion]** → `is_on_deletion = True`
- **"may"** → `is_optional = True`
- **"by [doing X]"** → cost targeting own permanent, optional
- **Rush**, **Blocker**, **Piercing**, **Security Attack +/-** — see RULES_CONTEXT.md

### 4c. Fix issues

If transpiler output is wrong:

1. **Transpiler pattern gap** → Fix in `tools/transpiler/` and re-transpile
2. **Individual card complexity** → Check `tools/transpiler/known_complex_cards.json`
3. **Manual fix needed** → Write the script by hand (place in generated dir)

**Transpiler-first policy**: Always prefer fixing the transpiler over manually
editing individual scripts, unless the card is truly unique.

---

## Phase 5: Promote to Frozen Lane

**Goal**: Move reviewed scripts from `generated/` to the frozen lane.

### 5a. Promote each card

```python
from pathlib import Path
from digimon_gym.engine.data.script_promotion import promote_script_from_generated, _sha256

cards_to_promote = [
    # (card_id, set_id, module_name) — only the MISSING cards from Phase 1
    ("BT13-001", "bt13", "bt13_001"),
    # ...
]

for card_id, set_id, module_name in cards_to_promote:
    gen_path = Path(f"digimon_gym/engine/data/scripts/generated/{set_id}/{module_name}.py")
    if not gen_path.exists():
        print(f"SKIP {card_id}: no generated script")
        continue
    gen_hash = _sha256(gen_path)
    result = promote_script_from_generated(
        card_id=card_id,
        set_id=set_id,
        module_name=module_name,
        expected_generated_hash=gen_hash,
    )
    print(f"Promoted {card_id}: manifest v{result['manifest_version']}")
```

### 5b. Check QA-validated cards

Before promoting, check if any of these cards have been QA-validated:

```python
import json
from pathlib import Path

validated_path = Path('docs/qa-reports/validated_cards.json')
if validated_path.exists():
    validated = json.loads(validated_path.read_text())
    validated_ids = {k for k, v in validated.get('cards', {}).items()
                     if v['status'] in ('PASS', 'PARTIAL')}
    conflicts = [c for c, _, _ in cards_to_promote if c in validated_ids]
    if conflicts:
        print(f"WARNING: {len(conflicts)} QA-validated cards would be overwritten")
        # Skip these unless --force
```

### 5c. Verify frozen integrity

```bash
python scripts/check_frozen_integrity.py
```

### 5d. Run tests

```bash
python -m pytest tests/ -v
```

---

## Phase 6: Verify Archetype Playability

**Goal**: Confirm the archetype's decklists are now fully playable.

### 6a. Check coverage

```bash
python -m digimon_gym.engine.data.deck_finder --min-coverage 1.0 --max-results 50 \
    2>/dev/null | python -c "
import json, sys
decks = json.load(sys.stdin)
target = '$ARGUMENTS'
matches = [d for d in decks if target.lower() in d['archetype_name'].lower()]
print(f'Fully playable decklists for {target}: {len(matches)}')
for d in matches:
    print(f'  {d[\"deck_id\"]}: {d[\"card_count\"]} cards, meta_share={d[\"meta_share\"]:.4f}')
"
```

### 6b. Smoke test a game

Create a quick test game with one of the archetype's decklists to verify it loads:

```python
import json, requests
from pathlib import Path
from digimon_gym.engine.data.deck_finder import find_playable_decks

decks = find_playable_decks(min_coverage=1.0, max_results=50)
target = [d for d in decks if 'ARCHETYPE_NAME'.lower() in d.archetype_name.lower()]
if target:
    deck = target[0]
    print(f'Testing: {deck.archetype_name} ({deck.deck_id})')
    print(f'Egg deck: {len(deck.egg_deck)} cards')
    print(f'Main deck: {len(deck.main_deck)} cards')
```

### 6c. Report near-complete decklists

If some decklists are still not fully covered, report which cards remain:

```bash
python -m digimon_gym.engine.data.deck_finder --min-coverage 0.9 --max-results 50
```

---

## Phase 7: Commit and Push

### 7a. Stage changes

```bash
git add digimon_gym/engine/data/cards.json
git add digimon_gym/engine/data/scripts/_frozen_manifest.json

# Add frozen scripts for each set that had missing cards
git add digimon_gym/engine/data/scripts/{set_lower}/

# Add generated scripts if any new ones were created
git add digimon_gym/engine/data/scripts/generated/
```

### 7b. Commit

```bash
git commit -m "feat: implement {ARCHETYPE_NAME} archetype — {N} cards across {M} sets

Cards implemented: {card_id_1}, {card_id_2}, ...
Coverage: {before}% → 100% ({N} decklists now fully playable)"
```

---

## Output Summary

When complete, provide:

1. **Archetype analyzed**: name, meta share, number of decklists
2. **Card pool**: total unique cards, previously implemented, newly implemented
3. **Sets touched**: which sets had cards added
4. **Coverage change**: before% → after% (number of fully-playable decklists)
5. **Cards implemented**: list of card IDs with names
6. **Cross-archetype impact**: other archetypes that benefit from these cards
7. **Remaining gaps**: any cards that could not be implemented and why
8. **Test results**: pass/fail from frozen integrity check and test suite
