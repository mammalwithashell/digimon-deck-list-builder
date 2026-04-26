"""Prompt builders for role-based AI review agents."""

from __future__ import annotations

from textwrap import dedent
from typing import Iterable


def _join_context_chunks(chunks: Iterable[dict]) -> str:
    lines = []
    for idx, chunk in enumerate(chunks, start=1):
        source = chunk.get("source", "unknown")
        text = chunk.get("text", "")
        lines.append(f"[Context {idx}] Source: {source}\n{text}")
    return "\n\n".join(lines) if lines else "No additional context retrieved."


def build_script_fidelity_messages(
    *,
    card_id: str,
    card_text: str,
    generated_script: str,
    context_chunks: list[dict],
) -> tuple[str, str]:
    system = dedent(
        """
        You are the Script Fidelity Agent for a Digimon TCG engine.
        Evaluate whether the script implementation is faithful to card text and
        whether mechanics are engine-supported.
        Do not invent unsupported mechanics. Be concrete and concise.
        """
    ).strip()

    user = dedent(
        f"""
        Card ID: {card_id}

        Official card text:
        {card_text}

        Generated script:
        {generated_script}

        Retrieved rules/engine context:
        {_join_context_chunks(context_chunks)}
        """
    ).strip()
    return system, user


def _join_pinned_engine_chunks(chunks: list[dict]) -> str:
    """Format pinned engine API method chunks for insertion into prompts."""
    lines = []
    for idx, chunk in enumerate(chunks, start=1):
        fn_name = chunk.get("function_name", "unknown")
        text = chunk.get("text", "")
        lines.append(f"[Method {idx}] {fn_name}\n{text}")
    return "\n\n".join(lines)


def build_script_autofix_messages(
    *,
    card_id: str,
    issue_description: str,
    scope_profile: str,
    allowed_paths: list[str],
    file_contexts: list[dict[str, str]],
    context_chunks: list[dict],
    pinned_engine_chunks: list[dict] | None = None,
) -> tuple[str, str]:
    system = dedent(
        """
        You are the Digimon AI Autofix agent.
        Produce only safe, minimal Python edits that address the issue.
        Rules:
        - Edit only the explicitly allowed paths.
        - Return complete new file contents for each edited file.
        - Keep deterministic, testable behavior; do not add placeholders.
        - If no safe fix exists, return an empty edits list with an explanation in summary.
        """
    ).strip()

    file_blocks = []
    for idx, file_ctx in enumerate(file_contexts, start=1):
        truncated = file_ctx.get("content_truncated", "false")
        file_blocks.append(
            dedent(
                f"""
                [File {idx}] path={file_ctx.get("path", "")}
                sha256={file_ctx.get("hash", "")}
                truncated={truncated}
                ----
                {file_ctx.get("content", "")}
                """
            ).strip()
        )
    files_text = "\n\n".join(file_blocks) if file_blocks else "No file context available."
    allowed_text = "\n".join(f"- {path}" for path in allowed_paths) if allowed_paths else "- (none)"

    pinned_section = ""
    if pinned_engine_chunks:
        pinned_text = _join_pinned_engine_chunks(pinned_engine_chunks)
        pinned_section = f"\n\nEngine API methods used by this script:\n{pinned_text}\n"

    user = dedent(
        f"""
        Card ID: {card_id}
        Scope profile: {scope_profile}

        Issue description:
        {issue_description}

        Allowed edit paths:
        {allowed_text}

        Current file context:
        {files_text}
        {pinned_section}
        Retrieved rules/engine context:
        {_join_context_chunks(context_chunks)}
        """
    ).strip()
    return system, user


def build_engine_capability_messages(
    *,
    mechanic: str,
    context_chunks: list[dict],
) -> tuple[str, str]:
    system = (
        "You are the Engine Capability Agent. Determine if a mechanic is supported by the engine."
    )
    user = dedent(
        f"""
        Mechanic to evaluate:
        {mechanic}

        Retrieved context:
        {_join_context_chunks(context_chunks)}
        """
    ).strip()
    return system, user


def build_qa_triage_messages(
    *,
    report_text: str,
    card_text: str,
    script_text: str,
    engine_version: str,
    context_chunks: list[dict],
) -> tuple[str, str]:
    system = (
        "You are the QA Triage Agent. Classify likely root cause and debugging direction."
    )
    user = dedent(
        f"""
        Bug report:
        {report_text}

        Card text:
        {card_text}

        Script:
        {script_text}

        Engine version:
        {engine_version}

        Retrieved context:
        {_join_context_chunks(context_chunks)}
        """
    ).strip()
    return system, user


def build_llm_transpile_messages(
    *,
    card_id: str,
    card_text: str,
    cs_source: str,
    regex_output: str,
    regex_score: float,
    context_chunks: list[dict],
    pinned_engine_chunks: list[dict] | None,
    few_shot_examples: list[dict],
) -> tuple[str, str]:
    """Build system + user prompts for LLM retranspilation."""
    context_block = _join_context_chunks(context_chunks)
    pinned_block = _join_pinned_engine_chunks(pinned_engine_chunks) if pinned_engine_chunks else ""

    examples_block = ""
    if few_shot_examples:
        parts = []
        for ex in few_shot_examples:
            parts.append(f"### {ex['card_id']}\n```python\n{ex['script']}\n```")
        examples_block = "## Working Examples from This Set\n\n" + "\n\n".join(parts)

    system = (
        "You are a Digimon TCG card script transpiler. You convert C# card effect "
        "classes from DCGO into Python CardScript subclasses for a headless game engine.\n\n"
        "You will receive:\n"
        "1. The original C# source file\n"
        "2. The official card text\n"
        "3. A partial Python output from a regex-based transpiler (with a confidence score)\n"
        "4. Engine API documentation for available methods\n"
        "5. Working examples of similar cards from the same set\n\n"
        "Your job: produce a COMPLETE, CORRECT Python CardScript module that faithfully "
        "implements the card's effects using the available engine API. Build on the regex "
        "output where it is correct. Fix or rewrite parts that are wrong or missing.\n\n"
        "Rules:\n"
        "- Output the complete Python module (imports through class definition)\n"
        "- Use only engine methods shown in the API reference\n"
        "- If an effect requires an engine mechanic that doesn't exist, list it in engine_gaps and skip that effect\n"
        "- Follow the exact CardScript structure shown in the working examples\n"
        "- Do not invent engine methods\n"
    )

    user = (
        f"## Card: {card_id}\n\n"
        f"**Official card text:**\n{card_text}\n\n"
        f"## C# Source\n```csharp\n{cs_source}\n```\n\n"
        f"## Regex Transpiler Output (score: {regex_score})\n```python\n{regex_output}\n```\n\n"
    )
    if examples_block:
        user += f"{examples_block}\n\n"
    if pinned_block:
        user += f"## Engine API Reference\n{pinned_block}\n\n"
    if context_block:
        user += f"## Rules & Context\n{context_block}\n\n"
    user += (
        "Produce the complete corrected Python module. List any effects you "
        "skipped and any engine gaps you identified."
    )

    return system, user


def build_transpiler_learn_messages(
    *,
    cluster: dict,
    extractors_source: str,
    generators_source: str,
    patterns_source: str,
    cs_examples: list[dict],
) -> tuple[str, str]:
    """Build prompts for transpiler pattern learning."""
    import json as _json

    system = (
        "You are a Python developer improving a regex-based C# to Python transpiler. "
        "You will receive a cluster of similar autofix diffs — changes that were needed "
        "to fix transpiled card scripts. Your job is to propose changes to the transpiler "
        "source code (extractors.py, generators.py, or patterns.py) that would produce "
        "the correct output in the first place, eliminating the need for these fixes.\n\n"
        "Rules:\n"
        "- Propose minimal, targeted changes\n"
        "- Show exact before/after snippets that can be applied to the transpiler\n"
        "- Each patch should reference actual code from the transpiler source\n"
        "- Estimate how many cards would benefit\n"
    )

    examples_block = ""
    if cs_examples:
        parts = [f"### {ex['card_id']}\n```csharp\n{ex['cs_source']}\n```" for ex in cs_examples]
        examples_block = "## Representative C# Sources\n\n" + "\n\n".join(parts)

    diffs_block = _json.dumps(cluster.get("representative_diffs", []), indent=2)

    user = (
        f"## Diff Cluster\n\n"
        f"**Type:** {cluster.get('change_type', 'unknown')}\n"
        f"**Count:** {cluster.get('count', 0)} cards affected\n"
        f"**Description:** {cluster.get('description', '')}\n\n"
        f"### Representative Diffs\n```json\n{diffs_block}\n```\n\n"
        f"{examples_block}\n\n"
        f"## Transpiler Source: extractors.py\n```python\n{extractors_source}\n```\n\n"
        f"## Transpiler Source: generators.py\n```python\n{generators_source}\n```\n\n"
        f"## Transpiler Source: patterns.py\n```python\n{patterns_source}\n```\n\n"
        f"Propose transpiler changes to handle this pattern automatically."
    )

    return system, user
