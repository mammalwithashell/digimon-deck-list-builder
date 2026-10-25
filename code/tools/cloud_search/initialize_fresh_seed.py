"""Create a new CPU bootstrap and frozen anchor, without reading old weights."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path('/experiment'))
    args = parser.parse_args()
    root = args.root
    inputs, work = root / 'inputs', root / 'bootstrap'
    if any(work.iterdir()) or (root / 'generation').exists():
        raise FileExistsError('Fresh bootstrap requires empty preparation directories')
    from digimon_gym.agents.gauntlet import require_mechanics_ready, validate_admissible_deck
    import digimon_engine
    from model_contract import current_contract, validate_checkpoint
    from tools.english_meta_search_job import collect_bootstrap, bootstrap
    from sb3_contrib import MaskablePPO
    require_mechanics_ready()
    registered = set(digimon_engine.load_implemented_card_ids())
    decks = json.loads((inputs / 'decks.json').read_text())
    for deck in decks:
        validate_admissible_deck(deck['cards'], label=deck['name'])
        if not set(deck['cards']) <= registered or not digimon_engine.validate_deck(deck['cards']).is_valid:
            raise ValueError(f"Invalid deck: {deck['name']}")
    contract = current_contract('standard_lite_v2')
    if contract['action_schema_version'] != 2:
        raise ValueError('Fresh campaign requires action schema 2')
    shutil.copy2(inputs / 'decks.json', work / 'decks.json')
    # Same fresh initialization recipe as the previous experiment; diagnostic
    # demonstrations remain separate from search replay and are never pooled.
    shards = collect_bootstrap(work, games=400, seed=50261025)
    manifest = json.loads((shards / 'manifest.json').read_text())
    if manifest['truncated_games']:
        raise RuntimeError('Inspect capped bootstrap games before any fitting')
    bootstrap(work, games=400, seed=50261025, device='cpu')
    validate_checkpoint(MaskablePPO.load(work / 'seed.zip', device='cpu'), 'standard_lite_v2')
    target = root / 'generation/gen0'
    target.mkdir(parents=True, exist_ok=False)
    for suffix in ('.zip', '.onnx', '.onnx.meta.json', '.calibration.json'):
        shutil.copy2(work / f'seed{suffix}', inputs / f'seed{suffix}')
        shutil.copy2(work / f'seed{suffix}', target / f'model{suffix}')
    anchors = json.loads((work / 'anchors.json').read_text())
    anchors['champions'][0]['weights_path'] = '/experiment/inputs/seed.zip'
    (inputs / 'anchors.json').write_text(json.dumps(anchors, indent=2))
    evidence = dict(initialization='fresh', inherited_weights=False, contract=contract,
        initialization_seed=50261025, bootstrap_games=400, bootstrap_truncated_games=0,
        bootstrap_rows=sum(s['rows'] for s in manifest['shards']),
        seed_sha256=hashlib.sha256((inputs / 'seed.zip').read_bytes()).hexdigest())
    (inputs / 'initialization.json').write_text(json.dumps(evidence, indent=2))
    print(json.dumps(evidence), flush=True)


if __name__ == '__main__':
    main()
