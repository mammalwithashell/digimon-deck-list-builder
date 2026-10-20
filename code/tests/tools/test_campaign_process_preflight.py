"""Refuse unknown process state before reserving an offline campaign run."""
import json
import subprocess

import pytest

from tools.card_loop import supervision


def snapshot(*others):
    return json.dumps([{"ProcessId": 10, "Name": "python3.13.exe", "CommandLine": "python own.py"}, *others])


def worker(command="python other.py", **changes):
    return {"ProcessId": 11, "Name": "python.exe", "CommandLine": command, **changes}


@pytest.mark.parametrize("command", [None, "", " ", 99, False])
def test_unreadable_command_is_not_treated_as_idle(command):
    with pytest.raises(supervision.ProcessPreflightError):
        supervision.validate_python_snapshot(snapshot(worker(command)), own_pid=10, forbidden=("full99",))


def test_missing_command_is_not_treated_as_idle():
    p = worker()
    del p["CommandLine"]
    with pytest.raises(supervision.ProcessPreflightError):
        supervision.validate_python_snapshot(snapshot(p), own_pid=10, forbidden=("full99",))


@pytest.mark.parametrize("pid", [None, 0, -1, True, "11", 11.5])
def test_invalid_pid_is_refused(pid):
    with pytest.raises(supervision.ProcessPreflightError):
        supervision.validate_python_snapshot(snapshot(worker(ProcessId=pid)), own_pid=10, forbidden=("full99",))


@pytest.mark.parametrize("raw", ["null", "{}", "[]", "[null]", "[1]", "not json", snapshot(worker(ProcessId=10))])
def test_incomplete_or_duplicate_process_snapshot_is_refused(raw):
    with pytest.raises(supervision.ProcessPreflightError):
        supervision.validate_python_snapshot(raw, own_pid=10, forbidden=("full99",))


@pytest.mark.parametrize("name", ["python.exe", "pythonw.exe", "python3.13.exe", "Python3.14t.exe"])
@pytest.mark.parametrize("command", ['python "C:/runs/run-diagnostic-source-book-full99-r4.py"', "PYTHON -m TOOLS.CARD_DATA.COMPLETE_CANDIDATE", "python -m=tools.card_data.complete_candidate"])
def test_duplicate_matches_versioned_python_and_quoted_or_joined_arguments(name, command):
    with pytest.raises(supervision.ProcessPreflightError):
        supervision.validate_python_snapshot(snapshot(worker(command, Name=name)), own_pid=10, forbidden=("run-diagnostic-source-book-full99", "tools.card_data.complete_candidate"))


def test_unrelated_python_is_allowed_and_snapshot_retained():
    raw = snapshot(worker())
    assert supervision.validate_python_snapshot(raw, own_pid=10, forbidden=("full99",)) == json.loads(raw)


def test_single_own_process_is_a_valid_array():
    assert len(supervision.validate_python_snapshot(snapshot(), own_pid=10, forbidden=("full99",))) == 1


def test_enumeration_failure_is_fatal(monkeypatch):
    def fail(*args, **kwargs):
        raise subprocess.CalledProcessError(1, args[0])
    monkeypatch.setattr(supervision.subprocess, "check_output", fail)
    with pytest.raises(subprocess.CalledProcessError):
        supervision.enumerate_python_processes(own_pid=10, forbidden=("full99",))


def test_command_enumerates_versioned_names_and_preserves_single_row_array(monkeypatch):
    def run(cmd, **kwargs):
        script = cmd[-1]
        assert "$ErrorActionPreference='Stop'" in script
        assert "'^python'" in script
        assert "ConvertTo-Json -InputObject @(" in script
        assert kwargs["timeout"] == 30
        return snapshot()
    monkeypatch.setattr(supervision.subprocess, "check_output", run)
    assert len(supervision.enumerate_python_processes(own_pid=10, forbidden=("full99",))) == 1


def test_stop_marker_refuses_command(tmp_path):
    stop = tmp_path / "STOP"
    supervision.require_no_stop(stop)
    stop.write_text("drain")
    with pytest.raises(supervision.ProcessPreflightError):
        supervision.require_no_stop(stop)
