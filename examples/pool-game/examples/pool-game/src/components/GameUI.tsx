// src/components/GameUI.tsx
import React, { useEffect, useState } from 'react';
import { useGameStore } from '../store/gameStore';
import { BALL_COLORS } from '../constants/balls';
import type { BallId } from '../types';

export default function GameUI() {
  const phase = useGameStore((s) => s.phase);
  const currentPlayer = useGameStore((s) => s.currentPlayer);
  const ballGroups = useGameStore((s) => s.ballGroups);
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const foul = useGameStore((s) => s.foul);
  const winner = useGameStore((s) => s.winner);
  const ballInHand = useGameStore((s) => s.ballInHand);
  const resetGame = useGameStore((s) => s.resetGame);

  const [showFoul, setShowFoul] = useState(false);
  const [foulMessage, setFoulMessage] = useState('');

  useEffect(() => {
    if (foul) {
      const messages: Record<string, string> = {
        SCRATCH: 'Scratch! Cue ball pocketed.',
        NO_BALL_HIT: 'Foul! No ball was hit.',
        WRONG_BALL_FIRST: 'Foul! Wrong ball hit first.',
        NO_RAIL_CONTACT: 'Foul! No rail contact after hit.',
      };
      setFoulMessage(messages[foul] || 'Foul!');
      setShowFoul(true);
      const timer = setTimeout(() => setShowFoul(false), 3000);
      return () => clearTimeout(timer);
    }
  }, [foul]);

  const player1Group = ballGroups.player1;
  const player2Group = ballGroups.player2;

  const getGroupLabel = (group: string | null) => {
    if (!group) return '';
    return group === 'solids' ? 'Solids (1-7)' : 'Stripes (9-15)';
  };

  const player1Pocketed = pocketedBalls.filter((id) => {
    if (player1Group === 'solids') return id >= 1 && id <= 7;
    if (player1Group === 'stripes') return id >= 9 && id <= 15;
    return false;
  });

  const player2Pocketed = pocketedBalls.filter((id) => {
    if (player2Group === 'solids') return id >= 1 && id <= 7;
    if (player2Group === 'stripes') return id >= 9 && id <= 15;
    return false;
  });

  return (
    <div style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', pointerEvents: 'none', fontFamily: 'Arial, sans-serif' }}>
      {/* Player indicators */}
      <div style={{ position: 'absolute', top: 16, left: 16, display: 'flex', gap: 24 }}>
        <div style={{
          padding: '8px 16px',
          borderRadius: 8,
          background: currentPlayer === 1 ? 'rgba(255,255,255,0.2)' : 'rgba(255,255,255,0.05)',
          border: currentPlayer === 1 ? '2px solid #FFD700' : '2px solid transparent',
          color: 'white',
          backdropFilter: 'blur(4px)',
        }}>
          <div style={{ fontWeight: 'bold', fontSize: 14 }}>
            Player 1 {currentPlayer === 1 ? '◄' : ''}
          </div>
          <div style={{ fontSize: 11, opacity: 0.8 }}>
            {player1Group ? getGroupLabel(player1Group) : 'Unassigned'}
          </div>
          <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
            {player1Pocketed.map((id) => (
              <div key={id} style={{
                width: 14, height: 14, borderRadius: '50%',
                background: BALL_COLORS[id],
                border: '1px solid rgba(255,255,255,0.3)',
              }} />
            ))}
          </div>
        </div>

        <div style={{
          padding: '8px 16px',
          borderRadius: 8,
          background: currentPlayer === 2 ? 'rgba(255,255,255,0.2)' : 'rgba(255,255,255,0.05)',
          border: currentPlayer === 2 ? '2px solid #FFD700' : '2px solid transparent',
          color: 'white',
          backdropFilter: 'blur(4px)',
        }}>
          <div style={{ fontWeight: 'bold', fontSize: 14 }}>
            Player 2 {currentPlayer === 2 ? '◄' : ''}
          </div>
          <div style={{ fontSize: 11, opacity: 0.8 }}>
            {player2Group ? getGroupLabel(player2Group) : 'Unassigned'}
          </div>
          <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
            {player2Pocketed.map((id) => (
              <div key={id} style={{
                width: 14, height: 14, borderRadius: '50%',
                background: BALL_COLORS[id],
                border: '1px solid rgba(255,255,255,0.3)',
              }} />
            ))}
          </div>
        </div>
      </div>

      {/* Phase indicator */}
      <div style={{
        position: 'absolute', top: 16, right: 16,
        color: 'rgba(255,255,255,0.5)', fontSize: 12,
        background: 'rgba(0,0,0,0.3)', padding: '4px 12px', borderRadius: 4,
      }}>
        {phase}
      </div>

      {/* Controls hint */}
      <div style={{
        position: 'absolute', bottom: 16, left: 16,
        color: 'rgba(255,255,255,0.4)', fontSize: 11,
        lineHeight: 1.6,
      }}>
        <div>Click & drag to aim</div>
        <div>Hold mouse to charge power</div>
        <div>Release to shoot</div>
        <div>T: Toggle camera | U: Undo | R: Place ball</div>
      </div>

      {/* Foul notification */}
      {showFoul && (
        <div style={{
          position: 'absolute', top: '50%', left: '50%',
          transform: 'translate(-50%, -50%)',
          background: 'rgba(200, 0, 0, 0.8)',
          color: 'white',
          padding: '16px 32px',
          borderRadius: 12,
          fontSize: 18,
          fontWeight: 'bold',
          animation: 'fadeInOut 3s ease',
          backdropFilter: 'blur(8px)',
        }}>
          {foulMessage}
        </div>
      )}

      {/* Ball in hand indicator */}
      {ballInHand && (
        <div style={{
          position: 'absolute', top: '50%', left: '50%',
          transform: 'translate(-50%, -50%)',
          background: 'rgba(0, 100, 200, 0.7)',
          color: 'white',
          padding: '12px 24px',
          borderRadius: 8,
          fontSize: 14,
          marginTop: -40,
        }}>
          Click on table to place cue ball, then press R
        </div>
      )}

      {/* Game over modal */}
      {phase === 'GAME_OVER' && winner && (
        <div style={{
          position: 'absolute', top: 0, left: 0, width: '100%', height: '100%',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          background: 'rgba(0,0,0,0.6)', pointerEvents: 'auto',
        }}>
          <div style={{
            background: 'rgba(30,30,50,0.95)',
            padding: 40,
            borderRadius: 16,
            textAlign: 'center',
            color: 'white',
            border: '2px solid #FFD700',
          }}>
            <h1 style={{ margin: 0, fontSize: 32 }}>🏆</h1>
            <h2 style={{ margin: '8px 0', fontSize: 24 }}>Player {winner} Wins!</h2>
            <button
              onClick={resetGame}
              style={{
                marginTop: 16,
                padding: '12px 32px',
                fontSize: 16,
                background: '#FFD700',
                border: 'none',
                borderRadius: 8,
                cursor: 'pointer',
                fontWeight: 'bold',
                color: '#1a1a2e',
              }}
            >
              Play Again
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
