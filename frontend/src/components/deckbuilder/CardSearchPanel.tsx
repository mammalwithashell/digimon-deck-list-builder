import { useEffect, useRef } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { searchCards } from '@/api/digimonCardApi';
import { FilterBar } from './FilterBar';
import { CardGrid } from './CardGrid';

export function CardSearchPanel() {
  const {
    searchQuery,
    setSearchQuery,
    filters,
    setSearchResults,
    setIsSearching,
  } = useDeckBuilderStore();

  const debounceRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      // Build search params from query + filters
      const params: Record<string, string> = {};
      if (searchQuery.trim()) params.n = searchQuery.trim();
      if (filters.color.length === 1) params.color = filters.color[0]!;
      if (filters.type.length === 1) params.type = filters.type[0]!;

      // Set filter: use API's `card` param with set prefix (e.g. "BT20")
      if (filters.set) params.card = filters.set;

      // Only search if we have at least one param
      if (Object.keys(params).length === 0) {
        setSearchResults([]);
        return;
      }

      setIsSearching(true);
      try {
        let results = await searchCards(params);

        // Client-side filtering for multi-value filters the API can't handle
        if (filters.color.length > 1) {
          results = results.filter((c) =>
            filters.color.some(
              (fc) => c.color.toLowerCase().includes(fc.toLowerCase()),
            ),
          );
        }
        if (filters.type.length > 1) {
          results = results.filter((c) =>
            filters.type.some(
              (ft) => c.type.toLowerCase() === ft.toLowerCase(),
            ),
          );
        }
        if (filters.level.length > 0) {
          results = results.filter((c) =>
            filters.level.includes(c.level),
          );
        }

        // When using set filter, filter to exact prefix match
        // (API may return partial matches like BT2 matching BT20)
        if (filters.set) {
          const prefix = filters.set.toUpperCase() + '-';
          results = results.filter((c) =>
            c.cardnumber.toUpperCase().startsWith(prefix),
          );
        }

        setSearchResults(results);
      } catch {
        setSearchResults([]);
      } finally {
        setIsSearching(false);
      }
    }, 300);

    return () => clearTimeout(debounceRef.current);
  }, [searchQuery, filters, setSearchResults, setIsSearching]);

  return (
    <div className="flex flex-col gap-2 h-full">
      <input
        type="text"
        placeholder="Search cards by name..."
        value={searchQuery}
        onChange={(e) => setSearchQuery(e.target.value)}
        className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-100 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
      />
      <FilterBar />
      <div className="flex-1 overflow-y-auto">
        <CardGrid />
      </div>
    </div>
  );
}
