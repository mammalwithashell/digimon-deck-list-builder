# Training Jobs

Each file in this directory defines a named training job. Run with:

```bash
python tools/run_training_job.py training_jobs/<job>.json
```

## Config Schema

```jsonc
{
  "job_name": "my_job",            // Used as model filename suffix
  "description": "...",            // Human-readable note

  "agent_deck": {
    // Option 1: load from file (JSON array or digimoncard.io text format)
    "source": "file",
    "path": "decks/my_deck.json"

    // Option 2: pick a specific deck from deck_library.json by ID
    // "source": "deck_id",
    // "deck_id": "digilab_a44a688681be"

    // Option 3: use the default ST1 starter deck
    // "source": "default"
  },

  "meta_scope": {
    // Filter DigiLab data to one or more local stores
    "store_ids": [6],              // Store IDs (see: python -c "from digimon_gym.digilab_client import list_stores; [print(s) for s in list_stores()]")
    "since_date": "2025-10-01",   // ISO date — only events on/after this date
    // "scene_id": 1,             // Alternative: filter by geographic scene
    // "event_type": "Weekly"     // Optional: filter by event type string
  },

  "training": {
    "timesteps": 500000,
    "opponent": "greedy",          // "greedy" | "random" | "self-play"
    "use_lstm": true,
    "lstm_hidden_size": 256,
    "learning_rate": 3e-4,
    "n_steps": 2048,
    "batch_size": 64,
    "eval_freq": 10000,
    "n_eval_episodes": 20,
    "bounty_threshold": 0.15,
    "bounty_bonus": 0.5
  },

  "output": {
    "name": "my_job_v1",           // Model saved as models/pilot_ppo_<name>.zip
    "save_dir": "models",
    "log_dir": "runs/pilot_ppo"
  }
}
```

## Omit meta_scope for global meta

If you leave out `meta_scope`, the job uses the full global `deck_library.json`
weighted by the standard Threat Index (DigiLab meta_share + conversion_rate).

## Finding store IDs

```bash
python -c "
from digimon_gym.digilab_client import list_stores
for s in list_stores():
    print(s.store_id, s.name, s.city, s.state)
"
```
