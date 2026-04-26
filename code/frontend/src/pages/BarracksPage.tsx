import { useState, useEffect } from 'react';
import { useTrainingStore } from '@/stores/trainingStore';
import { createAgent, deleteAgent, updateAgent, getDeckLibraryArchetypes } from '@/api/trainingApi';
import type { AgentItem, ArchetypeInfo } from '@/api/trainingApi';

type Algorithm = 'mlp' | 'lstm';
type DeckSourceTab = 'library' | 'import';

export function BarracksPage() {
  const agents = useTrainingStore((s) => s.agents);
  const fetchAgents = useTrainingStore((s) => s.fetchAgents);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Create form state
  const [formOpen, setFormOpen] = useState(false);
  const [name, setName] = useState('');
  const [archetype, setArchetype] = useState('');
  const [algorithm, setAlgorithm] = useState<Algorithm>('mlp');
  const [deckSourceTab, setDeckSourceTab] = useState<DeckSourceTab>('library');
  const [selectedArchetype, setSelectedArchetype] = useState('');
  const [importText, setImportText] = useState('');
  const [archetypes, setArchetypes] = useState<ArchetypeInfo[]>([]);
  const [creating, setCreating] = useState(false);

  // Edit state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editName, setEditName] = useState('');
  const [editArchetype, setEditArchetype] = useState('');

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      await fetchAgents();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load agents';
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!formOpen) return;
    let ignore = false;
    const loadArchetypes = async () => {
      try {
        const data = await getDeckLibraryArchetypes();
        if (!ignore) setArchetypes(data);
      } catch {
        // Silently ignore archetype load failures
      }
    };
    void loadArchetypes();
    return () => { ignore = true; };
  }, [formOpen]);

  const onCreateAgent = async () => {
    if (!name.trim()) return;
    setCreating(true);
    setError(null);
    try {
      const deckCardIds =
        deckSourceTab === 'import'
          ? importText
              .split(/[,\n]+/)
              .map((s) => s.trim())
              .filter(Boolean)
          : undefined;

      const deck = deckCardIds ?? (selectedArchetype
        ? archetypes.find((a) => a.name === selectedArchetype)?.sample_decklist ?? []
        : []);
      const deckSource = deckSourceTab === 'library' && selectedArchetype
        ? `library:${selectedArchetype}`
        : 'inline';
      await createAgent({
        name: name.trim(),
        archetype: archetype.trim() || selectedArchetype || '',
        algorithm,
        deck,
        deck_source: deckSource,
      });
      setName('');
      setArchetype('');
      setAlgorithm('mlp');
      setSelectedArchetype('');
      setImportText('');
      setFormOpen(false);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to create agent';
      setError(message);
    } finally {
      setCreating(false);
    }
  };

  const onDeleteAgent = async (agentId: string) => {
    setError(null);
    try {
      await deleteAgent(agentId);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to delete agent';
      setError(message);
    }
  };

  const onStartEdit = (agent: AgentItem) => {
    setEditingId(agent.id);
    setEditName(agent.name);
    setEditArchetype(agent.archetype ?? '');
  };

  const onSaveEdit = async () => {
    if (!editingId || !editName.trim()) return;
    setError(null);
    try {
      await updateAgent(editingId, {
        name: editName.trim(),
        archetype: editArchetype.trim() || undefined,
      });
      setEditingId(null);
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to update agent';
      setError(message);
    }
  };

  const onCancelEdit = () => {
    setEditingId(null);
  };

  const formatDate = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
  };

  return (
    <div className="max-w-7xl mx-auto px-4 py-6">
      <h1 className="text-2xl font-bold text-white mb-6">The Barracks</h1>

      {error ? <div className="text-red-400 text-sm mb-4">{error}</div> : null}

      {/* Create Agent Form */}
      <div className="bg-gray-800 rounded-lg border border-gray-700 mb-6">
        <button
          onClick={() => setFormOpen(!formOpen)}
          className="w-full flex items-center justify-between px-4 py-3 text-left"
        >
          <span className="text-sm font-medium text-gray-300">Create New Agent</span>
          <span className="text-gray-400 text-xs">{formOpen ? '[ collapse ]' : '[ expand ]'}</span>
        </button>

        {formOpen ? (
          <div className="px-4 pb-4 space-y-4 border-t border-gray-700 pt-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs text-gray-400 mb-1">Name</label>
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="Agent name"
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                />
              </div>
              <div>
                <label className="block text-xs text-gray-400 mb-1">Archetype</label>
                <input
                  value={archetype}
                  onChange={(e) => setArchetype(e.target.value)}
                  placeholder="e.g. Aggro, Control, Midrange"
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                />
              </div>
            </div>

            {/* Algorithm Toggle */}
            <div>
              <label className="block text-xs text-gray-400 mb-1">Algorithm</label>
              <div className="flex gap-2">
                <button
                  onClick={() => setAlgorithm('mlp')}
                  className={`px-3 py-1.5 rounded text-sm font-medium ${
                    algorithm === 'mlp'
                      ? 'bg-blue-600 text-white'
                      : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                  }`}
                >
                  MLP
                </button>
                <button
                  onClick={() => setAlgorithm('lstm')}
                  className={`px-3 py-1.5 rounded text-sm font-medium ${
                    algorithm === 'lstm'
                      ? 'bg-purple-600 text-white'
                      : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                  }`}
                >
                  LSTM
                </button>
              </div>
            </div>

            {/* Deck Source Tabs */}
            <div>
              <label className="block text-xs text-gray-400 mb-1">Deck Source</label>
              <div className="flex gap-1 mb-3">
                <button
                  onClick={() => setDeckSourceTab('library')}
                  className={`px-3 py-1.5 rounded text-sm font-medium ${
                    deckSourceTab === 'library'
                      ? 'bg-gray-600 text-white'
                      : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                  }`}
                >
                  Library
                </button>
                <button
                  onClick={() => setDeckSourceTab('import')}
                  className={`px-3 py-1.5 rounded text-sm font-medium ${
                    deckSourceTab === 'import'
                      ? 'bg-gray-600 text-white'
                      : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                  }`}
                >
                  Import
                </button>
              </div>

              {deckSourceTab === 'library' ? (
                <select
                  value={selectedArchetype}
                  onChange={(e) => setSelectedArchetype(e.target.value)}
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                >
                  <option value="">Select archetype...</option>
                  {archetypes.map((a) => (
                    <option key={a.name} value={a.name}>
                      {a.name} ({(a.meta_share * 100).toFixed(1)}%)
                    </option>
                  ))}
                </select>
              ) : (
                <textarea
                  value={importText}
                  onChange={(e) => setImportText(e.target.value)}
                  placeholder="Paste card IDs, one per line or comma-separated"
                  rows={5}
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm font-mono"
                />
              )}
            </div>

            <button
              onClick={() => void onCreateAgent()}
              disabled={creating || !name.trim()}
              className="px-3 py-1.5 rounded text-sm font-medium bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 text-white"
            >
              {creating ? 'Creating...' : 'Create Agent'}
            </button>
          </div>
        ) : null}
      </div>

      {/* Agent Table */}
      {loading ? (
        <div className="text-gray-300 text-sm">Loading agents...</div>
      ) : agents.length === 0 ? (
        <div className="bg-gray-800 rounded-lg border border-gray-700 p-8 text-center">
          <div className="text-gray-400 text-sm">No agents created yet.</div>
          <div className="text-gray-500 text-xs mt-1">
            Use the form above to create your first agent.
          </div>
        </div>
      ) : (
        <div className="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden">
          <table className="w-full text-sm text-left">
            <thead>
              <tr className="border-b border-gray-700">
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Name
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Archetype
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Algorithm
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider text-right">
                  Games Played
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider text-right">
                  Win Rate %
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Created
                </th>
                <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider text-right">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-700">
              {agents.map((agent) => (
                <tr key={agent.id} className="bg-gray-800 hover:bg-gray-750">
                  <td className="px-4 py-3">
                    {editingId === agent.id ? (
                      <input
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-white text-sm w-full"
                      />
                    ) : (
                      <span className="text-white font-medium">{agent.name}</span>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    {editingId === agent.id ? (
                      <input
                        value={editArchetype}
                        onChange={(e) => setEditArchetype(e.target.value)}
                        className="bg-gray-700 border border-gray-600 rounded px-2 py-1 text-white text-sm w-full"
                      />
                    ) : (
                      <span className="text-gray-300">{agent.archetype ?? '-'}</span>
                    )}
                  </td>
                  <td className="px-4 py-3">
                    <span
                      className={`px-2 py-0.5 rounded text-xs font-medium ${
                        agent.algorithm === 'lstm'
                          ? 'bg-purple-900 text-purple-300'
                          : 'bg-blue-900 text-blue-300'
                      }`}
                    >
                      {agent.algorithm.toUpperCase()}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-right text-gray-300">
                    {agent.total_games.toLocaleString()}
                  </td>
                  <td className="px-4 py-3 text-right text-gray-300">
                    {agent.total_games > 0
                      ? `${(agent.win_rate * 100).toFixed(1)}%`
                      : '-'}
                  </td>
                  <td className="px-4 py-3 text-gray-400 text-xs">
                    {formatDate(agent.created_at)}
                  </td>
                  <td className="px-4 py-3 text-right">
                    {editingId === agent.id ? (
                      <div className="flex gap-2 justify-end">
                        <button
                          onClick={() => void onSaveEdit()}
                          className="px-3 py-1.5 rounded text-sm font-medium bg-blue-600 hover:bg-blue-500 text-white"
                        >
                          Save
                        </button>
                        <button
                          onClick={onCancelEdit}
                          className="px-3 py-1.5 rounded text-sm font-medium bg-gray-700 hover:bg-gray-600 text-gray-300"
                        >
                          Cancel
                        </button>
                      </div>
                    ) : (
                      <div className="flex gap-2 justify-end">
                        <button
                          onClick={() => onStartEdit(agent)}
                          className="px-3 py-1.5 rounded text-sm font-medium bg-gray-700 hover:bg-gray-600 text-gray-300"
                        >
                          Edit
                        </button>
                        <button
                          onClick={() => void onDeleteAgent(agent.id)}
                          className="px-3 py-1.5 rounded text-sm font-medium bg-red-900 hover:bg-red-800 text-red-300"
                        >
                          Delete
                        </button>
                      </div>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
