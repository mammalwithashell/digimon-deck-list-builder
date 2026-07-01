"""Upload a trained ONNX model to the hosted API, confirm it, and (optionally)
publish it to the public manifest.

Flow (matches the implemented admin API in ``server/db/routers/admin_models.py``):
  POST /admin/models                 → create row + get a presigned Spaces PUT URL
  PUT  <presigned_url>               → upload the .onnx directly to Spaces
  POST /admin/models/{id}/confirm    → server validates ONNX, fills sha256/shape
  PATCH /admin/models/{id}           → set published=true  (only with --publish)

Auth — two options, admin-key preferred:
  * ``ADMIN_API_KEY`` (env or --admin-key) → sent as the ``X-Admin-Key`` header.
    This is how PROD is configured (no seeded admin user; ``/auth/login`` 401s).
  * ``ADMIN_USERNAME`` + ``ADMIN_PASSWORD`` → JWT via ``/auth/login`` (local dev).

Two gotchas this tool gets right (and the old version got wrong):
  * The presigned PUT signs ``ContentType`` AND ``ACL=public-read``
    (``spaces.generate_presigned_put``), so the upload MUST send both the
    ``Content-Type: application/octet-stream`` and ``x-amz-acl: public-read``
    headers or Spaces rejects the signature (fast HTTP 400, or a mid-body
    ``ConnectionReset`` with requests). The PUT is retried on transient errors.
  * ``--starter-deck`` tags the model with a built-in (non-DB) deck slug
    (e.g. ``starter_st3_heavens_yellow``) so the desktop AI-Starter mode routes
    that deck to this specialist.

Usage:
    # Prod specialist (admin key from the droplet):
    ADMIN_API_KEY=... python code/tools/publish_model.py \\
        --onnx models/st3.onnx --name "ST-3 Specialist" --type mlp \\
        --api-base https://inbetweentheatre.duckdns.org \\
        --starter-deck starter_st3_heavens_yellow --publish

    # Local dev (JWT login):
    ADMIN_USERNAME=... ADMIN_PASSWORD=... python code/tools/publish_model.py \\
        --onnx models/agent.onnx --name agent_v1 --type lstm \\
        --api-base http://localhost:8000

Env vars:
    ADMIN_API_KEY                  — operator key (preferred); → X-Admin-Key
    ADMIN_USERNAME / ADMIN_PASSWORD — admin login fallback (local dev)
    API_BASE_URL                  — overrides --api-base
"""

from __future__ import annotations

import argparse
import os
import sys
import time
from pathlib import Path

import requests

# The presigned URL signs these exact headers; the PUT must echo both.
_PUT_HEADERS = {
    "Content-Type": "application/octet-stream",
    "x-amz-acl": "public-read",
}
_PUT_ATTEMPTS = 4


def _login(session: requests.Session, base_url: str, username: str, password: str) -> str:
    """Log in and return the access token (local-dev JWT path)."""
    resp = session.post(
        f"{base_url}/auth/login",
        json={"username": username, "password": password},
    )
    if resp.status_code != 200:
        print(f"Login failed ({resp.status_code}): {resp.text}", file=sys.stderr)
        sys.exit(1)
    token = resp.json().get("access_token")
    if not token:
        print(f"No access_token in login response: {resp.json()}", file=sys.stderr)
        sys.exit(1)
    return token


def _authenticate(
    session: requests.Session,
    base_url: str,
    admin_key: str = "",
    username: str = "",
    password: str = "",
) -> None:
    """Attach admin auth to the session. Prefers the static admin key."""
    if admin_key:
        session.headers["X-Admin-Key"] = admin_key
        print("Auth: X-Admin-Key")
        return
    print(f"Auth: logging in as {username!r}")
    token = _login(session, base_url, username, password)
    session.headers["Authorization"] = f"Bearer {token}"


def _put_blob(url: str, data: bytes) -> None:
    """Upload bytes to the presigned Spaces URL with the signed headers + retry.

    Uses a bare ``requests.put`` (not the API session) so admin auth headers
    aren't sent to Spaces.
    """
    last = ""
    for attempt in range(1, _PUT_ATTEMPTS + 1):
        try:
            resp = requests.put(url, data=data, headers=_PUT_HEADERS, timeout=300)
            if resp.status_code in (200, 204):
                return
            last = f"HTTP {resp.status_code}: {resp.text[:160]}"
        except requests.exceptions.RequestException as exc:
            last = repr(exc)
        if attempt < _PUT_ATTEMPTS:
            print(f"  PUT attempt {attempt} failed ({last}); retrying...", file=sys.stderr)
            time.sleep(3 * attempt)
    print(f"Spaces upload failed after {_PUT_ATTEMPTS} attempts: {last}", file=sys.stderr)
    sys.exit(1)


def publish_model(
    onnx_path: str,
    name: str,
    model_type: str,
    api_base: str,
    *,
    admin_key: str = "",
    username: str = "",
    password: str = "",
    notes: str = "",
    starter_deck: str = "",
    deck_id: str = "",
    trained_at: str = "",
    publish: bool = False,
) -> str:
    """Upload (and optionally publish) a model via the admin API. Returns the id."""
    session = requests.Session()
    base_url = api_base.rstrip("/")

    print(f"Target: {base_url}")
    _authenticate(session, base_url, admin_key=admin_key, username=username, password=password)

    payload: dict = {
        "name": name,
        "model_type": model_type,
        "notes": notes or f"Uploaded by publish_model.py from {Path(onnx_path).name}",
    }
    if starter_deck:
        payload["starter_deck"] = starter_deck
    if deck_id:
        payload["deck_id"] = deck_id
    if trained_at:
        payload["trained_at"] = trained_at

    print(f"Creating model record: name={name!r} type={model_type} starter_deck={starter_deck or '-'}")
    resp = session.post(f"{base_url}/admin/models", json=payload)
    if resp.status_code != 201:
        print(f"Create model failed ({resp.status_code}): {resp.text}", file=sys.stderr)
        sys.exit(1)
    data = resp.json()
    model_id = data["id"]
    upload_url = data["upload_url"]
    print(f"Model ID: {model_id}")

    onnx_bytes = Path(onnx_path).read_bytes()
    print(f"Uploading {len(onnx_bytes) / 1_048_576:.1f} MB to Spaces...")
    _put_blob(upload_url, onnx_bytes)

    print("Confirming upload...")
    confirm_resp = session.post(f"{base_url}/admin/models/{model_id}/confirm")
    if confirm_resp.status_code != 200:
        print(f"Confirm failed ({confirm_resp.status_code}): {confirm_resp.text}", file=sys.stderr)
        sys.exit(1)
    c = confirm_resp.json()
    print(
        f"Confirmed: state={c['state']} tensor_size={c['tensor_size']} "
        f"action_space_size={c['action_space_size']} sha256={c['file_sha256'][:16]}..."
    )

    if publish:
        print("Publishing...")
        pub = session.patch(f"{base_url}/admin/models/{model_id}", json={"published": True})
        if pub.status_code != 200:
            print(f"Publish failed ({pub.status_code}): {pub.text}", file=sys.stderr)
            sys.exit(1)
        print("Published — visible in /models/manifest.json.")
    else:
        print()
        print("Model is uploaded and confirmed but NOT published. To publish:")
        print(f"  re-run with --publish, or PATCH {base_url}/admin/models/{model_id} {{\"published\": true}}")

    print(f"\nModel ID: {model_id}")
    return model_id


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Upload + (optionally) publish an ONNX model to the hosted API.")
    parser.add_argument("--onnx", required=True, help="Path to the .onnx model file")
    parser.add_argument("--name", required=True, help="Human-readable model name")
    parser.add_argument("--type", required=True, choices=["mlp", "lstm"], dest="model_type", help="Model type")
    parser.add_argument(
        "--api-base",
        default=os.environ.get("API_BASE_URL", "http://localhost:8000"),
        help="Hosted API base URL (or set API_BASE_URL)",
    )
    parser.add_argument("--notes", default="", help="Optional notes for admin display")
    parser.add_argument(
        "--starter-deck",
        default="",
        help="Built-in deck slug (e.g. starter_st3_heavens_yellow) for AI-Starter per-deck routing",
    )
    parser.add_argument("--deck-id", default="", help="Optional DB deck id (FK) to associate")
    parser.add_argument("--trained-at", default="", help="Optional ISO-8601 trained_at (orders the manifest)")
    parser.add_argument("--publish", action="store_true", help="Publish after confirm (sets published=true)")
    parser.add_argument(
        "--admin-key",
        default=os.environ.get("ADMIN_API_KEY", ""),
        help="Operator admin key → X-Admin-Key (or set ADMIN_API_KEY). Preferred over login.",
    )
    return parser


def main() -> None:
    args = build_parser().parse_args()

    admin_key = args.admin_key
    username = os.environ.get("ADMIN_USERNAME", "")
    password = os.environ.get("ADMIN_PASSWORD", "")

    if not admin_key and not (username and password):
        print(
            "Error: need ADMIN_API_KEY (or --admin-key), or ADMIN_USERNAME + ADMIN_PASSWORD.",
            file=sys.stderr,
        )
        sys.exit(1)

    if not Path(args.onnx).exists():
        print(f"Error: ONNX file not found: {args.onnx}", file=sys.stderr)
        sys.exit(1)

    publish_model(
        onnx_path=args.onnx,
        name=args.name,
        model_type=args.model_type,
        api_base=args.api_base,
        admin_key=admin_key,
        username=username,
        password=password,
        notes=args.notes,
        starter_deck=args.starter_deck,
        deck_id=args.deck_id,
        trained_at=args.trained_at,
        publish=args.publish,
    )


if __name__ == "__main__":
    main()
