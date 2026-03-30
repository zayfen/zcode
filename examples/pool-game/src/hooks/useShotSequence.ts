import { useEffect, useRef } from 'react';
import type * as THREE from 'three';
import { useGameStore } from '../store/gameStore';
import { useAimStore } from '../store/aimStore';
import { evaluateShotResult } from '../game-logic/rules';
import type { ShotEvaluationInput } from '../game-logic/rules';
import type { Vec3Tuple } from '../types';
import { MAX_IMPULSE } from '../constants/physics';

// ─────────────────────────────────────────────────────────────────────────────
// Ball API registry — physics bodies register their cannon-es API handles
// here so the shot sequence can apply impulses to the cue ball.
// ─────────────────────────────────────────────────────────────────────────────

export interface BallPhysicsApi {
  applyImpulse: (impulse: Vec3Tuple) => void;
  position: { subscribe: (cb: (v: Vec3Tuple) => void) => () => void };
  velocity: { subscribe: (cb: (v: Vec3Tuple) => void) => () => void };
}

const ballApis = new Map<number, BallPhysicsApi>();

export function registerBallApi(id: number, api: BallPhysicsApi): void {
  ballApis.set(id, api);
}

export function unregisterBallApi(id: number): void {
  ballApis.delete(id);
}

/**
 * Get the physics API for a ball (used by other systems).
 */
export function getBallApi(id: number): BallPhysicsApi | undefined {
  return ballApis.get(id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Shot sequence orchestrator
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Hook that orchestrates the full shot lifecycle:
 *
 * 1. Listens for mouse/keyboard input to drive phase transitions
 *    (IDLE → AIMING → POWER → SIMULATING)
 * 2. Applies the cue-ball impulse on shoot
 * 3. After settle detection transitions to EVALUATING, runs the rules engine
 * 4. Applies evaluation results (fouls, group assignment, game over, next turn)
 *
 * Also integrates the useAim hook for camera-based raycasting on mousemove.
 */
export default function useShotSequence(): void {
  const evaluatedRef = useRef(false);

  // ── Watch for EVALUATING phase and run rules ────────────────────────────
  useEffect(() => {
    let frameId: ReturnType<typeof requestAnimationFrame>;

    const tick = () => {
      const state = useGameStore.getState();

      if (state.phase === 'EVALUATING' && !evaluatedRef.current) {
        evaluatedRef.current = true;

        // Determine if this is the break shot
        const isBreak = state.pocketedBalls.length === 0 && state.shotHistory.length === 0;

        // Build evaluation input using current store state
        // allPocketedBalls: balls pocketed BEFORE this shot
        const allPocketedBeforeShot = state.shotHistory.length > 0
          ? state.shotHistory[0].state.pocketedBalls
          : [];

        const input: ShotEvaluationInput = {
          currentPlayer: state.currentPlayer,
          playerGroups: { ...state.playerGroups },
          pocketedThisShot: [...state.ballsPocketedThisShot],
          firstContact: state.firstContact,
          railContact: state.railContacted,
          allPocketedBalls: allPocketedBeforeShot,
          isBreak,
        };

        const result = evaluateShotResult(input);

        // ── Apply results to store ────────────────────────────────────────

        // Foul
        if (result.foul) {
          useGameStore.getState().setFoul(result.foul);
        }

        // Group assignment
        if (result.assignGroup) {
          useGameStore.getState().assignGroups(
            result.assignGroup.player,
            result.assignGroup.group,
          );
        }

        // Game over
        if (result.gameOver && result.winner !== null) {
          useGameStore.getState().setGameOver(result.winner);
          evaluatedRef.current = false;
          return; // stop processing
        }

        // Ball-in-hand
        if (result.ballInHand) {
          useGameStore.getState().setBallInHand(true);
        }

        // Transition to next turn
        useGameStore.getState().nextTurn(result.nextPlayer);

        // Reset aim power for next shot
        useAimStore.getState().resetPower();

        // Reset evaluation guard after a short delay to allow state to settle
        setTimeout(() => {
          evaluatedRef.current = false;
        }, 100);
      }

      frameId = requestAnimationFrame(tick);
    };

    frameId = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frameId);
  }, []);

  // ── Input handling: mouse & keyboard ────────────────────────────────────
  useEffect(() => {
    const canvas = document.querySelector('canvas');
    if (!canvas) return;

    // ── Helpers for raycasting from camera to Y=0 plane ──────────────────
    // We lazily import THREE to avoid requiring it at module level in this hook
    let raycaster: THREE.Raycaster | null = null;
    let plane: THREE.Plane | null = null;
    let vec2: THREE.Vector2 | null = null;
    let vec3: THREE.Vector3 | null = null;

    const getThreeObjects = async () => {
      if (raycaster) return;
      const THREE = await import('three');
      raycaster = new THREE.Raycaster();
      plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
      vec2 = new THREE.Vector2();
      vec3 = new THREE.Vector3();
    };

    // Pre-initialize
    getThreeObjects();

    /**
     * Compute aim direction from mouse position via camera raycasting.
     * Falls back to simple NDC-based mapping if Three.js not ready.
     */
    const computeAimFromMouse = (e: MouseEvent): Vec3Tuple | null => {
      const rect = canvas.getBoundingClientRect();
      const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      const ndcY = -((e.clientY - rect.top) / rect.height) * 2 + 1;

      const store = useGameStore.getState();
      const cuePos = store.cueBallPosition;

      if (raycaster && plane) {
        // Camera-based raycasting onto Y=0 table plane
        vec2!.set(ndcX, ndcY);

        // Get camera from the R3F store — we access it through a global reference
        // set by the Scene component
        const camera = (window as any).__r3f_camera as THREE.Camera;
        if (camera) {
          raycaster.setFromCamera(vec2!, camera);
          const hit = raycaster.ray.intersectPlane(plane, vec3!);
          if (hit) {
            const dx = hit.x - cuePos[0];
            const dz = hit.z - cuePos[2];
            const len = Math.sqrt(dx * dx + dz * dz);
            if (len > 0.001) {
              return [dx / len, 0, dz / len];
            }
          }
        }
      }

      // Fallback: simple NDC angle-based aim
      const angle = ndcX * Math.PI;
      return [Math.sin(angle), 0, Math.cos(angle)];
    };

    // ── Mouse down ───────────────────────────────────────────────────────
    const handleMouseDown = (e: MouseEvent) => {
      // Ignore right-click
      if (e.button !== 0) return;

      const { phase, ballInHand } = useGameStore.getState();

      // Ball-in-hand placement
      if (ballInHand && phase === 'IDLE') {
        const rect = canvas.getBoundingClientRect();
        const ndcX = ((e.clientX - rect.left) / rect.width) * 2 - 1;
        const ndcY = -((e.clientY - rect.top) / rect.height) * 2 + 1;

        // Map NDC to approximate table coordinates
        const x = ndcX * 0.56; // HALF_WIDTH
        const z = -ndcY * 1.12; // HALF_LENGTH

        useGameStore.getState().placeBallInHand([x, 0.0285, z]);
        return;
      }

      if (phase === 'IDLE') {
        // IDLE → AIMING
        const aimDir = computeAimFromMouse(e);
        if (aimDir) {
          useAimStore.getState().setDirection(aimDir);
        }
        useGameStore.getState().startAiming();
      } else if (phase === 'AIMING') {
        // AIMING → POWER (start charging)
        useGameStore.getState().startPower();
        useAimStore.getState().startCharging();
      }
    };

    // ── Mouse up ─────────────────────────────────────────────────────────
    const handleMouseUp = (e: MouseEvent) => {
      if (e.button !== 0) return;

      const { phase } = useGameStore.getState();
      const { power, direction, isCharging } = useAimStore.getState();

      if (phase === 'POWER' && isCharging) {
        // Calculate impulse vector
        const impulseMagnitude = power * MAX_IMPULSE;
        const impulse: Vec3Tuple = [
          direction[0] * impulseMagnitude,
          0,
          direction[2] * impulseMagnitude,
        ];

        // Apply impulse to the cue ball via physics API
        const cueApi = ballApis.get(0);
        if (cueApi) {
          cueApi.applyImpulse(impulse);
        }

        // Transition store: POWER → SIMULATING (also takes snapshot internally)
        useGameStore.getState().shoot(impulse);
        useAimStore.getState().stopCharging();
      }
    };

    // ── Mouse move: update aim direction during AIMING/POWER ─────────────
    const handleMouseMove = (e: MouseEvent) => {
      const { phase } = useGameStore.getState();
      if (phase !== 'AIMING' && phase !== 'POWER' && phase !== 'IDLE') return;

      const aimDir = computeAimFromMouse(e);
      if (aimDir) {
        useAimStore.getState().setDirection(aimDir);
      }
    };

    // ── Keyboard shortcuts ───────────────────────────────────────────────
    const handleKeyDown = (e: KeyboardEvent) => {
      // Undo (U)
      if (e.key === 'u' || e.key === 'U') {
        const snapshot = useGameStore.getState().undo();
        if (snapshot) {
          useGameStore.getState().restoreSnapshot(snapshot);
          useAimStore.getState().resetPower();
        }
      }

      // Camera toggle (T)
      if (e.key === 't' || e.key === 'T') {
        useAimStore.getState().toggleCameraMode();
      }

      // Reset game (R)
      if (e.key === 'r' || e.key === 'R') {
        useGameStore.getState().resetGame();
        useAimStore.getState().resetPower();
      }
    };

    window.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);
}
