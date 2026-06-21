import { Panel } from './Panel';
import { Screen } from './Screen';
import { StatChip } from './StatChip';
import { DigimonSprite } from './DigimonSprite';

export interface AnalyzerStat {
  label: string;
  value: string;
  tone?: 'default' | 'signal';
}

export interface AnalyzerFrameProps {
  name: string;
  spriteId?: string;
  stats?: AnalyzerStat[];
  className?: string;
}

/**
 * Card analyzer. Dark renders a phosphor terminal readout (`▸ ANALYZER`); light
 * renders the DIGITALMONSTER frame. The brand line carries both labels and CSS
 * shows the right one per theme — call sites stay theme-agnostic.
 */
export function AnalyzerFrame({ name, spriteId, stats = [], className = '' }: AnalyzerFrameProps) {
  return (
    <Panel className={`ds-analyzer ${className}`.trim()}>
      <Screen>
        <span className="ds-analyzer__brand">
          <span className="ds-b-dark">▸ ANALYZER</span>
          <span className="ds-b-light">DIGITALMONSTER</span>
        </span>
        <span className="ds-analyzer__name">{name}</span>
        <div className="ds-analyzer__row">
          <div className="ds-analyzer__art">
            <DigimonSprite spriteId={spriteId} size={56} alt={name} />
          </div>
          <div className="ds-analyzer__stats">
            {stats.map((s, i) => (
              <StatChip key={i} label={s.label} value={s.value} tone={s.tone} />
            ))}
          </div>
        </div>
      </Screen>
    </Panel>
  );
}
