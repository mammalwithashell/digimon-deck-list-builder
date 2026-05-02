import { useState, useEffect, useCallback } from 'react';
import { useTrainingStore } from '@/stores/trainingStore';
import { createJob, cancelJob } from '@/api/trainingApi';
import type { TrainingJobItem } from '@/api/trainingApi';

type OpponentType = 'greedy' | 'random' | 'agent';
type JobStatus = TrainingJobItem['status'];

const STATUS_STYLES: Record<JobStatus, string> = {
  queued: 'bg-gray-700 text-gray-300',
  running: 'bg-blue-900 text-blue-300 animate-pulse',
  completed: 'bg-green-900 text-green-300',
  failed: 'bg-red-900 text-red-300',
  canceled: 'bg-gray-700 text-gray-400',
};

export function ArenaPage() {
  const agents = useTrainingStore((s) => s.agents);
  const jobs = useTrainingStore((s) => s.jobs);
  const fetchAgents = useTrainingStore((s) => s.fetchAgents);
  const fetchJobs = useTrainingStore((s) => s.fetchJobs);

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  // Training panel state
  const [trainAgentId, setTrainAgentId] = useState('');
  const [opponentType, setOpponentType] = useState<OpponentType>('greedy');
  const [opponentAgentId, setOpponentAgentId] = useState('');
  const [timesteps, setTimesteps] = useState('100000');
  const [queueingTraining, setQueueingTraining] = useState(false);

  // Evaluation panel state
  const [evalAgentA, setEvalAgentA] = useState('');
  const [evalAgentB, setEvalAgentB] = useState('');
  const [gameCount, setGameCount] = useState('100');
  const [queueingEval, setQueueingEval] = useState(false);

  const loadData = useCallback(async () => {
    setError(null);
    try {
      await Promise.all([fetchAgents(), fetchJobs()]);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load data';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [fetchAgents, fetchJobs]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // Auto-refresh when active jobs exist
  useEffect(() => {
    const hasActive = jobs.some((j) => j.status === 'queued' || j.status === 'running');
    if (hasActive) {
      const interval = setInterval(() => {
        void fetchJobs();
      }, 5000);
      return () => clearInterval(interval);
    }
  }, [jobs.some((j) => j.status === 'queued' || j.status === 'running'), fetchJobs]);

  const onQueueTraining = async () => {
    if (!trainAgentId) return;
    setQueueingTraining(true);
    setError(null);
    try {
      const jobType = opponentType === 'agent'
        ? 'train_vs_agent' as const
        : opponentType === 'random'
          ? 'train_vs_random' as const
          : 'train_vs_greedy' as const;
      await createJob({
        job_type: jobType,
        agent_id: trainAgentId,
        opponent_agent_id: opponentType === 'agent' ? opponentAgentId || undefined : undefined,
        total_timesteps: Number(timesteps) || 100000,
      });
      await fetchJobs();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to queue training job';
      setError(message);
    } finally {
      setQueueingTraining(false);
    }
  };

  const onQueueEvaluation = async () => {
    if (!evalAgentA || !evalAgentB) return;
    setQueueingEval(true);
    setError(null);
    try {
      await createJob({
        job_type: 'evaluate',
        agent_id: evalAgentA,
        opponent_agent_id: evalAgentB,
        total_games: Number(gameCount) || 100,
      });
      await fetchJobs();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to queue evaluation job';
      setError(message);
    } finally {
      setQueueingEval(false);
    }
  };

  const onCancelJob = async (jobId: string) => {
    setError(null);
    try {
      await cancelJob(jobId);
      await fetchJobs();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to cancel job';
      setError(message);
    }
  };

  const getAgentName = (agentId: string | undefined | null): string => {
    if (!agentId) return '-';
    const agent = agents.find((a) => a.id === agentId);
    return agent?.name ?? agentId.slice(0, 8);
  };

  const formatDate = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const renderProgress = (job: TrainingJobItem) => {
    if (job.status !== 'running') return null;
    const total = job.total_games ?? 0;
    const completed = job.completed_games ?? 0;
    if (total === 0) return null;
    const pct = Math.min(100, Math.round((completed / total) * 100));
    return (
      <div className="flex items-center gap-2">
        <div className="flex-1 bg-gray-700 rounded-full h-2 overflow-hidden">
          <div
            className="bg-blue-500 h-2 rounded-full transition-all"
            style={{ width: `${pct}%` }}
          />
        </div>
        <span className="text-xs text-gray-400 whitespace-nowrap">
          {completed}/{total}
        </span>
      </div>
    );
  };

  return (
    <div className="max-w-7xl mx-auto px-4 py-6">
      <h1 className="text-2xl font-bold text-white mb-6">The Arena</h1>

      {error ? <div className="text-red-400 text-sm mb-4">{error}</div> : null}

      {loading ? (
        <div className="text-gray-300 text-sm">Loading...</div>
      ) : (
        <>
          {/* Training and Evaluation Panels */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
            {/* Training Panel */}
            <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
              <h2 className="text-sm font-medium text-gray-300 mb-4">Training</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Agent</label>
                  <select
                    value={trainAgentId}
                    onChange={(e) => setTrainAgentId(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  >
                    <option value="">Select agent...</option>
                    {agents.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name} ({a.algorithm})
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-xs text-gray-400 mb-1">Opponent Type</label>
                  <div className="flex gap-2">
                    {(['greedy', 'random', 'agent'] as const).map((type) => (
                      <button
                        key={type}
                        onClick={() => setOpponentType(type)}
                        className={`px-3 py-1.5 rounded text-sm font-medium capitalize ${
                          opponentType === type
                            ? 'bg-blue-600 text-white'
                            : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
                        }`}
                      >
                        {type}
                      </button>
                    ))}
                  </div>
                </div>

                {opponentType === 'agent' ? (
                  <div>
                    <label className="block text-xs text-gray-400 mb-1">Opponent Agent</label>
                    <select
                      value={opponentAgentId}
                      onChange={(e) => setOpponentAgentId(e.target.value)}
                      className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                    >
                      <option value="">Select opponent...</option>
                      {agents.map((a) => (
                        <option key={a.id} value={a.id}>
                          {a.name} ({a.algorithm})
                        </option>
                      ))}
                    </select>
                  </div>
                ) : null}

                <div>
                  <label className="block text-xs text-gray-400 mb-1">Timesteps</label>
                  <input
                    type="number"
                    value={timesteps}
                    onChange={(e) => setTimesteps(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  />
                </div>

                <button
                  onClick={() => void onQueueTraining()}
                  disabled={queueingTraining || !trainAgentId}
                  className="px-3 py-1.5 rounded text-sm font-medium bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 text-white"
                >
                  {queueingTraining ? 'Queueing...' : 'Queue Training'}
                </button>
              </div>
            </div>

            {/* Evaluation Panel */}
            <div className="bg-gray-800 rounded-lg border border-gray-700 p-4">
              <h2 className="text-sm font-medium text-gray-300 mb-4">Evaluation</h2>
              <div className="space-y-4">
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Agent A</label>
                  <select
                    value={evalAgentA}
                    onChange={(e) => setEvalAgentA(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  >
                    <option value="">Select agent...</option>
                    {agents.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name} ({a.algorithm})
                      </option>
                    ))}
                  </select>
                </div>

                <div className="text-center text-gray-500 text-xs font-medium">VS</div>

                <div>
                  <label className="block text-xs text-gray-400 mb-1">Agent B</label>
                  <select
                    value={evalAgentB}
                    onChange={(e) => setEvalAgentB(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  >
                    <option value="">Select agent...</option>
                    {agents.map((a) => (
                      <option key={a.id} value={a.id}>
                        {a.name} ({a.algorithm})
                      </option>
                    ))}
                  </select>
                </div>

                <div>
                  <label className="block text-xs text-gray-400 mb-1">Game Count</label>
                  <input
                    type="number"
                    value={gameCount}
                    onChange={(e) => setGameCount(e.target.value)}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  />
                </div>

                <button
                  onClick={() => void onQueueEvaluation()}
                  disabled={queueingEval || !evalAgentA || !evalAgentB}
                  className="px-3 py-1.5 rounded text-sm font-medium bg-blue-600 hover:bg-blue-500 disabled:bg-gray-700 disabled:text-gray-500 text-white"
                >
                  {queueingEval ? 'Queueing...' : 'Queue Evaluation'}
                </button>
              </div>
            </div>
          </div>

          {/* Jobs Table */}
          <div className="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden">
            <div className="px-4 py-3 border-b border-gray-700 flex items-center justify-between">
              <h2 className="text-sm font-medium text-gray-300">Jobs</h2>
              <button
                onClick={() => void fetchJobs()}
                className="px-3 py-1.5 rounded text-sm font-medium bg-gray-700 hover:bg-gray-600 text-gray-300"
              >
                Refresh
              </button>
            </div>

            {jobs.length === 0 ? (
              <div className="p-8 text-center">
                <div className="text-gray-400 text-sm">No jobs yet.</div>
                <div className="text-gray-500 text-xs mt-1">
                  Queue a training or evaluation job to get started.
                </div>
              </div>
            ) : (
              <table className="w-full text-sm text-left">
                <thead>
                  <tr className="border-b border-gray-700">
                    <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                      Type
                    </th>
                    <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                      Agent
                    </th>
                    <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                      Opponent
                    </th>
                    <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider">
                      Status
                    </th>
                    <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider w-40">
                      Progress
                    </th>
                    <th className="px-4 py-3 text-xs font-medium text-gray-400 uppercase tracking-wider text-right">
                      Win Rate
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
                  {jobs.map((job) => (
                    <tr key={job.id} className="bg-gray-800 hover:bg-gray-750">
                      <td className="px-4 py-3">
                        <span className="text-gray-300 capitalize">{job.job_type.replace(/_/g, ' ')}</span>
                      </td>
                      <td className="px-4 py-3">
                        <span className="text-white">{getAgentName(job.agent_id)}</span>
                      </td>
                      <td className="px-4 py-3">
                        <span className="text-gray-300">
                          {job.opponent_type === 'agent'
                            ? getAgentName(job.opponent_agent_id)
                            : job.opponent_type ?? '-'}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        <span
                          className={`px-2 py-0.5 rounded text-xs font-medium ${
                            STATUS_STYLES[job.status] ?? 'bg-gray-700 text-gray-300'
                          }`}
                        >
                          {job.status}
                        </span>
                      </td>
                      <td className="px-4 py-3">
                        {renderProgress(job)}
                      </td>
                      <td className="px-4 py-3 text-right text-gray-300">
                        {job.win_rate != null
                          ? `${(job.win_rate * 100).toFixed(1)}%`
                          : '-'}
                      </td>
                      <td className="px-4 py-3 text-gray-400 text-xs">
                        {formatDate(job.created_at)}
                      </td>
                      <td className="px-4 py-3 text-right">
                        {job.status === 'queued' || job.status === 'running' ? (
                          <button
                            onClick={() => void onCancelJob(job.id)}
                            className="px-3 py-1.5 rounded text-sm font-medium bg-red-900 hover:bg-red-800 text-red-300"
                          >
                            Cancel
                          </button>
                        ) : null}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}
    </div>
  );
}
