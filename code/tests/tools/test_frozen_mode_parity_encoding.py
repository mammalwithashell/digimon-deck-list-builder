import copy
import json

import pytest

from tools.cloud_search import compare_frozen_mode_preflight as parity
from tools.cloud_search import frozen_mode_preflight as controller


def pair(tmp_path, windows=None, linux=None):
    roots = [tmp_path / name for name in ("windows", "linux")]
    lf = json.dumps(controller.schedule(), indent=2).encode()
    values = [lf.replace(b"\n", b"\r\n") if windows is None else windows,
              lf if linux is None else linux]
    manifests = []
    for root, value in zip(roots, values):
        (root / "inputs").mkdir(parents=True)
        (root / "assets").mkdir()
        (root / "inputs/schedule.json").write_bytes(value)
        (root / "assets/model.onnx").write_bytes(b"unchanged model bytes")
        manifests.append({name: controller.digest(root / name)
                          for name in ("inputs/schedule.json", "assets/model.onnx")})
    return roots, manifests


def test_native_schedule_encoding_is_exact_recorded_and_nonmutating(tmp_path):
    roots, manifests = pair(tmp_path)
    before = [(root / "inputs/schedule.json").read_bytes() for root in roots]
    proof = parity.verify_shared_inputs(*roots, manifests)
    assert len(proof) == 1
    assert proof[0]["windows_sha256"] == manifests[0]["inputs/schedule.json"]
    assert proof[0]["linux_sha256"] == manifests[1]["inputs/schedule.json"]
    assert proof[0]["canonical_lf_sha256"] == manifests[1]["inputs/schedule.json"]
    assert proof[0]["original_frozen_files_modified"] is False
    assert proof[0]["newline_count"] == 1877
    assert [(root / "inputs/schedule.json").read_bytes() for root in roots] == before


@pytest.mark.parametrize("crlf", [False, True])
def test_identical_frozen_input_bytes_need_no_encoding_exception(tmp_path, crlf):
    value = json.dumps(controller.schedule(), indent=2).encode()
    if crlf:
        value = value.replace(b"\n", b"\r\n")
    roots, manifests = pair(tmp_path, value, value)
    assert parity.verify_shared_inputs(*roots, manifests) == []


@pytest.mark.parametrize("mutation", ["seed", "deck", "model", "mode", "missing"])
def test_even_exact_crlf_transform_cannot_change_frozen_schedule(tmp_path, mutation):
    jobs = copy.deepcopy(controller.schedule())
    if mutation == "seed":
        jobs[0]["search_seed"] += 1
    elif mutation == "deck":
        jobs[0]["deck_indices"] = [1, 2]
    elif mutation == "model":
        jobs[0]["checkpoint"] = 20
    elif mutation == "mode":
        jobs[0]["candidate_mode"] = "known-list"
    else:
        jobs.pop()
    lf = json.dumps(jobs, indent=2).encode()
    roots, manifests = pair(tmp_path, lf.replace(b"\n", b"\r\n"), lf)
    with pytest.raises(ValueError, match="coverage/seeds"):
        parity.verify_shared_inputs(*roots, manifests)


@pytest.mark.parametrize("mutation", ["mixed", "extra_space", "key_order", "reverse_encoding"])
def test_other_text_differences_are_rejected(tmp_path, mutation):
    lf = json.dumps(controller.schedule(), indent=2).encode()
    win = lf.replace(b"\n", b"\r\n")
    if mutation == "mixed":
        win = win.replace(b"\r\n", b"\n", 1)
    elif mutation == "extra_space":
        win = win.replace(b"[", b"[ ", 1)
    elif mutation == "key_order":
        win = json.dumps(controller.schedule(), indent=2, sort_keys=True).encode().replace(b"\n", b"\r\n")
    else:
        win, lf = lf, win
    roots, manifests = pair(tmp_path, win, lf)
    with pytest.raises(ValueError, match="beyond native"):
        parity.verify_shared_inputs(*roots, manifests)


def test_other_frozen_input_changes_and_missing_keys_are_rejected(tmp_path):
    roots, manifests = pair(tmp_path)
    manifests[1]["assets/model.onnx"] = "changed"
    with pytest.raises(ValueError, match="frozen inputs mismatch"):
        parity.verify_shared_inputs(*roots, manifests)
    del manifests[1]["assets/model.onnx"]
    with pytest.raises(ValueError, match="frozen inputs mismatch"):
        parity.verify_shared_inputs(*roots, manifests)


def test_manifest_cannot_hide_changed_schedule_bytes(tmp_path):
    roots, manifests = pair(tmp_path)
    (roots[0] / "inputs/schedule.json").write_bytes(b"changed")
    with pytest.raises(ValueError, match="input changed"):
        parity.verify_shared_inputs(*roots, manifests)
