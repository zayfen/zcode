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

        return (
          <Ball
            key={id}
            id={id}
            position={pos as [number, number, number]}
          />
        );
      })}
    </group>
  );
}