import { Link } from 'react-router-dom';
import type { LauncherReleaseSummary } from './launcherData';

interface LauncherNewsPanelProps {
  release: LauncherReleaseSummary;
}

export function LauncherNewsPanel({ release }: LauncherNewsPanelProps) {
  return (
    <section className="launcher-news" aria-labelledby="launcher-news-heading">
      <div className="launcher-panel-head">
        <h2 id="launcher-news-heading">{release.title}</h2>
        <span>{release.versionLabel}</span>
      </div>
      <ul>
        {release.bullets.map((bullet) => (
          <li key={bullet}>{bullet}</li>
        ))}
      </ul>
      <Link to="/patch-notes">View patch notes →</Link>
    </section>
  );
}
