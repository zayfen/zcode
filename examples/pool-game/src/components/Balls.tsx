import { useMemo } from 'react'
import { ALL_BALL_IDS, getRackPositions } from '../constants/balls'
import type { BallId } from '../types'
import Ball from './Ball'

/** Renders all 16 balls at their initial rack positions */
export default function Balls() {
  const positions = useMemo(() => getRackPositions(), [])
  const ballIds = useMemo(() => ALL_BALL_IDS as BallId[], [])

  return (
    <group>
      {ballIds.map(id => (
        <Ball
          key={id}
          id={id}
          position={positions[id]}
        />
      ))}
    </group>
  )
}
