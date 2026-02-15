"""Cross-validation patterns for checking transpiled scripts against digimoncard.io effect text.

Phases implemented:
  1. Effect text preprocessing (strip tokens, normalize brackets)
  2. Per-effect-block parsing from API text
  3. Timing validation (API timing → script flags)
  4. Forward validation patterns (API mentions X → script should have it)
  5. Reverse validation patterns (script claims X → API should mention it)
  6. Structural validation (inherited/security presence checks)
  7. Look-ahead pattern discovery (scan API data for pattern coverage)
"""
import json
import os
import re
import urllib.request
from dataclasses import dataclass, field
from typing import List, Dict, Optional, Tuple

# ─── Phase 1: Preprocessing ─────────────────────────────────────────

# Strip token stat blocks: (Digimon/Color/DP/＜Keywords＞)
RE_TOKEN_DESC = re.compile(r'\(Digimon/[^)]*\)')
# Strip parenthesized keyword explanations: (This Digimon checks 1 additional security card.)
RE_KEYWORD_EXPLANATION = re.compile(r'\([^)]{15,}\.\)')


def preprocess_effect_text(text: str) -> str:
    """Clean API effect text for pattern matching.

    1. Strip token stat descriptions (prevents false keyword matches)
    2. Strip long parenthesized explanations
    3. Normalize full-width angle brackets ＜/＞ to </>
    """
    if not text:
        return ""
    text = RE_TOKEN_DESC.sub("", text)
    text = RE_KEYWORD_EXPLANATION.sub("", text)
    text = text.replace("\uff1c", "<").replace("\uff1e", ">")
    return text


# ─── Phase 2: Per-Effect-Block Parsing ───────────────────────────────

TIMING_LABELS = {
    "On Play", "When Digivolving", "Your Turn", "All Turns",
    "When Attacking", "Security", "Main", "On Deletion",
    "Once Per Turn", "Opponent's Turn", "Start of Your Turn",
    "Start of Your Main Phase", "End of Your Turn", "End of Attack",
    "End of All Turns", "End of Opponent's Turn", "Counter", "Hand",
}

# Regex to extract [Label1] [Label2] prefix from an effect line
RE_TIMING_PREFIX = re.compile(
    r'^\s*(\[(?:' + '|'.join(re.escape(t) for t in sorted(TIMING_LABELS, key=len, reverse=True)) + r')\]\s*)+'
)


@dataclass
class ParsedEffectBlock:
    """A single effect block extracted from API effect text."""
    timing_labels: List[str] = field(default_factory=list)
    text: str = ""
    source: str = "main"         # "main", "inherited", "security"
    is_once_per_turn: bool = False


def _parse_effect_field(text: str, source: str) -> List[ParsedEffectBlock]:
    """Parse a single effect text field into effect blocks."""
    if not text or not text.strip():
        return []

    blocks = []
    # Split on newlines (API uses \r\n)
    lines = re.split(r'\r?\n', text.strip())

    for line in lines:
        line = line.strip()
        if not line:
            continue

        block = ParsedEffectBlock(source=source)

        # Extract timing labels from the front
        remaining = line
        while True:
            m = re.match(r'\s*\[([^\]]+)\]\s*', remaining)
            if m:
                label = m.group(1).strip()
                if label in TIMING_LABELS:
                    if label == "Once Per Turn":
                        block.is_once_per_turn = True
                    else:
                        block.timing_labels.append(label)
                    remaining = remaining[m.end():]
                else:
                    break
            else:
                break

        block.text = remaining.strip()
        if block.text or block.timing_labels:
            blocks.append(block)

    return blocks


def parse_api_effect_text(card_meta: dict) -> List[ParsedEffectBlock]:
    """Parse all effect text from a card's metadata into structured blocks."""
    blocks = []

    # Main effect
    main_text = card_meta.get("effect_description_eng", "") or ""
    blocks.extend(_parse_effect_field(main_text, "main"))

    # Inherited / source effect
    inherited_text = card_meta.get("inherited_effect_description_eng", "") or ""
    # The API stores security effects in source_effect prefixed with "Security Effect"
    if inherited_text:
        security_prefix = "Security Effect"
        lines = re.split(r'\r?\n', inherited_text.strip())
        inh_lines = []
        sec_lines = []
        in_security = False
        for line in lines:
            stripped = line.strip()
            if stripped.startswith(security_prefix):
                in_security = True
                # Remove the "Security Effect" prefix
                remainder = stripped[len(security_prefix):].strip()
                if remainder:
                    sec_lines.append(remainder)
            elif in_security:
                sec_lines.append(stripped)
            else:
                inh_lines.append(stripped)

        if inh_lines:
            blocks.extend(_parse_effect_field("\n".join(inh_lines), "inherited"))
        if sec_lines:
            blocks.extend(_parse_effect_field("\n".join(sec_lines), "security"))

    # Security effect (explicit field, usually empty in our data)
    sec_text = card_meta.get("security_effect_description_eng", "") or ""
    if sec_text.strip():
        blocks.extend(_parse_effect_field(sec_text, "security"))

    return blocks


# ─── Phase 3: Timing Validation ──────────────────────────────────────

API_TIMING_TO_SCRIPT_FLAG = {
    "On Play": "is_on_play",
    "When Digivolving": "is_when_digivolving",
    "Security": "is_security_effect",
    "On Deletion": "is_on_deletion",
    "When Attacking": "is_on_attack",
}


def check_timing_flags(parsed_blocks: List[ParsedEffectBlock], script_code: str) -> List[str]:
    """Check that API timing labels have corresponding script flags."""
    issues = []

    # Collect all timing labels from API blocks
    for block in parsed_blocks:
        for label in block.timing_labels:
            flag = API_TIMING_TO_SCRIPT_FLAG.get(label)
            if flag and flag not in script_code:
                issues.append(f"timing '{label}' -> {flag} not found")

    # Check inherited effects
    has_inherited = any(b.source == "inherited" and b.text for b in parsed_blocks)
    if has_inherited and "is_inherited_effect" not in script_code:
        issues.append("has inherited effect text but no is_inherited_effect flag")

    # Check once-per-turn
    has_once = any(b.is_once_per_turn for b in parsed_blocks)
    if has_once and "set_max_count_per_turn" not in script_code:
        issues.append("[Once Per Turn] in API but no set_max_count_per_turn")

    return issues


# ─── Phase 4: Forward Validation Patterns ─────────────────────────────

def _build_validation_patterns():
    """Build forward validation patterns: API mentions X → script should implement it.

    All patterns assume preprocessed text (full-width brackets normalized to </>).
    """
    patterns = []

    def _p(name, check_fn, pattern):
        patterns.append((name, check_fn, re.compile(pattern, re.IGNORECASE)))

    # --- Existing patterns (refined) ---
    _p("reveal_top", lambda s: "effect_reveal_and_select" in s,
       r"reveal\s+(?:the\s+)?top\s+\d+\s+cards?")
    _p("play", lambda s: "effect_play_from_zone" in s,
       r"play\s+(?:1|up\s+to\s+\d+|it\b)")
    _p("digivolve_into", lambda s: "effect_digivolve_from_hand" in s,
       r"(?:digivolve\s+into|may\s+digivolve|can\s+digivolve)")
    _p("delete_opponent",
       lambda s: "delete_permanent" in s or "effect_select_opponent_permanent" in s,
       r"delete\s+\d+\s+of\s+your\s+opponent")
    _p("de_digivolve", lambda s: "de_digivolve" in s,
       r"<De-Digivolve\s*\d*>")
    _p("draw_keyword", lambda s: ".draw_cards(" in s,
       r"<Draw\s*\d*>")
    _p("recovery", lambda s: ".recovery(" in s,
       r"<Recovery\s*\+?\d*>")
    _p("memory_gain", lambda s: "add_memory" in s,
       r"gain\s+\d+\s+memory")
    _p("mind_link", lambda s: "effect_link_to_permanent" in s,
       r"<Mind\s*Link>")
    _p("blocker", lambda s: "_is_blocker" in s, r"<Blocker>")
    _p("piercing", lambda s: "_is_piercing" in s, r"<Piercing>")
    _p("rush", lambda s: "_is_rush" in s, r"<Rush>")
    _p("reboot", lambda s: "_is_reboot" in s, r"<Reboot>")
    _p("raid", lambda s: "_is_raid" in s, r"<Raid>")
    _p("dp_modification",
       lambda s: "change_dp" in s or "dp_modifier" in s,
       r"(?:get|gain)s?\s+[+\-]\d+\s*DP")
    _p("suspend_target",
       lambda s: ".suspend(" in s or "SuspendPermanents" in s,
       r"suspend\s+\d+")
    _p("security_attack_mod", lambda s: "_security_attack_modifier" in s,
       r"<Security\s+Attack\s+[+\-]\d+>")

    # --- New Phase 4 patterns ---
    _p("bounce",
       lambda s: "bounce_permanent_to_hand" in s or "on_bounce" in s,
       r"return\s+\d+|return\s+(?:it|them|all)\s+to\s+(?:the\s+|their\s+)?(?:hand|owner)")
    _p("trash_from_hand",
       lambda s: "effect_select_hand_card" in s,
       r"(?:trash|discard)\s+\d+.*from.*hand")
    _p("mill",
       lambda s: "mill_count" in s or "mill(" in s,
       r"(?:trash|place).*top.*cards?.*(?:deck|trash)")
    _p("unsuspend", lambda s: "unsuspend" in s, r"\bunsuspend\b")
    _p("token_play",
       lambda s: "play_token" in s,
       r"play\s+\d+\s+\[.*?\]\s*(?:\(.*?\)\s*)?Token")
    _p("security_trash",
       lambda s: "destroy_security" in s,
       r"(?:trash|remove)\s+\d+.*(?:your\s+opponent'?s?\s+)?security")
    _p("once_per_turn",
       lambda s: "set_max_count_per_turn" in s,
       r"\[Once Per Turn\]")
    _p("jamming", lambda s: "_is_jamming" in s, r"<Jamming>")
    _p("evade", lambda s: "_is_evade" in s, r"<Evade>")
    _p("armor_purge", lambda s: "_is_armor_purge" in s, r"<Armor Purge>")
    _p("retaliation", lambda s: "_is_retaliation" in s, r"<Retaliation>")
    _p("alliance", lambda s: "_is_alliance" in s, r"<Alliance>")
    _p("delay", lambda s: "_is_delay" in s, r"<Delay>")

    return patterns


# ─── Phase 5: Reverse Validation Patterns ────────────────────────────

def _build_reverse_patterns():
    """Build reverse validation patterns: script claims X → API should mention it."""
    patterns = []

    def _r(script_check, api_pattern):
        patterns.append((script_check, re.compile(api_pattern, re.IGNORECASE)))

    _r("_is_blocker", r"<Blocker>")
    _r("_is_rush", r"<Rush>")
    _r("_is_piercing", r"<Piercing>")
    _r("_is_reboot", r"<Reboot>")
    _r("_is_raid", r"<Raid>")
    _r("_is_jamming", r"<Jamming>")
    _r("_is_evade", r"<Evade>")
    _r("_is_armor_purge", r"<Armor Purge>")
    _r("_is_retaliation", r"<Retaliation>")
    _r("_is_alliance", r"<Alliance>")
    _r("_is_barrier", r"<Barrier>")
    _r("_is_delay", r"<Delay>")
    _r("_security_attack_modifier", r"<Security\s+A(?:ttack)?\.?\s*[+\-]\d+>")

    return patterns


# ─── Phase 6: Structural Validation ──────────────────────────────────

def check_structural_match(
    parsed_blocks: List[ParsedEffectBlock],
    script_code: str,
    card_id: str,
) -> List[str]:
    """Check structural alignment between API effect text and script."""
    warnings = []

    has_inherited = any(b.source == "inherited" and b.text for b in parsed_blocks)
    has_security = any(b.source == "security" and b.text for b in parsed_blocks)

    if has_inherited and "is_inherited_effect" not in script_code:
        warnings.append(f"API has inherited effect but script has no is_inherited_effect")

    if has_security and "is_security_effect" not in script_code:
        warnings.append(f"API has security effect but script has no is_security_effect")

    return warnings


# ─── Phase 7: Look-Ahead Pattern Discovery ───────────────────────────

# Patterns for look-ahead scanning (superset of forward validation)
SCAN_KEYWORD_PATTERNS = [
    ("Blocker", re.compile(r"<Blocker>", re.I)),
    ("Rush", re.compile(r"<Rush>", re.I)),
    ("Piercing", re.compile(r"<Piercing>", re.I)),
    ("Reboot", re.compile(r"<Reboot>", re.I)),
    ("Raid", re.compile(r"<Raid>", re.I)),
    ("Jamming", re.compile(r"<Jamming>", re.I)),
    ("Evade", re.compile(r"<Evade>", re.I)),
    ("Armor Purge", re.compile(r"<Armor Purge>", re.I)),
    ("Retaliation", re.compile(r"<Retaliation>", re.I)),
    ("Alliance", re.compile(r"<Alliance>", re.I)),
    ("Barrier", re.compile(r"<Barrier>", re.I)),
    ("Delay", re.compile(r"<Delay>", re.I)),
    ("Decoy", re.compile(r"<Decoy>", re.I)),
    ("Fortitude", re.compile(r"<Fortitude>", re.I)),
    ("Save", re.compile(r"<Save>", re.I)),
    ("Material Save", re.compile(r"<Material Save>", re.I)),
    ("Collision", re.compile(r"<Collision>", re.I)),
    ("Blitz", re.compile(r"<Blitz>", re.I)),
    ("Overclock", re.compile(r"<Overclock>", re.I)),
    ("Ice Clad", re.compile(r"<Ice Clad>", re.I)),
    ("Mind Link", re.compile(r"<Mind\s*Link>", re.I)),
    ("Blast Digivolve", re.compile(r"<Blast\s+(?:DNA\s+)?Digivolve>", re.I)),
    ("Security Attack +/-", re.compile(r"<Security\s+A(?:ttack)?\.?\s*[+\-]\d+>", re.I)),
    ("Draw", re.compile(r"<Draw\s*\d*>", re.I)),
    ("Recovery", re.compile(r"<Recovery\s*\+?\d*>", re.I)),
    ("De-Digivolve", re.compile(r"<De-Digivolve\s*\d*>", re.I)),
]

SCAN_ACTION_PATTERNS = [
    ("play", re.compile(r"play\s+(?:1|up\s+to\s+\d+|it\b)", re.I)),
    ("reveal_top", re.compile(r"reveal\s+(?:the\s+)?top\s+\d+\s+cards?", re.I)),
    ("delete_opponent", re.compile(r"delete\s+\d+\s+of\s+your\s+opponent", re.I)),
    ("bounce", re.compile(r"return\s+\d+|return\s+(?:it|them|all)\s+to\s+(?:the\s+)?(?:hand|owner)", re.I)),
    ("digivolve", re.compile(r"(?:digivolve\s+into|may\s+digivolve|can\s+digivolve)", re.I)),
    ("suspend", re.compile(r"suspend\s+\d+", re.I)),
    ("memory_gain", re.compile(r"gain\s+\d+\s+memory", re.I)),
    ("dp_change", re.compile(r"(?:get|gain)s?\s+[+\-]\d+\s*DP", re.I)),
    ("token", re.compile(r"play\s+\d+\s+\[.*?\].*Token", re.I)),
    ("trash_from_hand", re.compile(r"(?:trash|discard)\s+\d+.*from.*hand", re.I)),
    ("mill", re.compile(r"(?:trash|place).*top.*cards?.*(?:deck|trash)", re.I)),
    ("unsuspend", re.compile(r"\bunsuspend\b", re.I)),
    ("security_trash", re.compile(r"(?:trash|remove)\s+\d+.*security", re.I)),
    ("cost_reduction", re.compile(r"reduce\s+(?:the\s+)?(?:play|digivolve|digivolution|evolution)\s+cost", re.I)),
    ("multi_choice", re.compile(r"(?:activate|use)\s+\d+\s+of\s+(?:the\s+)?(?:effects?|following)", re.I)),
    ("stack_manipulation", re.compile(r"(?:place|add)\s+.*(?:as\s+(?:a\s+)?digivolution\s+cards?|(?:under|to)\s+(?:this|it|one).*digivolution)", re.I)),
    ("protection", re.compile(r"(?:prevent\s+(?:it|that|them)\s+from\s+(?:leaving|being)|isn't\s+(?:deleted|destroyed))", re.I)),
    ("restriction", re.compile(r"can'?t\s+(?:be\s+)?(?:deleted|attack|blocked|suspend|play)", re.I)),
]

SCAN_TIMING_PATTERNS = [
    ("[On Play]", re.compile(r"\[On Play\]")),
    ("[When Digivolving]", re.compile(r"\[When Digivolving\]")),
    ("[Your Turn]", re.compile(r"\[Your Turn\]")),
    ("[All Turns]", re.compile(r"\[All Turns\]")),
    ("[When Attacking]", re.compile(r"\[When Attacking\]")),
    ("[Security]", re.compile(r"\[Security\]")),
    ("[Main]", re.compile(r"\[Main\]")),
    ("[On Deletion]", re.compile(r"\[On Deletion\]")),
    ("[Once Per Turn]", re.compile(r"\[Once Per Turn\]")),
    ("[Opponent's Turn]", re.compile(r"\[Opponent'?s Turn\]")),
    ("[Start of Your Turn]", re.compile(r"\[Start of Your Turn\]")),
    ("[Start of Your Main Phase]", re.compile(r"\[Start of Your Main Phase\]")),
    ("[End of Your Turn]", re.compile(r"\[End of Your Turn\]")),
    ("[Counter]", re.compile(r"\[Counter\]")),
    ("[Hand]", re.compile(r"\[Hand\]")),
]

# Keywords the transpiler currently handles (emit flags for)
TRANSPILER_KEYWORDS = {
    "Blocker", "Rush", "Piercing", "Reboot", "Raid", "Jamming", "Evade",
    "Armor Purge", "Retaliation", "Alliance", "Barrier", "Delay", "Decoy",
    "Fortitude", "Save", "Material Save", "Collision", "Blitz", "Overclock",
    "Ice Clad", "Mind Link", "Blast Digivolve", "Security Attack +/-",
    "Draw", "Recovery", "De-Digivolve",
}

# Actions the transpiler currently handles
TRANSPILER_ACTIONS = {
    "play", "reveal_top", "delete_opponent", "bounce", "digivolve",
    "suspend", "memory_gain", "dp_change", "token", "trash_from_hand",
    "mill", "unsuspend", "security_trash", "cost_reduction",
    "stack_manipulation",
}


def scan_cards_for_patterns(cards: List[dict]) -> dict:
    """Analyze a list of card metadata dicts for pattern coverage.

    Returns dict with keyword_counts, action_counts, timing_counts,
    uncovered_keywords, uncovered_actions.
    """
    keyword_counts = {}
    action_counts = {}
    timing_counts = {}

    for card in cards:
        # Build combined preprocessed text
        main_text = preprocess_effect_text(card.get("effect_description_eng", "") or "")
        inh_text = preprocess_effect_text(card.get("inherited_effect_description_eng", "") or "")
        sec_text = preprocess_effect_text(card.get("security_effect_description_eng", "") or "")
        combined = main_text + " " + inh_text + " " + sec_text

        for name, pat in SCAN_KEYWORD_PATTERNS:
            if pat.search(combined):
                keyword_counts[name] = keyword_counts.get(name, 0) + 1

        for name, pat in SCAN_ACTION_PATTERNS:
            if pat.search(combined):
                action_counts[name] = action_counts.get(name, 0) + 1

        # Timing: use raw (not preprocessed) text to preserve brackets
        raw_combined = (card.get("effect_description_eng", "") or "") + " " + \
                       (card.get("inherited_effect_description_eng", "") or "")
        for name, pat in SCAN_TIMING_PATTERNS:
            if pat.search(raw_combined):
                timing_counts[name] = timing_counts.get(name, 0) + 1

    uncovered_keywords = {k for k in keyword_counts if k not in TRANSPILER_KEYWORDS}
    uncovered_actions = {k for k in action_counts if k not in TRANSPILER_ACTIONS}

    return {
        "keyword_counts": keyword_counts,
        "action_counts": action_counts,
        "timing_counts": timing_counts,
        "uncovered_keywords": uncovered_keywords,
        "uncovered_actions": uncovered_actions,
    }


def fetch_api_set(set_id: str) -> List[dict]:
    """Fetch card data from digimoncard.io for a set ID.

    Uses the simpler search.php?card= prefix format.
    """
    url = f"https://digimoncard.io/api-public/search.php?card={set_id}"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    resp = urllib.request.urlopen(req, timeout=30)
    api_data = json.loads(resp.read().decode())

    # Deduplicate, filtering to exact set prefix
    prefix = set_id + "-"
    seen = {}
    for card in api_data:
        cid = card.get("id", "")
        if not cid.startswith(prefix):
            continue
        if cid not in seen:
            seen[cid] = card

    # Convert to our format
    from tools.ingest_cards import convert_card
    return [convert_card(seen[cid]) for cid in sorted(seen.keys())]


def scan_api_set(set_id: str, cards: Optional[List[dict]] = None) -> dict:
    """Scan a set's effect text for pattern coverage.

    If cards is None, fetches from the API. Otherwise uses provided cards.
    Returns scan results dict.
    """
    if cards is None:
        cards = fetch_api_set(set_id)

    results = scan_cards_for_patterns(cards)
    results["set_id"] = set_id
    results["card_count"] = len(cards)
    return results


def format_scan_report(results: dict) -> str:
    """Format scan results into a human-readable report."""
    lines = []
    set_id = results.get("set_id", "???")
    count = results.get("card_count", 0)
    lines.append(f"=== {set_id} API Pattern Scan ({count} cards) ===")
    lines.append("")

    # Keywords
    kw = results["keyword_counts"]
    if kw:
        lines.append("Keyword Coverage:")
        for name, cnt in sorted(kw.items(), key=lambda x: -x[1]):
            marker = "  ** NOT in transpiler **" if name not in TRANSPILER_KEYWORDS else ""
            lines.append(f"  {name:<25s} {cnt:>3d} cards{marker}")
        lines.append("")

    # Actions
    ac = results["action_counts"]
    if ac:
        lines.append("Action Coverage:")
        for name, cnt in sorted(ac.items(), key=lambda x: -x[1]):
            marker = "  ** NOT in transpiler **" if name not in TRANSPILER_ACTIONS else ""
            lines.append(f"  {name:<25s} {cnt:>3d} cards{marker}")
        lines.append("")

    # Timings
    tc = results["timing_counts"]
    if tc:
        lines.append("Timing Labels:")
        for name, cnt in sorted(tc.items(), key=lambda x: -x[1]):
            lines.append(f"  {name:<30s} {cnt:>3d} cards")
        lines.append("")

    # Gaps
    gaps = results.get("uncovered_keywords", set()) | results.get("uncovered_actions", set())
    if gaps:
        lines.append(f"Uncovered patterns ({len(gaps)}):")
        for g in sorted(gaps):
            lines.append(f"  {g}")
        lines.append("")

    return "\n".join(lines)


def read_priority_sets() -> List[str]:
    """Read set IDs from priority_sets.txt."""
    path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
        "digimon_gym", "scraper", "priority_sets.txt"
    )
    import re as _re
    if not os.path.exists(path):
        return []
    set_ids = []
    with open(path, "r") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("Processed") or line.startswith("Set Priority"):
                continue
            set_id = line.split(":")[0].strip()
            if set_id and _re.match(r'^[A-Z]+\d*$', set_id):
                set_ids.append(set_id)
    return set_ids


# ─── Unified Validation Entry Point ──────────────────────────────────

@dataclass
class ValidationResult:
    """Complete validation results for a single card."""
    card_id: str = ""
    forward_issues: List[str] = field(default_factory=list)
    reverse_issues: List[str] = field(default_factory=list)
    timing_issues: List[str] = field(default_factory=list)
    structural_warnings: List[str] = field(default_factory=list)


def validate_card(
    card_id: str,
    card_meta: dict,
    script_code: str,
    forward_patterns=None,
    reverse_patterns=None,
) -> ValidationResult:
    """Run all validation checks for a single card."""
    if forward_patterns is None:
        forward_patterns = _build_validation_patterns()
    if reverse_patterns is None:
        reverse_patterns = _build_reverse_patterns()

    result = ValidationResult(card_id=card_id)

    # Preprocess effect text
    main_text = preprocess_effect_text(card_meta.get("effect_description_eng", "") or "")
    inh_text = preprocess_effect_text(card_meta.get("inherited_effect_description_eng", "") or "")
    sec_text = preprocess_effect_text(card_meta.get("security_effect_description_eng", "") or "")
    combined = main_text + " " + inh_text + " " + sec_text

    if not combined.strip():
        return result

    # Phase 2: Parse effect blocks
    parsed_blocks = parse_api_effect_text(card_meta)

    # Phase 3: Timing validation
    result.timing_issues = check_timing_flags(parsed_blocks, script_code)

    # Phase 4: Forward validation
    for pname, check_fn, pregex in forward_patterns:
        if pregex.search(combined) and not check_fn(script_code):
            result.forward_issues.append(pname)

    # Phase 5: Reverse validation
    for script_check, api_regex in reverse_patterns:
        if script_check in script_code and not api_regex.search(combined):
            result.reverse_issues.append(script_check)

    # Phase 6: Structural validation
    result.structural_warnings = check_structural_match(parsed_blocks, script_code, card_id)

    return result
