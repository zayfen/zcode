// src/components/Ball.tsx
import React, { useRef, useState, useEffect, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import { useSphere } from '@react-three/cannon';
import * as THREE from 'three';
import { BALL_RADIUS, TABLE_SURFACE_Y, POCKET_POSITIONS, POCKET_RADIUS } from '../constants/table';
import { BALL_MASS, LINEAR_DAMPING, ANGULAR_DAMPING } from '../constants/physics';
import { getBallTexture } from '../utils/textures';
import { useGameStore } from '../store/gameStore';

interface BallProps {
  id: number;
  position: [number, number, number];
}

// Global registry for ball APIs
const ballApiRegistry = new Map<number, any>();

export function registerBallApi(id: number, api: any) {
  ballApiRegistry.set(id, api);
}

export function unregisterBallApi(id: number) {
  ballApiRegistry.delete(id);
}

export function getRegisteredBallApi(id: number) {
  return ballApiRegistry.get(id);
}

export function getAllBallApis() {
  return Array.from(ballApiRegistry.entries()).map(([id, api]) => ({ id, api }));
}

export default function Ball({ id, position }: BallProps) {
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const isPocketed = pocketedBalls.includes(id as any);
  const [hidden, setHidden] = useState(false);

  const [ref, api] = useSphere<THREE.Mesh>(() => ({
    mass: BALL_MASS,
    position,
    args: [BALL_RADIUS],
    material: {
      friction: 0.3,
      restitution: 0.9,
    },
    linearDamping: LINEAR_DAMPING,
    angularDamping: ANGULAR_DAMPING,
    allowSleep: true,
    sleepSpeedLimit: 0.1,
    sleepTimeLimit: 1,
  }));

  // Register API for shot controller
  useEffect(() => {
    registerBallApi(id, api);
    return () => {
      unregisterBallApi(id);
    };
  }, [api, id]);

  const positionRef = useRef<[number, number, number]>([...position]);

  useEffect(() => {
    const unsubPos = api.position.subscribe((p) => {
      positionRef.current = [p[0], p[1], p[2]];
      // Track position in game store
      useGameStore.getState().updateBallPosition(id, [p[0], p[1], p[2]]);
    });
    return unsubPos;
  }, [api, id]);

  // Handle pocketed state
  useFrame(() => {
    if (isPocketed && !hidden) {
      setHidden(true);
      api.position.set(0, -1, 0);
      api.velocity.set(0, 0, 0);
      api.angularVelocity.set(0, 0, 0);
      try {
        api.mass.set(0);
      } catch {}
    }
  });

  const texture = useMemo(() => getBallTexture(id), [id]);

  if (hidden && isPocketed) return null;

  return (
    <mesh ref={ref} castShadow receiveShadow>
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
