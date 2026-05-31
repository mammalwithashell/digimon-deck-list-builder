"""Run a named training job from a JSON config file.

Reads a job config, optionally queries DigiLab for store-scoped meta stats,
configures MetaGauntlet, and kicks off pilot_training.train().

Usage:
    python tools/run_training_job.py training_jobs/my_job.json
    python tools/run_training_job.py training_jobs/my_job.json --dry-run
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional


# ---------------------------------------------------------------------------
# Deck loading
# ---------------------------------------------------------------------------

def _load_deck_from_file(path: str) -> List[str]:
    """Load a deck list from a JSON array or digimoncard.io text file."""
    from digimon_engine import parse_tts, parse_text

    raw = Path(path).read_text(encoding="utf-8")
    stripped = raw.strip()
    if stripped.startswith("["):
        return parse_tts(stripped)
    return parse_text(stripped)


def _load_deck_from_library(deck_id: str) -> List[str]:
    """Fetch a specific deck by ID from deck_library.json."""
    from data_paths import DECK_LIBRARY
    with open(DECK_LIBRARY, encoding="utf-8") as f:
        library = json.load(f)
    from digimon_engine import parse_tts
    for arch_data in library.get("archetypes", {}).values():
        for deck in arch_data.get("decklists", []):
            if deck.get("deck_id") == deck_id:
                return parse_tts(deck["decklist"])
    raise ValueError(f"deck_id {deck_id!r} not found in deck_library.json")


def load_deck(deck_config: Dict[str, Any]) -> Optional[List[str]]:
    """Resolve an agent deck from the job config's agent_deck block.

    Supported sources:
      "file"      — path to a JSON array (.json) or text format (.txt) deck
      "deck_id"   — a deck_id string from deck_library.json
      "default"   — use the engine's default ST1 starter deck (None)
    """
    source = deck_config.get("source", "default")
    if source == "file":
        return _load_deck_from_file(deck_config["path"])
    if source == "deck_id":
        return _load_deck_from_library(deck_config["deck_id"])
    if source == "default":
        return None
    raise ValueError(f"Unknown agent_deck source: {source!r}. Use 'file', 'deck_id', or 'default'.")


# ---------------------------------------------------------------------------
# Gauntlet setup
# ---------------------------------------------------------------------------

def build_gauntlet(
    meta_scope: Optional[Dict[str, Any]],
    verbose: bool = True,
    allowed_archetypes: Optional[List[str]] = None,
):
    """Build and optionally scope a MetaGauntlet."""
    from digimon_gym.agents.gauntlet import MetaGauntlet

    allowed_set = set(allowed_archetypes) if allowed_archetypes else None
    gauntlet = MetaGauntlet(alpha=1.0, beta=2.0, allowed_archetypes=allowed_set)
    gauntlet.load()
    if verbose and allowed_set is not None:
        print(
            f"Archetype filter: declared {len(allowed_set)} entries, "
            f"resolved {gauntlet.archetype_count} archetypes"
        )

    if not meta_scope:
        if verbose:
            print(f"Meta scope: global ({gauntlet.archetype_count} archetypes, {gauntlet.deck_count} decks)")
        return gauntlet

    store_ids = meta_scope.get("store_ids")
    scene_id = meta_scope.get("scene_id")
    since_date = meta_scope.get("since_date")
    event_type = meta_scope.get("event_type")

    if verbose:
        scope_parts = []
        if store_ids:
            scope_parts.append(f"stores={store_ids}")
        if scene_id:
            scope_parts.append(f"scene={scene_id}")
        if since_date:
            scope_parts.append(f"since={since_date}")
        print(f"Meta scope: local ({', '.join(scope_parts)})")
        print("Querying DigiLab for scoped meta stats...")

    try:
        from server.digilab_client import get_scoped_meta
    except ModuleNotFoundError as exc:
        raise RuntimeError(
            "Scoped-meta training (meta_scope set) requires the hosted-server "
            "package (`server.digilab_client`), which is not present in this "
            "environment (e.g. the lean training image). Run scoped-meta jobs "
            "where `server` is importable, or omit `meta_scope` for a global / "
            "generalist run."
        ) from exc

    result = get_scoped_meta(
        store_ids=store_ids,
        scene_id=scene_id,
        since_date=since_date,
        event_type=event_type,
    )

    local_shares = {
        arch_name: stats.meta_share
        for arch_name, stats in result.archetypes.items()
        if stats.times_played > 0
    }

    if not local_shares:
        print("WARNING: No scoped meta data found — falling back to global meta.")
        return gauntlet

    gauntlet.override_meta_shares(local_shares)

    if verbose:
        print(f"Scoped meta: {len(local_shares)} archetypes from {result.total_results} results")
        print("Top 10 opponents by Threat Index:")
        for row in gauntlet.get_archetype_summary()[:10]:
            print(
                f"  {row['archetype_name']:<30} "
                f"TI={row['threat_index']:.3f}  "
                f"share={row['meta_share']:.1%}  "
                f"decks={row.get('deck_count', '?')}"
            )

    return gauntlet


# ---------------------------------------------------------------------------
# Main runner
# ---------------------------------------------------------------------------

_TRAIN_KWARGS = {
    "eval_freq", "n_eval_episodes", "learning_rate", "n_steps",
    "batch_size", "n_epochs", "gamma", "bounty_threshold", "bounty_bonus",
    "use_lstm", "lstm_hidden_size", "reward_profile_override", "tensor_profile",
}


def _resolve_allowed_archetypes(config: Dict[str, Any]) -> Optional[List[str]]:
    """Resolve `allowed_archetypes` from training block or top-level."""
    training_cfg = config.get("training", {})
    if "allowed_archetypes" in training_cfg:
        return list(training_cfg["allowed_archetypes"])
    if "allowed_archetypes" in config:
        return list(config["allowed_archetypes"])
    return None


def run_job(config_path: str, dry_run: bool = False) -> None:
    with open(config_path, encoding="utf-8") as f:
        config = json.load(f)

    job_name = config.get("job_name", Path(config_path).stem)
    description = config.get("description", "")
    training_cfg = config.get("training", {})
    output_cfg = config.get("output", {})

    print("=" * 60)
    print(f"Training Job: {job_name}")
    if description:
        print(f"  {description}")
    print("=" * 60)

    # Deck
    agent_deck = load_deck(config.get("agent_deck", {"source": "default"}))
    if agent_deck:
        print(f"Agent deck: {len(agent_deck)} cards")
    else:
        print("Agent deck: ST1 default")

    allowed_archetypes = _resolve_allowed_archetypes(config)

    # Generalist runs supersede gauntlet runs: build the generalist pool and
    # pass it through; do not build a gauntlet.
    generalist_mode = bool(training_cfg.get("generalist", False))
    gauntlet = None
    generalist_deck_pool = None
    if generalist_mode:
        from digimon_gym.agents.gauntlet import load_generalist_deck_pool

        allowed_set = set(allowed_archetypes) if allowed_archetypes else None
        generalist_deck_pool = load_generalist_deck_pool(
            allowed_archetypes=allowed_set,
        )
        print(
            f"Generalist pool: {generalist_deck_pool.archetype_count} archetypes, "
            f"{generalist_deck_pool.deck_count} decks"
        )
        if allowed_set is not None:
            print(
                f"Archetype filter: declared {len(allowed_set)} entries, "
                f"resolved {generalist_deck_pool.archetype_count} archetypes"
            )
    else:
        gauntlet = build_gauntlet(
            config.get("meta_scope"),
            verbose=True,
            allowed_archetypes=allowed_archetypes,
        )

    # Resolve training kwargs
    train_kwargs = {k: training_cfg[k] for k in _TRAIN_KWARGS if k in training_cfg}
    # Reward-profile selection: when the job requests a reward profile, resolve
    # the reward YAMLs relative to the installed `digimon_gym` package so they
    # load regardless of CWD / image layout. The TrainingConfig defaults are
    # repo-relative ("code/digimon_gym/agents/reward/...") which do NOT exist in
    # the flattened training image (code/ → /app/), so without this the profile
    # system silently falls back to the legacy reward path.
    if "reward_profile_override" in train_kwargs:
        import digimon_gym as _dg

        _reward_dir = Path(_dg.__file__).resolve().parent / "agents" / "reward"
        train_kwargs.setdefault("reward_profiles_path", str(_reward_dir / "profiles.yaml"))
        train_kwargs.setdefault("reward_gameplay_path", str(_reward_dir / "gameplay.yaml"))
        print(
            f"Reward profile: {train_kwargs['reward_profile_override']} "
            f"(profiles={train_kwargs['reward_profiles_path']})"
        )
    opponent = training_cfg.get("opponent", "greedy")
    total_timesteps = training_cfg.get("timesteps", 100_000)

    save_dir = output_cfg.get("save_dir", "models")
    log_dir = output_cfg.get("log_dir", "runs/pilot_ppo")
    job_id = output_cfg.get("name", job_name)

    print(f"\nOutput: models/pilot_ppo_{job_id}.zip")
    print(f"TensorBoard: {log_dir}")

    if dry_run:
        print("\n[dry-run] Skipping training.")
        return

    from digimon_gym.agents.pilot_training import train

    train(
        total_timesteps=total_timesteps,
        opponent=opponent,
        deck1=agent_deck,
        gauntlet=gauntlet,
        generalist_deck_pool=generalist_deck_pool,
        save_dir=save_dir,
        tensorboard_log=log_dir,
        job_id=job_id,
        **train_kwargs,
    )


def main():
    parser = argparse.ArgumentParser(description="Run a Digimon TCG training job from a JSON config.")
    parser.add_argument("config", help="Path to training job JSON config file")
    parser.add_argument("--dry-run", action="store_true", help="Validate config and print plan without training")
    args = parser.parse_args()

    if not os.path.exists(args.config):
        print(f"Error: config file not found: {args.config}", file=sys.stderr)
        sys.exit(1)

    run_job(args.config, dry_run=args.dry_run)


if __name__ == "__main__":
    main()
