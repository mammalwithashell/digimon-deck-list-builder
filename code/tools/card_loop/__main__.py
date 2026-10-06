"""`python -m tools.card_loop <command> [args]` — thin dispatcher.

Each command lives in the module that owns it and is imported lazily, so a
command whose module is not built yet reports that instead of breaking the CLI.
Every target is `fn(argv: list[str]) -> int` and parses its own arguments.
"""
from __future__ import annotations

import importlib
import sys

COMMANDS = {
    "plan": ("tools.card_loop.workset", "cli_plan", "resolve inputs into a frozen run plan"),
    "candidates": ("tools.card_loop.candidates", "cli_candidates", "rank meta archetypes/decklists for the next run"),
    "preflight": ("tools.card_loop.preflight", "cli_preflight", "new-set / node readiness checks"),
    "run": ("tools.card_loop.driver", "cli_run", "execute a planned run"),
    "resume": ("tools.card_loop.driver", "cli_resume", "continue an interrupted run"),
    "status": ("tools.card_loop.driver", "cli_status", "summarise a run's items"),
    "report": ("tools.card_loop.scorecard", "cli_report", "reports (e.g. `report models`)"),
    "audit": ("tools.card_loop.audit", "cli_audit", "human audit sample: next / record"),
    "interactions": ("tools.card_loop.interactions.denominator", "cli",
                     "build or --check the gating interaction denominator"),
}

NOT_IMPLEMENTED = 3


def _usage() -> str:
    width = max(len(c) for c in COMMANDS)
    lines = ["usage: python -m tools.card_loop <command> [args]", "", "commands:"]
    lines += [f"  {name.ljust(width)}  {help_}" for name, (_, _, help_) in COMMANDS.items()]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else list(argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(_usage())
        return 0 if argv else 2
    command = argv[0]
    if command not in COMMANDS:
        print(f"unknown command {command!r}\n\n{_usage()}", file=sys.stderr)
        return 2
    module_name, func_name, _ = COMMANDS[command]
    try:
        module = importlib.import_module(module_name)
    except ModuleNotFoundError as e:
        if e.name and module_name.startswith(e.name):
            print(f"`{command}` is not implemented yet ({module_name})", file=sys.stderr)
            return NOT_IMPLEMENTED
        raise
    func = getattr(module, func_name, None)
    if func is None:
        print(f"`{command}` is not implemented yet ({module_name}.{func_name})", file=sys.stderr)
        return NOT_IMPLEMENTED
    return int(func(argv[1:]) or 0)


if __name__ == "__main__":
    sys.exit(main())
