"""Export a freshly-initialized (untrained) MaskablePPO to ONNX.

Used to publish the `null-agent-v0` placeholder for the F&F alpha.
Masked action selection over random logits produces legal random play —
a sufficient placeholder opponent until real models are trained.

Requires torch + SB3 (same as tools/export_onnx.py). Not run from the
hosted API image (torch-less); run from a dev machine with full deps.

Usage:
    python tools/export_null_agent.py --output models/null_agent.onnx
"""
from __future__ import annotations

import argparse
from pathlib import Path

from sb3_contrib import MaskablePPO
from stable_baselines3.common.vec_env import DummyVecEnv

from digimon_gym.digimon_gym import DigimonEnv
from tools.export_onnx import export_mlp


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    args.output.parent.mkdir(parents=True, exist_ok=True)

    env = DummyVecEnv([lambda: DigimonEnv()])
    model = MaskablePPO("MlpPolicy", env, verbose=0)

    zip_path = args.output.with_suffix(".zip")
    model.save(str(zip_path))

    export_mlp(str(zip_path), str(args.output))

    zip_path.unlink()
    print(f"null-agent ONNX written to {args.output}")


if __name__ == "__main__":
    main()
