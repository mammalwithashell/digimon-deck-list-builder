import type { ReactNode } from 'react';
import {
  Backdrop,
  Panel,
  Window,
  StatusBar,
  Button,
  ThemeSwitch,
  NavRail,
  NavRailItem,
  AnalyzerFrame,
  CardBack,
  CardTile,
  DeckColorBadge,
  DigimonSprite,
  StatChip,
  Badge,
  MemoryGauge,
  Credits,
} from './components';
import type { Theme } from './theme/themeStore';

const THEMES: Theme[] = ['dark', 'light'];

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ marginBottom: 20 }}>
      <div
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--text-xs)',
          letterSpacing: '0.16em',
          textTransform: 'uppercase',
          color: 'var(--ink-3)',
          marginBottom: 8,
        }}
      >
        {label}
      </div>
      {children}
    </div>
  );
}

const row = { display: 'flex', gap: 10, flexWrap: 'wrap' as const, alignItems: 'flex-start' };

function Kitchen() {
  return (
    <div style={{ padding: 16 }}>
      <Section label="Window">
        <Window title="Deck Builder">
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--text-sm)', color: 'var(--ink-1)', lineHeight: 1.7 }}>
            50 / 50 cards
            <br />
            status: <span style={{ color: 'var(--good)' }}>legal</span>
          </div>
        </Window>
      </Section>

      <Section label="Window · with close">
        <Window title="Settings" onClose={() => undefined}>
          <div style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--text-sm)', color: 'var(--ink-1)' }}>window body</div>
        </Window>
      </Section>

      <Section label="Button">
        <div style={row}>
          <Button variant="primary">Play</Button>
          <Button>Cancel</Button>
          <Button variant="accent">Accent</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="danger">Delete</Button>
          <Button disabled>Disabled</Button>
        </div>
      </Section>

      <Section label="Panel">
        <div style={row}>
          <Panel><div style={{ padding: 12, fontSize: 'var(--text-sm)' }}>Panel surface</div></Panel>
          <Panel raised><div style={{ padding: 12, fontSize: 'var(--text-sm)' }}>Raised panel</div></Panel>
        </div>
      </Section>

      <Section label="NavRail">
        <div style={{ width: 180 }}>
          <NavRail>
            <NavRailItem active>Play</NavRailItem>
            <NavRailItem>Decks</NavRailItem>
            <NavRailItem>Models</NavRailItem>
            <NavRailItem>Settings</NavRailItem>
          </NavRail>
        </div>
      </Section>

      <Section label="AnalyzerFrame + sprite">
        <div style={{ maxWidth: 280 }}>
          <AnalyzerFrame
            name="V-mon"
            spriteId="veemon"
            stats={[
              { label: 'Lv', value: 'Rookie' },
              { label: 'Type', value: 'Reptile' },
              { label: 'Atk', value: 'V-mon Head', tone: 'signal' },
            ]}
          />
        </div>
      </Section>

      <Section label="Card · back / tile / deck colors">
        <div style={{ ...row, alignItems: 'flex-end' }}>
          <div style={{ width: 60 }}><CardBack /></div>
          <div style={{ width: 60 }}>
            <CardTile name="V-mon" art={<DigimonSprite spriteId="veemon" size={44} />} />
          </div>
          <DeckColorBadge color="r" />
          <DeckColorBadge color="b" />
          <DeckColorBadge color="g" />
          <DeckColorBadge color="b" sleeveId="ST19-03" />
        </div>
      </Section>

      <Section label="StatChip">
        <div style={row}>
          <StatChip label="Lv" value="Mega" />
          <StatChip label="Type" value="Vaccine" />
          <StatChip label="Atk" value="Gaia Force" tone="signal" />
        </div>
      </Section>

      <Section label="Badge">
        <div style={row}>
          <Badge>Neutral</Badge>
          <Badge tone="player">Player</Badge>
          <Badge tone="opp">Opp</Badge>
          <Badge tone="good">Legal</Badge>
          <Badge tone="warn">Draft</Badge>
          <Badge tone="danger">Illegal</Badge>
        </div>
      </Section>

      <Section label="MemoryGauge">
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <MemoryGauge value={3} />
          <MemoryGauge value={-2} />
        </div>
      </Section>

      <Section label="ThemeSwitch">
        <ThemeSwitch />
      </Section>

      <Section label="StatusBar">
        <StatusBar>
          <span>ready</span>
          <span style={{ marginLeft: 'auto' }}>v0.1 · alpha</span>
        </StatusBar>
      </Section>

      <Section label="Credits">
        <Credits />
      </Section>
    </div>
  );
}

/**
 * In-app style guide — every primitive rendered in both themes side by side.
 * Each column forces its own theme via a `data-theme` wrapper, so the page
 * shows both regardless of the globally active theme.
 */
export function StyleGuidePage() {
  return (
    <div style={{ minHeight: '100vh', display: 'grid', gridTemplateColumns: '1fr 1fr' }}>
      {THEMES.map((t) => (
        <div key={t} data-theme={t} style={{ minHeight: '100vh' }}>
          <Backdrop style={{ minHeight: '100vh' }}>
            <div
              style={{
                padding: '12px 16px',
                fontFamily: 'var(--font-mono)',
                fontSize: 'var(--text-xs)',
                letterSpacing: '0.16em',
                textTransform: 'uppercase',
                color: 'var(--ink-2)',
                borderBottom: '1px solid var(--line-0)',
              }}
            >
              theme · {t}
            </div>
            <Kitchen />
          </Backdrop>
        </div>
      ))}
    </div>
  );
}
