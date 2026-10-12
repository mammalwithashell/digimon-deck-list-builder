"""Read-only, digest-pinned run source validation. Never upgrades evidence.

A configured manifest selects immutable source artifacts. Every use verifies
the bytes again; no fallback to global artifacts and no cached approval.

Manifest v1 (generated from reviewed extractor/build_denominator outputs):
  version: 1
  clause_text_json / interaction_denominator: repo-relative snapshot paths
  pool: the exact authorized run workset (not the full reference universe)
  files: {repo-relative source/artifact path: lowercase SHA256}
  expected: {clause_ids, interaction_ids, gating_interaction_ids}
The expected ID lists are sorted and complete for the pool. Files must pin all
SOURCE_INPUTS and both artifacts; additional provenance/tool inputs may also
be pinned. The complete reference universe is derived independently from the
pinned cards.json and card_qa.json, using the denominator producer's all-card
resolver. Both artifacts must contain exactly that universe, including
shared-ruling members. Config pins the manifest's own SHA256. Nothing here
writes manifests, source files, verdicts or ledgers. Generate a new reviewed
snapshot when sources change; never edit one in place or rehash old verdicts.
"""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path


SOURCE_INPUTS = (
    "data/cards.json", "data/card_official.json", "data/card_overrides.json",
    "data/card_qa.json", "code/tools/card_loop/interactions/promotion.json",
)


def validate_snapshot(ctx, *, clause_book=None, required_cards=(), scenario=None):
    from .stages import base
    cfg = getattr(ctx, "config", None)
    selected = getattr(cfg, "source_manifest", None)
    if not selected:
        return
    try:
        _validate(ctx, selected, clause_book, required_cards, scenario)
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise base.StageDeferred(f"immutable source snapshot refused: {error}") from error


def _validate(ctx, selected, clause_book, required_cards, scenario):
    from .stages import base
    from tools.clause_coverage import extract
    from tools.clause_coverage.book import validate_book
    from tools.clause_coverage.exam_binding import _parse_scenario_header
    from .interactions import denominator as den

    repo = Path(ctx.repo).resolve()
    raw = (repo / selected).read_bytes()
    if hashlib.sha256(raw).hexdigest() != getattr(ctx.config, "source_manifest_sha256", None):
        raise ValueError("manifest SHA256 does not match configured pin")
    manifest = json.loads(raw)
    if not isinstance(manifest, dict):
        raise ValueError("source manifest must be an object")
    if type(manifest.get("version")) is not int or manifest["version"] != 1:
        raise ValueError("unsupported source manifest version")
    book_name = clause_book or base.clause_text_json(ctx)
    denominator_name = base.interaction_denominator(ctx)
    for key, value in (("clause_text_json", book_name), ("interaction_denominator", denominator_name)):
        if (repo / manifest[key]).resolve() != (repo / value).resolve():
            raise ValueError(f"configured {key} differs from manifest")
    files = manifest["files"]
    if not isinstance(files, dict):
        raise ValueError("source manifest files must be a path-to-SHA256 object")
    pinned = {}
    for name, digest in files.items():
        if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError(f"invalid source file SHA256: {name}")
        path = (repo / name).resolve()
        if not path.is_relative_to(repo):
            raise ValueError(f"source file is outside run tree: {name}")
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError(f"source file SHA256 changed: {name}")
        pinned[path] = digest
    for name in (*SOURCE_INPUTS, book_name, denominator_name):
        if (repo / name).resolve() not in pinned:
            raise ValueError(f"required source file not pinned: {name}")

    pool = manifest["pool"]
    if not isinstance(pool, list) or not pool or len(set(pool)) != len(pool):
        raise ValueError("manifest needs a nonempty distinct pool")
    planned = list((getattr(ctx, "plan", {}).get("work_set") or {}).get("pool") or [])
    if planned and set(planned) != set(pool):
        raise ValueError("run workset differs from manifest pool")
    book_path, denominator_path = repo / book_name, repo / denominator_name
    book = json.loads(book_path.read_text(encoding="utf-8"))
    artifact = den.load_denominator(denominator_path)
    card_qa = den.load_card_qa(repo / SOURCE_INPUTS[3])
    # Do not trust the paired artifacts to define their own completeness: both
    # can be narrowed together while keeping the focused denominator unchanged.
    universe = set(den.all_card_ids(repo / SOURCE_INPUTS[0], card_qa))
    if not universe or set(book["cards"]) != universe or set(artifact["cards"]) != universe:
        raise ValueError("both immutable artifacts must contain the full pinned source universe")
    for iid, row in artifact["interactions"].items():
        members = row.get("card_ids")
        if not isinstance(members, list) or not members or not set(members) <= universe:
            raise ValueError(f"interaction membership outside full source universe: {iid}")
    covered = set(required_cards) | set(pool)
    header = None
    if scenario:
        header = _parse_scenario_header((repo / scenario).read_text(encoding="utf-8"))
        if header.get("card") not in pool:
            raise ValueError("scenario primary card is outside authorized pool")
        ids = list(header.get("covers") or [])
        if header.get("clause") and not header.get("interaction"):
            ids.append(header["clause"])
        book_ids = {row["id"] for row in book["clauses"]}
        if any(ident not in book_ids for ident in ids):
            raise ValueError("scenario covers a clause absent from frozen source")
        covered.update(ident.split("#")[0] for ident in ids)
        interaction = header.get("interaction")
        if interaction and not interaction["id"].startswith("combo:"):
            row = artifact["interactions"].get(interaction["id"])
            if row is None:
                raise ValueError("scenario interaction absent from frozen denominator")
            if header["card"] not in row["card_ids"]:
                raise ValueError("scenario interaction card membership mismatch")
            covered.update(row["card_ids"])
    if not covered <= set(book["cards"]):
        raise ValueError("required card missing from immutable book; regenerate a new snapshot")

    def fresh(ids, description):
        return extract.run(ids, description, cards_json_path=repo / SOURCE_INPUTS[0],
                           official_json_path=repo / SOURCE_INPUTS[1],
                           overrides_path=repo / SOURCE_INPUTS[2])
    findings = validate_book(book_path, card_ids=sorted(covered), extract=fresh)
    if findings:
        raise ValueError(f"stale or incomplete clause book: {findings[:3]}")
    current = fresh(sorted(covered), "immutable run source pairing")["clauses"]
    generated = den.build_denominator(sorted(covered), current,
        card_qa, den.load_promotion(repo / SOURCE_INPUTS[4]),
        universe_desc={"mode": "cards", "source": "immutable run validation"})
    # Exact memberships and gating, not a subset or total-only comparison.
    for card in covered:
        if artifact["cards"].get(card) != generated["cards"].get(card):
            raise ValueError(f"interaction membership changed for {card}")
        for iid in generated["cards"].get(card, []):
            if artifact["interactions"].get(iid) != generated["interactions"][iid]:
                raise ValueError(f"interaction source changed: {iid}")
    expected = manifest["expected"]
    clause_ids = sorted(r["id"] for r in book["clauses"] if r["card_id"] in pool)
    interaction_ids = sorted({i for card in pool for i in artifact["cards"].get(card, [])})
    gating = [i for i in interaction_ids if artifact["interactions"][i]["gating"]]
    for key, actual in (("clause_ids", clause_ids), ("interaction_ids", interaction_ids),
                        ("gating_interaction_ids", gating)):
        if expected[key] != actual:
            raise ValueError(f"full focused denominator changed: {key}")
