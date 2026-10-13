"""Fail-closed process preflight for isolated campaign supervisors.

The caller must still atomically claim its output directory: a process snapshot
cannot prevent two simultaneous launchers from racing after enumeration.
"""
from pathlib import Path
import json
import subprocess


class ProcessPreflightError(RuntimeError):
    """The supervisor cannot establish exclusive, observable work."""


def validate_python_snapshot(raw: str, *, own_pid: int, forbidden: tuple[str, ...]) -> list[dict]:
    """Validate all observed Python records before looking for other workers."""
    if type(own_pid) is not int or own_pid <= 0:
        raise ProcessPreflightError("Invalid supervisor process ID")
    if not forbidden or any(not isinstance(value, str) or not value.strip() for value in forbidden):
        raise ProcessPreflightError("Missing duplicate-worker patterns")
    try:
        observed = json.loads(raw)
    except (ValueError, TypeError) as exc:
        raise ProcessPreflightError("Process enumeration returned invalid JSON") from exc
    if not isinstance(observed, list) or not observed:
        raise ProcessPreflightError("Process enumeration omitted the supervisor")
    seen = set()
    for record in observed:
        if not isinstance(record, dict):
            raise ProcessPreflightError("Invalid process record")
        pid = record.get("ProcessId")
        name = record.get("Name")
        command = record.get("CommandLine")
        if type(pid) is not int or pid <= 0 or pid in seen:
            raise ProcessPreflightError("Invalid or duplicate process ID")
        seen.add(pid)
        if not isinstance(name, str) or not name.casefold().startswith("python"):
            raise ProcessPreflightError(f"Unknown Python process identity: {pid}")
        if not isinstance(command, str) or not command.strip():
            raise ProcessPreflightError(f"Unreadable Python command line: {pid}")
        if pid != own_pid and any(value.casefold() in command.casefold() for value in forbidden):
            raise ProcessPreflightError(f"Another campaign worker is running: {pid}")
    if own_pid not in seen:
        raise ProcessPreflightError("Process enumeration omitted the supervisor")
    return observed


def enumerate_python_processes(*, own_pid: int, forbidden: tuple[str, ...]) -> list[dict]:
    """Observe versioned Python names; errors/timeouts remain fatal."""
    script = (
        "$ErrorActionPreference='Stop'; "
        "$campaignProcesses = @(Get-CimInstance Win32_Process | "
        "Where-Object { $_.Name -match '^python' } | "
        "Select-Object ProcessId,Name,CommandLine); "
        "ConvertTo-Json -InputObject @($campaignProcesses) -Compress"
    )
    raw = subprocess.check_output(
        ["powershell", "-NoProfile", "-Command", script],
        text=True, encoding="utf-8", timeout=30,
    )
    return validate_python_snapshot(raw, own_pid=own_pid, forbidden=forbidden)


def require_no_stop(*paths: Path) -> None:
    """Refuse new commands after an explicitly scoped STOP marker appears."""
    for path in paths:
        # A directory or dangling symlink named STOP must also stop the command.
        if path.exists() or path.is_symlink():
            raise ProcessPreflightError(f"STOP requested: {path}")
