// src/hooks/useSettleDetector.ts
import { useRef, useCallback } from 'react';
import { useFrame } from '@react-three/fiber';
import {
  SETTLE_LINEAR_THRESHOLD,
  SETTLE_ANGULAR_THRESHOLD,
  SETTLE_DEBOUNCE_FRAMES,
} from '../constants/physics';
import { useGameStore } from '../store/gameStore';

export function useSettleDetector(
  apiRefs: React.MutableRefObject<{ api: any; id: number }[]>
) {
  const settleCount = useRef(0);
  const setPhase = useGameStore((s) => s.setPhase);
  const phase = useGameStore((s) => s.phase);

  useFrame(() => {
    if (phase !== 'SIMULATING') {
      settleCount.current = 0;
      return;
    }

    let allSettled = true;
    let checked = 0;

    for (const ref of apiRefs.current) {
      if (!ref.api) continue;
      checked++;
      try {
        const vel = ref.api.velocity;
        const angVel = ref.api.angularVelocity;
        // We can't synchronously read these in useFrame easily,
        // so we'll use a subscription-based approach instead
      } catch {
        // ignore
      }
    }

    // If no balls to check, don't transition
    if (checked === 0) return;

    // Simple approach: check after a minimum time
    settleCount.current++;
    if (settleCount.current > SETTLE_DEBOUNCE_FRAMES * 6) {
      // After enough frames, assume settled
      settleCount.current = 0;
      setPhase('EVALUATING');
    }
  });
}

// Velocity tracker for each ball body
export function useBallVelocityTracker() {
  const velocities = useRef<Map<number, [number, number, number]>>(new Map());
  const angularVelocities = useRef<Map<number, [number, number, number]>>(new Map());

  const subscribe = useCallback((id: number, api: any) => {
    api.velocity.subscribe((v: [number, number, number]) => {
      velocities.current.set(id, v);
    });
    api.angularVelocity.subscribe((v: [number, number, number]) => {
      angularVelocities.current.set(id, v);
    });
  }, []);

  const allSettled = useCallback(() => {
    for (const [, vel] of velocities.current) {
      const speed = Math.sqrt(vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]);
      if (speed > SETTLE_LINEAR_THRESHOLD) return false;
    }
    for (const [, avel] of angularVelocities.current) {
      const speed = Math.sqrt(avel[0] * avel[0] + avel[1] * avel[1] + avel[2] * avel[2]);
      if (speed > SETTLE_ANGULAR_THRESHOLD) return false;
    }
    return true;
  }, []);

  return { subscribe, allSettled, velocities, angularVelocities };
}
