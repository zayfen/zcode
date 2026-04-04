import { useRef, useMemo, useState } from 'react'
import { Canvas, useFrame, useThree } from '@react-three/fiber'
import { OrbitControls } from '@react-three/drei'
import * as THREE from 'three'

type CameraMode = 'orbit' | 'topdown'

interface SceneProps {
  children: React.ReactNode
}

function CameraController() {
  const controlsRef = useRef<any>(null)
  const { camera } = useThree()
  const targetPosition = useRef(new THREE.Vector3(0, 1.5, 1.2))
  const targetLookAt = useRef(new THREE.Vector3(0, 0, 0))
  const [mode, setMode] = useState<CameraMode>('orbit')

  // T key toggle
  useMemo(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 't' || e.key === 'T') {
        setMode(prev => prev === 'orbit' ? 'topdown' : 'orbit')
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [])

  useFrame(() => {
    if (mode === 'topdown') {
      targetPosition.current.set(0, 2.5, 0)
      targetLookAt.current.set(0, 0, 0)
    } else {
      targetPosition.current.set(0, 1.5, 1.2)
    }
    camera.position.lerp(targetPosition.current, 0.05)
    camera.lookAt(targetLookAt.current)
    if (controlsRef.current) {
      controlsRef.current.target.lerp(targetLookAt.current, 0.05)
      controlsRef.current.enabled = mode === 'orbit'
    }
  })

  return (
    <OrbitControls
      ref={controlsRef}
      enablePan={false}
      minPolarAngle={0.3}
      maxPolarAngle={Math.PI / 2.2}
      minDistance={0.8}
      maxDistance={3}
      target={[0, 0, 0]}
    />
  )
}

export default function Scene({ children }: SceneProps) {
  return (
    <Canvas
      shadows
      camera={{ position: [0, 1.5, 1.2], fov: 45, near: 0.01, far: 50 }}
      style={{ width: '100vw', height: '100vh', background: '#1a1a2e' }}
      gl={{ antialias: true, toneMapping: THREE.ACESFilmicToneMapping, toneMappingExposure: 1.2 }}
    >
      {/* Ambient fill light */}
      <ambientLight intensity={0.4} color="#ffffff" />

      {/* Main directional light with shadows */}
      <directionalLight
        position={[0, 3, 0]}
        intensity={1.0}
        castShadow
        shadow-mapSize-width={2048}
        shadow-mapSize-height={2048}
        shadow-camera-near={0.1}
        shadow-camera-far={6}
        shadow-camera-left={-1.5}
        shadow-camera-right={1.5}
        shadow-camera-top={1.5}
        shadow-camera-bottom={-1.5}
        shadow-bias={-0.0001}
        color="#fff5e6"
      />

      {/* Point lights at table ends */}
      <pointLight position={[-0.5, 1.5, -1.0]} intensity={0.5} color="#ffeedd" />
      <pointLight position={[0.5, 1.5, 1.0]} intensity={0.5} color="#ffeedd" />

      <CameraController />
      {children}
    </Canvas>
  )
}
