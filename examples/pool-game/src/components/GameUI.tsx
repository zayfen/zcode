import { useEffect, useState } from 'react'
import { useGameStore } from '../store/gameStore'
import { useAimStore } from '../store/aimStore'
import { getBallGroup, BALL_COLORS, ALL_BALL_IDS } from '../constants/balls'
import type { BallId, BallGroup } from '../types'

/** HTML overlay for all game UI */
export default function GameUI() {
  const phase = useGameStore(s => s.phase)
  const currentPlayer = useGameStore(s => s.currentPlayer)
  const ballGroups = useGameStore(s => s.ballGroups)
  const pocketedBalls = useGameStore(s => s.pocketedBalls)
  const foul = useGameStore(s => s.foul)
  const winner = useGameStore(s => s.winner)
  const ballInHand = useGameStore(s => s.ballInHand)
  const power = useAimStore(s => s.power)
  const resetGame = useGameStore(s => s.resetGame)

  const [foulToast, setFoulToast] = useState<string | null>(null)
  const [showFoul, setShowFoul] = useState(false)

  useEffect(() => {
    if (foul) {
      const messages: Record<string, string> = {
        SCRATCH: 'Scratch! Cue ball pocketed.',
        NO_BALL_HIT: 'Foul! No ball contacted.',
        WRONG_BALL_FIRST: 'Foul! Wrong ball contacted first.',
        NO_RAIL_CONTACT: 'Foul! No rail contacted after hit.',
      }
      setFoulToast(messages[foul] || `Foul: ${foul}`)
      setShowFoul(true)
      const timer = setTimeout(() => setShowFoul(false), 3000)
      return () => clearTimeout(timer)
    }
  }, [foul])

  const groupLabel = (player: 1 | 2): string => {
    const g = player === 1 ? ballGroups.player1 : ballGroups.player2
    if (!g) return ''
    return g === 'solids' ? '(Solids 1-7)' : '(Stripes 9-15)'
  }

  const playerPocketed = (player: 1 | 2): BallId[] => {
    const g = player === 1 ? ballGroups.player1 : ballGroups.player2
    if (!g) return []
    return pocketedBalls.filter(id => {
      const bg = getBallGroup(id)
      return bg === g
    })
  }

  return (
    <div style={{
      position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
      pointerEvents: 'none', fontFamily: 'Arial, sans-serif', color: '#fff',
    }}>
      {/* Player indicators */}
      <div style={{ position: 'absolute', top: 16, left: 16, display: 'flex', gap: 24 }}>
        <PlayerCard
          label={`Player 1 ${groupLabel(1)}`}
          active={currentPlayer === 1 && phase !== 'GAME_OVER'}
          pocketed={playerPocketed(1)}
        />
        <PlayerCard
          label={`Player 2 ${groupLabel(2)}`}
          active={currentPlayer === 2 && phase !== 'GAME_OVER'}
          pocketed={playerPocketed(2)}
        />
      </div>

      {/* Phase indicator */}
      <div style={{
        position: 'absolute', top: 16, right: 16,
        background: 'rgba(0,0,0,0.5)', padding: '6px 14px', borderRadius: 6,
        fontSize: 13, opacity: 0.7,
      }}>
        {phase}
      </div>

      {/* Ball in hand message */}
      {ballInHand && (
        <div style={{
          position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%, -50%)',
          background: 'rgba(255,200,0,0.9)', color: '#000', padding: '10px 20px', borderRadius: 8,
          fontSize: 16, fontWeight: 'bold',
        }}>
          Click on the table to place the cue ball
        </div>
      )}

      {/* Power meter */}
      {(phase === 'POWER' || (phase === 'AIMING' && power > 0)) && (
        <div style={{
          position: 'absolute', bottom: 40, left: '50%', transform: 'translateX(-50%)',
          width: 300, height: 20, background: 'rgba(0,0,0,0.5)', borderRadius: 10, overflow: 'hidden',
        }}>
          <div style={{
            width: `${power * 100}%`, height: '100%',
            background: power < 0.33 ? '#4CAF50' : power < 0.66 ? '#FFC107' : '#F44336',
            borderRadius: 10, transition: 'width 0.05s',
          }} />
        </div>
      )}

      {/* Foul toast */}
      {showFoul && foulToast && (
        <div style={{
          position: 'absolute', top: '20%', left: '50%', transform: 'translateX(-50%)',
          background: 'rgba(220,50,50,0.9)', padding: '12px 24px', borderRadius: 8,
          fontSize: 18, fontWeight: 'bold', animation: 'fadeIn 0.3s',
        }}>
          {foulToast}
        </div>
      )}

      {/* Game over modal */}
      {phase === 'GAME_OVER' && winner && (
        <div style={{
          position: 'absolute', top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(0,0,0,0.7)', display: 'flex', alignItems: 'center', justifyContent: 'center',
          pointerEvents: 'auto',
        }}>
          <div style={{
            background: '#1a1a2e', padding: 40, borderRadius: 16, textAlign: 'center',
            border: '2px solid #FFD700',
          }}>
            <h1 style={{ fontSize: 36, margin: 0, color: '#FFD700' }}>
              🏆 Player {winner} Wins! 🏆
            </h1>
            <p style={{ fontSize: 18, marginTop: 12, opacity: 0.8 }}>
              Congratulations!
            </p>
            <button
              onClick={resetGame}
              style={{
                marginTop: 24, padding: '12px 32px', fontSize: 18,
                background: '#FFD700', color: '#000', border: 'none', borderRadius: 8,
                cursor: 'pointer', fontWeight: 'bold',
              }}
            >
              Play Again
            </button>
          </div>
        </div>
      )}

      {/* Controls hint */}
      <div style={{
        position: 'absolute', bottom: 16, right: 16,
        background: 'rgba(0,0,0,0.4)', padding: '6px 12px', borderRadius: 6, fontSize: 11, opacity: 0.6,
      }}>
        Click & hold to aim/charge · T = top-down view · U = undo
      </div>
    </div>
  )
}

function PlayerCard({ label, active, pocketed }: { label: string; active: boolean; pocketed: BallId[] }) {
  return (
    <div style={{
      background: active ? 'rgba(255,215,0,0.3)' : 'rgba(0,0,0,0.4)',
      border: active ? '2px solid #FFD700' : '2px solid transparent',
      padding: '8px 16px', borderRadius: 8, minWidth: 160,
    }}>
      <div style={{ fontSize: 14, fontWeight: active ? 'bold' : 'normal' }}>
        {label}
      </div>
      <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
        {pocketed.map(id => (
          <div key={id} style={{
            width: 14, height: 14, borderRadius: '50%',
            background: BALL_COLORS[id as BallId],
            border: '1px solid rgba(255,255,255,0.3)',
          }} />
        ))}
      </div>
    </div>
  )
}
