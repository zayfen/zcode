// src/components/Ball.tsx
import React from 'react';
import * as THREE from 'three';
import { BALL_RADIUS } from '../constants/table';
import { getBallTexture } from '../utils/textures';
import { useGameStore } from '../store/gameStore';

interface BallProps {
  id: number;
  position: [number, number, number];
  visible?: boolean;
  onPocketed?: (id: number) => void;
}

export default function Ball({ id, position }: BallProps) {
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const isPocketed = pocketedBalls.includes(id as any);

  if (isPocketed) return null;

  const texture = getBallTexture(id);

  return (
    <mesh position={position} castShadow receiveShadow>
      <sphereGeometry args={[BALL_RADIUS, 32, 32]} />
      <meshStandardMaterial
        map={texture}
        roughness={0.15}
        metalness={0.0}
        envMapIntensity={0.5}
      />
    </mesh>
  );
}
