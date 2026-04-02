import { useEffect, useRef } from 'react';
import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';
import { POWER_CHARGE_RATE, MAX_IMPULSE } from '../constants/physics';
import type { Vec3Tuple } from '../types';
import { getBallApi } from './useShotSequence';

/**
 * Hook that manages power charging during the POWER phase.
 *
 * On mousedown (during AIMING):
 *   - Transitions to POWER phase
 *   - Starts charging: power increases 0→1 over ~2 seconds
 *
 * On mouseup (during POWER):
 *   - Fires the shot: applies impulse to cue ball via physics API
 *   - Transitions to SIMULATING
 */
export default function usePower(): void {
  const frameRef = useRef<number>(0);
  const lastTimeRef = useRef<number>(0);

  // ── Charging animation loop ──────────────────────────────────────────────
  useEffect(() => {
    let running = true;

    const animate = (time: number) => {
      if (!running) return;

      const { isCharging, power } = useAimStore.getState();
      const { phase } = useGameStore.getState();

      if (isCharging && phase === 'POWER') {
        const delta = lastTimeRef.current ? (time - lastTimeRef.current) / 1000 : 0;
        const newPower = Math.min(1, power + delta * POWER_CHARGE_RATE);
        useAimStore.getState().setPower(newPower);
      }

      lastTimeRef.current = time;
      frameRef.current = requestAnimationFrame(animate);
    };

    frameRef.current = requestAnimationFrame(animate);

    return () => {
      running = false;
      cancelAnimationFrame(frameRef.current);
    };
  }, []);

  // ── Mouse event handlers ─────────────────────────────────────────────────
  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      // Only left-click
      if (e.button !== 0) return;

      const { phase, ballInHand } = useGameStore.getState();

      // Ball-in-hand placement (IDLE + ballInHand)
      if (ballInHand && phase === 'IDLE') {
        // Ball-in-hand placement is handled by useShotSequence
        // We just skip aim/power for this click
        return;
      }

      // IDLE → AIMING
      if (phase === 'IDLE') {
        useGameStore.getState().startAiming();
        return;
      }

      // AIMING → POWER (start charging)
      if (phase === 'AIMING') {
        useGameStore.getState().startPower();
        useAimStore.getState().startCharging();
      }
    };

    const handleMouseUp = (e: MouseEvent) => {
      if (e.button !== 0) return;

      const { phase } = useGameStore.getState();
      const { power, direction, isCharging } = useAimStore.getState();

      if (phase === 'POWER' && isCharging) {
        // Stop charging
        useAimStore.getState().stopCharging();

        // Calculate impulse vector: direction * power * MAX_IMPULSE
        const impulseMagnitude = power * MAX_IMPULSE;
        const impulse: Vec3Tuple = [
          direction[0] * impulseMagnitude,
          0, // Keep impulse on the table plane (Y = 0)
          direction[2] * impulseMagnitude,
        ];

        // Apply impulse to the cue ball via the physics API registry
        const cueApi = getBallApi(0);
        if (cueApi) {
          cueApi.applyImpulse(impulse);
        }

        // Transition store: POWER → SIMULATING
        useGameStore.getState().shoot(impulse);
      }
    };

    window.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('mouseup', handleMouseUp);

    return () => {
      window.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, []);
}
