/**
 * Aim hook — Phase 6 (T18)
 *
 * Uses R3F's `useThree` to raycast from the camera through the current
 * mouse position onto the Y = 0 table plane. The aim direction is computed
 * as the normalized XZ vector from the cue ball position to the ray hit
 * point, then pushed into `aimStore`.
 *
 * Must be used inside a `<Canvas>` component (uses `useThree`).
 *
 * Input flow:
 *   mousemove (window) → computeAim(clientX, clientY) → raycast → setDirection
 */
import { useCallback, useEffect, useRef } from 'react';
import { useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';
import type { Vec3Tuple } from '../types';

export default function useAim(): void {
  const { camera, gl } = useThree();

  // Reusable objects to avoid GC pressure on every mouse move
  const raycasterRef = useRef(new THREE.Raycaster());
  const ndcRef = useRef(new THREE.Vector2());
  const hitRef = useRef(new THREE.Vector3());
  const tablePlane = useRef(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0));

  const computeAim = useCallback(
    (mouseX: number, mouseY: number) => {
      const { phase, cueBallPosition } = useGameStore.getState();
      if (phase !== 'AIMING' && phase !== 'POWER' && phase !== 'IDLE') return;

      const rect = gl.domElement.getBoundingClientRect();
      const ndcX = ((mouseX - rect.left) / rect.width) * 2 - 1;
      const ndcY = -((mouseY - rect.top) / rect.height) * 2 + 1;

      ndcRef.current.set(ndcX, ndcY);
      raycasterRef.current.setFromCamera(ndcRef.current, camera);

      const didHit = raycasterRef.current.ray.intersectPlane(
        tablePlane.current,
        hitRef.current,
      );
      if (!didHit) return;

      // Direction from cue ball to hit point (XZ only)
      const dx = hitRef.current.x - cueBallPosition[0];
      const dz = hitRef.current.z - cueBallPosition[2];

      // Ignore if hit point is too close to cue ball (degenerate)
      const len = Math.sqrt(dx * dx + dz * dz);
      if (len < 0.001) return;

      const aimDir: Vec3Tuple = [dx / len, 0, dz / len];
      useAimStore.getState().setDirection(aimDir);
    },
    [camera, gl],
  );

  // Attach window mousemove listener that feeds into computeAim
  useEffect(() => {
    const onMouseMove = (e: MouseEvent) => {
      computeAim(e.clientX, e.clientY);
    };

    window.addEventListener('mousemove', onMouseMove);
    return () => window.removeEventListener('mousemove', onMouseMove);
  }, [computeAim]);
}
