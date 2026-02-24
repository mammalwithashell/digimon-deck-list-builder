const GITHUB_BASE = 'https://github.com/mammalwithashell/digimon-deck-list-builder';

export function CommitLink({ sha }: { sha?: string | null }) {
  if (!sha) return <span className="text-gray-500 text-xs">--</span>;
  return (
    <a
      href={`${GITHUB_BASE}/commit/${sha}`}
      target="_blank"
      rel="noopener noreferrer"
      className="text-xs text-blue-400 hover:text-blue-300 font-mono"
    >
      {sha.slice(0, 8)}
    </a>
  );
}

export function PrLink({ url }: { url?: string | null }) {
  if (!url) return null;
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="text-xs text-blue-400 hover:text-blue-300 underline"
    >
      PR
    </a>
  );
}
