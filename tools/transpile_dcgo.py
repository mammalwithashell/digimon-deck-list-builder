#!/usr/bin/env python3
"""
Transpile DCGO C# card effect scripts into Python CardScript files.

Reads .cs files from the DCGO-Card-Scripts repo and generates
Python equivalents compatible with the digimon_gym engine.

Usage:
    python tools/transpile_dcgo.py <DCGO_DIR> <OUTPUT_DIR>
    python tools/transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT14 digimon_gym/engine/data/scripts/bt14
    python tools/transpile_dcgo.py /tmp/dcgo-scripts/CardEffect/BT24 digimon_gym/engine/data/scripts/bt24
"""
import os
import sys

# Ensure the tools directory is on the path so the transpiler package can be imported
sys.path.insert(0, os.path.dirname(__file__))

from transpiler.cli import main

if __name__ == "__main__":
    main()
