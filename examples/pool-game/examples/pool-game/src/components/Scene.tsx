// src/components/Scene.tsx
import React, { useRef } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import * as THREE from 'three';
import { useAimStore } from '../store/aimStore';
import { TABLE_LENGTH, TABLE_WIDTH } from '../constants/table';

function CameraController() {
  const { camera } = useThree();
  const target = useRef(new THREE.Vector3(0, 0, 0));
  const modeRef = useRef<string>('orbit');

  useFrame(() => {
    const mode = useAimStore.getState().cameraMode;
    if (mode !== modeRef.current) {
      modeRef.current = mode;
      if (mode === 'topdown') {
        camera.position.set(0, 3, 0.01);
        camera.lookAt(0, 0, 0);
      }
    }
  });

  return null;
}

export default function Scene({ children }: { children: React.ReactNode }) {
  return (
    <Canvas
      shadows
      camera={{
        position: [0, 2.2, 1.8],
        fov: 45,
        near: 0.01,
        far: 100,
      }}
      style={{ background: '#1a1a2e' }}
    >
      <CameraController />
      <ambientLight intensity={0.4} />
      <directionalLight
        position={[0, 3, 0]}
        intensity={1.2}
        castShadow
        shadow-mapSize-width={2048}
        shadow-mapSize-height={2048}
        shadow-camera-far={10}
        shadow-camera-left={-TABLE_WIDTH}
        shadow-camera-right={TABLE_WIDTH}
        shadow-camera-top={TABLE_LENGTH / 2}
        shadow-camera-bottom={-TABLE_LENGTH / 2}
      />
      <pointLight position={[-TABLE_WIDTH, 2, -TABLE_LENGTH / 2]} intensity={0.5} />
      <pointLight position={[TABLE_WIDTH, 2, TABLE_LENGTH / 2]} intensity={0.5} />

      {children}

      <OrbitControls
        enablePan={false}
        minDistance={0.5}
        maxDistance={5}
        maxPolarAngle={Math.PI / 2 - 0.05}
        target={[0, 0, 0]}
      />
    </Canvas>
  );
}
