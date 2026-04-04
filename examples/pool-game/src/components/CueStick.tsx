import { useRef, useMemo } from 'react'
import * as THREE from 'three'
import { useFrame } from '@react-three/fiber'
import { useAimStore } from '../store/aimStore'
import { useGameStore } from '../store/gameStore'
import { getCueBallPosition } from './GameController'
import { BALL_RADIUS } from '../constants/table'

/**
 * A cue stick that follows the aim direction behind the cue ball.
 * Pulls back during POWER phase. Strikes forward on shoot.
 */
export default function CueStick() {
  const meshRef = useRef<THREE.Group>(null)
  const phase = useGameStore(s => s.phase)
  const power = useAimStore(s => s.power)
  const direction = useAimStore(s => s.direction)
  const stickLength = 1.4
  const stickRadius = 0.008

  useFrame(() => {
    if (!meshRef.current) return
    if (phase !== 'AIMING' && phase !== 'POWER') {
      meshRef.current.visible = false
      return
    }

    meshRef.current.visible = true
    const cuePos = getCueBallPosition()

    // Position the stick behind the cue ball
    const pullBack = phase === 'POWER' ? 0.05 + power * 0.3 : 0.05
    const stickCenter = cuePos.clone().add(
      direction.clone().multiplyScalar(-(pullBack + stickLength / 2))
    )

    meshRef.current.position.copy(stickCenter)

    // Rotate to point in aim direction
    const angle = Math.atan2(direction.x, direction.z)
    meshRef.current.rotation.set(0, angle + Math.PI, 0)
  })

  return (
    <group ref={meshRef}>
      {/* Main shaft */}
      <mesh rotation={[Math.PI / 2, 0, 0]}>
        <cylinderGeometry args={[stickRadius, stickRadius * 1.3, stickLength, 8]} />
        <meshStandardMaterial color="#c8a254" roughness={0.4} metalness={0.1} />
      </mesh>
      {/* Tip */}
      <mesh position={[0, 0, stickLength / 2]} rotation={[Math.PI / 2, 0, 0]}>
        <cylinderGeometry args={[stickRadius * 0.8, stickRadius * 0.8, 0.015, 8]} />
        <meshStandardMaterial color="#4488cc" roughness={0.6} />
      </mesh>
    </group>
  )
}
