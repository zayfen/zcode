// src/components/GameInner.tsx
import React, { useRef, useEffect, useCallback } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import { useGameStore } from '../store/gameStore';
import { useAimStore } from '../store/aimStore';
import { useAim } from '../hooks/useAim';
import { useBallVelocityTracker, useSettleDetector } from '../hooks/useSettleDetector';
import { BALL_RADIUS, TABLE_SURFACE_Y, TABLE_LENGTH, TABLE_WIDTH, POCKET_POSITIONS, POCKET_RADIUS } from '../constants/table';
import { MAX_IMPULSE, POWER_CHARGE_DURATION, SETTLE_DEBOUNCE_FRAMES } from '../constants/physics';
import { BALL_IDS } from '../types';
import PhysicsWorld from '../physics/PhysicsWorld';
import Table from './Table';
import Balls from './Balls';
import CueStick from './CueStick';

// Ball API references for physics interaction
const ballApis: { api: any; id: number }[] = [];

export function registerBallApi(id: number, api: any) {
  const existing = ballApis.find(b => b.id === id);
  if (existing) {
    existing.api = api;
  } else {
    ballApis.push({ id, api });
  }
}

export function getBallApi(id: number) {
  return ballApis.find(b => b.id === id)?.api;
}

// Hook to manage power charging and shot execution
function useShotController() {
  const chargeStartRef = useRef<number | null>(null);
  const shotFiredRef = useRef(false);
  const firstContactRef = useRef<number | null>(null);
  const pocketedDuringShotRef = useRef<number[]>([]);
  const { subscribe, allSettled } = useBallVelocityTracker();
  const settleFramesRef = useRef(0);
  const phase = useGameStore(s => s.phase);
  const setPhase = useGameStore(s => s.setPhase);
  const evaluateShot = useGameStore(s => s.evaluateShot);

  // Subscribe to ball velocities
  useEffect(() => {
    for (const ball of ballApis) {
      if (ball.api) {
        subscribe(ball.id, ball.api);
      }
    }
  }, [subscribe]);

  // Track first contact for the shot
  useEffect(() => {
    const checkCollisions = () => {
      if (useGameStore.getState().phase !== 'SIMULATING') return;
      // First contact is tracked via collision subscriptions
    };
    // We use a simpler approach - check first contact during evaluation
  }, []);

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
    if (state.phase === 'POWER' && !aimState.isCharging && chargeStartRef.current !== null && !shotFiredRef.current) {
      shotFiredRef.current = true;
      const power = aimState.power;
      const dir = aimState.direction;

      // Take snapshot for undo
      state.takeSnapshot();

      // Apply impulse to cue ball
      const cueApi = getBallApi(0);
      if (cueApi) {
        const impulse = [
          dir.x * power * MAX_IMPULSE,
          0,
          dir.z * power * MAX_IMPULSE,
        ] as [number, number, number];
        cueApi.applyImpulse(impulse, [0, 0, 0] as [number, number, number]);

        // Wake the ball
        cueApi.wakeUp();
      }

      // Reset
      chargeStartRef.current = null;
      firstContactRef.current = null;
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
      const newlyPocketed: number[] = [];

      for (const id of BALL_IDS) {
        if (alreadyPocketed.includes(id as any)) continue;
        const pos = currentPositions[id];
        if (!pos) continue;

        for (const pocket of POCKET_POSITIONS) {
          const dx = pos[0] - pocket[0];
          const dz = pos[2] - pocket[2];
          const dist = Math.sqrt(dx * dx + dz * dz);
          if (dist < POCKET_RADIUS * 0.85) {
            newlyPocketed.push(id);
            break;
          }
        }
      }

      // Track pocketed during this shot
      for (const pid of newlyPocketed) {
        if (!pocketedDuringShotRef.current.includes(pid)) {
          pocketedDuringShotRef.current.push(pid);
        }
      }

      // Check if all balls have settled
      if (allSettled()) {
        settleFramesRef.current++;
        if (settleFramesRef.current > SETTLE_DEBOUNCE_FRAMES) {
          // Balls have settled - evaluate the shot
          const shotPocketed = [...pocketedDuringShotRef.current];
          const foul = shotPocketed.includes(0) ? 'SCRATCH' as const : null;

          // Determine first contact (simplified - we'll track this differently)
          const firstContact: number | null = firstContactRef.current;

          evaluateShot(shotPocketed as any[], firstContact as any, foul);
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
  const phase = useGameStore(s => s.phase);
  const ballInHand = useGameStore(s => s.ballInHand);
  const setBallInHand = useGameStore(s => s.setBallInHand);
  const updateBallPosition = useGameStore(s => s.updateBallPosition);

  const tablePlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -TABLE_SURFACE_Y);

  useFrame(() => {
    if (phase !== 'IDLE' || !ballInHand) return;

    const mouse = new THREE.Vector2(pointer.x, pointer.y);
    raycaster.setFromCamera(mouse, camera);

    const intersection = new THREE.Vector3();
    const hit = raycaster.ray.intersectPlane(tablePlane, intersection);

    if (hit) {
      // Clamp to table bounds
      const halfW = TABLE_WIDTH / 2 - 0.05;
      const halfL = TABLE_LENGTH / 2 - 0.05;
      intersection.x = Math.max(-halfW, Math.min(halfW, intersection.x));
      intersection.z = Math.max(-halfL, Math.min(halfL, intersection.z));

      // Update ball position for visual
      updateBallPosition(0, [intersection.x, TABLE_SURFACE_Y + BALL_RADIUS, intersection.z]);
    }
  });

  useEffect(() => {
    const handleClick = () => {
      const state = useGameStore.getState();
      if (state.phase === 'IDLE' && state.ballInHand) {
        // Place the cue ball at current position
        const pos = state.ballPositions[0];
        if (pos) {
          setBallInHand(null);
          // Wake up the cue ball body
          const cueApi = getBallApi(0);
          if (cueApi) {
            cueApi.position.set(pos[0], pos[1], pos[2]);
            cueApi.velocity.set(0, 0, 0);
            cueApi.wakeUp();
          }
        }
      }
    };

    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, [setBallInHand]);

  return null;
}

// Input handler component
function InputHandler() {
  const phase = useGameStore(s => s.phase);
  const startAiming = useGameStore(s => s.startAiming);
  const startPower = useGameStore(s => s.startPower);
  const ballInHand = useGameStore(s => s.ballInHand);

  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      const state = useGameStore.getState();
      const aimState = useAimStore.getState();

      if (e.button !== 0) return; // Left click only

      if (state.phase === 'IDLE' && !state.ballInHand) {
        startAiming();
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

      // R key for reset
      if (e.key === 'r' || e.key === 'R') {
        if (state.phase === 'GAME_OVER') {
          state.resetGame();
          // Reset ball positions in physics
          for (const ball of ballApis) {
            if (ball.api) {
              const pos = useGameStore.getState().ballPositions[ball.id];
              if (pos) {
                ball.api.position.set(pos[0], pos[1], pos[2]);
                ball.api.velocity.set(0, 0, 0);
                ball.api.angularVelocity.set(0, 0, 0);
                ball.api.wakeUp();
              }
            }
          }
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
  }, [phase, startAiming, startPower, ballInHand]);

  return null;
}

// Invisible aim line
function AimLine() {
  const phase = useGameStore(s => s.phase);
  const direction = useAimStore(s => s.direction);
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
      new THREE.Vector3(cuePos[0], TABLE_SURFACE_Y + BALL_RADIUS, cuePos[2]),
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
