# syntax=docker/dockerfile:1.6

# ── Stage 1: Rust builder (produces the PyO3 wheel) ───────────────────────
FROM rust:1.94-slim AS rust-builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 python3-dev python3-pip pkg-config g++ \
    && rm -rf /var/lib/apt/lists/*
RUN pip3 install --no-cache-dir --break-system-packages 'maturin>=1.7,<2'
COPY Cargo.lock ./
# The server build only needs the three engine crates. The full workspace
# lists tauri/tools/mcp members whose dirs aren't in the build context, so
# write a reduced workspace manifest instead of copying the root Cargo.toml.
RUN printf '[workspace]\nmembers = [\n    "code/digimon-dsl",\n    "code/digimon-engine",\n    "code/digimon-engine-py",\n]\nresolver = "2"\n' > Cargo.toml
# data/ is include_str!'d into the engine crate at compile time
# (code/digimon-engine/src/deck_tools.rs), so it must exist in the build tree.
COPY data/ data/
COPY code/digimon-dsl/ code/digimon-dsl/
COPY code/digimon-engine/ code/digimon-engine/
COPY code/digimon-engine-py/ code/digimon-engine-py/
RUN cd code/digimon-engine-py && maturin build --release --out /wheels

# ── Stage 2: Python wheel builder ─────────────────────────────────────────
FROM python:3.11-slim AS py-builder
WORKDIR /build
COPY requirements-server.txt .
RUN pip wheel --no-cache-dir -r requirements-server.txt -w /wheels
COPY --from=rust-builder /wheels/*.whl /wheels/

# ── Stage 3: Runtime ──────────────────────────────────────────────────────
FROM python:3.11-slim AS runtime
# PYTHONPATH: alembic (unlike uvicorn) does not add the cwd to sys.path,
# so `alembic upgrade head` can't import server.db.models without it.
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1 PYTHONPATH=/app
# libstdc++6: the digimon_engine wheel links the ort (ONNX Runtime) crate,
# which needs the C++ runtime at import time.
RUN apt-get update && apt-get install -y --no-install-recommends libstdc++6 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd -u 1001 -m app
WORKDIR /app
COPY --from=py-builder /wheels /wheels
RUN pip install --no-cache-dir /wheels/*.whl && rm -rf /wheels
COPY code/server/ server/
COPY code/digimon_gym/ digimon_gym/
# Transitional: server.api's import graph still reaches engine_py_legacy
# (see openspec change excise-legacy-engine-from-hosted-api) and
# tools.decklist_analysis (server.classifier.meta_tier). Remove these COPYs
# once the excise change lands.
COPY code/engine_py_legacy/ engine_py_legacy/
COPY code/tools/decklist_analysis.py tools/decklist_analysis.py
# Ops one-shot: lets the runbook's CI-user provisioning run in-container
# (postgres is not exposed outside the compose network).
COPY code/tools/provision_ci_release_user.py tools/provision_ci_release_user.py
COPY code/data_paths.py .
COPY alembic/ alembic/
COPY alembic.ini .
# Shared game data: the PyO3 binding walks up from CWD for data/cards.json;
# data_paths.py resolves REPO_ROOT to / inside the image, so pin both
# explicitly.
COPY data/ data/
ENV DIGIMON_DATA_DIR=/app/data \
    DIGIMON_CARDS_JSON=/app/data/cards.json
USER app
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health').read()" || exit 1
CMD ["sh", "-c", "alembic upgrade head && uvicorn server.api:app --host 0.0.0.0 --port 8000"]
