"""Prepare an exact-list EX13 refresh without changing a running experiment.

The public HTML collector never uses DigiLab's authenticated API. Collection
and preparation are separate so selection and admission can be reproduced
offline from the recorded candidates. Existing frozen runs are read-only.
"""
from __future__ import annotations

import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import date, datetime, timezone
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
from threading import Event
import time
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[3]
BASE = "https://digimon.digilab.cards"
ARCHETYPES = {
    "Chronicle": "chronicle", "Chronomon": "chronomon", "Examon": "examon",
    "Glowing Dawn": "glowing-dawn", "Light Hybrid": "light-hybrid",
    "Medusamon": "medusamon", "SW": "saiyu-warriors",
    "ShineGreymon Data Squad": "data-squad-shinegreymon",
    "TS Jupitermon": "jupitermon", "TS Mervamon": "ts-minervamon",
    "Three Musketeers": "beelstarmon-3m", "Toho Braves": "toho-braves",
    "Virus Busters": "virus-busters",
}
PETS = {"Light Hybrid", "Medusamon", "SW", "ShineGreymon Data Squad", "Virus Busters"}
START, AS_OF = date(2026, 10, 1), date(2026, 10, 26)
PREPARED_ARTIFACTS = (
    "train_snapshot.json", "heldout_snapshot.json", "pet_regression_snapshot.json",
    "supplemental_snapshot.json", "combined_diagnostic_snapshot.json", "decks.json",
    "distinct_training_decks.json", "pool_report.json", "README.md",
)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def native_extension_hashes(package_dir: Path) -> dict[str, str]:
    return {str(path):digest(path.read_bytes()) for path in sorted(package_dir.iterdir())
            if path.is_file() and path.suffix in (".pyd", ".so")}


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def fingerprint(cards: list[str]) -> str:
    """Content identity includes every copy and the egg deck; ignores order."""
    return digest(json.dumps(sorted(Counter(cards).items()), separators=(",", ":")).encode())


def parsed_event_date(value) -> date | None:
    try:
        return date.fromisoformat(value) if isinstance(value, str) else None
    except ValueError:
        return None


def copy_distance(first: dict, second: dict) -> int:
    a, b = Counter(first["cards"]), Counter(second["cards"])
    # This is replacements plus egg-count changes, not a card-mechanics metric.
    return max(sum((a - b).values()), sum((b - a).values()))


def parse_digilab_page(raw: bytes, url: str, archetype: str) -> dict:
    """Read displayed counts and metadata; reject a renamed/mismatched archetype."""
    from bs4 import BeautifulSoup
    soup = BeautifulSoup(raw, "html.parser")
    structured = [json.loads(node.string) for node in soup.select('script[type="application/ld+json"]') if node.string]
    work = next(row for row in structured if row.get("@type") == "CreativeWork")
    breadcrumbs = next(row for row in structured if row.get("@type") == "BreadcrumbList")["itemListElement"]
    source_slug = breadcrumbs[0]["item"].rstrip("/").rsplit("/", 1)[-1]
    if source_slug != ARCHETYPES[archetype]:
        raise ValueError(f"archetype mismatch: {source_slug} != {ARCHETYPES[archetype]}")
    stats = {node.select_one(".stat-label").get_text(" ", strip=True): node.select_one(".stat-value").get_text(" ", strip=True)
             for node in soup.select(".decklist-stat-cell")}
    counts: Counter = Counter()
    for node in soup.select('a.card-cell-open[href^="/card/"]'):
        card = node["href"].rsplit("/", 1)[-1]
        if not re.fullmatch(r"[A-Z]+[0-9]*-[0-9]+", card):
            raise ValueError(f"unrecognized card ID {card}")
        badge = node.select_one(".card-count-badge")
        count = int(badge.get_text(strip=True).lstrip("×x"))
        if count <= 0 or card in counts:
            raise ValueError(f"invalid or repeated count row: {card}")
        counts[card] = count
    if not counts:
        raise ValueError("no displayed exact card counts")
    record_match = re.fullmatch(r"(\d+)-(\d+)(?:-(\d+))?", stats.get("Record", ""))
    record = dict(zip(("wins", "losses", "draws"), [int(x or 0) for x in record_match.groups()])) if record_match else None
    placement = re.fullmatch(r"(\d+)(?:st|nd|rd|th)", stats.get("Place", ""))
    cards = [card for card, count in sorted(counts.items()) for _ in range(count)]
    return dict(archetype=archetype, source="digilab", source_deck_id="digilab_result_" + url.rsplit("/", 1)[-1],
                source_url=url, source_archetype=breadcrumbs[0]["name"], source_slug=source_slug,
                event_url=breadcrumbs[1]["item"], event_title=breadcrumbs[1]["name"],
                event_date=work["dateCreated"], format=stats.get("Format"),
                player=work.get("author", {}).get("name"), placement=int(placement[1]) if placement else None,
                field_size=int(stats["Players"]) if stats.get("Players", "").isdigit() else None,
                record=record, rounds=int(stats["Rounds"]) if stats.get("Rounds", "").isdigit() else None,
                cards=cards, card_counts=dict(sorted(counts.items())), fingerprint=fingerprint(cards),
                source_page_sha256=digest(raw))


def collect(cache: Path, output: Path) -> dict:
    """Bounded discovery: public first-page best-finish and newest listings."""
    import requests
    from bs4 import BeautifulSoup
    cache.mkdir(parents=True, exist_ok=True)
    denied = Event()

    def get(url: str) -> tuple[bytes, dict]:
        key = digest(url.encode())
        html, meta = cache / f"{key}.html", cache / f"{key}.json"
        if html.exists() and meta.exists():
            info = json.loads(meta.read_text(encoding="utf-8"))
            raw = html.read_bytes()
            if info["url"] != url or info["sha256"] != digest(raw):
                raise ValueError("source cache identity mismatch")
            return raw, info
        if denied.is_set():
            raise requests.RequestException("Collection stopped after access denial; no retry or alternate route")
        time.sleep(.5)
        response = requests.get(url, timeout=45)
        if response.status_code in (401, 403, 429):
            denied.set()
        response.raise_for_status()
        info = dict(url=url, retrieved_utc=datetime.now(timezone.utc).isoformat(), sha256=digest(response.content))
        html.write_bytes(response.content)
        write_json(meta, info)
        return response.content, info

    discovered, discovery, errors = {}, [], []
    jobs = [(arch, BASE + "/decklists?" + urlencode(dict(archetype=slug, format="EX13", source="tournament",
             date_from=START.isoformat(), date_to=AS_OF.isoformat(), sort=sort)))
            for arch, slug in ARCHETYPES.items() for sort in ("finish", "newest")]
    def index(job):
        arch, url = job
        raw, info = get(url)
        soup = BeautifulSoup(raw, "html.parser")
        links = sorted({a["href"] for a in soup.select('a[href]') if re.fullmatch(r"/decklist/\d+", a["href"])})
        return arch, links, info
    with ThreadPoolExecutor(max_workers=1) as executor:
        for arch, links, info in executor.map(index, jobs):
            discovery.append(dict(archetype=arch, numeric_deck_links=len(links), **info))
            for link in links:
                discovered[BASE + link] = arch
    # Explicit user-investigation targets, regardless of a live index reorder.
    for result_id in (126919, 125077):
        discovered[f"{BASE}/decklist/{result_id}"] = "Three Musketeers"

    def detail(job):
        url, arch = job
        try:
            raw, info = get(url)
            return parse_digilab_page(raw, url, arch) | {"retrieved_utc": info["retrieved_utc"], "source_verification":"fresh_public_HTML_capture"}, None
        except (ValueError, KeyError, StopIteration, AttributeError, requests.RequestException) as exc:
            return None, dict(url=url, archetype=arch, error=f"{type(exc).__name__}: {exc}")
    rows = []
    with ThreadPoolExecutor(max_workers=1) as executor:
        for row, error in executor.map(detail, sorted(discovered.items())):
            if row:
                rows.append(row)
            else:
                errors.append(error)
    result = dict(schema="ex13-primary-candidates-v1", start=START.isoformat(), as_of=AS_OF.isoformat(),
                  source_scope="Global EX13; no language or country restriction",
                  discovery_scope="First public best-finish and newest browse page per archetype; bounded candidates, not exhaustive tournament coverage",
                  discovery=discovery, candidates=rows, errors=errors)
    write_json(output, result)
    print(json.dumps({"candidate_counts":dict(Counter(row["archetype"] for row in rows)), "fetch_errors":len(errors)}), flush=True)
    return result


def qualification_reasons(row: dict) -> list[str]:
    reasons = []
    if row.get("format") != "EX13":
        reasons.append("not_explicit_EX13")
    when = parsed_event_date(row.get("event_date"))
    if when is None:
        reasons.append("event_date_unmeasured")
    elif not START <= when <= AS_OF:
        reasons.append("outside_October_window")
    n, place, record = row.get("field_size"), row.get("placement"), row.get("record")
    if n is None or n < 8:
        reasons.append("field_below_8_or_unmeasured")
    if not n or not place or place > math.ceil(n / 3):
        reasons.append("outside_top_third_or_unmeasured")
    if record and (record["wins"] < 2 or record["wins"] < record["losses"]):
        reasons.append("no_nonlosing_two_win_record")
    return reasons


def merge_sources(fresh_path: Path, library_path: Path, extras_path: Path | None, output: Path) -> dict:
    """Preserve provenance when supplementing a bounded fetch with prior records."""
    fresh = json.loads(fresh_path.read_text(encoding="utf-8"))
    library = json.loads(library_path.read_text(encoding="utf-8"))
    rows = [row | {"source_verification":row.get("source_verification", "fresh_public_HTML_capture")} for row in fresh["candidates"]]
    aliases = {name:name for name in ARCHETYPES if name not in PETS} | {"SW":"Saiyu Warriors", "Virus Busters":"Virus Busters", "Medusamon":"Medusamon"}
    for arch, library_name in aliases.items():
        for raw in library["archetypes"].get(library_name, {}).get("decklists", []):
            when = raw.get("event_date")
            parsed_date = parsed_event_date(when)
            if raw.get("format") != "EX13" or parsed_date is None or not START <= parsed_date <= AS_OF:
                continue
            cards = json.loads(raw["decklist"]) if isinstance(raw["decklist"], str) else list(raw["decklist"])
            placement = str(raw.get("placement") or "")
            # No missing record is filled with zero and no event identity is
            # invented from a list URL. Unknown events receive no diversity bonus.
            rows.append(dict(archetype=arch, source=raw["source"], source_deck_id=raw["deck_id"], source_url=raw["source_url"],
                             source_archetype=library_name, source_slug=None, event_url=None, event_title=None,
                             event_date=when, format="EX13", player=raw.get("player_name"),
                             placement=int(placement) if placement.isdigit() else None, field_size=raw.get("event_players"), record=None, rounds=None,
                             cards=cards, card_counts=dict(sorted(Counter(cards).items())), fingerprint=fingerprint(cards),
                             source_verification="archived_local_primary_source_record_not_refetched",
                             source_record_sha256=digest(json.dumps(raw,sort_keys=True,ensure_ascii=False).encode()),
                             library_generated_at=library.get("generated_at"), retrieved_utc=None))
    extra = json.loads(extras_path.read_text(encoding="utf-8")) if extras_path else {"candidates":[]}
    rows.extend(extra["candidates"])
    # Keep distinct source records even for identical lists: selection deduplicates
    # by content later, while provenance can still show corroborating events.
    unique = {}
    for row in rows:
        key = (row["source"],row["source_deck_id"])
        if key in unique and row != unique[key]:
            raise ValueError(f"conflicting source records: {key}")
        unique[key] = row
    result = fresh | {"candidates":list(unique.values()), "source_inputs":{
        str(fresh_path):digest(fresh_path.read_bytes()), str(library_path):digest(library_path.read_bytes())},
        "alternate_source_notes":extra.get("discovery_notes", [])}
    if extras_path:
        result["source_inputs"][str(extras_path)] = digest(extras_path.read_bytes())
    write_json(output,result)
    return result


def quality_score(row: dict) -> float:
    """Balance relative finish, event size and freshness after qualification."""
    relative = 1 - (row["placement"] - 1) / max(1, row["field_size"] - 1)
    field = min(1., math.log2(row["field_size"] / 8 + 1) / math.log2(9))
    recency = max(0., 1 - (AS_OF - date.fromisoformat(row["event_date"])).days / 30)
    return .45 * relative + .25 * field + .30 * recency - (.05 if row.get("record") is None else 0)


def current_compatible_build(row: dict) -> bool:
    """A reviewed mixed-label global build may be usable without event claims."""
    when = parsed_event_date(row.get("event_date"))
    in_window = when is not None and START <= when <= AS_OF
    return in_window and (row.get("format") == "EX13" or (
        row.get("format") == "BT26/EX13" and row.get("ex13_compatible_reviewed") is True))


def choose_diverse(rows: list[dict], count: int = 4) -> list[dict]:
    """Deduplicate counts and prefer independent events without inventing lists."""
    remaining = sorted(rows, key=lambda row: (-quality_score(row), row["source_deck_id"]))
    unique = {}
    for row in remaining:
        unique.setdefault(row["fingerprint"], row)
    remaining, chosen = list(unique.values()), []
    while remaining and len(chosen) < count:
        def score(row):
            fresh_event = row.get("event_url") and not any(row["event_url"] == other["event_url"] for other in chosen)
            distance = min((copy_distance(row, other) for other in chosen), default=0)
            return quality_score(row) + .12 * bool(fresh_event) + .10 * min(distance / 12, 1)
        winner = min(remaining, key=lambda row: (-score(row), row["source_deck_id"]))
        chosen.append(winner)
        remaining.remove(winner)
    return chosen


def assign_partitions(chosen: list[dict]) -> tuple[list[dict], list[dict]]:
    # At fewer than four qualified lists, prioritize two distinct training lists;
    # missing held-out coverage stays missing. With four, alternate quality rank.
    if len(chosen) == 4:
        return [chosen[0], chosen[2]], [chosen[1], chosen[3]]
    return chosen[:2], chosen[2:]


def select_partitions(rows: list[dict]) -> tuple[list[dict], list[dict]]:
    training, evaluation = assign_partitions(choose_diverse(rows))
    for row in list(evaluation):
        if row.get("seen_in_previous_training", False):
            replacement = next((x for x in training if not x.get("seen_in_previous_training", False)), None)
            evaluation.remove(row)
            if replacement:
                training.remove(replacement)
                training.append(row)
                evaluation.append(replacement)
    used = {row["fingerprint"] for row in training + evaluation}
    alternatives = [row for row in rows if row["fingerprint"] not in used and not row.get("seen_in_previous_training", False)]
    evaluation.extend(choose_diverse(alternatives, count=2 - len(evaluation)))
    return training, evaluation


def balanced_runtime_slots(distinct_decks: list[dict]) -> list[dict]:
    """Weight the flat native sampler uniformly by archetype, never as evidence."""
    by_arch = {}
    for row in distinct_decks:
        by_arch.setdefault(row["archetype"], []).append(row)
    if not by_arch or any(len({fingerprint(row["cards"]) for row in group}) != len(group) for group in by_arch.values()):
        raise ValueError("runtime weighting requires nonempty distinct-list groups")
    slots = math.lcm(*(len(group) for group in by_arch.values()))
    result = []
    for arch, group in sorted(by_arch.items()):
        for row in sorted(group, key=lambda row: row["deck_id"]):
            for slot in range(1,slots // len(group)+1):
                result.append({"name":f"{arch}:{row['deck_id']}:slot{slot}", "cards":row["cards"]})
    return result


def require_new_output(out: Path) -> None:
    existing = [name for name in PREPARED_ARTIFACTS if (out/name).exists()]
    if existing:
        raise FileExistsError(f"Preserving existing or partial prepared pool: {out}: {existing}")


def combine_diagnostic_pools(heldout: dict, regression: dict, archetypes) -> tuple[dict, list[str]]:
    """Prefer unseen lists; use a known pet regression only when none exists."""
    combined, fallbacks = {}, []
    for arch in sorted(archetypes):
        if heldout.get(arch):
            combined[arch] = heldout[arch]
        elif regression.get(arch):
            combined[arch] = regression[arch]
            fallbacks.append(arch)
        else:
            raise ValueError(f"No diagnostic list for {arch}; refuse silent scope reduction")
    return combined, fallbacks


def add_diagnostic_panel(out: Path) -> dict:
    """Add the all-archetype convenience panel without rewriting existing pools."""
    from digimon_gym.agents.gauntlet import GeneralistDeckPool
    target = out / "combined_diagnostic_snapshot.json"
    if target.exists():
        raise FileExistsError(f"Preserving existing diagnostic panel: {target}")
    report_path = out / "pool_report.json"
    report = json.loads(report_path.read_text(encoding="utf-8"))
    original_hashes = dict(report["artifacts"])
    for name, expected in original_hashes.items():
        if digest((out/name).read_bytes()) != expected:
            raise ValueError(f"Prepared artifact changed before combining: {name}")
    heldout = GeneralistDeckPool.from_snapshot(out / "heldout_snapshot.json")
    regression = GeneralistDeckPool.from_snapshot(out / "pet_regression_snapshot.json")
    combined, fallbacks = combine_diagnostic_pools(heldout.archetypes, regression.archetypes, report["coverage"])
    GeneralistDeckPool(combined).write_snapshot(target)
    checked = GeneralistDeckPool.from_snapshot(target)
    panel = dict(path=target.name, purpose="All-thirteen baseline diagnostics, not an all-held-out evaluation",
                 archetypes=checked.archetype_count, lists=checked.deck_count,
                 heldout_archetypes=sorted(heldout.archetypes), same_list_regression_archetypes=fallbacks,
                 evidence_by_archetype={arch:("same_list_original_pet_regression" if arch in fallbacks else "unseen_list_evidence_tier_in_selected_metadata") for arch in combined})
    report["combined_diagnostic_panel"] = panel
    report["artifacts"][target.name] = digest(target.read_bytes())
    report["driver_sha256"] = digest(Path(__file__).read_bytes())
    write_json(report_path,report)
    readme = out / "README.md"
    readme.write_text(readme.read_text(encoding="utf-8") +
                     f"\n## All-thirteen diagnostic consumer\n\n`{target.name}` contains {checked.deck_count} lists across all {checked.archetype_count} archetypes. It uses the unseen lists for {len(heldout.archetypes)} archetypes and the original same-list pet regression for **{', '.join(fallbacks)}** only. This combined panel is for a complete baseline, **not an all-held-out evaluation**. Keep same-list results labeled separately.\n", encoding="utf-8")
    for name, expected in original_hashes.items():
        assert digest((out/name).read_bytes()) == expected, name
    return panel


def prepare(sources: Path, baseline: Path, out: Path, cli_bin: Path | None = None) -> dict:
    import digimon_engine
    from digimon_gym.agents.gauntlet import (
        DeckEntry, GeneralistDeckPool, require_mechanics_ready, stable_deck_id, validate_admissible_deck,
    )
    require_new_output(out)
    require_mechanics_ready()
    registered = set(digimon_engine.load_implemented_card_ids())
    database = json.loads((ROOT / "data/cards.json").read_text(encoding="utf-8"))
    data_ids = {row["card_id"] for row in database} if isinstance(database, list) else set(database)
    playable = registered & data_ids
    native_info = {"status":"unmeasured", "reason":"No native CLI supplied"}
    if cli_bin:
        native = set(json.loads(subprocess.check_output([str(cli_bin), "--cards-json", str(ROOT / "data/cards.json"), "pool"], text=True)))
        native_info = dict(path=str(cli_bin.resolve()), sha256=digest(cli_bin.read_bytes()), status="matched" if native == playable else "registry_mismatch",
                           native_count=len(native), python_playable_count=len(playable),
                           native_only=sorted(native-playable), python_only=sorted(playable-native))
        # A known older binary is evidence of its own coverage, never a replacement
        # for the active registry. Every selected card must exist in BOTH surfaces.
        playable &= native

    def admission(cards: list[str]) -> list[str]:
        reasons = []
        missing = sorted(set(cards) - playable)
        if missing:
            reasons.append("not_in_verified_registries: " + ", ".join(missing))
        try:
            validate_admissible_deck(cards, label="EX13 exact-list candidate")
        except ValueError as exc:
            reasons.append(str(exc))
        legality = digimon_engine.validate_deck(cards)
        if not legality.is_valid:
            reasons.append("deck_legality: " + str(legality.errors))
        return reasons

    frozen = json.loads(baseline.read_text(encoding="utf-8"))
    if set(frozen["eligible_archetypes"]) != set(ARCHETYPES):
        raise ValueError("baseline archetype scope differs from the requested thirteen")
    old_fingerprints = {fingerprint(row["card_ids"]) for row in frozen["decks"]}
    candidates_doc = json.loads(sources.read_text(encoding="utf-8"))
    assessed = []
    for raw in candidates_doc["candidates"]:
        if raw["archetype"] not in ARCHETYPES or raw["fingerprint"] != fingerprint(raw["cards"]):
            raise ValueError("candidate identity mismatch")
        if Counter(raw["cards"]) != raw["card_counts"]:
            raise ValueError("candidate card-count mismatch")
        assessed.append(raw | {"qualification_reasons":qualification_reasons(raw), "admission_reasons":admission(raw["cards"]),
                               "seen_in_previous_training":raw["fingerprint"] in old_fingerprints})
    train, heldout, regression, supplemental, coverage = {}, {}, {}, {}, {}
    selected_metadata = []

    def entry(row):
        return DeckEntry(deck_id=stable_deck_id(row["cards"]), archetype_name=row["archetype"], card_ids=row["cards"],
                         source=row["source"], source_deck_id=row["source_deck_id"], source_url=row["source_url"])

    for arch in ARCHETYPES:
        rows = [row for row in assessed if row["archetype"] == arch]
        qualified = [row for row in rows if not row["qualification_reasons"] and not row["admission_reasons"]]
        training, evaluation = select_partitions(qualified)
        fallback = []
        old_pet = [row for row in frozen["decks"] if row["archetype"] == arch] if arch in PETS else []
        for row in old_pet:
            cards = row["card_ids"]
            errors = admission(cards)
            if errors:
                raise ValueError(f"Original pet regression list is no longer admitted: {arch}: {errors}")
            pet = dict(archetype=arch, cards=cards, source=row["source"], source_deck_id=row["source_deck_id"], source_url=row["source_url"],
                       fingerprint=fingerprint(cards), evidence="original_pet_regression", seen_in_previous_training=True)
            regression.setdefault(arch, []).append(entry(pet))
            # Keep all thirteen archetypes represented if no qualified refresh
            # exists. A single exact pet list remains one list, never two slots.
            if not training and not fallback:
                fallback.append(pet)
        if not training and not fallback:
            raise ValueError(f"No qualified list and no original pet fallback for {arch}")
        train[arch] = [entry(row) for row in training + fallback]
        selected = {row["fingerprint"] for row in training + evaluation + fallback}
        # Generalization can still be measured on contemporary exact lists that
        # lack a strong tournament result. Keep their evidence tier explicit;
        # never confuse a weak pilot's finish with proof that a build is bad.
        lower_heldout_candidates = [row for row in rows if not row["admission_reasons"] and row["qualification_reasons"]
                                   and row["fingerprint"] not in selected and not row["seen_in_previous_training"]
                                   and current_compatible_build(row)]
        # No unknown field/placement is invented for source quality: unmeasured
        # metadata sorts after measured candidates, then by date and identity.
        measured_lower = [row for row in lower_heldout_candidates if row["field_size"] and row["placement"]]
        lower_heldout = choose_diverse(measured_lower, count=max(0, 2-len(evaluation)))
        lower_seen = {row["fingerprint"] for row in lower_heldout}
        for row in sorted(lower_heldout_candidates, key=lambda row: (row["event_date"],row["source_deck_id"]), reverse=True):
            if len(evaluation)+len(lower_heldout) < 2 and row["fingerprint"] not in lower_seen:
                lower_heldout.append(row)
                lower_seen.add(row["fingerprint"])
        if evaluation or lower_heldout:
            heldout[arch] = [entry(row) for row in evaluation + lower_heldout]
        selected.update(row["fingerprint"] for row in lower_heldout)
        lower_evidence = [row for row in rows if not row["admission_reasons"] and row["qualification_reasons"] and row["fingerprint"] not in selected
                          and row["format"] in ("EX13", "BT26/EX13") and (when := parsed_event_date(row.get("event_date"))) is not None and START <= when <= AS_OF]
        # Retain up to four exact contemporary lists for optional diagnostics;
        # they are never silently added to training or competitive held-out.
        seen = set()
        for row in sorted(lower_evidence, key=lambda row: (row["placement"] / row["field_size"] if row["placement"] and row["field_size"] else 2, row["source_deck_id"])):
            if row["fingerprint"] not in seen and len(seen) < 4:
                seen.add(row["fingerprint"])
                supplemental.setdefault(arch, []).append(entry(row))
        for partition, group in (("training", training), ("heldout", evaluation), ("lower_evidence_heldout", lower_heldout), ("pet_fallback_training", fallback)):
            selected_metadata.extend(row | {"partition":partition} for row in group)
        all_selected = training + evaluation + lower_heldout + fallback
        coverage[arch] = dict(discovered_candidates=len(rows), qualified_distinct=len({row["fingerprint"] for row in qualified}),
                              admitted_distinct=len({row["fingerprint"] for row in rows if not row["admission_reasons"]}),
                              blocked_distinct=len({row["fingerprint"] for row in rows if row["admission_reasons"]}),
                              lower_evidence_distinct=len({row["fingerprint"] for row in rows if not row["admission_reasons"] and row["qualification_reasons"]}),
                              qualified_training=len(training), qualified_heldout=len(evaluation), lower_evidence_heldout=len(lower_heldout),
                              heldout=len(evaluation)+len(lower_heldout), pet_fallback_training=len(fallback),
                              regression_lists=len(regression.get(arch, [])), supplemental_lists=len(supplemental.get(arch, [])),
                              shortfall={"training":max(0,2-len(training)), "heldout":max(0,2-len(evaluation))},
                              total_list_count_shortfall={"training":max(0,2-len(train[arch])), "heldout":max(0,2-len(heldout.get(arch,[])))},
                              source_evidence_counts=dict(Counter(row.get("source_verification", "unmeasured") for row in rows)),
                              source_retrieval_failures=[row for row in candidates_doc.get("errors", []) if row["archetype"] == arch],
                              copy_differences=[dict(first=a["source_deck_id"],second=b["source_deck_id"],copies_changed=copy_distance(a,b))
                                                for i,a in enumerate(all_selected) for b in all_selected[i+1:]],
                              admission_blockers=[{"source_url":row["source_url"], "reasons":row["admission_reasons"]} for row in rows if row["admission_reasons"]])

    all_train_ids = {entry.deck_id for group in train.values() for entry in group}
    all_eval_ids = {entry.deck_id for group in heldout.values() for entry in group}
    if all_train_ids & all_eval_ids:
        raise ValueError("training/held-out content leakage")
    out.mkdir(parents=True, exist_ok=True)
    artifacts = {}
    for name, pool in (("train_snapshot", train), ("heldout_snapshot", heldout), ("pet_regression_snapshot", regression), ("supplemental_snapshot", supplemental)):
        GeneralistDeckPool(pool).write_snapshot(out / f"{name}.json")
        artifacts[f"{name}.json"] = digest((out / f"{name}.json").read_bytes())
    distinct_manifest = [{"archetype":arch,"deck_id":deck.deck_id,"cards":deck.card_ids} for arch,decks in train.items() for deck in decks]
    runtime_slots = balanced_runtime_slots(distinct_manifest)
    write_json(out / "distinct_training_decks.json", distinct_manifest)
    write_json(out / "decks.json", runtime_slots)
    for name in ("distinct_training_decks.json", "decks.json"):
        artifacts[name] = digest((out/name).read_bytes())
    report = dict(schema="ex13-pool-refresh-v1", as_of=AS_OF.isoformat(), scope="Same thirteen archetypes; global October EX13 exact lists; no card substitutions",
                  status="prepared_with_explicit_shortfalls" if any(any(row["shortfall"].values()) for row in coverage.values()) else "prepared",
                  qualification="Explicit EX13, October1-26, field>=8, top third (ceil). Where a match record is published, at least two wins and wins>=losses. Missing records stay None, incur a ranking penalty, and rely on placement evidence. This is a source-evidence filter, not proof of deck strength.",
                  selection="Weighted relative placement(.45), bounded field size(.25), recency(.30), minus.05 for an unmeasured record; greedy verified distinct-event and copy-diversity bonuses. Four distinct contents -> quality-interleaved2train/2heldout; fewer lists retain explicit shortfalls.",
                  heldout_definition="No identical card-count fingerprint in refreshed training or prior frozen training; not a claim of unseen cards or statistically independent pilots. Where qualified lists are scarce, separately tagged lower-evidence EX13 tournament lists or explicitly reviewed EX13-compatible October global builds may fill evaluation slots; those are not competitive qualification evidence.",
                  sampling="decks.json is the ready native flat-sampler input: equal slots per archetype, with explicit :slotN repetition only for weighting. Snapshots and distinct_training_decks.json retain distinct contents. Runtime repetitions are never additional lists or unseen-list evidence.",
                  distinct_training_lists=len(distinct_manifest), native_sampling_slots=len(runtime_slots),
                  slots_per_archetype=dict(Counter(row["name"].split(":sha256:",1)[0] for row in runtime_slots)),
                  admission_scope="Current committed mechanics/light-card gates, card database, registry intersection, and standard deck legality; not whole-deck oracle certification.",
                  prior_training_snapshot=dict(path=str(baseline), sha256=digest(baseline.read_bytes())),
                  candidate_source_sha256=digest(sources.read_bytes()), driver_sha256=digest(Path(__file__).read_bytes()),
                  native_registry=native_info, python_binding=str(digimon_engine.__file__),
                  python_extensions=native_extension_hashes(Path(digimon_engine.__file__).parent),
                  source_commit=subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
                  gate_hashes={name:digest((ROOT/name).read_bytes()) for name in ["data/cards.json","data/card_admissibility.json","data/mechanics_readiness.json","data/deck_formats.json"]},
                  coverage=coverage, selected=selected_metadata,
                  assessed_candidates=[{k:v for k,v in row.items() if k not in ("cards", "card_counts")} for row in assessed], artifacts=artifacts,
                  original_pet_policy="Exact original pet lists are retained in a separate regression panel. A fallback may also be used in training when no competitive refresh exists; it is never held-out evidence.")
    write_json(out / "pool_report.json", report)
    lines = ["# Global EX13 pool prepared 2026-10-26", "", report["scope"], "",
             "Prepared assets only; the active cloud run and its frozen snapshots have not been changed.", "",
             f"Native consumer: `decks.json` ({len(runtime_slots)} explicitly weighted slots, {len(distinct_manifest)} distinct training lists). The snapshots retain unique lists.", "",
             "| Archetype | Qualified train | Qualified held-out | Lower-evidence held-out | Pet fallback train | Admitted / blocked candidate contents |",
             "| --- | ---: | ---: | ---: | ---: | ---: |"]
    for arch,row in coverage.items():
        lines.append(f"| {arch} | {row['qualified_training']} | {row['qualified_heldout']} | {row['lower_evidence_heldout']} | {row['pet_fallback_training']} | {row['admitted_distinct']} / {row['blocked_distinct']} |")
    lines += ["", "`heldout_snapshot.json` includes both evidence tiers shown above. `pet_regression_snapshot.json` preserves the five exact original pet lists. `supplemental_snapshot.json` holds additional admitted builds with weaker or mixed-format evidence; these are separate from the training pool.", "",
              report["qualification"], "", report["selection"], "", report["heldout_definition"], "",
              "Fresh public DigiLab captures are supplemented by archived exact primary-source records in the local deck library. Archived records were not refetched and missing match records/event identities remain null. Public collection encountered HTTP 403; no access-control bypass or retry was used. Source quality and collection limits are recorded per list and in source_candidates.json. Qualification gaps are not engine-admission gaps.", "",
              "Read pool_report.json for all source URLs, dates, placements, field sizes, W-L-D records, fingerprints, card-count distances, admission blockers and gate/binary hashes. No cards were substituted. Admission is the current light training gate and legal-deck check, not complete oracle certification.", "",
              "## Selected exact lists", "", "| Archetype | Panel | Source | Date | Place / field | Record |", "| --- | --- | --- | --- | --- | --- |"]
    for row in selected_metadata:
        record=row.get("record")
        result=f"{record['wins']}-{record['losses']}-{record['draws']}" if record else "unmeasured"
        source=f"[{row['source_deck_id']}]({row['source_url']})" if row.get("source_url") else row["source_deck_id"]
        lines.append(f"| {row['archetype']} | {row['partition']} | {source} | {row.get('event_date') or 'unmeasured'} | {row.get('placement') or '?'} / {row.get('field_size') or '?'} | {result} |")
    (out/"README.md").write_text("\n".join(lines)+"\n",encoding="utf-8")
    add_diagnostic_panel(out)
    print(json.dumps({arch:{k:v for k,v in row.items() if k in ("qualified_distinct","qualified_training","heldout","pet_fallback_training")} for arch,row in coverage.items()},indent=2),flush=True)
    return json.loads((out / "pool_report.json").read_text(encoding="utf-8"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    source = sub.add_parser("collect")
    source.add_argument("--cache", type=Path, required=True)
    source.add_argument("--output", type=Path, required=True)
    merge = sub.add_parser("merge-sources")
    merge.add_argument("--fresh", type=Path, required=True)
    merge.add_argument("--library", type=Path, default=ROOT / "data/deck_library.json")
    merge.add_argument("--extras", type=Path)
    merge.add_argument("--output", type=Path, required=True)
    build = sub.add_parser("prepare")
    build.add_argument("--sources", type=Path, required=True)
    build.add_argument("--baseline", type=Path, required=True)
    build.add_argument("--out", type=Path, required=True)
    build.add_argument("--cli-bin", type=Path)
    diagnostic = sub.add_parser("add-diagnostic-panel")
    diagnostic.add_argument("--pool", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "collect":
        collect(args.cache, args.output)
    elif args.command == "merge-sources":
        merge_sources(args.fresh, args.library, args.extras, args.output)
    elif args.command == "add-diagnostic-panel":
        print(json.dumps(add_diagnostic_panel(args.pool),indent=2))
    else:
        prepare(args.sources, args.baseline, args.out, args.cli_bin)


if __name__ == "__main__":
    main()
