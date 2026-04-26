"""Upload a trained ONNX model to the hosted API and confirm it.

Calls:
  POST /admin/models           → get presigned PUT URL
  PUT  <presigned_url>         → upload the .onnx file to Spaces
  POST /admin/models/{id}/confirm → validate ONNX, populate sha256/shape

Prints the model ID. Publishing (setting published=True) is left as a
manual admin step so the model can be reviewed before going live.

Usage:
    python tools/publish_model.py \\
        --onnx models/pilot_ppo_my_job.onnx \\
        --name "my_job_v1" \\
        --type lstm \\
        --api-base http://localhost:8000

Required env vars:
    ADMIN_USERNAME  — admin account username
    ADMIN_PASSWORD  — admin account password

Optional env vars:
    API_BASE_URL    — overrides --api-base
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path


def _login(session, base_url: str, username: str, password: str) -> str:
    """Log in and return the access token."""
    import requests

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


def publish_model(
    onnx_path: str,
    name: str,
    model_type: str,
    api_base: str,
    username: str,
    password: str,
    notes: str = "",
) -> str:
    """Upload model to Spaces via the admin API. Returns the model ID."""
    import requests

    session = requests.Session()
    base_url = api_base.rstrip("/")

    # Auth
    print(f"Authenticating with {base_url}...")
    token = _login(session, base_url, username, password)
    session.headers["Authorization"] = f"Bearer {token}"

    # Create model record
    print(f"Creating model record: name={name!r}, type={model_type}")
    resp = session.post(
        f"{base_url}/admin/models",
        json={
            "name": name,
            "model_type": model_type,
            "notes": notes or f"Uploaded by publish_model.py from {Path(onnx_path).name}",
        },
    )
    if resp.status_code != 201:
        print(f"Create model failed ({resp.status_code}): {resp.text}", file=sys.stderr)
        sys.exit(1)

    data = resp.json()
    model_id = data["id"]
    upload_url = data["upload_url"]
    print(f"Model ID: {model_id}")
    print(f"Uploading to Spaces...")

    # PUT the ONNX file to the presigned URL (direct to Spaces, not through API)
    onnx_bytes = Path(onnx_path).read_bytes()
    put_resp = requests.put(
        upload_url,
        data=onnx_bytes,
        headers={"Content-Type": "application/octet-stream"},
    )
    if put_resp.status_code not in (200, 204):
        print(f"Spaces upload failed ({put_resp.status_code}): {put_resp.text}", file=sys.stderr)
        sys.exit(1)
    print(f"Uploaded {len(onnx_bytes) / 1_048_576:.1f} MB")

    # Confirm: API streams from Spaces, validates ONNX, computes sha256
    print("Confirming upload...")
    confirm_resp = session.post(f"{base_url}/admin/models/{model_id}/confirm")
    if confirm_resp.status_code != 200:
        print(f"Confirm failed ({confirm_resp.status_code}): {confirm_resp.text}", file=sys.stderr)
        sys.exit(1)

    confirmed = confirm_resp.json()
    print(
        f"Confirmed: state={confirmed['state']}  "
        f"tensor_size={confirmed['tensor_size']}  "
        f"action_space_size={confirmed['action_space_size']}  "
        f"sha256={confirmed['file_sha256'][:16]}..."
    )

    print()
    print("Model is uploaded and confirmed.")
    print("To publish it (make it visible in the manifest):")
    print(f"  PATCH {base_url}/admin/models/{model_id}")
    print(f"  Body: {{\"published\": true}}")
    print()
    print(f"Model ID: {model_id}")
    return model_id


def main():
    parser = argparse.ArgumentParser(description="Upload a trained ONNX model to the hosted API.")
    parser.add_argument("--onnx", required=True, help="Path to the .onnx model file")
    parser.add_argument("--name", required=True, help="Human-readable model name")
    parser.add_argument("--type", required=True, choices=["mlp", "lstm"], dest="model_type", help="Model type")
    parser.add_argument("--api-base", default=os.environ.get("API_BASE_URL", "http://localhost:8000"), help="Hosted API base URL")
    parser.add_argument("--notes", default="", help="Optional notes for admin display")
    args = parser.parse_args()

    username = os.environ.get("ADMIN_USERNAME", "")
    password = os.environ.get("ADMIN_PASSWORD", "")

    if not username or not password:
        print("Error: ADMIN_USERNAME and ADMIN_PASSWORD env vars are required.", file=sys.stderr)
        sys.exit(1)

    if not Path(args.onnx).exists():
        print(f"Error: ONNX file not found: {args.onnx}", file=sys.stderr)
        sys.exit(1)

    publish_model(
        onnx_path=args.onnx,
        name=args.name,
        model_type=args.model_type,
        api_base=args.api_base,
        username=username,
        password=password,
        notes=args.notes,
    )


if __name__ == "__main__":
    main()
