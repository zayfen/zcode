// src/components/GameInner.tsx
import React, { useRef, useEffect, useCallback } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { useGameStore } from '../store/gameStore';
import { useAimStore } from '../store/aimStore';
import { useAim } from '../hooks/useAim';
import { useBallVelocityTracker } from '../hooks/useSettleDetector';
import {
  BALL_RADIUS,
  TABLE_SURFACE_Y,
  TABLE_LENGTH,
  TABLE_WIDTH,
  POCKET_POSITIONS,
  POCKET_RADIUS,
} from '../constants/table';
import {
  MAX_IMPULSE,
  POWER_CHARGE_DURATION,
  SETTLE_DEBOUNCE_FRAMES,
} from '../constants/physics';
import { BALL_IDS } from '../types';
import PhysicsWorld from '../physics/PhysicsWorld';
import Table from './Table';
import Balls from './Balls';
import CueStick from './CueStick';
import { getRegisteredBallApi } from './Ball';

// Hook to manage power charging and shot execution
function useShotController() {
  const chargeStartRef = useRef<number | null>(null);
  const shotFiredRef = useRef(false);
  const pocketedDuringShotRef = useRef<number[]>([]);
  const { subscribe, allSettled } = useBallVelocityTracker();
  const settleFramesRef = useRef(0);
  const hasSubscribed = useRef(false);

  const phase = useGameStore((s) => s.phase);
  const evaluateShot = useGameStore((s) => s.evaluateShot);

  // Subscribe to ball velocities once APIs are available
  useEffect(() => {
    const timer = setInterval(() => {
      if (hasSubscribed.current) return;
      let count = 0;
      for (const id of BALL_IDS) {
        const api = getRegisteredBallApi(id);
        if (api) {
          subscribe(id, api);
          count++;
        }
      }
      if (count >= 16) {
        hasSubscribed.current = true;
        clearInterval(timer);
      }
    }, 200);
    return () => clearInterval(timer);
  }, [subscribe]);

  useFrame(() => {
    const state = useGameStore.getState();
    const aimState = useAimStore.getState();

    // Handle power charging
    if (state.phase === 'POWER' && aimState.isCharging) {
      if (chargeStartRef.current === null) {
        chargeStartRef.current = performance.now();
        shotFiredRef.current = false;
      }
      const elapsed = performance.now() - chargeStartRef.current;
      const power = Math.min(1, elapsed / POWER_CHARGE_DURATION);
      useAimStore.getState().setPower(power);
    }

    // Handle shot firing (when mouse released during POWER)
    if (
      state.phase === 'POWER' &&
      !aimState.isCharging &&
      chargeStartRef.current !== null &&
      !shotFiredRef.current
    ) {
      shotFiredRef.current = true;
      const power = aimState.power;
      const dir = aimState.direction;

      // Take snapshot for undo
      state.takeSnapshot();

      // Apply impulse to cue ball
      const cueApi = getRegisteredBallApi(0);
      if (cueApi) {
        const impulse: [number, number, number] = [
          dir.x * power * MAX_IMPULSE,
          0,
          dir.z * power * MAX_IMPULSE,
        ];
        cueApi.applyImpulse(impulse, [0, 0, 0]);
        cueApi.wakeUp();
      }

      // Reset tracking refs
      chargeStartRef.current = null;
      pocketedDuringShotRef.current = [];
      settleFramesRef.current = 0;

      // Transition to simulating
      useGameStore.getState().setPhase('SIMULATING');
      useAimStore.getState().resetShot();
    }

    // Handle settling detection during SIMULATING
    if (state.phase === 'SIMULATING') {
      // Check for pocketed balls
      const currentPositions = state.ballPositions;
      const alreadyPocketed = state.pocketedBalls;

      for (const id of BALL_IDS) {
        if (alreadyPocketed.includes(id as any)) continue;
        if (pocketedDuringShotRef.current.includes(id)) continue;
        const pos = currentPositions[id];
        if (!pos) continue;

        for (const pocket of POCKET_POSITIONS) {
          const dx = pos[0] - pocket[0];
          const dz = pos[2] - pocket[2];
          const dist = Math.sqrt(dx * dx + dz * dz);
          if (dist < POCKET_RADIUS * 0.85) {
            pocketedDuringShotRef.current.push(id);
            break;
          }
        }
      }

      // Check if all balls have settled
      if (allSettled()) {
        settleFramesRef.current++;
        if (settleFramesRef.current > SETTLE_DEBOUNCE_FRAMES) {
          const shotPocketed = [...pocketedDuringShotRef.current];
          const foul = shotPocketed.includes(0)
            ? ('SCRATCH' as const)
            : null;

          evaluateShot(shotPocketed as any[], null as any, foul);
          settleFramesRef.current = 0;
          pocketedDuringShotRef.current = [];
        }
      } else {
        settleFramesRef.current = 0;
      }
    }
  });
}

// Ball-in-hand placement component
function BallInHandPlacer() {
  const { camera, pointer, raycaster } = useThree();
  const phase = useGameStore((s) => s.phase);
  const ballInHand = useGameStore((s) => s.ballInHand);

  const tablePlane = useRef(
    new THREE.Plane(new THREE.Vector3(0, 1, 0), -TABLE_SURFACE_Y)
  );

  useFrame(() => {
    const state = useGameStore.getState();
    if (state.phase !== 'IDLE' || !state.ballInHand) return;

    const mouse = new THREE.Vector2(pointer.x, pointer.y);
    raycaster.setFromCamera(mouse, camera);

    const intersection = new THREE.Vector3();
    const hit = raycaster.ray.intersectPlane(tablePlane.current, intersection);

    if (hit) {
      const halfW = TABLE_WIDTH / 2 - 0.06;
      const halfL = TABLE_LENGTH / 2 - 0.06;
      intersection.x = Math.max(-halfW, Math.min(halfW, intersection.x));
      intersection.z = Math.max(-halfL, Math.min(halfL, intersection.z));

      // Update visual position
      state.updateBallPosition(0, [
        intersection.x,
        TABLE_SURFACE_Y + BALL_RADIUS,
        intersection.z,
      ]);

      // Also update physics body position
      const cueApi = getRegisteredBallApi(0);
      if (cueApi) {
        cueApi.position.set(
          intersection.x,
          TABLE_SURFACE_Y + BALL_RADIUS,
          intersection.z
        );
        cueApi.velocity.set(0, 0, 0);
      }
    }
  });

  return null;
}

// Input handler component
function InputHandler() {
  const phase = useGameStore((s) => s.phase);
  const startAiming = useGameStore((s) => s.startAiming);
  const startPower = useGameStore((s) => s.startPower);

  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      if (e.button !== 0) return;
      const state = useGameStore.getState();

      if (state.phase === 'IDLE' && !state.ballInHand) {
        startAiming();
      } else if (state.phase === 'IDLE' && state.ballInHand) {
        // Place cue ball
        const pos = state.ballPositions[0];
        if (pos) {
          const cueApi = getRegisteredBallApi(0);
          if (cueApi) {
            cueApi.position.set(pos[0], pos[1], pos[2]);
            cueApi.velocity.set(0, 0, 0);
            cueApi.angularVelocity.set(0, 0, 0);
            cueApi.wakeUp();
          }
          state.setBallInHand(null);
          startAiming();
        }
      } else if (state.phase === 'AIMING') {
        useAimStore.getState().setIsCharging(true);
        startPower();
      }
    };

    const handleMouseUp = (e: MouseEvent) => {
      if (e.button !== 0) return;
      const state = useGameStore.getState();
      if (state.phase === 'POWER') {
        useAimStore.getState().setIsCharging(false);
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      const state = useGameStore.getState();

      // T key for camera toggle
      if (e.key === 't' || e.key === 'T') {
        useAimStore.getState().toggleCamera();
      }

      // U key for undo
      if (e.key === 'u' || e.key === 'U') {
        if (state.phase === 'AIMING' || state.phase === 'IDLE') {
          state.undo();
        }
      }

      // R key for reset (game over) or place ball (ball in hand)
      if (e.key === 'r' || e.key === 'R') {
        if (state.phase === 'GAME_OVER') {
          state.resetGame();
          // Reset ball positions in physics after a tick
          setTimeout(() => {
            const initState = useGameStore.getState();
            for (const id of BALL_IDS) {
              const api = getRegisteredBallApi(id);
              if (api) {
                const pos = initState.ballPositions[id];
                if (pos) {
                  api.position.set(pos[0], pos[1], pos[2]);
                  api.velocity.set(0, 0, 0);
                  api.angularVelocity.set(0, 0, 0);
                  api.wakeUp();
                }
              }
            }
          }, 100);
        }
      }
    };

    window.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [phase, startAiming, startPower]);

  return null;
}

// Aim line visualization
function AimLine() {
  const phase = useGameStore((s) => s.phase);
  const direction = useAimStore((s) => s.direction);
  const lineRef = useRef<THREE.Line>(null);

  useFrame(() => {
    if (!lineRef.current) return;
    if (phase !== 'AIMING' && phase !== 'POWER') {
      lineRef.current.visible = false;
      return;
    }

    lineRef.current.visible = true;
    const state = useGameStore.getState();
    const cuePos = state.ballPositions[0];
    if (!cuePos) return;

    const geometry = lineRef.current.geometry as THREE.BufferGeometry;
    const points = [
      new THREE.Vector3(
        cuePos[0],
        TABLE_SURFACE_Y + BALL_RADIUS,
        cuePos[2]
      ),
      new THREE.Vector3(
        cuePos[0] + direction.x * 2,
        TABLE_SURFACE_Y + BALL_RADIUS,
        cuePos[2] + direction.z * 2
      ),
    ];
    geometry.setFromPoints(points);
  });

  return (
    <line ref={lineRef as any}>
      <bufferGeometry />
      <lineBasicMaterial color="#ffffff" transparent opacity={0.5} />
    </line>
  );
}

export default function GameInner() {
  useAim();
  useShotController();

  return (
    <>
      <InputHandler />
      <BallInHandPlacer />
      <PhysicsWorld>
        <Table />
        <Balls />
        <CueStick />
        <AimLine />
      </PhysicsWorld>
    </>
  );
}
