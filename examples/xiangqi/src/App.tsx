import { Board } from './components/Board';
import { CapturedPieces } from './components/CapturedPieces';
import { GameOverModal } from './components/GameOverModal';
import { ResetButton } from './components/ResetButton';
import { TurnIndicator } from './components/TurnIndicator';
import { useGameState } from './hooks/useGameState';

export default function App() {
  const game = useGameState();

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        minHeight: '100vh',
        maxWidth: '100%',
        overflow: 'hidden',
        /* Warm parchment background with subtle gradient */
        background: 'linear-gradient(180deg, #faf0e6 0%, #f0dbbf 40%, #e8d0aa 100%)',
        fontFamily: "'Noto Serif SC', serif",
      }}
    >
      {/* Title */}
      <h1
        style={{
          fontSize: 'clamp(1.1rem, 2vw, 1.8rem)',
          margin: '16px 0 4px',
          color: '#5D3A1A',
          textShadow: '0 1px 2px rgba(0,0,0,0.08)',
          letterSpacing: '0.08em',
          fontFamily: "'Ma Shan Zheng', 'Noto Serif SC', serif",
        }}
      >
        中国象棋 — Xiangqi
      </h1>

      <TurnIndicator
        currentPlayer={game.boardState.currentPlayer}
        gameStatus={game.gameStatus}
      />

      <Board game={game} />

      <CapturedPieces capturedPieces={game.capturedPieces} />

      <GameOverModal gameStatus={game.gameStatus} onPlayAgain={game.resetGame} />

      <ResetButton onClick={game.resetGame} />
    </div>
  );
}
