import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import { useGameStore } from '../store/gameStore';
import { SETTLE_LINEAR_THRESHOLD, SETTLE_FRAMES } from '../constants/physics';
import type { Vec3Tuple } from '../types';

// ─────────────────────────────────────────────────────────────────────────────
// Ball velocity registry — physics bodies subscribe their velocity callbacks
// here so the settle detector (running inside useFrame) can poll them.
// ─────────────────────────────────────────────────────────────────────────────

type VelocityProvider = () => Vec3Tuple;

const velocityProviders = new Map<number, VelocityProvider>();

/**
 * Register a function that returns the current velocity of ball `id`.
 * Called once from BallBody's useEffect.
 */
export function registerVelocityProvider(id: number, provider: VelocityProvider): void {
  velocityProviders.set(id, provider);
}

/**
 * Unregister when the ball component unmounts.
 */
export function unregisterVelocityProvider(id: number): void {
  velocityProviders.delete(id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Hook
// ─────────────────────────────────────────────────────────────────────────────

/**
 * `useSettleDetector` — runs inside the R3F render loop and watches every
 * active ball's linear velocity magnitude.  Once **all** balls are below
 * `SETTLE_LINEAR_THRESHOLD` for `SETTLE_FRAMES` consecutive frames (while
 * the game phase is `SIMULATING`), it transitions the store to `EVALUATING`.
 *
 * Must be used inside a `<Canvas>` component.
 */
export default function useSettleDetector(): void {
  const settleCountRef = useRef(0);

  useFrame(() => {
    const { phase } = useGameStore.getState();

    // Only active during SIMULATING
    if (phase !== 'SIMULATING') {
      settleCountRef.current = 0;
      return;
    }

    // Check every registered ball velocity
    let allSettled = true;

    for (const provider of velocityProviders.values()) {
      const vel = provider();
      const speed = Math.sqrt(vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]);
      if (speed > SETTLE_LINEAR_THRESHOLD) {
        allSettled = false;
        break;
      }
    }

    if (allSettled && velocityProviders.size > 0) {
      settleCountRef.current++;
      if (settleCountRef.current >= SETTLE_FRAMES) {
        settleCountRef.current = 0;
        // Transition to EVALUATING phase — rules engine will run next
        useGameStore.getState().evaluateShot();
      }
    } else {
      // Reset the counter if any ball is still moving
      settleCountRef.current = 0;
    }
  });
}
