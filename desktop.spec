# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller spec for the Digimon TCG desktop sidecar.

Builds a single-file executable containing the lightweight game engine
server (no DB, no auth, no ML training). The output binary follows Tauri's
platform-specific naming convention for sidecar discovery.

Usage:
    pyinstaller desktop.spec

The build script (scripts/build-sidecar.sh) handles platform naming and
copying the binary to src-tauri/binaries/.
"""

import platform
import sys
from pathlib import Path

block_cipher = None

# Determine data files to include (card data, scripts, etc.)
engine_data = Path("digimon_gym/engine/data")
datas = []

# Include card database files
for pattern in ["*.json", "*.csv", "cards/**/*"]:
    for f in engine_data.glob(pattern):
        if f.is_file():
            datas.append((str(f), str(f.parent.relative_to("."))))

# Include card scripts
scripts_dir = Path("digimon_gym/engine/data/scripts")
if scripts_dir.exists():
    datas.append((str(scripts_dir), "digimon_gym/engine/data/scripts"))

a = Analysis(
    ["digimon_gym/desktop_main.py"],
    pathex=["."],
    binaries=[],
    datas=datas,
    hiddenimports=[
        "digimon_gym.engine",
        "digimon_gym.engine.game",
        "digimon_gym.engine.data",
        "digimon_gym.engine.data.enums",
        "digimon_gym.engine.data.deck_loader",
        "digimon_gym.engine.data.card_database",
        "digimon_gym.engine.runners",
        "digimon_gym.engine.runners.interactive_game",
        "digimon_gym.engine.runners.base_runner",
        "digimon_gym.engine.onnx_policy",
        "digimon_gym.engine.loggers",
        "digimon_gym.engine.recording",
        "digimon_gym.routers.schemas",
        "digimon_gym.routers.deck_tools",
        "uvicorn",
        "uvicorn.logging",
        "uvicorn.loops",
        "uvicorn.loops.auto",
        "uvicorn.protocols",
        "uvicorn.protocols.http",
        "uvicorn.protocols.http.auto",
        "uvicorn.protocols.websockets",
        "uvicorn.protocols.websockets.auto",
        "uvicorn.lifespan",
        "uvicorn.lifespan.on",
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    # Exclude heavy dependencies not needed for desktop sidecar
    excludes=[
        "torch",
        "torchvision",
        "torchaudio",
        "stable_baselines3",
        "sb3_contrib",
        "gymnasium",
        "openai",
        "anthropic",
        "sqlalchemy",
        "aiosqlite",
        "alembic",
        "bcrypt",
        "jose",
        "python_jose",
        "duckdb",
        "rank_bm25",
        "pypdf",
        "matplotlib",
        "scipy",
        "pandas",
        "sklearn",
        "tensorboard",
        "wandb",
    ],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.zipfiles,
    a.datas,
    [],
    name="digimon-server",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,  # Sidecar needs console for stdout/stderr piping to Tauri
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
