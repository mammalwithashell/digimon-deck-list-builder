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


def build_script_autofix_messages(
    *,
    card_id: str,
    issue_description: str,
    scope_profile: str,
    allowed_paths: list[str],
    file_contexts: list[dict[str, str]],
    context_chunks: list[dict],
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
        file_blocks.append(
            dedent(
                f"""
                [File {idx}] path={file_ctx.get("path", "")}
                sha256={file_ctx.get("hash", "")}
                ----
                {file_ctx.get("content", "")}
                """
            ).strip()
        )
    files_text = "\n\n".join(file_blocks) if file_blocks else "No file context available."
    allowed_text = "\n".join(f"- {path}" for path in allowed_paths) if allowed_paths else "- (none)"

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
