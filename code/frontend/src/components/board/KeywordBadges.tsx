import type { ReactNode } from 'react';
import { KEYWORD_COLORS, KEYWORD_DISPLAY } from '@/utils/constants';

export type KeywordOrigin = 'innate' | 'gained';

export type KeywordIconCategory =
  | 'shield'
  | 'ward'
  | 'bolt'
  | 'sword'
  | 'crosshair'
  | 'recycle'
  | 'sparkle';

export interface KeywordMeta {
  label: string;
  description: string;
  color: string;
  icon: KeywordIconCategory;
}

const KEYWORD_META: Record<string, KeywordMeta> = {
  blocker: {
    label: 'Blocker',
    description:
      "When an opponent's Digimon attacks, you may suspend this Digimon to force the opponent to attack it instead.",
    color: '#2563eb',
    icon: 'shield',
  },
  guard: {
    label: 'Guard',
    description:
      "When another of your Digimon would leave the battle area by an opponent's effect, you may delete this Digimon to keep it.",
    color: '#0284c7',
    icon: 'shield',
  },
  progress: {
    label: 'Progress',
    description: "While this Digimon attacks, it isn't affected by your opponent's effects.",
    color: '#7c3aed',
    icon: 'shield',
  },
  barrier: {
    label: 'Barrier',
    description:
      'When this Digimon would be deleted in battle, you may trash the top card of your security stack to prevent that deletion.',
    color: '#0284c7',
    icon: 'ward',
  },
  armor_purge: {
    label: 'Armor Purge',
    description:
      'When this Digimon would be deleted, you may trash the top card of this Digimon to prevent that deletion.',
    color: '#64748b',
    icon: 'ward',
  },
  decoy: {
    label: 'Decoy',
    description:
      "When one of your other named Digimon would be deleted by an opponent's effect, you may delete this Digimon to prevent it.",
    color: '#db2777',
    icon: 'ward',
  },
  evade: {
    label: 'Evade',
    description:
      'When this Digimon would be deleted, you may suspend it to prevent that deletion.',
    color: '#475569',
    icon: 'ward',
  },
  fortitude: {
    label: 'Fortitude',
    description:
      'When this Digimon is deleted while it has digivolution cards, play it without paying the cost.',
    color: '#0369a1',
    icon: 'ward',
  },
  scapegoat: {
    label: 'Scapegoat',
    description:
      'When this Digimon would be deleted other than by your effects, you may delete 1 of your other Digimon to prevent it.',
    color: '#b91c1c',
    icon: 'ward',
  },
  fragment: {
    label: 'Fragment',
    description:
      'When this Digimon would be deleted, you may trash X of its digivolution cards to prevent that deletion.',
    color: '#059669',
    icon: 'ward',
  },
  rush: {
    label: 'Rush',
    description: 'This Digimon can attack the turn it comes into play.',
    color: '#16a34a',
    icon: 'bolt',
  },
  blitz: {
    label: 'Blitz',
    description:
      'When this effect activates, this Digimon may attack if your opponent has 1 or more memory.',
    color: '#d97706',
    icon: 'bolt',
  },
  blast_digivolve: {
    label: 'Blast Digivolve',
    description:
      'One of your Digimon may digivolve into this card in your hand without paying the cost.',
    color: '#d97706',
    icon: 'bolt',
  },
  blast_dna_digivolve: {
    label: 'Blast DNA Digivolve',
    description:
      'The named Digimon and a named card in your hand may DNA digivolve into this card without paying the cost.',
    color: '#b45309',
    icon: 'bolt',
  },
  arts_digivolve: {
    label: 'Arts Digivolve',
    description: 'Digivolve one of your Digimon into this Option card using its Arts Digivolve condition.',
    color: '#9333ea',
    icon: 'bolt',
  },
  piercing: {
    label: 'Piercing',
    description:
      "When this Digimon attacks and deletes an opponent's Digimon and survives, it performs any security checks it normally would.",
    color: '#dc2626',
    icon: 'sword',
  },
  retaliation: {
    label: 'Retaliation',
    description:
      'When this Digimon is deleted after losing a battle, delete the Digimon it was battling.',
    color: '#ea580c',
    icon: 'sword',
  },
  execute: {
    label: 'Execute',
    description:
      'At the end of your turn, this Digimon may attack (even an unsuspended Digimon), then delete it.',
    color: '#b91c1c',
    icon: 'sword',
  },
  engage: {
    label: 'Engage',
    description: 'At the end of your turn, this Digimon may attack.',
    color: '#e11d48',
    icon: 'sword',
  },
  overclock: {
    label: 'Overclock',
    description:
      'At the end of your turn, you may delete 1 Token or other named Digimon to attack a player without suspending.',
    color: '#e11d48',
    icon: 'sword',
  },
  security_attack_plus: {
    label: 'Security Attack +',
    description: 'Increases the number of security cards this Digimon checks when attacking.',
    color: '#ca8a04',
    icon: 'sword',
  },
  raid: {
    label: 'Raid',
    description:
      "When this Digimon attacks, you may switch the attack target to your opponent's unsuspended Digimon with the highest DP.",
    color: '#dc2626',
    icon: 'crosshair',
  },
  alliance: {
    label: 'Alliance',
    description:
      "When this Digimon attacks, you may suspend 1 of your other Digimon to add its DP and <Security A. +1>.",
    color: '#4f46e5',
    icon: 'crosshair',
  },
  collision: {
    label: 'Collision',
    description:
      "While this Digimon attacks, all of your opponent's Digimon gain <Blocker> and must block if able.",
    color: '#db2777',
    icon: 'crosshair',
  },
  vortex: {
    label: 'Vortex',
    description:
      "At the end of your turn, this Digimon may attack an opponent's Digimon, even the turn it was played.",
    color: '#6d28d9',
    icon: 'crosshair',
  },
  reboot: {
    label: 'Reboot',
    description: "Unsuspend this Digimon during your opponent's unsuspend phase.",
    color: '#0891b2',
    icon: 'recycle',
  },
  save: {
    label: 'Save',
    description: 'You may place this card under 1 of your Tamers.',
    color: '#0d9488',
    icon: 'recycle',
  },
  material_save: {
    label: 'Material Save',
    description:
      'When deleted, place X named DigiXros cards from its digivolution cards under 1 of your Tamers.',
    color: '#57534e',
    icon: 'recycle',
  },
  decode: {
    label: 'Decode',
    description:
      'When leaving the battle area other than by battle, play 1 named Digimon from its digivolution cards without paying the cost.',
    color: '#2563eb',
    icon: 'recycle',
  },
  partition: {
    label: 'Partition',
    description:
      'When leaving the battle area other than by your effects or battle, play 1 of each named card from its digivolution cards.',
    color: '#0f766e',
    icon: 'recycle',
  },
  training: {
    label: 'Training',
    description:
      '[Main] Suspend this Digimon to place the top card of your deck at the bottom of its digivolution cards.',
    color: '#059669',
    icon: 'recycle',
  },
  ascension: {
    label: 'Ascension',
    description:
      'When this Digimon is deleted, you may place it face down on top of your security stack.',
    color: '#ca8a04',
    icon: 'recycle',
  },
  mind_link: {
    label: 'Mind Link',
    description:
      "[Main] Place this Tamer at the bottom of 1 of your Digimon's digivolution cards that has no Tamers.",
    color: '#7c3aed',
    icon: 'recycle',
  },
  digisorption: {
    label: 'Digisorption',
    description:
      'When digivolving into this card, suspend 1 of your Digimon to reduce the digivolution cost.',
    color: '#0d9488',
    icon: 'recycle',
  },
  jamming: {
    label: 'Jamming',
    description: "This Digimon can't be deleted in battles against Security Digimon.",
    color: '#9333ea',
    icon: 'sparkle',
  },
  iceclad: {
    label: 'Iceclad',
    description:
      'In battles against Digimon (not Security Digimon), compare digivolution card counts instead of DP.',
    color: '#0284c7',
    icon: 'sparkle',
  },
  delay: {
    label: 'Delay',
    description:
      'Trash this card in your battle area to activate the effect that follows (not on the turn it enters play).',
    color: '#6b7280',
    icon: 'sparkle',
  },
};

function humanizeKeyword(kw: string): string {
  return kw
    .replace(/_/g, ' ')
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .trim();
}

export function getKeywordMeta(kw: string): KeywordMeta {
  const known = KEYWORD_META[kw];
  if (known) return known;
  const label = KEYWORD_DISPLAY[kw] ?? humanizeKeyword(kw);
  return {
    label,
    description: `${label} keyword ability.`,
    color: KEYWORD_COLORS[kw] ?? '#4b5563',
    icon: 'sparkle',
  };
}

export function KeywordIcon({
  category,
  size = 11,
}: {
  category: KeywordIconCategory;
  size?: number;
}): ReactNode {
  const common = {
    width: size,
    height: size,
    viewBox: '0 0 16 16',
    fill: 'none',
    stroke: 'currentColor',
    strokeWidth: 1.75,
    strokeLinecap: 'round' as const,
    strokeLinejoin: 'round' as const,
    'aria-hidden': true as const,
  };

  switch (category) {
    case 'shield':
      return (
        <svg {...common}>
          <path d="M8 1.5L13.5 3.8v4.1c0 3.3-2.3 5.7-5.5 6.6-3.2-.9-5.5-3.3-5.5-6.6V3.8L8 1.5z" />
        </svg>
      );
    case 'ward':
      return (
        <svg {...common}>
          <path d="M8 1.5L13.5 3.8v4.1c0 3.3-2.3 5.7-5.5 6.6-3.2-.9-5.5-3.3-5.5-6.6V3.8L8 1.5z" />
          <path d="M5.6 8l1.7 1.7 3.2-3.4" />
        </svg>
      );
    case 'bolt':
      return (
        <svg {...common}>
          <path d="M9.2 1.5L3.5 9h4.3l-1 5.5L12.5 7H8.2l1-5.5z" />
        </svg>
      );
    case 'sword':
      return (
        <svg {...common}>
          <path d="M13.5 2.5L6 10M10.5 2.5h3v3M4.5 8.5l3 3M2.5 13.5l2.5-2.5" />
        </svg>
      );
    case 'crosshair':
      return (
        <svg {...common}>
          <circle cx="8" cy="8" r="4.5" />
          <path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2" />
        </svg>
      );
    case 'recycle':
      return (
        <svg {...common}>
          <path d="M13 6.5A5.2 5.2 0 0 0 3.6 4.7M3 9.5A5.2 5.2 0 0 0 12.4 11.3" />
          <path d="M13 3v3.5H9.5M3 13V9.5h3.5" />
        </svg>
      );
    case 'sparkle':
    default:
      return (
        <svg {...common}>
          <path d="M8 1.5l1.6 4.4L14 7.5l-4.4 1.6L8 13.5l-1.6-4.4L2 7.5l4.4-1.6L8 1.5z" />
        </svg>
      );
  }
}

interface KeywordBadgesProps {
  keywords: string[];
  gainedKeywords?: readonly string[];
  innateKeywords?: readonly string[];
  variant?: 'slot' | 'inspector';
  maxVisible?: number;
}

export function KeywordBadges({
  keywords,
  gainedKeywords = [],
  innateKeywords,
  variant = 'slot',
  maxVisible = 3,
}: KeywordBadgesProps) {
  if (keywords.length === 0) return null;

  const gainedSet = new Set(gainedKeywords);
  const innateSet = innateKeywords ? new Set(innateKeywords) : null;

  const resolveOrigin = (kw: string): KeywordOrigin => {
    if (gainedSet.has(kw)) return 'gained';
    if (innateSet && innateSet.has(kw)) return 'innate';
    return 'innate';
  };

  const showLimit = variant === 'slot' ? maxVisible : keywords.length;
  const visible = keywords.slice(0, showLimit);
  const hidden = keywords.slice(showLimit);

  return (
    <div
      className={`hs-kw-list hs-kw-list--${variant}`}
      data-testid={`keyword-badges-${variant}`}
    >
      {visible.map((kw) => {
        const meta = getKeywordMeta(kw);
        const origin = resolveOrigin(kw);
        const originTag = origin === 'gained' ? 'Granted by effect' : 'Innate';
        const fullTitle = `${meta.label} (${originTag}) — ${meta.description}`;
        return (
          <span
            key={kw}
            data-testid={`keyword-badge-${kw}`}
            data-keyword={kw}
            data-keyword-origin={origin}
            className={`hs-kw-badge hs-kw-badge--${origin}`}
            style={{ '--kw-color': meta.color, backgroundColor: meta.color } as React.CSSProperties}
            title={fullTitle}
            aria-label={fullTitle}
          >
            <span className="hs-kw-badge__icon">
              <KeywordIcon category={meta.icon} size={variant === 'inspector' ? 11 : 10} />
            </span>
            <span className="hs-kw-badge__label">{meta.label}</span>
            {origin === 'gained' && (
              <span className="hs-kw-badge__origin" aria-hidden="true">
                ✦
              </span>
            )}
          </span>
        );
      })}
      {hidden.length > 0 && (
        <span
          data-testid="keyword-overflow-badge"
          className="hs-kw-badge hs-kw-badge--more"
          title={hidden
            .map((kw) => {
              const meta = getKeywordMeta(kw);
              const origin = resolveOrigin(kw) === 'gained' ? 'Granted' : 'Innate';
              return `${meta.label} (${origin}): ${meta.description}`;
            })
            .join('\n')}
          aria-label={`${hidden.length} more keywords: ${hidden.map((kw) => getKeywordMeta(kw).label).join(', ')}`}
        >
          +{hidden.length}
        </span>
      )}
    </div>
  );
}
