import { useRef, useCallback } from 'react'
import { useFrame } from '@react-three/fiber'
import * as THREE from 'three'
import { getBallPositions, getBallVelocities, getBallAngularVelocities } from '../physics/ballBodies'
import { SETTLE_LINEAR_THRESHOLD, SETTLE_ANGULAR_THRESHOLD, SETTLE_FRAMES } from '../constants/physics'
import { useGameStore } from '../store/gameStore'
import type { BallId, BallState } from '../types'
import { ALL_BALL_IDS } from '../constants/balls'
import { BALL_RADIUS } from '../constants/table'

export function useSettleDetector() {
  const settleCount = useRef(0)
  const phase = useGameStore(s => s.phase)
  const evaluateShot = useGameStore(s => s.evaluateShot)

  useFrame(() => {
    if (phase !== 'SIMULATING') {
      settleCount.current = 0
      return
    }

    const velocities = getBallVelocities()
    const angularVelocities = getBallAngularVelocities()
    
    let allSettled = true
    for (let i = 0; i < velocities.length; i++) {
      const vel = velocities[i]
      const angVel = angularVelocities[i]
      if (!vel || !angVel) continue
      
      const linearSpeed = Math.sqrt(vel[0] ** 2 + vel[1] ** 2 + vel[2] ** 2)
      const angularSpeed = Math.sqrt(angVel[0] ** 2 + angVel[1] ** 2 + angVel[2] ** 2)
      
      if (linearSpeed > SETTLE_LINEAR_THRESHOLD || angularSpeed > SETTLE_ANGULAR_THRESHOLD) {
        allSettled = false
        break
      }
    }

    if (allSettled) {
      settleCount.current++
      if (settleCount.current >= SETTLE_FRAMES) {
        settleCount.current = 0
        evaluateShot()
      }
    } else {
      settleCount.current = 0
    }
  })
}

/** Get current ball states for snapshot */
export function getCurrentBallStates(): BallState[] {
  const positions = getBallPositions()
  const velocities = getBallVelocities()
  const angularVelocities = getBallAngularVelocities()
  
  return ALL_BALL_IDS.map((id) => ({
    id,
    position: positions[id] || [0, BALL_RADIUS, 0],
    velocity: velocities[id] || [0, 0, 0],
    angularVelocity: angularVelocities[id] || [0, 0, 0],
    pocketed: false,
  }))
}
