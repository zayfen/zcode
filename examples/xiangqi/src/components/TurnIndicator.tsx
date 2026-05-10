import type { Player, GameStatus } from '../engine';

interface TurnIndicatorProps {
  readonly currentPlayer: Player;
  readonly gameStatus: GameStatus;
}

export function TurnIndicator({ currentPlayer, gameStatus }: TurnIndicatorProps) {
  const isGameOver = gameStatus.type !== 'playing';

  if (isGameOver) {
    return null;
  }

  const isRed = currentPlayer === 'red';
  const label = isRed ? "Red's Turn" : "Black's Turn";
  const dotColor = isRed ? 'var(--piece-border-red)' : 'var(--piece-border-black)';
  const textColor = isRed ? 'var(--piece-text-red)' : 'var(--piece-text-black)';

  return (
    <div
      role="status"
      aria-live="polite"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '10px',
        margin: '8px 0',
        padding: '6px 20px',
        borderRadius: '20px',
        background: 'linear-gradient(180deg, #fdf6e3 0%, #f5e6c8 100%)',
        border: '1px solid #d4a76a',
        fontSize: '1.1rem',
        fontWeight: 600,
        color: textColor,
        fontFamily: "'Noto Serif SC', serif",
        userSelect: 'none',
        /* Subtle shadow for depth */
        boxShadow: '0 2px 6px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.5)',
        /* Smooth color transitions on turn change */
        transition: 'color 300ms ease, background 300ms ease',
      }}
    >
      <span
        style={{
          display: 'inline-block',
          width: '14px',
          height: '14px',
          borderRadius: '50%',
          background: dotColor,
          flexShrink: 0,
          /* Dot glow */
          boxShadow: `0 0 6px 1px ${isRed ? 'rgba(179,58,42,0.3)' : 'rgba(44,62,80,0.3)'}`,
          /* Pulse animation for active turn */
          animation: 'turnPulse 2s ease-in-out infinite',
        }}
      />
      {label}

      {/* Inline keyframes for pulse */}
      <style>{`
        @keyframes turnPulse {
          0%, 100% { opacity: 1; transform: scale(1); }
          50% { opacity: 0.7; transform: scale(0.85); }
        }
      `}</style>
    </div>
  );
}
