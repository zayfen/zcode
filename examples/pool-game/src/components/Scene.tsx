import { Canvas, useThree } from '@react-three/fiber';
import { OrbitControls } from '@react-three/drei';
import { useEffect } from 'react';
import Table from './Table';
import Balls from './Balls';
import CueStick from './CueStick';
import AimLine from './AimLine';
import PhysicsWorld from '../physics/PhysicsWorld';
import useSettleDetector from '../hooks/useSettleDetector';
import useShotSequence from '../hooks/useShotSequence';
import { useAimStore } from '../store/aimStore';
import {
  CAMERA_DEFAULT_POSITION,
  CAMERA_FOV,
  FLOOR_Y,
} from '../constants/table';

function GameHooks() {
  useSettleDetector();
  useShotSequence();
  return null;
}

function CameraRefSync() {
  const { camera } = useThree();
  useEffect(() => {
    (window as any).__r3f_camera = camera;
  }, [camera]);
  return null;
}

export default function Scene() {
  const cameraMode = useAimStore((s) => s.cameraMode);

  return (
    <Canvas
      shadows
      camera={{
        position: CAMERA_DEFAULT_POSITION,
        fov: CAMERA_FOV,
        near: 0.01,
        far: 50,
      }}
      style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%' }}
    >
      <color attach="background" args={['#1a1a2e']} />
      <CameraRefSync />

      {/* Lighting */}
      <ambientLight intensity={0.35} />
      <directionalLight
        position={[0, 3, 0.5]}
        intensity={1.2}
        castShadow
        shadow-mapSize-width={2048}
        shadow-mapSize-height={2048}
      />
      <pointLight position={[-0.3, 1.5, -0.7]} intensity={0.5} distance={5} decay={2} />
      <pointLight position={[0.3, 1.5, 0.7]} intensity={0.5} distance={5} decay={2} />

      {/* Physics world */}
      <PhysicsWorld>
        <Table />
        <Balls />
        <CueStick />
        <AimLine />
        <GameHooks />
      </PhysicsWorld>

      {/* Camera controls */}
      {cameraMode === 'orbit' && (
        <OrbitControls
          target={[0, 0, 0]}
          enablePan={false}
          enableDamping
          dampingFactor={0.08}
          minPolarAngle={0.2}
          maxPolarAngle={Math.PI / 2.15}
          minDistance={0.8}
          maxDistance={4.5}
        />
      )}

      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, FLOOR_Y, 0]} receiveShadow>
        <planeGeometry args={[20, 20]} />
        <meshStandardMaterial color="#0e0806" />
      </mesh>

      <fog attach="fog" args={['#1a1a2e', 6, 14]} />
    </Canvas>
  );
}
