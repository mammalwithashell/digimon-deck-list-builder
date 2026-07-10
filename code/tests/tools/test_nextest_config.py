import tomllib
from pathlib import Path


def test_nextest_config_serializes_heavy_engine_binaries_with_stack_wrapper():
    config = tomllib.loads(Path(".config/nextest.toml").read_text(encoding="utf-8"))

    overrides = config["profile"]["default"]["overrides"]
    filters = {override["filter"] for override in overrides}
    assert "binary(dsl) | binary(cards_behavioral)" in filters

    scripts = config["profile"]["default"]["scripts"]
    assert scripts[0]["run-wrapper"] == "rust-min-stack"
    assert config["scripts"]["rust-min-stack"]["command"].endswith(
        "scripts/nextest_rust_min_stack.py"
    )


def test_nextest_wrapper_sets_rust_min_stack():
    wrapper = Path("scripts/nextest_rust_min_stack.py").read_text(encoding="utf-8")

    assert "RUST_MIN_STACK" in wrapper
    assert "268435456" in wrapper
