import hashlib
import importlib.machinery
import sys
from types import ModuleType
import pytest
from digimon_gym.agents.belief_runtime import native_binding_fingerprints


def test_rebuilt_native_wheel_changes_fingerprint_when_package_shim_does_not(
    tmp_path, monkeypatch
):
    for name in list(sys.modules):
        if name == "digimon_engine" or name.startswith("digimon_engine."):
            monkeypatch.delitem(sys.modules, name)
    shim = tmp_path / "__init__.py"
    shim.write_text("# unchanged package shim")
    native = tmp_path / ("digimon_engine" + importlib.machinery.EXTENSION_SUFFIXES[0])
    native.write_bytes(b"native version one")
    package = ModuleType("digimon_engine")
    package.__file__ = str(shim)
    extension = ModuleType("digimon_engine.digimon_engine")
    extension.__file__ = str(native)
    monkeypatch.setitem(sys.modules, package.__name__, package)
    monkeypatch.setitem(sys.modules, extension.__name__, extension)
    first = native_binding_fingerprints()
    assert first == {
        extension.__name__: hashlib.sha256(native.read_bytes()).hexdigest()
    }
    native.write_bytes(b"native version two")
    assert native_binding_fingerprints() != first
    monkeypatch.delitem(sys.modules, extension.__name__)
    with pytest.raises(RuntimeError, match="no loaded native"):
        native_binding_fingerprints()
