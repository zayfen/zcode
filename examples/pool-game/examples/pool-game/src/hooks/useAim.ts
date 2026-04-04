// src/hooks/useAim.ts
import { useCallback } from 'react';
import { useThree, useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';
import { TABLE_SURFACE_Y, TABLE_LENGTH, TABLE_WIDTH } from '../constants/table';

export function useAim() {
  const { camera, pointer, raycaster } = useThree();
  const setDirection = useAimStore((s) => s.setDirection);
  const phase = useGameStore((s) => s.phase);
  const ballPositions = useGameStore((s) => s.ballPositions);

  const tablePlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -TABLE_SURFACE_Y);

  const updateAim = useCallback(() => {
    if (phase !== 'AIMING' && phase !== 'POWER') return;

    // Get cue ball position
    const cuePos = ballPositions[0];
    if (!cuePos) return;

    // Raycast from camera through mouse onto table plane
    const mouse = new THREE.Vector2(pointer.x, pointer.y);
    raycaster.setFromCamera(mouse, camera);

    const intersection = new THREE.Vector3();
    const ray = raycaster.ray;
    const hit = ray.intersectPlane(tablePlane, intersection);

    if (hit) {
      // Clamp to table bounds
      intersection.x = Math.max(-TABLE_WIDTH / 2, Math.min(TABLE_WIDTH / 2, intersection.x));
      intersection.z = Math.max(-TABLE_LENGTH / 2, Math.min(TABLE_LENGTH / 2, intersection.z));

      // Direction from cue ball to intersection point (on table surface plane)
      const dir = {
        x: intersection.x - cuePos[0],
        y: 0,
        z: intersection.z - cuePos[2],
      };

      // Normalize
      const len = Math.sqrt(dir.x * dir.x + dir.z * dir.z);
      if (len > 0.001) {
        setDirection({ x: dir.x / len, y: 0, z: dir.z / len });
      }
    }
  }, [phase, ballPositions, camera, pointer, raycaster, setDirection, tablePlane]);

  useFrame(() => {
    updateAim();
  });

  return null;
}
