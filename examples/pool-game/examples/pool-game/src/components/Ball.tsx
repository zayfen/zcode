// src/components/Ball.tsx
import React, { useRef, useState, useEffect } from 'react';
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
  visible?: boolean;
  onPocketed?: (id: number) => void;
}

export default function Ball({ id, position, onPocketed }: BallProps) {
  const pocketedBalls = useGameStore((s) => s.pocketedBalls);
  const isPocketed = pocketedBalls.includes(id as any);
  const [visible, setVisible] = useState(true);

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

  const velocityRef = useRef<[number, number, number]>([0, 0, 0]);
  const positionRef = useRef<[number, number, number]>([...position]);

  useEffect(() => {
    const unsubVel = api.velocity.subscribe((v) => {
      velocityRef.current = [v[0], v[1], v[2]];
    });
    const unsubPos = api.position.subscribe((p) => {
      positionRef.current = [p[0], p[1], p[2]];
    });
    return () => {
      unsubVel();
      unsubPos();
    };
  }, [api]);

  // Pocket detection
  useFrame(() => {
    if (isPocketed) {
      if (visible) {
        setVisible(false);
        api.position.set(0, -1, 0);
        api.velocity.set(0, 0, 0);
        api.angularVelocity.set(0, 0, 0);
        api.mass.set(0);
      }
      return;
    }

    // Check if ball is near a pocket
    const pos = positionRef.current;
    for (const pocket of POCKET_POSITIONS) {
      const dx = pos[0] - pocket[0];
      const dz = pos[2] - pocket[2];
      const dist = Math.sqrt(dx * dx + dz * dz);
      if (dist < POCKET_RADIUS * 0.8) {
        // Ball is in pocket
        if (onPocketed) onPocketed(id);
        break;
      }
    }
  });

  // Subscribe to ball position for game store tracking
  useEffect(() => {
    const unsub = api.position.subscribe((p) => {
      useGameStore.getState().updateBallPosition(id, [p[0], p[1], p[2]]);
    });
    return unsub;
  }, [api, id]);

  const texture = getBallTexture(id);

  if (!visible && isPocketed) return null;

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
