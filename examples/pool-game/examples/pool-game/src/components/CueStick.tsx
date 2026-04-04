// src/components/CueStick.tsx
import React, { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useGameStore } from '../store/gameStore';
import { useAimStore } from '../store/aimStore';
import { BALL_RADIUS } from '../constants/table';
import { getRackPositions } from '../constants/balls';

export default function CueStick() {
  const meshRef = useRef<THREE.Group>(null);
  const phase = useGameStore((s) => s.phase);
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const direction = useAimStore((s) => s.direction);
  const power = useAimStore((s) => s.power);

  const cueBallPos = useMemo(() => {
    const positions = getRackPositions();
    const p = positions[0];
    return new THREE.Vector3(p.x, p.y, p.z);
  }, []);

  const isVisible = phase === 'AIMING' || phase === 'POWER';

  useFrame(() => {
    if (!meshRef.current || !isVisible) return;

    // Get cue ball current position from store
    const storePositions = useGameStore.getState().ballPositions;
    const cbPos = storePositions[0];
    if (!cbPos) return;

    const pos = new THREE.Vector3(cbPos[0], cbPos[1], cbPos[2]);
    const dir = new THREE.Vector3(direction.x, 0, direction.z).normalize();

    // Position behind cue ball
    const pullBack = 0.08 + (power * 0.3); // Pull back with power
    const stickPos = pos.clone().add(dir.clone().multiplyScalar(-pullBack));

    meshRef.current.position.copy(stickPos);
    meshRef.current.position.y = pos.y + BALL_RADIUS;

    // Rotate to face aim direction
    const angle = Math.atan2(dir.x, dir.z);
    meshRef.current.rotation.y = angle;
    // Tilt slightly
    meshRef.current.rotation.x = -0.1;
  });

  if (!isVisible) return null;

  return (
    <group ref={meshRef}>
      {/* Cue stick shaft */}
      <mesh position={[0, 0, -0.5]} rotation={[Math.PI / 2, 0, 0]}>
        <cylinderGeometry args={[0.006, 0.01, 1.0, 8]} />
        <meshStandardMaterial color="#8B6914" roughness={0.3} />
      </mesh>
      {/* Cue tip */}
      <mesh position={[0, 0, 0]} rotation={[Math.PI / 2, 0, 0]}>
        <cylinderGeometry args={[0.005, 0.006, 0.02, 8]} />
        <meshStandardMaterial color="#4488AA" roughness={0.5} />
      </mesh>
    </group>
  );
}
