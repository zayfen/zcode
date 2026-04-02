import { useRef, useMemo } from 'react';
import * as THREE from 'three';
import type { BallId } from '../types';
import { BALL_RADIUS } from '../constants/table';
import { generateBallTexture } from '../utils/textures';

export interface BallProps {
  id: BallId;
  position: [number, number, number];
}

/**
 * A single billiard ball rendered as a sphere with a procedural canvas texture.
 *
 * The texture is generated once per `id` and cached via `useMemo`.
 * The sphere uses the standard BALL_RADIUS constant and a
 * physically-motivated material (low roughness, tiny metalness).
 */
export default function Ball({ id, position: initialPos }: BallProps) {
  const meshRef = useRef<THREE.Mesh>(null);

  const texture = useMemo(() => {
    const canvas = generateBallTexture(id);
    const tex = new THREE.CanvasTexture(canvas);
    tex.colorSpace = THREE.SRGBColorSpace;
    return tex;
  }, [id]);

  return (
    <mesh
      ref={meshRef}
      position={initialPos}
      castShadow
      receiveShadow
    >
      <sphereGeometry args={[BALL_RADIUS, 32, 32]} />
      <meshStandardMaterial
        map={texture}
        roughness={0.15}
        metalness={0.05}
      />
    </mesh>
  );
}
