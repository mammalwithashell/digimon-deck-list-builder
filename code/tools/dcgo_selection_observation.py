"""Add captured selection decline capability to an option-instrumented DCGO copy.

Dry-run by default; no build or launch. The reference remains read-only. Exact
anchors, independent files, original-byte backups and a receipt guard application.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

SELECTOR_REPLACEMENTS = (
    ('    Func<bool> _canNoSelect = null;', '    Func<bool> _canNoSelect = null;\n    // Recorder capability belongs to this prompt instance, before any picks.\n    bool? _selectionCanDecline = null;'),
    ('    public virtual IEnumerator Activate()\n    {', '    public virtual IEnumerator Activate()\n    {\n        _selectionCanDecline = null;'),
    ('''        if (_maxCount == 0)
        {
            _canNoSelect = () => true;
        }

        if (_root == Root.Security)
        {
            SetIsSecurity();
        }

        if (active())
        {
            if (_isSecurity)''', '''        if (_maxCount == 0)
        {
            _canNoSelect = () => true;
        }

        if (_root == Root.Security)
        {
            SetIsSecurity();
        }

        if (active())
        {
            // Capture the live predicate before UI/AutoSelect mutates selections.
            // Missing or throwing predicates are unknown, never mandatory.
            try
            {
                if (_canNoSelect != null) _selectionCanDecline = _canNoSelect();
            }
            catch (System.Exception) { _selectionCanDecline = null; }
            if (_isSecurity)'''),
    ('                        mechanic: __mechanic, zone: __zone);', '                        mechanic: __mechanic, zone: __zone, selectionCanDecline: _selectionCanDecline);'),
    ('                        cardIds: __ids, indexes: Indicies, mechanic: __mechanic, zone: __zone);', '                        cardIds: __ids, indexes: Indicies, mechanic: __mechanic, zone: __zone, selectionCanDecline: _selectionCanDecline);'),
)
RECORDER_REPLACEMENTS = (
    ('                                    string zone = null)', '                                    string zone = null,\n                                    bool? selectionCanDecline = null)'),
    ('''            // it left behind -- and while _stepIndex still names THIS row.
            Digimon.Harness.StateDumper.Dump();''', '''            // it left behind -- and while _stepIndex still names THIS row.
            Digimon.Harness.StateDumper.Dump(selectionActor: actorPlayerId,
                selectionCanDecline: prompt == "SelectCardEffect" ? selectionCanDecline : null);'''),
)
DUMPER_REPLACEMENTS = (
    ('public static void Dump(int decisionActor = -1, string decisionSource = null)', 'public static void Dump(int decisionActor = -1, string decisionSource = null, int selectionActor = -1, bool? selectionCanDecline = null)'),
    ('AppendPlayer(sb, "p0", PlayerOf(ctx, 0), decisionActor, decisionSource);', 'AppendPlayer(sb, "p0", PlayerOf(ctx, 0), decisionActor, decisionSource, selectionActor, selectionCanDecline);'),
    ('AppendPlayer(sb, "p1", PlayerOf(ctx, 1), decisionActor, decisionSource);', 'AppendPlayer(sb, "p1", PlayerOf(ctx, 1), decisionActor, decisionSource, selectionActor, selectionCanDecline);'),
    ('private static void AppendPlayer(StringBuilder sb, string key, Player p, int decisionActor, string decisionSource)', 'private static void AppendPlayer(StringBuilder sb, string key, Player p, int decisionActor, string decisionSource, int selectionActor, bool? selectionCanDecline)'),
    ('''                AppendStrings(sb, "legal_option_plays", legalOptions);
            }
            sb.Append('}');''', '''                AppendStrings(sb, "legal_option_plays", legalOptions);
            }
            // Omission denotes unknown; false is an observed mandatory prompt.
            if (selectionCanDecline.HasValue && p != null && p.PlayerID == selectionActor)
            {
                sb.Append(',').Append("\\\"selection_can_decline\\\":")
                    .Append(selectionCanDecline.Value ? "true" : "false");
            }
            sb.Append('}');'''),
)


def patch(source, replacements):
    # Reject fully or partially installed patches; never duplicate instrumentation.
    for old, new in replacements:
        if new in source or source.count(old) != 1:
            raise ValueError(f"expected exactly one unpatched source anchor: {old[:100]!r}")
    for old, new in replacements:
        source = source.replace(old, new, 1)
    return source


def patch_selector(source):
    return patch(source, SELECTOR_REPLACEMENTS)


def patch_recorder(source):
    return patch(source, RECORDER_REPLACEMENTS)


def patch_dumper(source):
    return patch(source, DUMPER_REPLACEMENTS)


PATCHES = (
    ("Assets/Scripts/Script/SelectCardEffect.cs", patch_selector),
    ("Assets/Scripts/Script/Recording/GameRecorder.cs", patch_recorder),
    ("Assets/Scripts/Script/Harness/StateDumper.cs", patch_dumper),
)


def validate_project(project, reference):
    project, reference = project.resolve(strict=True), reference.resolve(strict=True)
    if project.is_relative_to(reference) or reference.is_relative_to(project):
        raise ValueError("project must be separate from the read-only reference")


def digest(data):
    return hashlib.sha256(data).hexdigest()


def plan(project, reference):
    validate_project(project, reference)
    project, reference = project.resolve(), reference.resolve()
    planned = []
    for relative, patcher in PATCHES:
        path = (project / relative).resolve(strict=True)
        original = (reference / relative).resolve(strict=True)
        if not path.is_relative_to(project) or path.samefile(original) or path.stat().st_nlink > 1:
            raise ValueError(f"target is not an independent project file: {path}")
        before = path.read_bytes()
        updated = patcher(before.decode("utf-8-sig").replace("\r\n", "\n"))
        newline = "\r\n" if b"\r\n" in before else "\n"
        after = updated.replace("\n", newline).encode("utf-8")
        if before.startswith(b"\xef\xbb\xbf"):
            after = b"\xef\xbb\xbf" + after
        planned.append((path, before, after, {"path": relative, "before_sha256": digest(before),
                        "after_sha256": digest(after), "reference_sha256": digest(original.read_bytes())}))
    return planned


def receipt_for(planned, project, reference):
    return {"project": str(project.resolve()), "reference": str(reference.resolve()),
            "applied": False, "files": [row for _, _, _, row in planned]}


def apply_plan(planned, receipt_dir, project, reference):
    validate_project(project, reference)
    receipt_dir = receipt_dir.resolve()
    if receipt_dir.is_relative_to(reference.resolve()):
        raise ValueError("receipt must be outside the read-only reference")
    # Revalidate all files before touching any: plans may outlive their sources.
    for path, before, _, row in planned:
        original = (reference / row["path"]).resolve(strict=True)
        if not path.resolve().is_relative_to(project.resolve()) or path.samefile(original) or path.stat().st_nlink > 1:
            raise ValueError(f"target is not an independent project file: {path}")
        if path.read_bytes() != before:
            raise ValueError(f"source changed before application: {path}")
    receipt_dir.mkdir(parents=True, exist_ok=False)
    receipt = receipt_for(planned, project, reference)
    for path, before, _, _ in planned:
        (receipt_dir / path.name).write_bytes(before)
    receipt_path = receipt_dir / "receipt.json"
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    for path, _, after, _ in planned:
        with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as out:
            out.write(after)
            temporary = Path(out.name)
        try:
            os.replace(temporary, path)
        finally:
            temporary.unlink(missing_ok=True)
    receipt["applied"] = True
    receipt_path.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--receipt-dir", type=Path)
    args = parser.parse_args()
    planned = plan(args.project, args.reference)
    if args.apply:
        if args.receipt_dir is None:
            parser.error("--apply requires a new --receipt-dir")
        receipt = apply_plan(planned, args.receipt_dir, args.project, args.reference)
    else:
        receipt = receipt_for(planned, args.project, args.reference)
    print(json.dumps(receipt, indent=2))


if __name__ == "__main__":
    main()
