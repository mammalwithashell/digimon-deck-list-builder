# syntax=docker/dockerfile:1.6

# ── Stage 1: Rust builder (produces the PyO3 wheel) ───────────────────────
FROM rust:1.78-slim AS rust-builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3.11 python3.11-dev python3-pip pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN pip3 install --no-cache-dir --break-system-packages maturin==1.5.1
COPY Cargo.toml Cargo.lock ./
COPY digimon-engine/ digimon-engine/
COPY digimon-engine-py/ digimon-engine-py/
# Remove src-tauri from the workspace so Tauri deps don't block the server build
RUN sed -i 's|.*"src-tauri".*||' Cargo.toml
RUN cd digimon-engine-py && maturin build --release --out /wheels

# ── Stage 2: Python wheel builder ─────────────────────────────────────────
FROM python:3.11-slim AS py-builder
WORKDIR /build
COPY requirements-server.txt .
RUN pip wheel --no-cache-dir -r requirements-server.txt -w /wheels
COPY --from=rust-builder /wheels/*.whl /wheels/

# ── Stage 3: Runtime ──────────────────────────────────────────────────────
FROM python:3.11-slim AS runtime
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
RUN useradd -u 1001 -m app
WORKDIR /app
COPY --from=py-builder /wheels /wheels
RUN pip install --no-cache-dir /wheels/*.whl && rm -rf /wheels
COPY digimon_gym/ digimon_gym/
COPY alembic/ alembic/
COPY alembic.ini .
USER app
EXPOSE 8000
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8000/health').read()" || exit 1
CMD ["sh", "-c", "alembic upgrade head && uvicorn digimon_gym.api:app --host 0.0.0.0 --port 8000"]
