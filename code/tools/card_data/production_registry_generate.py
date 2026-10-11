"""Generate the raw production runtime-registry snapshot used by campaigns.

The Rust engine registry remains the authority for mechanical registration.
This command asks ``digimon-engine-cli pool`` for its implemented production
pool, intersects that result with an explicit compatibility-card universe,
removes synthetic fixtures, and renders deterministic JSON.  It does not
change ``tested_cards.json`` or select the validated campaign pool.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path
from typing import Any, Mapping

from .errors import ValidationError, WriteError
from .registry import is_gameplay_card_id
from .serialization import canonical_json_text, natural_card_id_key, sha256_hex


SCHEMA_VERSION = 1
DEFAULT_OUTPUT = Path("data/production_runtime_registry.json")
SNAPSHOT_KIND = "production_runtime_registry"
SNAPSHOT_SOURCE = "digimon-engine-cli:implemented"


def _read_json(path: Path, *, description: str) -> tuple[Any, bytes]:
    try:
        raw = path.read_bytes()
        return json.loads(raw.decode("utf-8")), raw
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValidationError(
            f"could not read {description}",
            path=path,
            details={"error": str(exc)},
        ) from exc


def _card_ids(payload: Any, *, path: Path) -> list[str]:
    if isinstance(payload, Mapping):
        payload = payload.get("card_ids")
    if not isinstance(payload, (list, tuple)) or not all(
        isinstance(item, str) for item in payload
    ):
        raise ValidationError(
            "production registry input must be a card-ID array",
            path=path,
        )
    if len(payload) != len(set(payload)):
        raise ValidationError(
            "production registry input contains duplicate card IDs",
            path=path,
        )
    return list(payload)


def acquire_engine_pool(
    *,
    repo_root: Path,
    cards_json_path: Path,
    pool_json_path: Path | None = None,
) -> list[str]:
    """Return the exact engine-reported pool, using a saved array when supplied."""

    if pool_json_path is not None:
        payload, _ = _read_json(pool_json_path, description="engine pool snapshot")
        return _card_ids(payload, path=pool_json_path)

    environment = dict(os.environ)
    environment.setdefault("RUST_MIN_STACK", "67108864")
    result = subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "digimon-engine-cli",
            "--",
            "--cards-json",
            str(cards_json_path),
            "pool",
        ],
        cwd=repo_root,
        capture_output=True,
        text=True,
        env=environment,
    )
    if result.returncode != 0:
        raise ValidationError(
            "digimon-engine-cli could not emit the production registry",
            path=cards_json_path,
            details={
                "exit_code": result.returncode,
                "stderr": result.stderr.strip(),
            },
        )
    try:
        payload = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise ValidationError(
            "digimon-engine-cli emitted invalid pool JSON",
            path=cards_json_path,
            details={"error": str(exc)},
        ) from exc
    return _card_ids(payload, path=cards_json_path)


def build_production_registry_payload(
    pool_ids: list[str],
    *,
    cards_payload: Any,
    cards_raw: bytes,
) -> dict[str, Any]:
    """Validate and render the deterministic real-card production snapshot."""

    if not isinstance(cards_payload, Mapping):
        raise ValidationError("compatibility cards JSON must be an object")
    if len(pool_ids) != len(set(pool_ids)):
        raise ValidationError("engine production pool contains duplicate card IDs")
    unknown = sorted(set(pool_ids) - set(cards_payload), key=natural_card_id_key)
    if unknown:
        raise ValidationError(
            "engine production pool contains IDs outside compatibility card data",
            details={"card_ids": unknown},
        )
    real_ids = tuple(
        sorted(
            (card_id for card_id in pool_ids if is_gameplay_card_id(card_id)),
            key=natural_card_id_key,
        )
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "snapshot_kind": SNAPSHOT_KIND,
        "source": SNAPSHOT_SOURCE,
        "cards_json_digest": sha256_hex(cards_raw),
        "card_count": len(real_ids),
        "card_ids": real_ids,
    }


def validate_production_registry_payload(
    payload: Any,
    *,
    cards_payload: Any,
    cards_raw: bytes,
    path: Path | None = None,
) -> tuple[str, ...]:
    """Validate one generated registry snapshot against exact card-data bytes.

    Campaign assembly must not reduce a structured snapshot to an unbound ID
    array.  The engine result is meaningful only for the precise compatibility
    artifact it loaded, so every metadata field emitted by this module is part
    of the validation boundary.
    """

    if not isinstance(payload, Mapping):
        raise ValidationError(
            "production runtime-registry snapshot must be a structured object",
            path=path,
        )
    expected_metadata = {
        "schema_version": SCHEMA_VERSION,
        "snapshot_kind": SNAPSHOT_KIND,
        "source": SNAPSHOT_SOURCE,
    }
    for field_name, expected in expected_metadata.items():
        actual = payload.get(field_name)
        if actual != expected:
            raise ValidationError(
                "production runtime-registry snapshot metadata is invalid",
                path=path,
                field_path=field_name,
                details={"expected": expected, "actual": actual},
            )

    expected_cards_digest = sha256_hex(cards_raw)
    actual_cards_digest = payload.get("cards_json_digest")
    if actual_cards_digest != expected_cards_digest:
        raise ValidationError(
            "production runtime-registry snapshot targets different cards.json bytes",
            path=path,
            field_path="cards_json_digest",
            details={
                "expected": expected_cards_digest,
                "actual": actual_cards_digest,
            },
        )

    card_ids = _card_ids(payload, path=path or Path("<production-registry>"))
    expected_order = sorted(card_ids, key=natural_card_id_key)
    if card_ids != expected_order:
        raise ValidationError(
            "production runtime-registry card IDs are not canonically ordered",
            path=path,
        )
    non_gameplay = [card_id for card_id in card_ids if not is_gameplay_card_id(card_id)]
    if non_gameplay:
        raise ValidationError(
            "production runtime-registry snapshot contains non-gameplay IDs",
            path=path,
            details={"card_ids": non_gameplay},
        )
    if not isinstance(cards_payload, Mapping):
        raise ValidationError("compatibility cards JSON must be an object", path=path)
    unknown = sorted(set(card_ids) - set(cards_payload), key=natural_card_id_key)
    if unknown:
        raise ValidationError(
            "production runtime-registry snapshot contains IDs outside compatibility card data",
            path=path,
            details={"card_ids": unknown},
        )
    card_count = payload.get("card_count")
    if (
        not isinstance(card_count, int)
        or isinstance(card_count, bool)
        or card_count != len(card_ids)
    ):
        raise ValidationError(
            "production runtime-registry card count does not match card_ids",
            path=path,
            field_path="card_count",
            details={"expected": len(card_ids), "actual": card_count},
        )
    return tuple(card_ids)


def render_production_registry(payload: Mapping[str, Any]) -> str:
    return canonical_json_text(payload)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--cards-json", type=Path, default=Path("data/cards.json"))
    parser.add_argument(
        "--pool-json",
        type=Path,
        help="offline engine pool array; omit to invoke digimon-engine-cli",
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args(argv)

    root = args.repo_root.resolve()

    def resolved(path: Path | None) -> Path | None:
        if path is None:
            return None
        return path if path.is_absolute() else root / path

    cards_path = resolved(args.cards_json)
    pool_path = resolved(args.pool_json)
    output_path = resolved(args.output)
    assert cards_path is not None and output_path is not None
    cards_payload, cards_raw = _read_json(
        cards_path, description="compatibility cards JSON"
    )
    pool_ids = acquire_engine_pool(
        repo_root=root,
        cards_json_path=cards_path,
        pool_json_path=pool_path,
    )
    rendered = render_production_registry(
        build_production_registry_payload(
            pool_ids,
            cards_payload=cards_payload,
            cards_raw=cards_raw,
        )
    )

    if args.check:
        try:
            current = output_path.read_text(encoding="utf-8")
        except OSError as exc:
            raise ValidationError(
                "production runtime-registry snapshot is unavailable",
                path=output_path,
            ) from exc
        if current != rendered:
            raise ValidationError(
                "production runtime-registry snapshot is stale",
                path=output_path,
            )
        return 0
    if args.write:
        try:
            output_path.parent.mkdir(parents=True, exist_ok=True)
            output_path.write_text(rendered, encoding="utf-8", newline="\n")
        except OSError as exc:
            raise WriteError(
                "could not write production runtime-registry snapshot",
                path=output_path,
            ) from exc
        return 0
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
