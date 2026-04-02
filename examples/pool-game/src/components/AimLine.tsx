import { useRef } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useAimStore } from '../store/aimStore';
import { useGameStore } from '../store/gameStore';
import { add, scale } from '../utils/vector';
import { BALL_RADIUS } from '../constants/table';

const LINE_LENGTH = 2.0;

export default function AimLine() {
  const groupRef = useRef<THREE.Group>(null);
  const lineRef = useRef<THREE.LineSegments>(null);
  const ghostRef = useRef<THREE.Mesh>(null);

  useFrame(() => {
    if (!groupRef.current || !ghostRef.current) return;

    const { phase } = useGameStore.getState();
    const { direction } = useAimStore.getState();

    if (phase !== 'AIMING' && phase !== 'POWER' && phase !== 'IDLE') {
      groupRef.current.visible = false;
      return;
    }

    groupRef.current.visible = true;

    const { cueBallPosition } = useGameStore.getState();
    const endPoint = add(cueBallPosition, scale(direction, LINE_LENGTH));

    // Update line geometry
    if (lineRef.current) {
      const positions = lineRef.current.geometry.attributes.position as THREE.BufferAttribute;
      if (positions) {
        positions.setXYZ(0, cueBallPosition[0], 0.01, cueBallPosition[2]);
        positions.setXYZ(1, endPoint[0], 0.01, endPoint[2]);
        positions.needsUpdate = true;
      }
    }

    // Ghost ball at projected contact point
    ghostRef.current.visible = phase === 'AIMING';
    if (phase === 'AIMING') {
      const ghostPos = add(cueBallPosition, scale(direction, 0.5));
      ghostRef.current.position.set(ghostPos[0], BALL_RADIUS, ghostPos[2]);
    }
  });

  // Create a simple line geometry with 2 points
  const geometry = new THREE.BufferGeometry();
  const positions = new Float32Array(6); // 2 points * 3 components
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));

  return (
    <group ref={groupRef}>
      <lineSegments ref={lineRef} geometry={geometry}>
        <lineBasicMaterial color="#ffffff" opacity={0.5} transparent />
      </lineSegments>
      <mesh ref={ghostRef} visible={false}>
        <sphereGeometry args={[BALL_RADIUS, 16, 16]} />
        <meshStandardMaterial color="#ffffff" opacity={0.2} transparent />
      </mesh>
    </group>
  );
}
