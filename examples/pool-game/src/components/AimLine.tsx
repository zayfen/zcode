import { useRef } from 'react'
import * as THREE from 'three'
import { useFrame } from '@react-three/fiber'
import { Line } from '@react-three/drei'
import { useAimStore } from '../store/aimStore'
import { useGameStore } from '../store/gameStore'
import { getCueBallPosition } from './GameController'
import { BALL_RADIUS } from '../constants/table'

/**
 * A line from the cue ball in the aim direction.
 * Also shows a ghost ball at the first intersection with another ball.
 */
export default function AimLine() {
  const direction = useAimStore(s => s.direction)
  const phase = useGameStore(s => s.phase)

  useFrame(() => {
    // nothing needed here, line updates via React re-render
  })

  if (phase !== 'AIMING' && phase !== 'POWER') return null

  const cuePos = getCueBallPosition()
  const endPoint = cuePos.clone().add(direction.clone().multiplyScalar(2.0))

  const linePoints = [
    new THREE.Vector3(cuePos.x, cuePos.y, cuePos.z),
    new THREE.Vector3(endPoint.x, cuePos.y, endPoint.z),
  ]

  return (
    <group>
      <Line
        points={linePoints}
        color="#ffffff"
        opacity={0.6}
        transparent
        lineWidth={1}
        dashed
        dashSize={0.03}
        gapSize={0.02}
      />
      {/* Ghost ball at aim direction end */}
      <mesh position={[cuePos.x + direction.x * 1.5, BALL_RADIUS, cuePos.z + direction.z * 1.5]}>
        <sphereGeometry args={[BALL_RADIUS, 16, 16]} />
        <meshBasicMaterial color="#ffffff" opacity={0.2} transparent />
      </mesh>
    </group>
  )
}
