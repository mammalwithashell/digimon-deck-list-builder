"""Add legal-Option observations to a separate DCGO project; dry-run by default.

This changes recorder instrumentation only. Build and launch the isolated player
separately. The reference project is read-only, and unknown source shapes fail
before any file is written. A receipt and exact backups accompany application.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile


RECORDER_OLD = """            // state-the-decision-was-made-in reason as LogSelectionRow.
            Digimon.Harness.StateDumper.Dump();"""
RECORDER_NEW = RECORDER_OLD.replace("Dump();", "Dump(actor, source);")

DUMPER_REPLACEMENTS = (
    ("public static void Dump()", "public static void Dump(int decisionActor = -1, string decisionSource = null)"),
    ('AppendPlayer(sb, "p0", PlayerOf(ctx, 0));', 'AppendPlayer(sb, "p0", PlayerOf(ctx, 0), decisionActor, decisionSource);'),
    ('AppendPlayer(sb, "p1", PlayerOf(ctx, 1));', 'AppendPlayer(sb, "p1", PlayerOf(ctx, 1), decisionActor, decisionSource);'),
    ('private static void AppendPlayer(StringBuilder sb, string key, Player p)',
     'private static void AppendPlayer(StringBuilder sb, string key, Player p, int decisionActor, string decisionSource)'),
    ('            sb.Append("]}");', '''            sb.Append(']');
            // Measure only the actual deciding Main seat. Selection callbacks
            // can retain Main as their phase, so phase alone is insufficient.
            // Missing context omits the field; it never means an empty set.
            if (decisionSource == "main_phase" && p != null && p.PlayerID == decisionActor)
            {
                var legalOptions = new List<string>();
                foreach (var card in p.HandCards)
                    if (card.IsOption && card.CanPlayFromHandDuringMainPhase)
                        legalOptions.Add(CardIdOf(card));
                // Preserve physical duplicate counts while normalizing order.
                legalOptions.Sort(System.StringComparer.Ordinal);
                sb.Append(',');
                AppendStrings(sb, "legal_option_plays", legalOptions);
            }
            sb.Append('}');'''),
    ('''        /// Takes no arguments on purpose. Every call site is inside
        /// <c>GameRecorder</c>, which holds no <c>GameContext</c>; resolving''',
     '''        /// Decision context is supplied only for recorded action boundaries.
        /// Every call site is inside <c>GameRecorder</c>; resolving'''),
)


def replace_once(source: str, old: str, new: str) -> str:
    if source.count(old) != 1:
        raise ValueError(f"expected exactly one source anchor: {old[:100]!r}")
    return source.replace(old, new, 1)


def patch_recorder(source: str) -> str:
    return replace_once(source, RECORDER_OLD, RECORDER_NEW)


def patch_dumper(source: str) -> str:
    for old, new in DUMPER_REPLACEMENTS:
        source = replace_once(source, old, new)
    return source


def validate_project(project: Path, reference: Path) -> None:
    project, reference = project.resolve(strict=True), reference.resolve(strict=True)
    if project.is_relative_to(reference) or reference.is_relative_to(project):
        raise ValueError("project must be separate from the read-only reference")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--receipt-dir", type=Path, help="new directory for receipt and original files")
    args = parser.parse_args()
    validate_project(args.project, args.reference)
    project, reference = args.project.resolve(), args.reference.resolve()
    planned = []
    for relative, patcher in (
        ("Assets/Scripts/Script/Harness/StateDumper.cs", patch_dumper),
        ("Assets/Scripts/Script/Recording/GameRecorder.cs", patch_recorder),
    ):
        path, original = (project / relative).resolve(strict=True), (reference / relative).resolve(strict=True)
        if not path.is_relative_to(project) or path.samefile(original) or path.stat().st_nlink > 1:
            raise ValueError(f"target is not an independent project file: {path}")
        before = path.read_bytes()
        text = before.decode("utf-8-sig").replace("\r\n", "\n")
        updated = patcher(text)
        newline = "\r\n" if b"\r\n" in before else "\n"
        after = updated.replace("\n", newline).encode("utf-8")
        if before.startswith(b"\xef\xbb\xbf"):
            after = b"\xef\xbb\xbf" + after
        planned.append((path, before, after, {"path":relative, "before_sha256":digest(before),
                        "after_sha256":digest(after), "reference_sha256":digest(original.read_bytes())}))
    receipt = {"project":str(project), "reference":str(reference), "applied":False,
               "files":[row for _, _, _, row in planned]}
    if args.apply:
        if args.receipt_dir is None:
            parser.error("--apply requires a new --receipt-dir")
        args.receipt_dir.mkdir(parents=True, exist_ok=False)
        for path, before, _, row in planned:
            (args.receipt_dir / path.name).write_bytes(before)
            if path.read_bytes() != before:
                raise ValueError(f"source changed before application: {path}")
        # Write via independent temporary files, preserving reversible backups.
        for path, _, after, _ in planned:
            with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as out:
                out.write(after)
                temporary = Path(out.name)
            try:
                os.replace(temporary, path)
            finally:
                temporary.unlink(missing_ok=True)
        receipt["applied"] = True
        (args.receipt_dir / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
