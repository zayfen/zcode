import { useCallback, useState } from 'react';

// ── Props ──

export interface ResetButtonProps {
  /** Callback invoked when the button is clicked (e.g. `game.resetGame`). */
  onClick: () => void;
  /** Optional label text. Defaults to "新局 · New Game". */
  label?: string;
  /** Optional aria-label override. Defaults to "Reset game". */
  ariaLabel?: string;
}

// ── Shared button styles ──

const BASE_STYLE: React.CSSProperties = {
  padding: '10px 28px',
  fontSize: 'clamp(0.9rem, 1.5vw, 1.1rem)',
  cursor: 'pointer',
  borderRadius: '6px',
  border: '1px solid #8B4513',
  background: 'linear-gradient(180deg, #fdf6e3 0%, #e8d5b0 100%)',
  color: '#5D4037',
  fontWeight: 600,
  fontFamily: "'Noto Serif SC', serif",
  letterSpacing: '0.05em',
  boxShadow: '0 2px 4px rgba(0,0,0,0.1), inset 0 1px 0 rgba(255,255,255,0.4)',
  transition: 'all 150ms ease',
};

const HOVER_BOX_SHADOW = '0 4px 8px rgba(0,0,0,0.15), inset 0 1px 0 rgba(255,255,255,0.4)';
const DEFAULT_BOX_SHADOW = '0 2px 4px rgba(0,0,0,0.1), inset 0 1px 0 rgba(255,255,255,0.4)';

// ── Component ──

export function ResetButton({ onClick, label = '新局 · New Game', ariaLabel = 'Reset game' }: ResetButtonProps) {
  const [hovered, setHovered] = useState(false);

  const handleMouseEnter = useCallback(() => setHovered(true), []);
  const handleMouseLeave = useCallback(() => setHovered(false), []);

  return (
    <button
      onClick={onClick}
      aria-label={ariaLabel}
      style={{
        ...BASE_STYLE,
        transform: hovered ? 'translateY(-1px)' : 'translateY(0)',
        boxShadow: hovered ? HOVER_BOX_SHADOW : DEFAULT_BOX_SHADOW,
      }}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {label}
    </button>
  );
}
