"""`python -m tools.card_loop.interactions ...` -- same CLI as `tools.card_loop interactions`."""
import sys

from tools.card_loop.interactions.denominator import cli

if __name__ == "__main__":
    sys.exit(cli(sys.argv[1:]))
