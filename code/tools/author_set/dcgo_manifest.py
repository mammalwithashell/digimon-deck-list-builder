"""Extract DCGO's keyword surface into a checked-in manifest and diff it against
the Rust ``Keyword`` enum (Phase 2, tasks 2.1 / 2.1b).

Fidelity audit (see ``openspec/changes/add-author-set-workflow/design.md``):
DCGO has NO central keyword enum. The canonical keyword registry is the UNION of
two one-file-per-keyword directories — neither complete alone:

- ``Assets/Scripts/Script/CardEffectFactory/KeyWordEffects/*.cs``   (has Link, ArtsDigivolve, Blast(DNA)Digivolution)
- ``Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/*.cs``   (has MindLink)

Plus a SECONDARY cross-check (``I…Effect`` interfaces in ``CardEffectInterfaces.cs``)
and a TERTIARY hand-curated allowlist of core-modeled keywords that appear in
NEITHER directory because they are stat/draw/cost mechanics handled inline in
core processing (``SecurityAttack±``, ``DrawX``, ``DeDigivolve``, ``DigiBurst``).
"""

from __future__ import annotations

import glob
import os
import re
import subprocess

KEYWORD_DIR_RELPATHS = (
    "Assets/Scripts/Script/CardEffectFactory/KeyWordEffects",
    "Assets/Scripts/Script/CardEffectCommons/KeyWordEffects",
)
INTERFACES_RELPATH = "Assets/Scripts/Script/CardEffectInterfaces.cs"

# Core-modeled keywords invisible to directory scanning — they exist as Rust
# enum variants but are handled inline in DCGO core processing, not as
# KeyWordEffects files. Hand-curated; revisit on DCGO rebase.
CORE_MODELED_ALLOWLIST = (
    "securityattackplus",
    "securityattackminus",
    "drawx",
    "dedigivolve",
    "digiburst",
)

# DCGO filename/interface spelling -> normalized Rust-enum spelling.
DCGO_TO_RUST_ALIAS = {
    "pierce": "piercing",
    "blastdigivolution": "blastdigivolve",
    "blastdnadigivolution": "blastdnadigivolve",
}


def normalize_keyword(name: str) -> str:
    """Lowercase, drop non-alphanumerics, strip trailing param/number, alias-map."""
    s = re.sub(r"[^a-z0-9]", "", name.strip().lower())
    s = re.sub(r"\d+$", "", s)
    return DCGO_TO_RUST_ALIAS.get(s, s)


def keywords_from_filenames(filenames) -> set[str]:
    """Normalized keyword names from a list of ``*.cs`` filenames/paths."""
    out = set()
    for fn in filenames:
        name = os.path.basename(str(fn))
        if name.endswith(".meta"):  # Unity sidecar — skip before stemming
            continue
        stem = os.path.splitext(name)[0]
        if stem:
            out.add(normalize_keyword(stem))
    return out


# Heuristic hint (NOT a verdict): a DCGO keyword whose KeyWordEffects file is an
# ACTIVATED effect (`ActivateClass`) that ALSO selects a permanent to mutate
# (`SelectPermanentEffect`) is Link-shaped — a likely subsystem (new action-space
# entry + attachment/board state + selection flow), not a passive flag. Requiring
# BOTH markers avoids over-flagging the many already-implemented replacement
# keywords (Decoy/Save/Fragment) that use `SelectPermanentEffect` alone. This is a
# coarse pre-flight hint to force assessment of an auto-ingest candidate before
# porting it as if it were cheap — confirm with a real read (see RUST_ENGINE_GAPS).
_SUBSYSTEM_MARKERS = ("ActivateClass", "SelectPermanentEffect")


def classify_keyword_complexity(cs_text: str) -> str:
    """Classify a DCGO keyword's C# source as ``"subsystem"`` or ``"simple"``."""
    t = cs_text or ""
    return "subsystem" if all(m in t for m in _SUBSYSTEM_MARKERS) else "simple"


def interface_keywords_from_text(text: str) -> set[str]:
    """Normalized names from ``interface I<Name>Effect`` declarations."""
    out = set()
    for m in re.findall(r"interface\s+I([A-Za-z0-9]+)Effect", text):
        out.add(normalize_keyword(m))
    return out


def parse_rust_enum_keywords(enum_src: str) -> set[str]:
    """Parse variant identifiers from the ``pub enum Keyword { ... }`` block."""
    m = re.search(r"pub enum Keyword\s*\{(.*?)\n\}", enum_src, re.DOTALL)
    body = m.group(1) if m else enum_src
    out = set()
    for line in body.splitlines():
        line = line.strip()
        if not line or line.startswith("//") or line.startswith("/*") or line.startswith("*"):
            continue
        vm = re.match(r"([A-Z][A-Za-z0-9]*)\s*(?:\(|,|$)", line)
        if vm:
            out.add(normalize_keyword(vm.group(1)))
    return out


def diff_registries(dcgo_keywords: set[str], rust_keywords: set[str]) -> dict:
    """Return the two directional diffs.

    - ``auto_ingest_candidates``: DCGO has, Rust lacks (port from DCGO C#).
    - ``core_modeled``: Rust has, DCGO keyword dirs lack (handled inline / allowlist).
    """
    return {
        "auto_ingest_candidates": sorted(dcgo_keywords - rust_keywords),
        "core_modeled": sorted(rust_keywords - dcgo_keywords),
    }


# ---- I/O wrappers -----------------------------------------------------------

def find_base_dcgo() -> str:
    """Resolve the base-repo DCGO checkout (rule 29). Worktree ``./DCGO`` is empty."""
    common = subprocess.check_output(
        ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
        text=True,
    ).strip()
    return os.path.join(os.path.dirname(common), "DCGO")


def extract_dcgo_keywords(base_dcgo: str) -> tuple[set[str], set[str]]:
    """Return ``(registry_keywords, interface_keywords)`` from a DCGO checkout."""
    files = []
    for rel in KEYWORD_DIR_RELPATHS:
        files += glob.glob(os.path.join(base_dcgo, rel, "*.cs"))
    registry = keywords_from_filenames(files)
    iface_path = os.path.join(base_dcgo, INTERFACES_RELPATH)
    interfaces = set()
    if os.path.exists(iface_path):
        with open(iface_path, encoding="utf-8") as f:
            interfaces = interface_keywords_from_text(f.read())
    return registry, interfaces


def keyword_complexity_map(base_dcgo: str) -> dict[str, str]:
    """Map each DCGO registry keyword -> ``"simple"`` | ``"subsystem"``.

    A keyword may have a file in either ``KeyWordEffects/`` dir; if any of its
    files carry subsystem markers, the keyword is a subsystem.
    """
    by_kw: dict[str, str] = {}
    for rel in KEYWORD_DIR_RELPATHS:
        for path in glob.glob(os.path.join(base_dcgo, rel, "*.cs")):
            if path.endswith(".meta"):
                continue
            kw = normalize_keyword(os.path.splitext(os.path.basename(path))[0])
            with open(path, encoding="utf-8") as f:
                c = classify_keyword_complexity(f.read())
            if by_kw.get(kw) != "subsystem":  # subsystem wins across dirs
                by_kw[kw] = c
    return by_kw


def build_manifest(base_dcgo: str | None = None, rust_enum_path: str | None = None) -> dict:
    """Build the manifest dict (does not write). See ``write_manifest``."""
    if base_dcgo is None:
        base_dcgo = find_base_dcgo()
    if rust_enum_path is None:
        rust_enum_path = os.path.join("code", "digimon-engine", "src", "enums.rs")

    registry, interfaces = extract_dcgo_keywords(base_dcgo)
    dcgo_all = registry | set(CORE_MODELED_ALLOWLIST)
    complexity = keyword_complexity_map(base_dcgo)
    subsystem = {k for k, c in complexity.items() if c == "subsystem"}

    with open(rust_enum_path, encoding="utf-8") as f:
        rust_keywords = parse_rust_enum_keywords(f.read())

    diff = diff_registries(registry, rust_keywords)
    candidates = diff["auto_ingest_candidates"]
    # Split auto-ingest candidates: simple flags can be ported directly; subsystem
    # keywords (Link-style ActivateClass) must be assessed/scheduled, not naively
    # auto-ported (see RUST_ENGINE_GAPS.md "[Link] keyword subsystem").
    return {
        "_comment": "Generated by tools/author_set/dcgo_manifest.py. "
                    "Refresh on DCGO rebase (CLAUDE.md rule 27/29).",
        "dcgo_registry_keywords": sorted(registry),
        "dcgo_interface_keywords": sorted(interfaces),
        "core_modeled_allowlist": sorted(CORE_MODELED_ALLOWLIST),
        "dcgo_available_keywords": sorted(dcgo_all),
        "subsystem_keywords": sorted(subsystem),
        "rust_enum_keywords": sorted(rust_keywords),
        "auto_ingest_candidates": candidates,
        "auto_ingest_simple": sorted(c for c in candidates if c not in subsystem),
        "auto_ingest_subsystem": sorted(c for c in candidates if c in subsystem),
        "rust_only_core_modeled": diff["core_modeled"],
    }


def write_manifest(out_path: str = "data/dcgo_keyword_manifest.json", **kw) -> dict:
    import json

    manifest = build_manifest(**kw)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)
    return manifest


if __name__ == "__main__":
    m = write_manifest()
    print(f"DCGO registry: {len(m['dcgo_registry_keywords'])} keywords")
    print(f"Auto-ingest candidates (DCGO has, Rust lacks): {m['auto_ingest_candidates']}")
    print(f"Core-modeled (Rust has, dirs lack): {m['rust_only_core_modeled']}")
