// src/components/Balls.tsx
import React from 'react';
import { BALL_IDS } from '../types';
import { BALL_RADIUS } from '../constants/table';
import { useGameStore } from '../store/gameStore';
import Ball from './Ball';

export default function Balls() {
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const positions = useGameStore((s) => s.ballPositions);
  const ballInHand = useGameStore((s) => s.ballInHand);
  const ballInHandPosition = useGameStore((s) => s.ballInHandPosition);

  return (
    <group>
      {BALL_IDS.map((id) => {
        const pos = id === 0 && ballInHand && ballInHandPosition
          ? ballInHandPosition
          : positions[id] || [0, BALL_RADIUS, 0];
        const isPocketed = pocketedBalls.includes(id);

        if (isPocketed) {
          return (
            <Ball
              key={id}
              id={id}
              position={[pos[0], -0.5, pos[2]]}
              onPocketed={() => {}}
            />
          );
        }

        return (
          <Ball
            key={id}
            id={id}
            position={pos}
            onPocketed={() => {}}
          />
        );
      })}
    </group>
  );
}
