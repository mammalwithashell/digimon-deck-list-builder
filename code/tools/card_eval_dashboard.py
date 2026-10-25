"""Run: streamlit run code/tools/card_eval_dashboard.py

Reads local reports or W&B checkpoint artifact versions. Credentials stay in
the local W&B SDK login/environment; this app never embeds keys in a URL.
"""
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from digimon_gym.agents.card_eval import load_local_reports, load_wandb_reports


def main():
    import pandas as pd
    import streamlit as st

    st.set_page_config(page_title="Pilot evaluation", layout="wide")
    st.title("Pilot evaluation")
    st.caption("Compare checkpoint card usage, entry routes and legal opportunities. Player 1 is the model.")
    source = st.sidebar.radio("Data source", ["Local reports", "Weights & Biases"])
    location = st.sidebar.text_input(
        "Report directory" if source == "Local reports" else "W&B run (entity/project/run_id)",
        value="runs/pilot_ppo/eval_cards" if source == "Local reports" else "",
        key=source,
    )
    if st.sidebar.button("Load / refresh", type="primary"):
        try:
            if source == "Weights & Biases" and len(location.strip("/").split("/")) != 3:
                raise ValueError("Enter entity/project/run_id.")
            st.session_state["reports"] = (
                load_local_reports(location) if source == "Local reports" else load_wandb_reports(location))
            st.session_state["loaded_source"] = (source, location)
        except Exception as exc:
            st.session_state.pop("reports", None)
            st.error(f"Could not load reports: {exc}")
    reports = st.session_state.get("reports", [])
    if not reports:
        st.info("Load an evaluation report directory or a W&B run with card-evaluation artifacts.")
        return
    st.caption(f"Loaded from: {st.session_state['loaded_source'][1]}")
    for report in reports:
        if report.get("schema_version") != 1:
            st.error("Unsupported report schema; update the dashboard.")
            return
    cards, routes = [], []
    for report in reports:
        identity = {k: report[k] for k in ("checkpoint_step", "eval_window", "window_id")}
        provenance = report.get("provenance", {})
        identity.update(candidate_sha256=provenance.get("candidate_sha256", ""),
                        evaluation_status=provenance.get("status", "legacy"))
        cards.extend(dict(**identity, **r) for r in report["cards"])
        routes.extend(dict(**identity, **r) for r in report["routes"])
    if not cards:
        st.info("The loaded reports contain no card samples.")
        return
    frame = pd.DataFrame(cards)
    route_frame = pd.DataFrame(routes)
    for col, label in (("panel", "Evaluation panel"), ("anchor", "Anchor policy"),
                       ("evaluation_mode", "Evaluation mode"), ("candidate_sha256", "Checkpoint hash"),
                       ("evaluation_status", "Evaluation status")):
        if col not in frame:
            continue
        frame[col] = frame[col].fillna("")
        options = sorted(frame[col].unique())
        chosen = st.sidebar.multiselect(label, options, default=options)
        frame = frame[frame[col].isin(chosen)]
        if not route_frame.empty and col in route_frame:
            route_frame[col] = route_frame[col].fillna("")
            route_frame = route_frame[route_frame[col].isin(chosen)]
    windows = list(dict.fromkeys(frame.window_id))
    # Periodic frozen-policy evaluations repeat the same seeded games. Never
    # pool those repetitions as independent evidence by default.
    grouping = [c for c in ("checkpoint_step", "panel", "anchor", "evaluation_mode") if c in frame]
    latest_per_step = list(dict.fromkeys(frame.drop_duplicates(grouping, keep="last").window_id))
    selected_windows = st.sidebar.multiselect("Evaluation windows", windows, default=latest_per_step)
    frame = frame[frame.window_id.isin(selected_windows)]
    if not route_frame.empty:
        route_frame = route_frame[route_frame.window_id.isin(selected_windows)]
    player = st.sidebar.selectbox("Player", [1, 2])
    frame = frame[frame.player == player]
    if not route_frame.empty:
        route_frame = route_frame[route_frame.player == player]
    for col, label in (("agent_archetype", "Model deck"), ("opponent_archetype", "Opponent")):
        options = sorted(frame[col].unique())
        chosen = st.sidebar.multiselect(label, options, default=options)
        frame = frame[frame[col].isin(chosen)]
        if not route_frame.empty:
            route_frame = route_frame[route_frame[col].isin(chosen)]
    card_options = sorted(frame.card_id.unique())
    selected_cards = st.sidebar.multiselect("Cards", card_options, default=card_options)
    frame = frame[frame.card_id.isin(selected_cards)]
    if not route_frame.empty:
        route_frame = route_frame[route_frame.card_id.isin(selected_cards)]
    if frame.empty:
        st.info("No samples match these filters.")
        return
    steps = sorted(frame.checkpoint_step.unique())
    selected_steps = st.multiselect("Checkpoints", steps, default=steps[-2:])
    frame = frame[frame.checkpoint_step.isin(selected_steps)].copy()
    if not route_frame.empty:
        route_frame = route_frame[route_frame.checkpoint_step.isin(selected_steps)].copy()
    if frame.empty:
        st.info("Select a checkpoint.")
        return
    st.subheader("Per-card comparison")
    st.caption("Games counts games containing this card in the deck, plus games where it was generated or observed. "
               "Seen in hand is a decision-boundary observation, not draw count. Win-when-entered is correlation.")
    frame["entry_rate"] = frame.games_entered / frame.games
    frame["win_rate_when_entered"] = frame.wins_when_entered / frame.completed_games_entered.replace(0, float("nan"))
    frame["mean_first_entry_turn"] = frame.first_entry_turn_sum / frame.games_entered.replace(0, float("nan"))
    st.dataframe(frame, hide_index=True)
    st.download_button("Download card CSV", frame.to_csv(index=False), "card_evaluation.csv", "text/csv")
    st.subheader("Resolved entry routes")
    entry_columns = ["hatches", "plays_direct", "plays_effect", "digivolves_raising", "promotions", "digivolves_battle", "dna_digivolves"]
    st.bar_chart(frame.groupby("checkpoint_step")[entry_columns].sum())
    st.caption("Promotion counts the stack's top card. DNA is a subset of digivolutions; "
               "a card can evolve in raising and later be promoted. Entry columns are not mutually exclusive.")
    st.subheader("Opportunities and choices")
    st.caption("One opportunity per card and route at each legal decision, even with multiple copies or targets. "
               "Repeated decisions count again. Pending-selection prompts are excluded. Choices are attempts; entries are resolved events.")
    if route_frame.empty:
        st.info("No tracked legal opportunities in this selection.")
    else:
        route_frame["choice_rate"] = route_frame.choices / route_frame.opportunities
        st.dataframe(route_frame, hide_index=True)
        trend = route_frame.groupby(["checkpoint_step", "route"])[["choices", "opportunities"]].sum()
        trend["choice_rate"] = trend.choices / trend.opportunities
        st.line_chart(trend.choice_rate.unstack("route"))
        st.download_button("Download opportunity CSV", route_frame.to_csv(index=False), "card_opportunities.csv", "text/csv")
    with st.expander("Measurement coverage"):
        st.json(reports[-1]["coverage"])
        st.write("Windows remain separate by window_id, including resumed runs at the same checkpoint step. "
                 "Changing opponents or deck pools can change these metrics independently of model quality.")


if __name__ == "__main__":
    main()
