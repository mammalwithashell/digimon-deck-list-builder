#!/usr/bin/env python3
"""Merge worker diffs into a tree (harden-card-authoring-pipeline D1, D2; tasks 1.2, 1.4).

The transport (D1): a worker leaves files at canonical paths in its own
worktree; `card_loop.workers.base.capture_artifacts` turns that into a binary
`git diff` against the pinned base plus a manifest
`{base_sha, files: [{path, status, sha256}]}` (a harden-style manifest whose
`files` are bare paths, with `card`/`verdict`/`test_result_lines`/... beside,
is accepted too). This module owns the merge invariants (D2), each asserted:

* `apply_manifest_diff` -- apply the diff 3-way, FILTERED to the manifest's
  paths. A path the diff carries but the manifest does not list is reported in
  `out_of_manifest` and never applied. The manifest's base must be the expected
  base, and the tree must descend from it (a stale worker base fails loudly).
  After applying, every file's bytes are checked against the manifest sha256
  (CRLF/LF-insensitive: `core.autocrlf` may rewrite line endings on checkout).
  Idempotent: a path already in its target state is a no-op, not an error.
  A failure restores every path it touched (worktree bytes and index entries)
  and leaves every other path alone, so a dirty unrelated file is never lost.
* Registration files: two cards of one set each add `mod <card>;` to the set's
  `mod.rs` against the same base, which a 3-way apply reports as a conflict.
  A registration file (`mod.rs` / `main.rs` under `code/digimon-engine/tests/`)
  whose change is purely added `mod x;` lines is therefore merged as a line
  union into the current file instead (`registered`).
* `assert_registration` -- a card YAML needs no registration (the pack build in
  `code/digimon-engine/build.rs` scans every directory under `cards/`), but a
  new test file must be declared on its whole `mod` chain up to the binary's
  `main.rs` (and a new `tests/<dir>/main.rs` needs a `[[test]]` entry), and it
  must contain at least one `#[test]`; otherwise it is a dead module that never
  compiles and its "passing" tests never ran.
* Pack-compile check: `cargo build -p digimon-engine` from the tree root, run
  only when a card YAML was applied. It is the pack build itself (build.rs
  panics on any YAML parse/compile error), so it is exactly D2's "verify YAML
  parses via the pack build". `dsl-lint` was not chosen: it depends on
  digimon-engine, so building it runs the same build.rs and fails the same way,
  only later.
* Scoped suite: `impact_scope.py --json` over the touched paths, then
  `cargo test --test cards_behavioral -- <filters> --test-threads=8` (CLAUDE.md
  rule 33) plus each side binary; the full binary when `full_suite_required`.
* `wave_summary` -- one row per entry plus the pack/suite results.

Every cargo / impact_scope subprocess goes through an injected runner
`(argv, cwd, timeout) -> (rc, stdout, stderr)` (the card loop passes
`RunContext.run_command`). The runner takes no environment, so cargo argv are
wrapped in a tiny `python -c` shim that sets `RUST_MIN_STACK` (rule 33) and,
for a scratch worktree, a per-worktree `CARGO_TARGET_DIR` (rule 31);
`split_env` undoes the wrapping for logs and tests. git runs directly.

Not built here (separate tasks): verdict writes to `validated_cards_dsl.json`,
`--apply-fixes` (1.3).

CLI:
    python code/tools/author_set/merge_wave.py --base <sha> --wave wave.json \
        [--approved BT21-029,...] [--register-missing] [--no-pack-check] \
        [--run-suite] [--json]
`wave.json` is `[{"card": "BT21-029", "diff": "...", "manifest": "..."}]`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping, Sequence

Runner = Callable[[list, str, float], tuple]

ENGINE_CARGO_MANIFEST = "code/digimon-engine/Cargo.toml"
ENGINE_TESTS_PREFIX = "code/digimon-engine/tests/"
CARD_YAML_PREFIX = "code/digimon-engine/cards/"
IMPACT_SCOPE_SCRIPT = "code/tools/impact_scope.py"
CARDS_BEHAVIORAL = "cards_behavioral"
RUST_MIN_STACK = "268435456"           # CLAUDE.md rule 33
DEFAULT_TEST_THREADS = 8               # CLAUDE.md rule 33
PACK_CHECK_ARGV = ("cargo", "build", "-p", "digimon-engine")

_GLOB_META = re.compile(r"[*?\[\]]")
_MOD_LINE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;\s*$")
_TEST_ATTR = re.compile(r"#\[\s*(?:[A-Za-z_][\w:]*::)?test\s*\]")
_SET_DIR = re.compile(r"^[a-z]+[0-9]*$")


class MergeWaveError(RuntimeError):
    pass


# ---------------------------------------------------------------------------
# git helpers (direct subprocess; never cargo)
# ---------------------------------------------------------------------------


def _git(repo: str | os.PathLike, *args: str, check: bool = True, stdin: bytes | None = None,
         text: bool = True) -> subprocess.CompletedProcess:
    cp = subprocess.run(["git", *args], cwd=str(repo), capture_output=True, input=stdin,
                        text=False)
    if text:
        cp = subprocess.CompletedProcess(cp.args, cp.returncode,
                                         cp.stdout.decode("utf-8", "replace"),
                                         cp.stderr.decode("utf-8", "replace"))
    if check and cp.returncode != 0:
        err = cp.stderr if text else cp.stderr.decode("utf-8", "replace")
        raise MergeWaveError(f"git {' '.join(args)} failed in {repo}: {str(err).strip()}")
    return cp


def _norm(path: str) -> str:
    return path.strip().replace("\\", "/").lstrip("./") if path else path


# ---------------------------------------------------------------------------
# manifest + diff inspection
# ---------------------------------------------------------------------------


def load_manifest(path: str | os.PathLike) -> dict:
    """The manifest, with `files` normalised to `[{path, status, sha256}]`
    (status/sha256 None when a harden-style manifest lists bare paths)."""
    raw = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(raw, Mapping) or not isinstance(raw.get("files"), list):
        raise MergeWaveError(f"{path}: not a worker manifest (no `files` list)")
    files = []
    for entry in raw["files"]:
        if isinstance(entry, str):
            files.append({"path": _norm(entry), "status": None, "sha256": None})
        elif isinstance(entry, Mapping) and isinstance(entry.get("path"), str):
            files.append({"path": _norm(entry["path"]), "status": entry.get("status"),
                          "sha256": entry.get("sha256")})
        else:
            raise MergeWaveError(f"{path}: bad manifest file entry {entry!r}")
    out = dict(raw)
    out["files"] = files
    return out


def diff_paths(repo: str | os.PathLike, diff_path: str | os.PathLike) -> list[str]:
    """Paths a diff touches, via `git apply --numstat -z` (handles quoting)."""
    cp = _git(repo, "apply", "--numstat", "-z", str(diff_path), text=False)
    out = []
    for rec in cp.stdout.split(b"\0"):
        if not rec:
            continue
        parts = rec.split(b"\t", 2)
        if len(parts) == 3:
            out.append(parts[2].decode("utf-8"))
    return out


def _diff_block(diff_bytes: bytes, path: str) -> bytes | None:
    """The `diff --git a/<path> b/<path>` section of a multi-file diff."""
    header = f"diff --git a/{path} b/{path}".encode("utf-8")
    start = diff_bytes.find(header)
    if start < 0 or (start > 0 and diff_bytes[start - 1:start] != b"\n"):
        return None
    nxt = diff_bytes.find(b"\ndiff --git ", start + len(header))
    return diff_bytes[start: nxt + 1 if nxt >= 0 else len(diff_bytes)]


def mod_line_additions(block: bytes) -> list[str] | None:
    """The `mod x;` lines a diff block adds, or None unless the block's change
    is purely added `mod` lines (blank added lines allowed, nothing removed)."""
    if b"GIT binary patch" in block:
        return None
    added: list[str] = []
    in_hunk = False
    for raw in block.decode("utf-8", "replace").splitlines():
        if raw.startswith("@@"):
            in_hunk = True
            continue
        if not in_hunk:
            continue
        if raw.startswith("+"):
            line = raw[1:].rstrip("\r")
            if not line.strip():
                continue
            if not _MOD_LINE.match(line):
                return None
            added.append(line.strip())
        elif raw.startswith("-"):
            return None
    return added or None


def _declared_mods(text: str) -> set[str]:
    out = set()
    for line in text.splitlines():
        m = _MOD_LINE.match(line)
        if m:
            out.add(m.group(1))
    return out


def content_hashes(data: bytes) -> set[str]:
    """sha256 of `data` as-is, LF-normalised and CRLF-expanded, so a manifest
    hash taken under one `core.autocrlf` matches a checkout under another."""
    lf = data.replace(b"\r\n", b"\n")
    crlf = lf.replace(b"\n", b"\r\n")
    return {hashlib.sha256(v).hexdigest() for v in (data, lf, crlf)}


def is_registration_file(path: str) -> bool:
    return path.startswith(ENGINE_TESTS_PREFIX) and path.rsplit("/", 1)[-1] in ("mod.rs", "main.rs")


def is_card_yaml(path: str) -> bool:
    return path.startswith(CARD_YAML_PREFIX) and path.endswith((".yaml", ".yml"))


def is_test_path(path: str) -> bool:
    """Test-only paths: anything under a crate's `tests/` directory."""
    p = _norm(path)
    return "/tests/" in f"/{p}" and p.startswith("code/")


# ---------------------------------------------------------------------------
# apply
# ---------------------------------------------------------------------------


@dataclass
class AppliedDiff:
    repo: str
    diff: str
    manifest: str
    base_sha: str
    statuses: dict = field(default_factory=dict)        # manifest path -> A/M/D/T/None
    applied: list = field(default_factory=list)          # written by this call (3-way apply)
    already_applied: list = field(default_factory=list)  # already in the target state (no-op)
    registered: list = field(default_factory=list)       # registration files merged as a mod-line union
    out_of_manifest: list = field(default_factory=list)  # in the diff, not in the manifest: never applied
    errors: list = field(default_factory=list)
    rolled_back: bool = False
    snapshot: dict = field(default_factory=dict, repr=False)  # path -> (bytes|None, index lines)

    @property
    def ok(self) -> bool:
        return not self.errors

    @property
    def noop(self) -> bool:
        return self.ok and not self.applied and not self.registered

    @property
    def touched(self) -> list[str]:
        """Manifest paths now in their target state (what to scope and commit)."""
        return sorted({*self.applied, *self.registered, *self.already_applied})

    @property
    def changed(self) -> list[str]:
        """Paths this call actually wrote."""
        return sorted({*self.applied, *self.registered})

    def to_dict(self) -> dict:
        return {"repo": self.repo, "diff": self.diff, "manifest": self.manifest,
                "base_sha": self.base_sha, "statuses": dict(self.statuses),
                "applied": list(self.applied), "already_applied": list(self.already_applied),
                "registered": list(self.registered), "out_of_manifest": list(self.out_of_manifest),
                "errors": list(self.errors), "rolled_back": self.rolled_back, "ok": self.ok,
                "noop": self.noop}


def _snapshot(repo: Path, paths: Iterable[str]) -> dict:
    snap = {}
    for p in paths:
        fp = repo / p
        data = fp.read_bytes() if fp.is_file() else None
        idx = _git(repo, "ls-files", "-s", "-z", "--", p).stdout
        snap[p] = (data, [e for e in idx.split("\0") if e])
    return snap


def _restore(repo: Path, snapshot: Mapping) -> None:
    for p, (data, index_lines) in snapshot.items():
        _git(repo, "update-index", "--force-remove", "--", p, check=False)
        if index_lines:
            _git(repo, "update-index", "--index-info",
                 stdin=("\n".join(index_lines) + "\n").encode("utf-8"), check=False)
        fp = repo / p
        if data is None:
            if fp.is_file():
                fp.unlink()
            parent = fp.parent
            while parent != repo and parent.is_dir() and not any(parent.iterdir()):
                parent.rmdir()
                parent = parent.parent
        else:
            fp.parent.mkdir(parents=True, exist_ok=True)
            fp.write_bytes(data)


def rollback(applied: AppliedDiff) -> None:
    """Restore every path `applied` changed to its pre-apply bytes and index
    entry. Paths it did not touch are left alone."""
    if applied.snapshot and not applied.rolled_back:
        _restore(Path(applied.repo), applied.snapshot)
        applied.rolled_back = True


def _union_mod_lines(repo: Path, path: str, lines: Sequence[str]) -> bool:
    """Append the missing `mod` lines to an existing registration file and
    stage it. Returns True when the file changed."""
    fp = repo / path
    data = fp.read_bytes()
    text = data.decode("utf-8")
    present = _declared_mods(text)
    missing = [l for l in lines if (_MOD_LINE.match(l).group(1) not in present)]
    if not missing:
        return False
    nl = "\r\n" if "\r\n" in text else "\n"
    if text and not text.endswith("\n"):
        text += nl
    text += "".join(l + nl for l in missing)
    fp.write_bytes(text.encode("utf-8"))
    _git(repo, "add", "--", path)
    return True


def apply_manifest_diff(repo: str | os.PathLike, diff_path: str | os.PathLike,
                        manifest_path: str | os.PathLike, base_sha: str) -> AppliedDiff:
    """Apply a worker diff 3-way onto `repo`'s working tree and index, filtered
    to the manifest's paths. Never raises for a bad diff: problems land in
    `errors` (and anything written is rolled back)."""
    root = Path(repo)
    res = AppliedDiff(repo=str(root), diff=str(diff_path), manifest=str(manifest_path),
                      base_sha=base_sha)
    try:
        manifest = load_manifest(manifest_path)
    except (OSError, ValueError, MergeWaveError) as e:
        res.errors.append(f"manifest unreadable: {e}")
        return res
    if not Path(diff_path).is_file():
        res.errors.append(f"diff {diff_path} not found")
        return res
    mbase = manifest.get("base_sha")
    if mbase and base_sha and mbase != base_sha:
        res.errors.append(
            f"stale worker base: the manifest was captured against {mbase}, this merge expects "
            f"{base_sha} -- re-run the worker in a worktree reset to {base_sha}")
        return res
    if _git(root, "cat-file", "-e", f"{base_sha}^{{commit}}", check=False).returncode != 0:
        res.errors.append(f"base {base_sha} is not a commit in {root}")
        return res
    if _git(root, "merge-base", "--is-ancestor", base_sha, "HEAD", check=False).returncode != 0:
        res.errors.append(f"{root} HEAD does not descend from base {base_sha}; the diff was "
                          f"made against that base -- merge onto a branch built from it")
        return res

    files = {f["path"]: f for f in manifest["files"]}
    res.statuses = {p: f["status"] for p, f in files.items()}
    try:
        in_diff = [_norm(p) for p in diff_paths(root, diff_path)]
    except MergeWaveError as e:
        res.errors.append(f"diff unreadable: {e}")
        return res
    res.out_of_manifest = sorted(set(in_diff) - set(files))
    for p in sorted(set(files) - set(in_diff)):
        res.errors.append(f"{p}: listed in the manifest but absent from the diff")
    for p in sorted(files):
        if _GLOB_META.search(p):
            res.errors.append(f"{p}: glob metacharacters in a path cannot be filtered with "
                              f"`git apply --include`")
    if res.errors:
        return res

    diff_bytes = Path(diff_path).read_bytes()
    to_apply: list[str] = []
    unions: dict[str, list[str]] = {}
    for p in sorted(set(in_diff) & set(files)):
        fp = root / p
        status = files[p]["status"]
        sha = files[p]["sha256"]
        if is_registration_file(p) and fp.is_file():
            block = _diff_block(diff_bytes, p)
            lines = mod_line_additions(block) if block is not None else None
            if lines is not None:
                if set(_MOD_LINE.match(l).group(1) for l in lines) <= _declared_mods(
                        fp.read_text(encoding="utf-8")):
                    res.already_applied.append(p)
                else:
                    unions[p] = lines
                continue
        if status == "D":
            if not fp.exists():
                res.already_applied.append(p)
                continue
        elif sha and fp.is_file() and sha in content_hashes(fp.read_bytes()):
            res.already_applied.append(p)
            continue
        elif not sha and status != "D" and fp.is_file():
            chk = _git(root, "apply", "--check", "--reverse", f"--include={p}", str(diff_path),
                       check=False)
            if chk.returncode == 0:
                res.already_applied.append(p)
                continue
        to_apply.append(p)

    res.snapshot = _snapshot(root, [*to_apply, *unions])
    try:
        for p, lines in unions.items():
            if _union_mod_lines(root, p, lines):
                res.registered.append(p)
            else:
                res.already_applied.append(p)
        if to_apply:
            argv = ["apply", "--3way", "--whitespace=nowarn",
                    *[f"--include={p}" for p in to_apply], str(diff_path)]
            cp = _git(root, *argv, check=False)
            if cp.returncode != 0:
                res.errors.append(f"git apply --3way failed (rc {cp.returncode}): "
                                  f"{(cp.stderr or cp.stdout).strip()[-1500:]}")
            else:
                res.applied.extend(to_apply)
        for p in res.applied:
            fp = root / p
            status, sha = files[p]["status"], files[p]["sha256"]
            if status == "D":
                if fp.exists():
                    res.errors.append(f"{p}: still present after applying its deletion")
            elif not fp.is_file():
                res.errors.append(f"{p}: missing after apply")
            elif sha and sha not in content_hashes(fp.read_bytes()):
                res.errors.append(f"{p}: content after apply does not match the manifest sha256 "
                                  f"{sha[:12]} (transport mismatch)")
    except (OSError, UnicodeDecodeError, MergeWaveError) as e:
        res.errors.append(f"apply failed: {e}")
    if res.errors:
        rollback(res)
        res.applied, res.registered = [], []
    res.applied.sort()
    res.already_applied.sort()
    res.registered.sort()
    return res


# ---------------------------------------------------------------------------
# registration
# ---------------------------------------------------------------------------


def _parent_declares(root: Path, parent_dir: str, name: str) -> tuple[bool, str]:
    """Does the module file of `parent_dir` (its `mod.rs`, or `<dir>.rs`)
    declare `mod <name>;`? Returns (declared, the file that should)."""
    for cand in (f"{parent_dir}/mod.rs", f"{parent_dir}.rs"):
        fp = root / cand
        if fp.is_file():
            return name in _declared_mods(fp.read_text(encoding="utf-8", errors="replace")), cand
    return False, f"{parent_dir}/mod.rs"


def assert_registration(repo: str | os.PathLike, applied: AppliedDiff | Iterable[str]) -> list[str]:
    """Problems that would leave a merged test file out of its test binary.

    For each applied `code/digimon-engine/tests/<bin>/.../<stem>.rs`: every
    module on the chain up to `<bin>/main.rs` is declared (`mod <stem>;` in the
    parent `mod.rs`, ...), the file holds at least one `#[test]` (a `mod.rs`
    is exempt), and a new `tests/<bin>/main.rs` has a `[[test]]` entry in the
    engine's Cargo.toml. A deleted test file must no longer be declared.
    An empty list means registered."""
    root = Path(repo)
    if isinstance(applied, AppliedDiff):
        paths = applied.touched
        statuses = applied.statuses
    else:
        paths = sorted(_norm(p) for p in applied)
        statuses = {}
    problems: list[str] = []
    for p in paths:
        if not (p.startswith(ENGINE_TESTS_PREFIX) and p.endswith(".rs")):
            continue
        rel = p[len(ENGINE_TESTS_PREFIX):]
        parts = rel.split("/")
        if len(parts) == 1:
            continue  # tests/<x>.rs: Cargo auto-discovers it as its own binary
        binary = parts[0]
        main_rs = f"{ENGINE_TESTS_PREFIX}{binary}/main.rs"
        # module chain: <bin>/a/b/c.rs -> [a, b, c]; <bin>/a/mod.rs -> [a]
        mods = [c[:-3] if c.endswith(".rs") else c for c in parts[1:]]
        if mods[-1] == "mod":
            mods = mods[:-1]

        def owner_declares(depth: int) -> tuple[bool, str]:
            name = mods[depth - 1]
            if depth == 1:
                return name in _declared_mods(_read(root / main_rs)), main_rs
            return _parent_declares(root, ENGINE_TESTS_PREFIX + "/".join([binary, *mods[:depth - 1]]),
                                    name)

        if statuses.get(p) == "D" or not (root / p).exists():
            if mods and parts[-1] != "mod.rs":
                declared, owner = owner_declares(len(mods))
                if declared:
                    problems.append(f"{p} was deleted but {owner} still declares "
                                    f"`mod {mods[-1]};` -- remove the declaration")
            continue
        if p == main_rs:
            cargo = _read(root / ENGINE_CARGO_MANIFEST)
            if f'path = "tests/{binary}/main.rs"' not in cargo:
                problems.append(f"{p} is a new test binary with no `[[test]]` entry in "
                                f"{ENGINE_CARGO_MANIFEST} -- add `[[test]] name = \"{binary}\" "
                                f"path = \"tests/{binary}/main.rs\"`")
            continue
        for depth in range(len(mods), 0, -1):
            name = mods[depth - 1]
            declared, owner = owner_declares(depth)
            if not declared:
                problems.append(f"dead module: {p} is not compiled -- {owner} does not declare "
                                f"`mod {name};` (add it, or the file's tests never run)")
                break
        if parts[-1] != "mod.rs" and not _TEST_ATTR.search(_read(root / p)):
            problems.append(f"{p} contains no #[test] -- a test module must contribute at least "
                            f"one test to the `{binary}` binary")
    return problems


def add_missing_registrations(repo: str | os.PathLike, applied: AppliedDiff | Iterable[str]) -> list[str]:
    """D2's "add if missing": append the `mod` lines `assert_registration`
    reports as missing (dead modules only) and stage them. Returns the files
    edited. Opt-in: the card loop does not call it (a worker that forgot its
    registration is corrected through the gate instead)."""
    root = Path(repo)
    edited: list[str] = []
    for _ in range(8):  # one chain level per pass
        progress = False
        for problem in assert_registration(root, applied):
            m = re.match(r"dead module: .* -- (\S+) does not declare `mod (\w+);`", problem)
            if not m:
                continue
            owner, name = m.group(1), m.group(2)
            fp = root / owner
            text = fp.read_text(encoding="utf-8") if fp.is_file() else ""
            if name in _declared_mods(text):
                continue
            nl = "\r\n" if "\r\n" in text else "\n"
            if text and not text.endswith("\n"):
                text += nl
            fp.parent.mkdir(parents=True, exist_ok=True)
            fp.write_bytes((text + f"mod {name};{nl}").encode("utf-8"))
            _git(root, "add", "--", owner)
            if owner not in edited:
                edited.append(owner)
            progress = True
        if not progress:
            break
    return edited


def _read(fp: Path) -> str:
    try:
        return fp.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


# ---------------------------------------------------------------------------
# commands: env shim, pack check, scope, scoped suite
# ---------------------------------------------------------------------------

# The runner contract takes no environment; this shim sets it for one child.
_ENV_SHIM = ("import os,subprocess,sys;a=sys.argv[1:];i=a.index('--');"
             "os.environ.update(x.split('=',1) for x in a[:i]);"
             "sys.exit(subprocess.call(a[i+1:]))")


def with_env(env: Mapping[str, str] | None, argv: Sequence[str]) -> list[str]:
    """`argv` run with `env` added, through a `python -c` shim."""
    if not env:
        return list(argv)
    return [sys.executable, "-c", _ENV_SHIM, *[f"{k}={v}" for k, v in sorted(env.items())],
            "--", *argv]


def split_env(argv: Sequence[str]) -> tuple[dict, list[str]]:
    """Inverse of `with_env`: (env, bare argv)."""
    argv = list(argv)
    if len(argv) >= 4 and argv[1] == "-c" and argv[2] == _ENV_SHIM:
        rest = argv[3:]
        i = rest.index("--")
        return dict(x.split("=", 1) for x in rest[:i]), rest[i + 1:]
    return {}, argv


def render_argv(argv: Sequence[str]) -> str:
    env, bare = split_env(argv)
    return " ".join([*(f"{k}={v}" for k, v in sorted(env.items())), *bare])


def cargo_env(*, cargo_target_dir: str | None = None) -> dict:
    env = {"RUST_MIN_STACK": RUST_MIN_STACK}
    if cargo_target_dir:
        env["CARGO_TARGET_DIR"] = cargo_target_dir
        env["CARGO_TARGET_DIR_PINNED"] = "1"
    return env


def output_tail(stdout: str, stderr: str, *, lines: int = 40, chars: int = 4000) -> str:
    text = "\n".join(s for s in ((stdout or "").rstrip(), (stderr or "").rstrip()) if s)
    tail = "\n".join(text.splitlines()[-lines:])
    return tail[-chars:]


def run_logged(runner: Runner, argv: Sequence[str], cwd: str, timeout: float) -> dict:
    """Run through the injected runner and return an evidence record."""
    try:
        rc, out, err = runner(list(argv), str(cwd), timeout)
    except Exception as e:  # a runner that raises (timeout, missing binary) is a red result
        rc, out, err = -1, "", f"{type(e).__name__}: {e}"
    return {"argv": list(argv), "command": render_argv(argv), "cwd": str(cwd), "rc": rc,
            "stdout": out or "", "stderr": err or "", "tail": output_tail(out, err)}


def strip_output(record: Mapping) -> dict:
    """An evidence record without the full stdout/stderr (keeps the tail)."""
    return {k: v for k, v in record.items() if k not in ("stdout", "stderr")}


def pack_check(runner: Runner, tree: str, *, env: Mapping[str, str] | None = None,
               timeout: float = 3600.0) -> dict:
    rec = run_logged(runner, with_env(env, PACK_CHECK_ARGV), tree, timeout)
    rec["ok"] = rec["rc"] == 0
    return rec


def card_filter(card_id: str) -> str:
    """cards_behavioral filter for a card id (`BT21-029` -> `bt21_029`), as
    `scripts/verify` builds it."""
    return card_id.lower().replace("-", "_")


def impact_scope_argv(paths: Sequence[str], *, paths_file: str | None = None) -> list[str]:
    argv = [sys.executable, IMPACT_SCOPE_SCRIPT, "--json"]
    if paths_file:
        return [*argv, "--paths-file", paths_file]
    for p in paths:
        argv += ["--path", p]
    return argv


def conservative_scope(scope: Mapping, touched: Sequence[str]) -> dict:
    """impact_scope maps engine sources, card YAML and test files; it maps no
    card *data* (`data/cards.json`, `data/card_overrides.json`), which changes
    behaviour. A touched `data/` file therefore escalates to the full suite."""
    out = dict(scope)
    data = sorted(p for p in touched if _norm(p).startswith("data/"))
    if data and not out.get("full_suite_required"):
        out["full_suite_required"] = True
        out["cards"] = []
        out["reasons"] = [*out.get("reasons", []),
                          f"{', '.join(data)}: card data is not mapped by impact_scope"]
    return out


def compute_scope(runner: Runner, tree: str, paths: Sequence[str], *,
                  timeout: float = 300.0) -> tuple[dict, dict]:
    """(scope, evidence). Runs `impact_scope.py --json` in `tree`. A failure
    is conservative: the full suite, with the reason recorded."""
    paths = sorted({_norm(p) for p in paths if p})
    tmp = None
    argv = impact_scope_argv(paths)
    if sum(len(p) + 8 for p in paths) > 16000:  # Windows command-line limit
        fd, tmp = tempfile.mkstemp(prefix="impact-paths-", suffix=".txt")
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write("\n".join(paths) + "\n")
        argv = impact_scope_argv(paths, paths_file=tmp)
    try:
        rec = run_logged(runner, argv, tree, timeout)
    finally:
        if tmp:
            os.unlink(tmp)
    scope: dict
    try:
        if rec["rc"] != 0:
            raise ValueError(f"rc {rec['rc']}")
        scope = json.loads(rec["stdout"])
        if not isinstance(scope, dict):
            raise ValueError("not a JSON object")
    except ValueError as e:
        scope = {"full_suite_required": True, "cards": [], "verbs": [], "side_binaries": [],
                 "reasons": [f"impact_scope failed ({e}); running the full suite"]}
    return conservative_scope(scope, paths), strip_output(rec)


def cargo_test_argv(binary: str, filters: Sequence[str] = (), *,
                    test_threads: int = DEFAULT_TEST_THREADS) -> list[str]:
    return ["cargo", "test", "--manifest-path", ENGINE_CARGO_MANIFEST, "--test", binary,
            "--", *filters, f"--test-threads={test_threads}"]


def _behavioral_module_filter(path: str) -> str | None:
    """`.../cards_behavioral/<set>/<stem>.rs` -> `<set>::<stem>`, `<set>/mod.rs`
    -> `<set>::`; None for anything that may be shared across the binary."""
    prefix = f"{ENGINE_TESTS_PREFIX}{CARDS_BEHAVIORAL}/"
    if not path.startswith(prefix):
        return None
    parts = path[len(prefix):].split("/")
    if len(parts) != 2 or not _SET_DIR.match(parts[0]) or not parts[1].endswith(".rs"):
        return None
    stem = parts[1][:-3]
    return f"{parts[0]}::" if stem == "mod" else f"{parts[0]}::{stem}"


def scoped_test_commands(scope: Mapping, touched: Sequence[str] = (), *,
                         test_threads: int = DEFAULT_TEST_THREADS) -> list[list[str]]:
    """Bare cargo argv for a scope (wrap with `with_env`).

    * `full_suite_required`: the whole `cards_behavioral` binary, plus every
      other side binary in full.
    * otherwise one `cards_behavioral` run filtered to the scope's cards
      (`card_filter`) and to the touched set-level test modules
      (`<set>::<stem>`); a touched `cards_behavioral` file that is not a
      set-level module (main.rs, a shared helper) runs the whole binary.
      Other side binaries run in full, as `scripts/verify` tier 2 does."""
    side = list(scope.get("side_binaries") or [])
    commands: list[list[str]] = []
    if scope.get("full_suite_required"):
        commands.append(cargo_test_argv(CARDS_BEHAVIORAL, test_threads=test_threads))
    else:
        filters = [card_filter(c) for c in scope.get("cards") or []]
        whole = False
        for p in sorted({_norm(t) for t in touched}):
            if p.startswith(f"{ENGINE_TESTS_PREFIX}{CARDS_BEHAVIORAL}/"):
                mf = _behavioral_module_filter(p)
                if mf is None:
                    whole = True
                elif mf not in filters:
                    filters.append(mf)
        if whole:
            commands.append(cargo_test_argv(CARDS_BEHAVIORAL, test_threads=test_threads))
        elif filters:
            commands.append(cargo_test_argv(CARDS_BEHAVIORAL, filters, test_threads=test_threads))
    for binary in side:
        if binary != CARDS_BEHAVIORAL:
            commands.append(cargo_test_argv(binary, test_threads=test_threads))
    return commands


def run_scoped_suite(runner: Runner, tree: str, scope: Mapping, touched: Sequence[str] = (), *,
                     env: Mapping[str, str] | None = None, timeout: float = 3600.0,
                     test_threads: int = DEFAULT_TEST_THREADS) -> dict:
    """{"green", "ran", "commands": [evidence...]}; stops at the first red run."""
    records = []
    green = True
    for argv in scoped_test_commands(scope, touched, test_threads=test_threads):
        rec = run_logged(runner, with_env(env, argv), tree, timeout)
        records.append(strip_output(rec))
        if rec["rc"] != 0:
            green = False
            break
    return {"green": green, "ran": bool(records), "commands": records}


# ---------------------------------------------------------------------------
# the wave
# ---------------------------------------------------------------------------


def wave_summary(rows: Sequence[Mapping], *, pack: Mapping | None = None,
                 suite: Mapping | None = None) -> dict:
    """Roll-up of a wave: counts, per-entry rows, out-of-manifest reports, and
    the pack/suite results. `ok` is False if any entry failed or a check is red."""
    by_status: dict[str, list[str]] = {}
    for r in rows:
        by_status.setdefault(r["status"], []).append(r["card"])
    failed = by_status.get("failed", [])
    ok = (not failed and (pack is None or pack.get("ok", True))
          and (suite is None or suite.get("green", True)))
    return {
        "ok": ok,
        "total": len(rows),
        "merged": by_status.get("merged", []),
        "noop": by_status.get("noop", []),
        "failed": failed,
        "skipped": by_status.get("not_approved", []),
        "out_of_manifest": {r["card"]: r["out_of_manifest"] for r in rows if r.get("out_of_manifest")},
        "rows": [dict(r) for r in rows],
        "pack_check": dict(pack) if pack else None,
        "suite": dict(suite) if suite else None,
    }


def render_wave_summary(summary: Mapping) -> str:
    lines = ["| card | status | applied | registered | already | out-of-manifest | problems |",
             "|---|---|---|---|---|---|---|"]
    for r in summary["rows"]:
        lines.append(f"| {r['card']} | {r['status']} | {len(r.get('applied', []))} | "
                     f"{len(r.get('registered', []))} | {len(r.get('already_applied', []))} | "
                     f"{', '.join(r.get('out_of_manifest', [])) or '-'} | "
                     f"{'; '.join(r.get('problems', [])) or '-'} |")
    pc = summary.get("pack_check")
    if pc:
        lines.append(f"\npack check: {'ok' if pc.get('ok') else 'FAILED'} ({pc.get('command')})")
    st = summary.get("suite")
    if st:
        lines.append(f"scoped suite: {'green' if st.get('green') else 'RED'} "
                     f"({len(st.get('commands', []))} command(s))")
    lines.append(f"\n{len(summary['merged'])} merged, {len(summary['noop'])} no-op, "
                 f"{len(summary['failed'])} failed, {len(summary['skipped'])} not approved")
    return "\n".join(lines)


def merge_wave(repo: str | os.PathLike, entries: Sequence[Mapping], base_sha: str, *,
               runner: Runner | None = None, approved: Iterable[str] | None = None,
               register_missing: bool = False, check_pack: bool = True, run_suite: bool = False,
               cargo_target_dir: str | None = None,
               test_threads: int = DEFAULT_TEST_THREADS) -> dict:
    """Apply each approved entry (`{card, diff, manifest}`) in order, assert its
    registration (optionally adding missing `mod` lines), then run the pack
    check if any card YAML changed and, with `run_suite`, the scoped suite.
    A failing entry is rolled back and the wave continues; re-running a merged
    wave is a no-op."""
    runner = runner or subprocess_runner
    approved_set = set(approved) if approved is not None else None
    rows = []
    changed_all: list[str] = []
    touched_all: list[str] = []
    for e in entries:
        card = e.get("card") or Path(e["diff"]).parent.name
        row: dict[str, Any] = {"card": card, "applied": [], "registered": [], "already_applied": [],
                               "out_of_manifest": [], "problems": []}
        if approved_set is not None and card not in approved_set:
            row["status"] = "not_approved"
            rows.append(row)
            continue
        res = apply_manifest_diff(repo, e["diff"], e["manifest"], base_sha)
        row.update(applied=res.applied, registered=res.registered,
                   already_applied=res.already_applied, out_of_manifest=res.out_of_manifest,
                   problems=list(res.errors))
        if res.ok:
            problems = assert_registration(repo, res)
            if problems and register_missing:
                added = add_missing_registrations(repo, res)
                row["registered"] = sorted({*row["registered"], *added})
                problems = assert_registration(repo, res)
            if problems:
                rollback(res)
                row["problems"] = problems
                row.update(applied=[], registered=[])
        row["status"] = ("failed" if row["problems"] else
                         "noop" if res.noop and not row["registered"] else "merged")
        if row["status"] != "failed":
            changed_all.extend([*row["applied"], *row["registered"]])
            touched_all.extend(res.touched)
        rows.append(row)
    env = cargo_env(cargo_target_dir=cargo_target_dir)
    pack = None
    if check_pack and any(is_card_yaml(p) for p in changed_all):
        pack = strip_output(pack_check(runner, str(repo), env=env))
    suite = None
    if run_suite and touched_all and (pack is None or pack["ok"]):
        scope, scope_rec = compute_scope(runner, str(repo), touched_all)
        suite = run_scoped_suite(runner, str(repo), scope, touched_all, env=env,
                                 test_threads=test_threads)
        suite["scope"] = scope
        suite["impact_scope"] = scope_rec
    return wave_summary(rows, pack=pack, suite=suite)


def subprocess_runner(argv: list, cwd: str, timeout: float) -> tuple[int, str, str]:
    cp = subprocess.run(argv, cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                        errors="replace", timeout=timeout)
    return cp.returncode, cp.stdout, cp.stderr


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--repo", default=".")
    ap.add_argument("--base", required=True, help="expected base sha the worker diffs were made on")
    ap.add_argument("--wave", required=True, type=Path,
                    help='JSON list of {"card", "diff", "manifest"}')
    ap.add_argument("--approved", help="comma-separated cards to merge (default: all)")
    ap.add_argument("--register-missing", action="store_true",
                    help="add missing `mod` lines instead of failing the entry")
    ap.add_argument("--no-pack-check", action="store_true")
    ap.add_argument("--run-suite", action="store_true", help="run the impact-scoped suite")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)
    entries = json.loads(args.wave.read_text(encoding="utf-8"))
    approved = [c.strip() for c in args.approved.split(",")] if args.approved else None
    summary = merge_wave(args.repo, entries, args.base, approved=approved,
                         register_missing=args.register_missing,
                         check_pack=not args.no_pack_check, run_suite=args.run_suite)
    print(json.dumps(summary, indent=2) if args.json else render_wave_summary(summary))
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
