import { useRef, useEffect, useMemo } from 'react';
import { useSphere } from '@react-three/cannon';
import type { BallId, Vec3Tuple } from '../types';
import { BALL_RADIUS, POCKET_POSITIONS, POCKET_RADIUS } from '../constants/table';
import {
  BALL_MASS,
  LINEAR_DAMPING,
  ANGULAR_DAMPING,
  BALL_BALL_FRICTION,
  BALL_BALL_RESTITUTION,
} from '../constants/physics';
import { useGameStore } from '../store/gameStore';
import { distance } from '../utils/vector';
import { generateBallTexture } from '../utils/textures';
import { registerBallApi, unregisterBallApi } from '../hooks/useShotSequence';
import {
  registerVelocityProvider,
  unregisterVelocityProvider,
} from '../hooks/useSettleDetector';
import * as THREE from 'three';

interface BallBodyProps {
  id: BallId;
  initialPosition: Vec3Tuple;
}

export default function BallBody({ id, initialPosition }: BallBodyProps) {
  const meshRef = useRef<THREE.Mesh>(null);
  const pocketedRef = useRef(false);

  const texture = useMemo(() => {
    const canvas = generateBallTexture(id);
    const tex = new THREE.CanvasTexture(canvas);
    tex.colorSpace = THREE.SRGBColorSpace;
    return tex;
  }, [id]);

  const [, api] = useSphere<THREE.Mesh>(() => ({
    mass: BALL_MASS,
    position: initialPosition,
    args: [BALL_RADIUS],
    material: {
      friction: BALL_BALL_FRICTION,
      restitution: BALL_BALL_RESTITUTION,
    },
    linearDamping: LINEAR_DAMPING,
    angularDamping: ANGULAR_DAMPING,
  }));

  // Register ball API for shot sequence impulse application
  useEffect(() => {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any -- useSphere API type doesn't expose applyImpulse in its TS defs
  registerBallApi(id, api as any);
    return () => unregisterBallApi(id);
  }, [api, id]);

  // Register velocity provider for settle detection
  useEffect(() => {
    let currentVelocity: Vec3Tuple = [0, 0, 0];

    const unsubVelocity = api.velocity.subscribe((v: [number, number, number]) => {
      currentVelocity = v;
    });

    const provider = () => currentVelocity;
    registerVelocityProvider(id, provider);

    return () => {
      unregisterVelocityProvider(id);
      unsubVelocity();
    };
  }, [api, id]);

  // Subscribe to position for pocket detection and state tracking
  useEffect(() => {
    const unsub = api.position.subscribe((pos: [number, number, number]) => {
      if (meshRef.current) {
        meshRef.current.position.set(pos[0], pos[1], pos[2]);
      }

      // Update ball state in store
      const store = useGameStore.getState();
      if (!store.ballStates[id]?.pocketed) {
        // We don't call updateBallState every frame to avoid excessive renders
        // Only update position in ballStates for cue ball (id 0) tracking
        if (id === 0) {
          store.updateBallState(id, pos, store.ballStates[id]?.velocity ?? [0, 0, 0]);
        }
      }

      // Pocket detection during simulation
      const { phase } = useGameStore.getState();
      if (phase === 'SIMULATING' && !pocketedRef.current) {
        for (const pocketPos of POCKET_POSITIONS) {
          const dist = distance(pos, pocketPos);
          if (dist < POCKET_RADIUS) {
            pocketedRef.current = true;
            useGameStore.getState().pocketBall(id);
            // Remove ball from play (drop below table)
            useGameStore.getState().removeBallFromPlay(id);
            api.position.set(pos[0], -0.5, pos[2]);
            api.velocity.set(0, 0, 0);
            api.angularVelocity.set(0, 0, 0);
            break;
          }
        }
      }
    });
    return unsub;
  }, [api, id]);

  // Collision detection for first contact tracking
  useEffect(() => {
    // cannon-es `onCollide` fires when this body contacts another
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const unsub = (api as any).onCollide?.(() => {
      const { phase, firstContact } = useGameStore.getState();
      if (phase !== 'SIMULATING') return;

      // Determine the other ball's id from the collision event
      // cannon-es bodies created by @react-three/cannon don't have a
      // reliable "name" by default, so we use the ball API registry
      // to map bodies to IDs. As a simpler approach, we use a dedicated
      // collision tracking system.

      // For the cue ball (id 0), we track first contact with any object ball
      if (id === 0 && firstContact === null) {
        // The collision event from cannon-es includes e.body which is the
        // other body. We need to identify which ball it is.
        // @react-three/cannon doesn't expose a clean body→id mapping,
        // so we use the position-based approach: check which ball is closest
        // to the cue ball at the moment of collision.

        const store = useGameStore.getState();
        const cuePos = store.ballStates[0]?.position;
        if (!cuePos) return;

        let closestId: BallId | null = null;
        let closestDist = Infinity;

        for (let ballId = 1; ballId <= 15; ballId++) {
          const ballState = store.ballStates[ballId as BallId];
          if (!ballState || ballState.pocketed) continue;

          const dx = cuePos[0] - ballState.position[0];
          const dz = cuePos[2] - ballState.position[2];
          const dist = Math.sqrt(dx * dx + dz * dz);

          if (dist < closestDist) {
            closestDist = dist;
            closestId = ballId as BallId;
          }
        }

        // Only register if close enough (within 2 ball diameters)
        if (closestId !== null && closestDist < BALL_RADIUS * 4) {
          useGameStore.getState().setFirstContact(closestId);
        }
      }
    });

    return () => {
      if (typeof unsub === 'function') unsub();
    };
  }, [api, id]);

  return (
    <mesh ref={meshRef} position={initialPosition} castShadow receiveShadow>
      <sphereGeometry args={[BALL_RADIUS, 32, 32]} />
      <meshStandardMaterial
        map={texture}
        roughness={0.15}
        metalness={0.05}
      />
    </mesh>
  );
}