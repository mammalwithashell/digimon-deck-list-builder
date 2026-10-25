"""Local training dashboard. Reads compact snapshots; never controls training."""
import os
from pathlib import Path
import sys
import time
from datetime import datetime
from zoneinfo import ZoneInfo

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from tools.cloud_search.dashboard_snapshot import latest_reports, read, search_progress


def stamp(value):
    return datetime.fromtimestamp(value, ZoneInfo("America/Chicago")).strftime("%b %d, %I:%M:%S %p CT") if value else "Unavailable"


def duration(seconds):
    if seconds is None:
        return "Not enough fresh data"
    minutes = max(0, round(seconds / 60))
    return f"{minutes // 60}h {minutes % 60:02d}m" if minutes >= 60 else f"{minutes} min"


def frame(reports, field):
    import pandas as pd
    return pd.DataFrame([{**row, "arm": report["arm"], "checkpoint": report["checkpoint"],
                          "panel": report["panel"], "window_id": report["window_id"]}
                         for report in reports for row in report.get(field, [])])


def overview(st, snapshot, reports, now):
    state = snapshot.get("state", {})
    stage = state.get("stage", "unknown").replace("_", " ").capitalize()
    st.subheader(f"{stage}" + (f" · {state['arm']}" if state.get("arm") else ""))
    if state.get("error"):
        st.error(state["error"])
    st.caption("Read-only view. This dashboard cannot start, stop, or change an experiment.")
    for column, arm in zip(st.columns(max(1, len(snapshot["arms"]))), snapshot["arms"]):
        with column:
            with st.container(border=True):
                st.subheader(arm["name"].capitalize())
                status = arm.get("state", {})
                active_stage = status.get("stage", "unknown").replace("_", " ")
                generations = arm.get("generations", [])
                saved = [g for g in generations if g["model_saved_at"]]
                st.metric("Latest model", f"Checkpoint {max(g['index'] for g in saved)}" if saved else "Not fitted")
                current = next((g for g in generations if g["index"] == status.get("generation_index", 0)), {})
                progress = current.get("progress", {})
                if progress:
                    p = search_progress(progress, now)
                    st.metric("Search games", f"{p['completed']:,} / {p['total']:,}")
                    st.progress(min(1., p["completed"] / max(1, p["total"])))
                    rate = p["games_per_hour"]
                    st.caption(f"{rate:,.0f} games/hour · {progress.get('workers', '?')} workers" if rate else "Measuring search throughput")
                    st.caption("Collection ETA: " + duration(p["eta_seconds"]))
                    st.caption(f"Truncated games: {progress.get('truncated_games', 0)}")
                    if p["stale"]:
                        st.warning("Search heartbeat is stale; ETA withheld.")
                index = status.get("generation_index", 0)
                selected = [r for r in reports if r["arm"] == arm["name"] and r["checkpoint"] == index]
                completed = sum(sum(x[k] for k in ("wins", "losses", "draws")) for r in selected for x in r["outcomes"])
                total = snapshot.get("plan", {}).get("games_per_evaluation", 0)
                live = [p for w in arm.get("windows", []) if w["name"] == f"window-{index:06d}"
                        for p in w.get("progress", []) if p.get("status") == "running"]
                for p in live:
                    if not any(r["panel"] == p.get("panel") for r in selected):
                        completed += p.get("games_completed", 0)
                if total and completed >= total and status.get("stage") == "checkpoint_evaluation":
                    active_stage = "evaluation complete"
                st.caption(active_stage)
                st.metric(f"Evaluated games · checkpoint {index}", f"{completed:,}" + (f" / {total:,}" if total else ""))
                if total:
                    st.progress(min(1., completed / total))
                if live:
                    p = live[-1]
                    age = now - p.get("heartbeat_unix", 0)
                    st.caption(f"{p.get('panel', '')} · {p.get('anchor', '')} · {p.get('archetype', '')}")
                    st.caption("Panel ETA: " + duration(p.get("eta_seconds") if age <= 30 else None))
                    if age > 30:
                        st.warning("Evaluation heartbeat is stale.")
                elif status.get("stage") == "checkpoint_evaluation" and completed < total:
                    st.caption("Completed-panel count only. This running process has no per-game progress feed.")
                uploaded = arm.get("publisher_at")
                st.caption("W&B last upload: " + stamp(uploaded))
                url = arm.get("wandb", {}).get("url", "")
                if url.startswith("https://"):
                    st.link_button("Open W&B run", url)
    st.caption("Checkpoint 0 is the fitted heuristic seed. Checkpoint 1 is the model after its search update. "
               "Search ETA covers game collection, not subsequent fitting and evaluation.")


def checkpoint_view(st, reports, comparisons):
    outcomes = frame(reports, "outcomes")
    if outcomes.empty:
        st.info("No completed evaluations yet.")
        return
    group = ["arm", "checkpoint", "panel", "anchor"]
    scores = outcomes.groupby(group)[["wins", "losses", "draws", "incomplete"]].sum().reset_index()
    scores["completed_games"] = scores.wins + scores.losses + scores.draws
    scores["win_percent"] = 100 * scores.wins / scores.completed_games.replace(0, float("nan"))
    st.subheader("Results against fixed opponents")
    st.caption("Every row uses game outcomes, not per-card denominators. Meta held-out lists and pet same-list evaluations stay separate.")
    st.dataframe(scores, hide_index=True, use_container_width=True)
    st.download_button("Download checkpoint CSV", scores.to_csv(index=False), "checkpoint_results.csv", "text/csv")
    st.subheader("Paired seed comparisons")
    rows = []
    for checkpoint, candidates in comparisons.items():
        for candidate, comparison in candidates.items():
            summary = comparison.get("aggregate", {})
            if summary:
                rows.append(dict(checkpoint=int(checkpoint), candidate=candidate, control="greedy",
                    score_difference_points=100*summary["difference"],
                    ci95_low_points=100*summary["ci95"][0], ci95_high_points=100*summary["ci95"][1],
                    paired_games=summary["paired_games"],
                    noninferior_at_five_points=summary["noninferior_at_five_points"]))
    if rows:
        st.dataframe(rows, hide_index=True, use_container_width=True)
        st.caption("Paired bootstrap intervals from the controller; draws score one half. These compare the full fixed cohort, independent of the filters below.")
    else:
        st.info("Paired intervals appear when all three seeds finish the same checkpoint evaluation.")
    with st.expander("Per-archetype win rates", expanded=True):
        outcomes["completed_games"] = outcomes.wins + outcomes.losses + outcomes.draws
        outcomes["win_percent"] = 100*outcomes.wins/outcomes.completed_games.replace(0, float("nan"))
        st.dataframe(outcomes.drop(columns=["window_id"]), hide_index=True, use_container_width=True)


def behavior_view(st, reports):
    cards, routes = frame(reports, "cards"), frame(reports, "routes")
    if cards.empty:
        st.info("No completed card telemetry yet.")
        return
    anchors = st.multiselect("Opponent policy", sorted(cards.anchor.unique()), default=sorted(cards.anchor.unique()))
    archetypes = st.multiselect("Model archetype", sorted(cards.agent_archetype.unique()), default=sorted(cards.agent_archetype.unique()))
    cards = cards[cards.anchor.isin(anchors) & cards.agent_archetype.isin(archetypes)]
    if not routes.empty:
        routes = routes[routes.anchor.isin(anchors) & routes.agent_archetype.isin(archetypes)]
    if cards.empty:
        st.info("No samples match these filters.")
        return
    st.subheader("Resolved entry routes")
    columns = ["hatches", "plays_direct", "plays_effect", "digivolves_raising", "promotions", "digivolves_battle", "dna_digivolves"]
    entries = cards.groupby(["arm", "checkpoint"])[columns].sum()
    entries.index = [f"{a} · checkpoint {c}" for a, c in entries.index]
    st.bar_chart(entries)
    st.caption("Counts of resolved events. DNA is a subset of digivolution; raising and later promotion both count. Compare equal game coverage, not raw totals alone.")
    st.subheader("Legal opportunities and choices")
    if not routes.empty:
        choice = routes.groupby(["arm", "checkpoint", "route"])[["opportunities", "choices"]].sum().reset_index()
        choice["choice_percent"] = 100*choice.choices/choice.opportunities.replace(0, float("nan"))
        st.dataframe(choice, hide_index=True, use_container_width=True)
    st.caption("Opportunities are card-route exposures at legal decision boundaries, not distinct games. Choices are attempts; entries are resolved effects. Omitted routes are unmeasured.")
    dna = frame(reports, "dna")
    if not dna.empty:
        dna = dna[dna.anchor.isin(anchors) & dna.archetype.isin(archetypes)]
        with st.expander("DNA-capable deck lists"):
            st.caption("Only exact deck lists containing printed DNA targets. Effect-only DNA eligibility is not inferred from archetype names.")
            st.dataframe(dna.drop(columns=["window_id", "targets"]), hide_index=True, use_container_width=True)
    st.subheader("Per-card detail")
    selection = st.multiselect("Card IDs (empty shows all)", sorted(cards.card_id.unique()))
    if selection:
        cards = cards[cards.card_id.isin(selection)]
    cards = cards.copy()
    cards["entry_percent"] = 100*cards.games_entered/cards.games.replace(0, float("nan"))
    cards["win_when_entered_percent"] = 100*cards.wins_when_entered/cards.completed_games_entered.replace(0, float("nan"))
    st.caption("Seen in hand is not draw count. Win-when-entered is correlation, not causal card strength. Player 1 is the evaluated model.")
    st.dataframe(cards.drop(columns=["player", "window_id"], errors="ignore"), hide_index=True, use_container_width=True)
    st.download_button("Download card detail CSV", cards.to_csv(index=False), "card_detail.csv", "text/csv")


def main():
    import streamlit as st
    st.set_page_config(page_title="Digimon training", page_icon="📊", layout="wide")
    st.title("Digimon training dashboard")
    st.caption("Progress, model comparisons, and card behavior — with the evidence behind each number.")
    cache = Path(st.sidebar.text_input("Snapshot folder", os.environ.get("DIGIMON_DASHBOARD_CACHE", "runs/training-dashboard")))
    st.sidebar.button("Refresh now")
    st.sidebar.caption("Refreshes every 30 seconds. Data is pulled separately; the browser never receives SSH or W&B credentials.")

    @st.fragment(run_every="30s")
    def body():
        snapshot = read(cache / "snapshot.json")
        sync = read(cache / "sync_status.json", {})
        if not snapshot:
            st.info("Waiting for a snapshot. Run tools.cloud_search.sync_dashboard with your experiment configuration.")
            return
        now = time.time()
        age = max(0, now - snapshot.get("collected_at", 0))
        st.caption(f"Last source check: {stamp(snapshot.get('collected_at'))} · {age:.0f}s ago")
        if age > 150 or sync.get("status") == "error":
            st.warning("Source data is stale or disconnected. Last known results remain visible; current activity is unconfirmed.")
        if sync.get("error"):
            st.error(sync["error"])
        for error in snapshot.get("errors", []):
            st.warning(f"Could not read {error['path']}: {error['error']}")
        reports = latest_reports(snapshot["reports"])
        overview_tab, results_tab, cards_tab, evidence_tab = st.tabs(["Overview", "Checkpoints", "Card behavior", "Evidence"])
        with overview_tab:
            overview(st, snapshot, reports, now)
        with results_tab:
            checkpoint_view(st, reports, snapshot.get("comparisons", {}))
        with cards_tab:
            arms = sorted({r["arm"] for r in reports})
            panels = sorted({r["panel"] for r in reports})
            selected_arms = st.multiselect("Seeds", arms, default=arms)
            selected_panels = st.multiselect("Evaluation panels", panels, default=panels[:1])
            checkpoints = sorted({r["checkpoint"] for r in reports})
            selected_steps = st.multiselect("Checkpoints", checkpoints, default=checkpoints)
            selected = [r for r in reports if r["arm"] in selected_arms and r["panel"] in selected_panels and r["checkpoint"] in selected_steps]
            behavior_view(st, selected)
        with evidence_tab:
            st.write("Completed windows are selected once per seed, checkpoint and panel. Failed/incomplete windows are excluded from comparisons. Resume archives are never pooled into current results.")
            st.dataframe([dict(arm=r["arm"], checkpoint=r["checkpoint"], panel=r["panel"], status=r["status"],
                window_id=r["window_id"], model=r["provenance"].get("candidate_sha256"),
                engine=r["provenance"].get("engine_image")) for r in snapshot["reports"]], hide_index=True)
            with st.expander("Experiment settings and current state"):
                st.json({"state": snapshot["state"], "plan": snapshot["plan"]})
            with st.expander("Measurement coverage"):
                for arm in snapshot["arms"]:
                    st.write(f"{arm['name']} DNA eligibility: {arm.get('dna_classification', 'unmeasured')}")
                for report in reports:
                    st.write(f"{report['arm']} · checkpoint {report['checkpoint']} · {report['panel']}")
                    st.json(report["coverage"])
    body()


if __name__ == "__main__":
    main()
