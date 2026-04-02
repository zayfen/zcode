import { useMemo } from 'react';
import { ALL_BALL_IDS } from '../types';
import type { BallId } from '../types';
import { getRackPositions } from '../constants/balls';
import Ball from './Ball';

/**
 * Container component that renders all 16 balls at their initial positions.
 * Maps ball IDs 0–15, rendering a <Ball> for each at INITIAL_POSITIONS.
 */
export default function Balls() {
  const initialPositions = useMemo(() => getRackPositions(), []);

  return (
    <group>
      {ALL_BALL_IDS.map((id: BallId) => (
        <Ball
          key={id}
          id={id}
          position={initialPositions[id]}
        />
      ))}
    </group>
  );
}
