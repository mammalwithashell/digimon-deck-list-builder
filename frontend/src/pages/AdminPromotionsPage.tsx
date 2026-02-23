import { useEffect, useState } from 'react';
import { createPromotion, getPromotions, type PromotionItem } from '@/api/adminApi';

export function AdminPromotionsPage() {
  const [promotions, setPromotions] = useState<PromotionItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [cardId, setCardId] = useState('BT24-001');
  const [setId, setSetId] = useState('bt24');
  const [moduleName, setModuleName] = useState('bt24_001');
  const [expectedHash, setExpectedHash] = useState('');
  const [notes, setNotes] = useState('');

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await getPromotions();
      setPromotions(data);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load promotions';
      setError(message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const onPromote = async () => {
    try {
      await createPromotion({
        card_id: cardId,
        set_id: setId,
        module_name: moduleName,
        expected_generated_hash: expectedHash,
        notes,
      });
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Promotion failed';
      setError(message);
    }
  };

  return (
    <div className="max-w-6xl mx-auto p-6 space-y-6">
      <h1 className="text-2xl font-semibold text-white">Admin Promotions</h1>
      {error ? <div className="text-red-400 text-sm">{error}</div> : null}

      <div className="border border-gray-700 rounded p-4 bg-gray-900 grid grid-cols-1 md:grid-cols-2 gap-3">
        <input
          value={cardId}
          onChange={(e) => setCardId(e.target.value)}
          placeholder="card_id"
          className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
        />
        <input
          value={setId}
          onChange={(e) => setSetId(e.target.value)}
          placeholder="set_id"
          className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
        />
        <input
          value={moduleName}
          onChange={(e) => setModuleName(e.target.value)}
          placeholder="module_name"
          className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
        />
        <input
          value={expectedHash}
          onChange={(e) => setExpectedHash(e.target.value)}
          placeholder="expected_generated_hash"
          className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white"
        />
        <input
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          placeholder="notes"
          className="bg-gray-800 border border-gray-700 rounded px-2 py-2 text-sm text-white md:col-span-2"
        />
        <button
          onClick={() => void onPromote()}
          className="px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-500 text-white text-sm w-fit"
        >
          Promote Script
        </button>
      </div>

      <div className="space-y-3">
        {loading ? <div className="text-gray-300">Loading...</div> : null}
        {promotions.map((item) => (
          <div key={item.id} className="border border-gray-700 rounded p-4 bg-gray-900">
            <div className="flex justify-between">
              <div className="text-white">
                {item.card_id} ({item.module_name})
              </div>
              <div className="text-xs text-gray-400">manifest v{item.manifest_version}</div>
            </div>
            <div className="text-xs text-gray-500 mt-2">
              generated={item.generated_hash.slice(0, 16)}... frozen={item.frozen_hash.slice(0, 16)}...
            </div>
            <div className="text-xs text-gray-500 mt-1">
              ai_task_id={item.ai_task_id ?? 'n/a'} batch_id={item.batch_id ?? 'n/a'}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
