import { useEffect, useState } from 'react';
import { useGameStore } from '../store/gameStore';
import { useAimStore } from '../store/aimStore';
import { BALL_COLORS } from '../constants/balls';
import { getBallGroup } from '../types';
import type { BallId, Player } from '../types';
import PowerMeter from './PowerMeter';

export default function GameUI() {
  const phase = useGameStore((s) => s.phase);
  const currentPlayer = useGameStore((s) => s.currentPlayer);
  const playerGroups = useGameStore((s) => s.playerGroups);
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const foul = useGameStore((s) => s.foul);
  const winner = useGameStore((s) => s.winner);
  const ballInHand = useGameStore((s) => s.ballInHand);

  const [foulToast, setFoulToast] = useState<string | null>(null);

  useEffect(() => {
    if (foul) {
      const messages: Record<string, string> = {
        SCRATCH: 'Scratch! Cue ball pocketed.',
        NO_BALL_HIT: 'Foul! No ball contacted.',
        WRONG_BALL_FIRST: 'Foul! Wrong ball hit first.',
        NO_RAIL_CONTACT: 'Foul! No rail after contact.',
        EIGHT_EARLY: 'Foul! 8-ball pocketed too early.',
      };
      setFoulToast(messages[foul] || `Foul: ${foul}`);
      const timer = setTimeout(() => setFoulToast(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [foul]);

  const handlePlayAgain = () => {
    useGameStore.getState().resetGame();
    useAimStore.getState().resetPower();
  };

  const player1Balls = pocketedBalls.filter((id) => {
    const group = playerGroups[1];
    if (!group) return false;
    return getBallGroup(id) === group;
  });

  const player2Balls = pocketedBalls.filter((id) => {
    const group = playerGroups[2];
    if (!group) return false;
    return getBallGroup(id) === group;
  });

  return (
    <>
      <PowerMeter />

      {/* Player indicators */}
      <div
        aria-label="Game status"
        style={{
          position: 'fixed',
          top: '16px',
          left: '50%',
          transform: 'translateX(-50%)',
          display: 'flex',
          gap: '40px',
          fontFamily: 'system-ui, sans-serif',
          zIndex: 10,
        }}
      >
        <PlayerIndicator
          player={1}
          isCurrent={currentPlayer === 1}
          group={playerGroups[1]}
          pocketedBalls={player1Balls}
        />
        <PlayerIndicator
          player={2}
          isCurrent={currentPlayer === 2}
          group={playerGroups[2]}
          pocketedBalls={player2Balls}
        />
      </div>

      {/* Phase indicator */}
      <div
        aria-label="Current player"
        style={{
          position: 'fixed',
          top: '80px',
          left: '50%',
          transform: 'translateX(-50%)',
          color: '#888',
          fontFamily: 'system-ui, sans-serif',
          fontSize: '12px',
          zIndex: 10,
        }}
      >
        {phase}
      </div>

      {/* Foul toast */}
      {foulToast && (
        <div
          aria-live="assertive"
          aria-label="Foul notification"
          style={{
            position: 'fixed',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            background: 'rgba(220, 38, 38, 0.9)',
            color: 'white',
            padding: '16px 32px',
            borderRadius: '8px',
            fontFamily: 'system-ui, sans-serif',
            fontSize: '18px',
            fontWeight: 'bold',
            zIndex: 20,
            animation: 'fadeIn 0.3s',
          }}
        >
          {foulToast}
        </div>
      )}

      {/* Ball-in-hand hint */}
      {ballInHand && (
        <div
          style={{
            position: 'fixed',
            bottom: '80px',
            left: '50%',
            transform: 'translateX(-50%)',
            color: '#eab308',
            fontFamily: 'system-ui, sans-serif',
            fontSize: '16px',
            zIndex: 10,
          }}
        >
          Click on the table to place the cue ball
        </div>
      )}

      {/* Controls hint */}
      <div
        style={{
          position: 'fixed',
          bottom: '16px',
          right: '16px',
          color: '#666',
          fontFamily: 'system-ui, sans-serif',
          fontSize: '11px',
          zIndex: 10,
        }}
      >
        <div>Click: Aim | Hold: Power | T: Top-down view | U: Undo</div>
      </div>

      {/* Game over modal */}
      {phase === 'GAME_OVER' && winner && (
        <div
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0, 0, 0, 0.7)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 30,
          }}
        >
          <div
            role="dialog"
            aria-label="Game over"
            style={{
              background: '#1e293b',
              padding: '40px 60px',
              borderRadius: '16px',
              textAlign: 'center',
              fontFamily: 'system-ui, sans-serif',
              color: 'white',
            }}
          >
            <h2 style={{ fontSize: '32px', margin: '0 0 16px' }}>
              Player {winner} Wins!
            </h2>
            <button
              aria-label="Play again"
              onClick={handlePlayAgain}
              style={{
                background: '#3b82f6',
                color: 'white',
                border: 'none',
                padding: '12px 32px',
                borderRadius: '8px',
                fontSize: '18px',
                cursor: 'pointer',
              }}
            >
              Play Again
            </button>
          </div>
        </div>
      )}
    </>
  );
}

function PlayerIndicator({
  player,
  isCurrent,
  group,
  pocketedBalls,
}: {
  player: Player;
  isCurrent: boolean;
  group: string | null;
  pocketedBalls: BallId[];
}) {
  const groupLabel = group
    ? group === 'solids'
      ? 'Solids (1-7)'
      : 'Stripes (9-15)'
    : 'Not assigned';

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        opacity: isCurrent ? 1 : 0.5,
        transform: isCurrent ? 'scale(1.1)' : 'scale(1)',
        transition: 'all 0.2s',
      }}
    >
      <div
        style={{
          color: isCurrent ? '#fff' : '#999',
          fontWeight: isCurrent ? 'bold' : 'normal',
          fontFamily: 'system-ui, sans-serif',
          fontSize: '16px',
          marginBottom: '4px',
        }}
      >
        Player {player}
      </div>
      <div
        style={{
          color: '#888',
          fontFamily: 'system-ui, sans-serif',
          fontSize: '11px',
          marginBottom: '4px',
        }}
      >
        {groupLabel}
      </div>
      <div
        aria-label="Pocketed balls"
        style={{
          display: 'flex',
          gap: '4px',
        }}
      >
        {pocketedBalls.map((id) => (
          <div
            key={id}
            style={{
              width: '16px',
              height: '16px',
              borderRadius: '50%',
              background: BALL_COLORS[id as BallId],
              border: '1px solid rgba(255,255,255,0.3)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: '8px',
              color: id === 8 ? '#fff' : '#000',
              fontWeight: 'bold',
            }}
          >
            {id}
          </div>
        ))}
      </div>
    </div>
  );
}
