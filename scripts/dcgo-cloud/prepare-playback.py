#!/usr/bin/env python3
"""Prepare isolated host/join clients for an existing native scripted recording.

Stop the source Wine prefix before invoking. The destination must not exist.
Large binaries stay outside the repository. This prepares files; it does not
start clients, connect to Photon, or establish an Oracle verdict.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import shutil
import subprocess
import sys


def wine_path(path):
    return "Z:" + str(path.resolve()).replace("/", "\\")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("recording", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--source", type=Path, default=Path("/workspace/dcgo-cloud"))
    parser.add_argument("--wss", action="store_true", help="Opt into Photon secure WebSockets")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    source, dest = args.source.resolve(), args.destination.resolve()
    recording = args.recording.resolve()
    document = json.loads(recording.read_text())
    initial = document.get("recording", document)["initial_state"]
    cards = json.loads((repo / "data/cards.json").read_text())
    decks = []
    for name in ("player1", "player2"):
        side = initial[name]
        main_cards = side["initial_hand"] + side["security_order"] + side["library_order"]
        eggs = side.get("digitama_library_order") or []
        if len(main_cards) != 50 or len(eggs) > 5:
            parser.error(f"{name}: expected 50 main cards and at most 5 eggs")
        counts = Counter(main_cards + eggs)
        for card in counts:
            if card not in cards:
                parser.error(f"unknown card {card}")
        decks.append((main_cards, counts))
    for path in (source / "client/DCGO_Standalone/Game/Mods/DcgoRecorder.dll", source / "prefix3", source / "runtime"):
        if not path.exists():
            parser.error(f"prepare the base runtime first: missing {path}")
    if dest.exists():
        parser.error(f"destination already exists; preserve its evidence and choose a new path: {dest}")
    (dest / "shared").mkdir(parents=True)
    (dest / "evidence").mkdir()
    subprocess.run([sys.executable, str(repo / "code/dcgo-melon-recorder/make_playback_plan.py"),
                    str(recording), str(dest / "shared/plan.txt")], check=True)
    for seat, role in enumerate(("host", "join")):
        root = dest / role
        shutil.copytree(source / "client", root / "client")
        shutil.copytree(source / "prefix3", root / "prefix3", symlinks=True)
        (root / "runtime").symlink_to(source / "runtime", target_is_directory=True)
        game = root / "client/DCGO_Standalone/Game"
        config = game / "UserData/DcgoRecorder"
        # Only the freshly copied sandbox is reset. Preserve the source runtime.
        if config.exists():
            shutil.rmtree(config)
        config.mkdir(parents=True)
        agent = dest / f"agent{seat}"
        agent.mkdir()
        (config / "auto.txt").write_text(role + "\n" + wine_path(dest / "shared") + "\n")
        (config / "agent.txt").write_text(wine_path(agent) + "\n")
        (config / "playback.txt").write_text(wine_path(dest / "shared/plan.txt") + "\n")
        if args.wss:
            (config / "photon.txt").write_text("wss\nus\n")
        deck_dir = root / "client/DCGO_Standalone/Assets/Decks"
        if deck_dir.exists():
            shutil.rmtree(deck_dir)
        deck_dir.mkdir(parents=True)
        main_cards, counts = decks[seat]
        lines = [f"Name: Script seat {seat}", f"Key Card: {cards[main_cards[0]]['index']}",
                 "Sort Index: 0", "", "// DeckList", ""]
        lines += [f"{count} {cards[card]['card_name_eng']}   {card}" for card, count in counts.items()]
        # DCGO silently ignores saved deck filenames without a name_ID separator.
        (deck_dir / f"Script_{seat}.txt").write_text("\n".join(lines) + "\n")
    (dest / "recording-path.txt").write_text(str(recording) + "\n")
    print(f"Prepared {dest}; start one dcgo-agent --script process per agent directory, then host and join.")


if __name__ == "__main__":
    main()
