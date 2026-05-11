import type { GameStatus } from '../engine';
import { ResetButton } from './ResetButton';

interface GameOverModalProps {
  gameStatus: GameStatus;
  onPlayAgain: () => void;
}

export function GameOverModal({ gameStatus, onPlayAgain }: GameOverModalProps) {
  if (gameStatus.type === 'playing') {
    return null;
  }

  const isCheckmate = gameStatus.type === 'checkmate';

  // Determine message and colour for the winning / losing side
  let heading: string;
  let detail: string;
  let detailColor: string;

  if (isCheckmate) {
    const winner = gameStatus.winner;
    heading = '将死！';
    detail = winner === 'red' ? '红方胜！' : '黑方胜！';
    detailColor = winner === 'red' ? '#b33a2a' : '#2c3e50';
  } else {
    const loser = gameStatus.loser;
    heading = '困毙！';
    detail = loser === 'red' ? '红方负！' : '黑方负！';
    detailColor = '#5D3A1A';
  }

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0, 0, 0, 0.45)',
        zIndex: 1000,
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={isCheckmate ? 'Game over — checkmate' : 'Game over — stalemate'}
        style={{
          padding: '36px 48px',
          borderRadius: '12px',
          background: 'linear-gradient(180deg, #fff8ee 0%, #fdf0d5 100%)',
          border: '1px solid #d4a76a',
          boxShadow: '0 8px 32px rgba(0,0,0,0.25)',
          textAlign: 'center',
          minWidth: '260px',
        }}
      >
        <h2
          style={{
            margin: '0 0 8px',
            fontSize: 'clamp(1.4rem, 2.5vw, 2rem)',
            color: '#5D3A1A',
            fontFamily: "'Ma Shan Zheng', 'Noto Serif SC', serif",
            letterSpacing: '0.1em',
          }}
        >
          {heading}
        </h2>

        <p
          style={{
            margin: '0 0 24px',
            fontSize: 'clamp(1rem, 1.8vw, 1.3rem)',
            fontWeight: 700,
            color: detailColor,
          }}
        >
          {detail}
        </p>

        <ResetButton onClick={onPlayAgain} label="再局 · Play Again" ariaLabel="Play Again" />
      </div>
    </div>
  );
}
