"""Publish a completed transition assessment from a read-only evidence mount."""
import argparse
import json
from pathlib import Path

from tools.selfplay_scheduler import atomic_json, controller_lock, sha256


def read(path):
    return json.loads(path.read_text(encoding='utf-8'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assessment', type=Path, required=True)
    parser.add_argument('--state-dir', type=Path, default=Path('/monitor'))
    parser.add_argument('--run-id', required=True)
    parser.add_argument('--entity', required=True)
    parser.add_argument('--project', required=True)
    args = parser.parse_args()
    root = args.assessment.resolve()
    state_dir = args.state_dir.resolve()
    if state_dir == root or state_dir.is_relative_to(root) or root.is_relative_to(state_dir):
        raise ValueError('Publisher state must be separate from assessment evidence')
    result, plan = read(root / 'result.json'), read(root / 'plan.json')
    if result.get('status') != 'complete' or result.get('plan_sha256') != sha256(root / 'plan.json'):
        raise ValueError('Only a complete, plan-bound assessment can be published')
    cohort = (root / 'cohort.json').resolve()
    if not cohort.is_relative_to(root) or sha256(cohort) != plan['files'].get('cohort.json'):
        raise ValueError('Assessment cohort is missing or changed')
    reports = []
    for index in (3, 4):
        entry = result['reports'][str(index)]
        path = (root / entry['path']).resolve()
        if not path.is_relative_to(root) or sha256(path) != entry['sha256']:
            raise ValueError('Assessment report is missing or changed')
        reports.append((index, path))
    args.state_dir.mkdir(parents=True, exist_ok=True)
    with controller_lock(args.state_dir / 'publish.lock'):
        state_path = args.state_dir / 'publisher.json'
        binding = dict(run_id=args.run_id, entity=args.entity, project=args.project,
                       result_sha256=sha256(root / 'result.json'))
        state = read(state_path) if state_path.exists() else dict(binding=binding, reports={}, comparison=False)
        if state['binding'] != binding:
            raise ValueError('Publisher state belongs to a different run or assessment')
        import wandb
        from digimon_gym.agents.card_eval import publish_report
        config = {key: plan[key] for key in ('image', 'protocol', 'source_engine_image', 'seed',
            'games_per_archetype', 'games_per_checkpoint', 'anchor_model_hashes', 'selection_policy')}
        config.update(automatic_promotion=False, comparison='checkpoint 4 minus checkpoint 3; same repaired engine')
        with wandb.init(entity=args.entity, project=args.project, id=args.run_id, resume='allow',
                        name=args.run_id, job_type='checkpoint-transition', dir=str(args.state_dir),
                        config=config, settings=wandb.Settings(disable_code=True, disable_git=True,
                                                              x_disable_stats=True)) as run:
            atomic_json(args.state_dir / 'wandb_run.json', dict(url=run.url, id=run.id,
                entity=args.entity, project=args.project))
            for step, (index, path) in enumerate(reports):
                saved = state['reports'].setdefault(str(index), {})
                if saved.get('complete'):
                    continue
                def artifact_logged():
                    saved['artifact'] = True
                    atomic_json(state_path, state)
                if not publish_report(read(path), path, run, step=step, wait=True,
                        skip_artifact=saved.get('artifact', False), on_artifact_logged=artifact_logged):
                    raise RuntimeError('Card report upload failed; publisher state retained for retry')
                saved['complete'] = True
                atomic_json(state_path, state)
            if not state['comparison']:
                artifact = wandb.Artifact(f'checkpoint-transition-{args.run_id}', type='checkpoint-comparison',
                    metadata=dict(image=plan['image'], automatic_promotion=False))
                for name in ('plan.json', 'result.json', 'cohort.json'):
                    artifact.add_file(str(root / name), name=name)
                run.log_artifact(artifact, aliases=['complete']).wait()
                state['comparison'] = True
                atomic_json(state_path, state)
            for anchor, row in result['comparison']['by_anchor'].items():
                run.summary[f'paired/{anchor}/difference'] = row['difference']
                run.summary[f'paired/{anchor}/ci95_low'] = row['ci95'][0]
                run.summary[f'paired/{anchor}/ci95_high'] = row['ci95'][1]
            run.summary['selected_checkpoint'] = result['selection']['checkpoint_index']
            run.summary['selected_checkpoint_sha256'] = result['selection']['checkpoint_sha256']
            run.summary['automatic_promotion'] = False
            run.summary['card_reports_uploaded'] = len(state['reports'])
            print(json.dumps(dict(status='published', url=run.url)), flush=True)


if __name__ == '__main__':
    main()
