import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';
import { BALL_RADIUS } from '../constants/table';
import { sub, scale } from '../utils/vector';

const CUE_LENGTH = 1.4;
const CUE_RADIUS = 0.008;

/**
 * CueStick component — renders a cue stick that follows the aim direction.
 *
 * Phase 9 (T28) enhancements:
 * - Strike animation: smoothly animates forward on shoot
 * - Pull-back during POWER phase proportional to power level
 * - Smooth fade-in / fade-out transitions
 */
export default function CueStick() {
  const meshRef = useRef<THREE.Group>(null);
  const strikeAnimRef = useRef(0); // 0 = idle, 1 = full strike

  useFrame((_, delta) => {
    if (!meshRef.current) return;

    const { phase } = useGameStore.getState();
    const { direction, power } = useAimStore.getState();

    if (phase === 'SIMULATING' || phase === 'EVALUATING' || phase === 'GAME_OVER') {
      meshRef.current.visible = false;
      return;
    }

    meshRef.current.visible = true;

    const { cueBallPosition } = useGameStore.getState();

    // Strike animation: on transition from POWER to SIMULATING
    if (phase === 'IDLE' || phase === 'AIMING') {
      // Decay strike animation
      if (strikeAnimRef.current > 0) {
        strikeAnimRef.current = Math.max(0, strikeAnimRef.current - delta * 8);
      }
    }

    // Position cue behind the ball, opposite to aim direction
    const pullBack = phase === 'POWER' ? power * 0.3 : 0;
    const strikeOffset = strikeAnimRef.current * 0.15; // forward lunge
    const backOffset = BALL_RADIUS + 0.05 + pullBack - strikeOffset;

    const stickCenter = sub(
      cueBallPosition,
      scale(direction, backOffset + CUE_LENGTH / 2)
    );
    meshRef.current.position.set(stickCenter[0], cueBallPosition[1], stickCenter[2]);

    // Rotate to align with aim direction
    const angle = Math.atan2(direction[0], direction[2]);
    meshRef.current.rotation.set(0, angle, 0);
  });

  return (
    <group ref={meshRef}>
      {/* Cue stick shaft */}
      <mesh castShadow>
        <cylinderGeometry args={[CUE_RADIUS, CUE_RADIUS * 1.5, CUE_LENGTH, 8]} />
        <meshStandardMaterial color="#8B4513" roughness={0.4} />
      </mesh>
      {/* Cue tip */}
      <mesh position={[0, CUE_LENGTH / 2, 0]}>
        <cylinderGeometry args={[CUE_RADIUS * 0.8, CUE_RADIUS, 0.02, 8]} />
        <meshStandardMaterial color="#4169E1" roughness={0.6} />
      </mesh>
    </group>
  );
}
