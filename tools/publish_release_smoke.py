"""Smoke-test the /admin/releases flow end-to-end against a running hosted API.

Usage (local dev):
    python tools/publish_release_smoke.py \\
        --api http://localhost:8000 \\
        --token "$CI_ADMIN_TOKEN" \\
        --version 0.0.1-smoke.1 \\
        --windows-installer /tmp/fake-installer.exe \\
        --windows-sig /tmp/fake-installer.exe.sig \\
        --linux-installer /tmp/fake-installer.AppImage \\
        --linux-sig /tmp/fake-installer.AppImage.sig

The "installers" can be arbitrary bytes for smoke purposes — we're not
verifying signatures here, just the hosted API + Spaces round trip. The
hosted API + SPACES_* env vars must be configured on the server side.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys
import urllib.request

import requests


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--api", required=True, help="hosted API base URL")
    p.add_argument("--token", required=True, help="admin JWT")
    p.add_argument("--version", required=True)
    p.add_argument("--channel", default="alpha")
    p.add_argument("--engine-commit", default="smoke")
    p.add_argument("--min-version", default="0.0.0")
    p.add_argument("--release-notes", default="smoke test")
    p.add_argument("--windows-installer", required=True)
    p.add_argument("--windows-sig", required=True)
    p.add_argument("--linux-installer", required=True)
    p.add_argument("--linux-sig", required=True)
    args = p.parse_args()

    headers = {"Authorization": f"Bearer {args.token}"}

    create = requests.post(
        f"{args.api}/admin/releases",
        headers=headers,
        json={
            "version": args.version,
            "channel": args.channel,
            "engine_commit": args.engine_commit,
            "min_version": args.min_version,
            "release_notes": args.release_notes,
            "targets": ["windows-x86_64", "linux-x86_64"],
        },
        timeout=30,
    )
    create.raise_for_status()
    body = create.json()
    release_id = body["release_id"]
    print(f"Created release {release_id} ({args.version}, {args.channel})")

    per_target = {
        "windows-x86_64": (args.windows_installer, args.windows_sig),
        "linux-x86_64": (args.linux_installer, args.linux_sig),
    }

    for slot in body["artifacts"]:
        target = slot["target"]
        installer_path, sig_path = per_target[target]

        with open(installer_path, "rb") as f:
            data = f.read()
        put = urllib.request.Request(
            slot["upload_url"],
            data=data,
            method="PUT",
            headers={
                "Content-Type": "application/octet-stream",
                "x-amz-acl": "public-read",
            },
        )
        with urllib.request.urlopen(put) as resp:
            if resp.status not in (200, 204):
                raise RuntimeError(f"upload {target}: HTTP {resp.status}")
        print(f"Uploaded {target} ({len(data)} bytes)")

        sig_b64 = pathlib.Path(sig_path).read_text().strip()
        confirm = requests.post(
            f"{args.api}/admin/releases/{release_id}/artifacts/{target}/confirm",
            headers=headers,
            json={"signature_b64": sig_b64},
            timeout=120,
        )
        confirm.raise_for_status()
        print(f"Confirmed {target}: sha256={confirm.json()['file_sha256']}")

    pub = requests.post(
        f"{args.api}/admin/releases/{release_id}/publish",
        headers=headers,
        timeout=30,
    )
    pub.raise_for_status()
    print(f"Published {release_id}")

    regen = requests.post(
        f"{args.api}/admin/releases/regenerate-manifest",
        params={"channel": args.channel},
        headers=headers,
        timeout=30,
    )
    regen.raise_for_status()
    print("Current manifest:")
    print(json.dumps(regen.json(), indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
