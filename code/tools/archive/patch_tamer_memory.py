"""Patch frozen tamer scripts to fix set_memory_3 and gain_memory_tamer factory effects.

These factory effects were generated as dead code (comment only, no timing, no callback).
This script patches them in-place with proper timing + condition + process callback.

Usage:
    python tools/patch_tamer_memory.py [--dry-run]
"""
import os
import re
import sys

SCRIPTS_DIR = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "digimon_gym", "engine", "data", "scripts"
)

# Already hand-fixed — skip these
SKIP_FILES = {"bt18_087.py", "bt23_090.py"}


def patch_set_memory_3(content: str) -> str:
    """Replace dead set_memory_3 blocks with working code."""
    # Pattern: match from "# Factory effect: set_memory_3" through "effects.append(effectN)"
    # The block looks like:
    #     # Factory effect: set_memory_3
    #     # ...description...
    #     effectN = ICardEffect()
    #     effectN.set_effect_name("...")
    #     effectN.set_effect_description("...")
    #     # [Start of Your Turn] Set memory to 3 if <= 2
    #
    #     def conditionN(context: ...):
    #         ...return True
    #     effectN.set_can_use_condition(conditionN)
    #     effects.append(effectN)
    pattern = re.compile(
        r'(        # Factory effect: set_memory_3\n'
        r'        # [^\n]+\n'                          # description line
        r'        (effect\d+) = ICardEffect\(\)\n'     # effectN = ...
        r'        \2\.set_effect_name\("[^"]+"\)\n'     # set_effect_name
        r'        \2\.set_effect_description\("[^"]+"\)\n'  # set_effect_description
        r'        # \[Start of Your Turn\][^\n]*\n'    # comment
        r'\n'                                           # blank line
        r'        def (condition\d+)\(context[^\n]+\n'  # def conditionN
        r'(?:            [^\n]*\n)*?'                   # condition body lines
        r'            return True\n'                    # return True
        r'        \2\.set_can_use_condition\(\3\)\n'    # set_can_use_condition
        r'        effects\.append\(\2\))'               # effects.append
    )

    def replacer(m):
        var = m.group(2)  # e.g. "effect0"
        idx = var.replace("effect", "")  # e.g. "0"
        card_id_match = re.search(r'set_effect_name\("([^"]+)"', m.group(0))
        card_id = card_id_match.group(1) if card_id_match else f"effect{idx}"
        # Extract just the card ID prefix (e.g. "BT10-088")
        name_parts = card_id.split(" ", 1)
        card_prefix = name_parts[0] if name_parts else card_id

        return (
            f"        # Factory effect: set_memory_3\n"
            f"        # [Start of Your Turn] Set memory to 3 if <= 2\n"
            f"        {var} = ICardEffect()\n"
            f"        {var}.set_timing(EffectTiming.OnStartMainPhase)\n"
            f'        {var}.set_effect_name("{card_prefix} Set memory to 3")\n'
            f'        {var}.set_effect_description("[Start of Your Turn] If your memory is at 2 or less, it becomes 3.")\n'
            f"\n"
            f"        def condition{idx}(context: Dict[str, Any]) -> bool:\n"
            f"            if card and card.permanent_of_this_card() is None:\n"
            f"                return False\n"
            f"            if not (card and card.owner and card.owner.is_my_turn):\n"
            f"                return False\n"
            f"            return True\n"
            f"        {var}.set_can_use_condition(condition{idx})\n"
            f"\n"
            f"        def process{idx}(ctx: Dict[str, Any]):\n"
            f'            """Action: Set memory to 3 if <= 2"""\n'
            f"            player = ctx.get('player')\n"
            f"            game = ctx.get('game')\n"
            f"            if player and game and game.memory <= 2:\n"
            f"                game.memory = 3\n"
            f"        {var}.set_on_process_callback(process{idx})\n"
            f"        effects.append({var})"
        )

    return pattern.sub(replacer, content)


def patch_gain_memory_tamer(content: str) -> str:
    """Replace dead gain_memory_tamer blocks with working code."""
    pattern = re.compile(
        r'(        # Factory effect: gain_memory_tamer\n'
        r'        # [^\n]+\n'                          # description line
        r'        (effect\d+) = ICardEffect\(\)\n'     # effectN = ...
        r'        \2\.set_effect_name\("[^"]+"\)\n'     # set_effect_name
        r'        \2\.set_effect_description\("[^"]+"\)\n'  # set_effect_description
        r'        # \[Start of Main\][^\n]*\n'         # comment
        r'\n'                                           # blank line
        r'        def (condition\d+)\(context[^\n]+\n'  # def conditionN
        r'(?:            [^\n]*\n)*?'                   # condition body lines
        r'            return True\n'                    # return True
        r'        \2\.set_can_use_condition\(\3\)\n'    # set_can_use_condition
        r'        effects\.append\(\2\))'               # effects.append
    )

    def replacer(m):
        var = m.group(2)
        idx = var.replace("effect", "")
        card_id_match = re.search(r'set_effect_name\("([^"]+)"', m.group(0))
        card_id = card_id_match.group(1) if card_id_match else f"effect{idx}"
        name_parts = card_id.split(" ", 1)
        card_prefix = name_parts[0] if name_parts else card_id

        return (
            f"        # Factory effect: gain_memory_tamer\n"
            f"        # [Start of Main] Gain 1 memory if opponent has Digimon\n"
            f"        {var} = ICardEffect()\n"
            f"        {var}.set_timing(EffectTiming.OnStartMainPhase)\n"
            f'        {var}.set_effect_name("{card_prefix} Gain 1 memory")\n'
            f'        {var}.set_effect_description("[Start of Your Main Phase] If your opponent has a Digimon in play, gain 1 memory.")\n'
            f"\n"
            f"        def condition{idx}(context: Dict[str, Any]) -> bool:\n"
            f"            if card and card.permanent_of_this_card() is None:\n"
            f"                return False\n"
            f"            if not (card and card.owner and card.owner.is_my_turn):\n"
            f"                return False\n"
            f"            return True\n"
            f"        {var}.set_can_use_condition(condition{idx})\n"
            f"\n"
            f"        def process{idx}(ctx: Dict[str, Any]):\n"
            f'            """Action: Gain 1 memory if opponent has Digimon"""\n'
            f"            player = ctx.get('player')\n"
            f"            game = ctx.get('game')\n"
            f"            if player and game:\n"
            f"                opponent = game.opponent_player\n"
            f"                if opponent and any(p.is_digimon for p in opponent.battle_area):\n"
            f"                    game.memory += 1\n"
            f"        {var}.set_on_process_callback(process{idx})\n"
            f"        effects.append({var})"
        )

    return pattern.sub(replacer, content)


def main():
    dry_run = "--dry-run" in sys.argv
    patched_mem3 = 0
    patched_gain = 0
    skipped = 0
    failed = []

    for root, dirs, files in os.walk(SCRIPTS_DIR):
        # Skip generated/ and __pycache__
        dirs[:] = [d for d in dirs if d not in ("generated", "__pycache__")]
        for fname in sorted(files):
            if not fname.endswith(".py"):
                continue
            if fname in SKIP_FILES:
                skipped += 1
                continue

            fpath = os.path.join(root, fname)
            with open(fpath, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()

            original = content
            has_mem3 = "# Factory effect: set_memory_3" in content
            has_gain = "# Factory effect: gain_memory_tamer" in content

            if not has_mem3 and not has_gain:
                continue

            if has_mem3:
                # Skip if THIS set_memory_3 block already has OnStartMainPhase
                # (already hand-fixed like bt18_087, bt23_090)
                lines_list = content.split("\n")
                in_mem3 = False
                already_fixed = False
                for line in lines_list:
                    if "# Factory effect: set_memory_3" in line:
                        in_mem3 = True
                    elif in_mem3 and "OnStartMainPhase" in line:
                        already_fixed = True
                        break
                    elif in_mem3 and ("# Factory effect:" in line or "effects.append" in line):
                        break
                if already_fixed:
                    skipped += 1
                    has_mem3 = False  # don't count as needing patch
                else:
                    content = patch_set_memory_3(content)
                if content != original:
                    patched_mem3 += 1

            if has_gain:
                prev = content
                content = patch_gain_memory_tamer(content)
                if content != prev:
                    patched_gain += 1

            if content != original:
                if dry_run:
                    rel = os.path.relpath(fpath, SCRIPTS_DIR)
                    print(f"  WOULD PATCH: {rel}")
                else:
                    with open(fpath, "w", encoding="utf-8", newline="\n") as f:
                        f.write(content)
                    rel = os.path.relpath(fpath, SCRIPTS_DIR)
                    print(f"  PATCHED: {rel}")
            elif has_mem3 or has_gain:
                rel = os.path.relpath(fpath, SCRIPTS_DIR)
                failed.append(rel)

    print(f"\nSummary:")
    print(f"  set_memory_3 patched: {patched_mem3}")
    print(f"  gain_memory_tamer patched: {patched_gain}")
    print(f"  Skipped (already fixed): {skipped}")
    if failed:
        print(f"  FAILED to match pattern ({len(failed)} files):")
        for f in failed:
            print(f"    {f}")


if __name__ == "__main__":
    main()
