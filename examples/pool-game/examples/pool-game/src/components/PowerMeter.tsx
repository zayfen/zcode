// src/components/PowerMeter.tsx
import React from 'react';
import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';

export default function PowerMeter() {
  const power = useAimStore((s) => s.power);
  const phase = useGameStore((s) => s.phase);

  if (phase !== 'POWER' && phase !== 'AIMING') return null;

  const percent = Math.round(power * 100);
  const hue = 120 - power * 120; // green → red
  const barColor = `hsl(${hue}, 80%, 50%)`;

  return (
    <div
      style={{
        position: 'absolute',
        bottom: 40,
        left: '50%',
        transform: 'translateX(-50%)',
        width: 300,
        height: 20,
        background: 'rgba(0,0,0,0.5)',
        borderRadius: 10,
        border: '2px solid rgba(255,255,255,0.3)',
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          width: `${percent}%`,
          height: '100%',
          background: barColor,
          borderRadius: 8,
          transition: 'width 0.05s ease-out',
        }}
      />
      <div
        style={{
          position: 'absolute',
          top: '50%',
          left: '50%',
          transform: 'translate(-50%, -50%)',
          color: 'white',
          fontSize: 12,
          fontWeight: 'bold',
          textShadow: '0 0 4px rgba(0,0,0,0.8)',
        }}
      >
        {phase === 'POWER' ? `${percent}%` : 'Click & hold to charge'}
      </div>
    </div>
  );
}
