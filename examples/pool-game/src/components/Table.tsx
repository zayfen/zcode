import { useRef, useMemo } from 'react'
import * as THREE from 'three'
import {
  TABLE_LENGTH,
  TABLE_WIDTH,
  RAIL_HEIGHT,
  RAIL_THICKNESS,
  FELT_THICKNESS,
  POCKET_RADIUS,
  CUSHION_HEIGHT,
  CUSHION_THICKNESS,
  POCKET_POSITIONS,
} from '../constants/table'
import { generateFeltTexture, generateWoodTexture } from '../utils/textures'

/** The table: felt bed, wooden rails, cushions, and pockets */
export default function Table() {
  const feltTexture = useMemo(() => generateFeltTexture(), [])
  const woodTexture = useMemo(() => generateWoodTexture(), [])

  const halfL = TABLE_LENGTH / 2
  const halfW = TABLE_WIDTH / 2

  // Outer rail positions (the wooden frame)
  const railY = RAIL_HEIGHT / 2 + FELT_THICKNESS

  return (
    <group>
      {/* ===== FELT BED ===== */}
      <mesh position={[0, 0, 0]} receiveShadow>
        <boxGeometry args={[TABLE_LENGTH + RAIL_THICKNESS * 2, FELT_THICKNESS, TABLE_WIDTH + RAIL_THICKNESS * 2]} />
        <meshStandardMaterial color={0x0d6b2e} />
      </mesh>

      {/* Playing surface (slightly above felt) */}
      <mesh position={[0, FELT_THICKNESS / 2 + 0.001, 0]} receiveShadow>
        <boxGeometry args={[TABLE_LENGTH, 0.001, TABLE_WIDTH]} />
        <meshStandardMaterial map={feltTexture} color={0x0d6b2e} />
      </mesh>

      {/* ===== WOODEN RAILS ===== */}
      {/* Long sides (2) */}
      <mesh position={[0, railY, -(halfW + RAIL_THICKNESS / 2)]} castShadow receiveShadow>
        <boxGeometry args={[TABLE_LENGTH + RAIL_THICKNESS * 2, RAIL_HEIGHT, RAIL_THICKNESS]} />
        <meshStandardMaterial map={woodTexture} color={0x5c3a1e} />
      </mesh>
      <mesh position={[0, railY, halfW + RAIL_THICKNESS / 2]} castShadow receiveShadow>
        <boxGeometry args={[TABLE_LENGTH + RAIL_THICKNESS * 2, RAIL_HEIGHT, RAIL_THICKNESS]} />
        <meshStandardMaterial map={woodTexture} color={0x5c3a1e} />
      </mesh>
      {/* Short sides (2) */}
      <mesh position={[-(halfL + RAIL_THICKNESS / 2), railY, 0]} castShadow receiveShadow>
        <boxGeometry args={[RAIL_THICKNESS, RAIL_HEIGHT, TABLE_WIDTH + RAIL_THICKNESS * 2]} />
        <meshStandardMaterial map={woodTexture} color={0x5c3a1e} />
      </mesh>
      <mesh position={[halfL + RAIL_THICKNESS / 2, railY, 0]} castShadow receiveShadow>
        <boxGeometry args={[RAIL_THICKNESS, RAIL_HEIGHT, TABLE_WIDTH + RAIL_THICKNESS * 2]} />
        <meshStandardMaterial map={woodTexture} color={0x5c3a1e} />
      </mesh>

      {/* ===== CUSHIONS (green rubber) ===== */}
      {/* Top side cushions (split by center pocket) */}
      <Cushion
        position={[halfL / 2, CUSHION_HEIGHT / 2 + FELT_THICKNESS, -(halfW - CUSHION_THICKNESS / 2)]}
        size={[halfL * 0.85, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />
      <Cushion
        position={[-halfL / 2, CUSHION_HEIGHT / 2 + FELT_THICKNESS, -(halfW - CUSHION_THICKNESS / 2)]}
        size={[halfL * 0.85, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />
      {/* Bottom side cushions */}
      <Cushion
        position={[halfL / 2, CUSHION_HEIGHT / 2 + FELT_THICKNESS, halfW - CUSHION_THICKNESS / 2]}
        size={[halfL * 0.85, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />
      <Cushion
        position={[-halfL / 2, CUSHION_HEIGHT / 2 + FELT_THICKNESS, halfW - CUSHION_THICKNESS / 2]}
        size={[halfL * 0.85, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />
      {/* Left end cushion */}
      <Cushion
        position={[-(halfL - CUSHION_THICKNESS / 2), CUSHION_HEIGHT / 2 + FELT_THICKNESS, 0]}
        size={[CUSHION_THICKNESS, CUSHION_HEIGHT, TABLE_WIDTH * 0.75]}
      />
      {/* Right end cushion */}
      <Cushion
        position={[halfL - CUSHION_THICKNESS / 2, CUSHION_HEIGHT / 2 + FELT_THICKNESS, 0]}
        size={[CUSHION_THICKNESS, CUSHION_HEIGHT, TABLE_WIDTH * 0.75]}
      />

      {/* ===== POCKETS ===== */}
      {POCKET_POSITIONS.map((pos, i) => (
        <mesh key={i} position={[pos.x, FELT_THICKNESS / 2, pos.z]} rotation={[-Math.PI / 2, 0, 0]}>
          <circleGeometry args={[POCKET_RADIUS, 32]} />
          <meshStandardMaterial color={0x111111} />
        </mesh>
      ))}

      {/* ===== POCKET RIMS ===== */}
      {POCKET_POSITIONS.map((pos, i) => (
        <mesh key={`rim-${i}`} position={[pos.x, FELT_THICKNESS / 2 + 0.002, pos.z]} rotation={[-Math.PI / 2, 0, 0]}>
          <ringGeometry args={[POCKET_RADIUS, POCKET_RADIUS + 0.008, 32]} />
          <meshStandardMaterial color={0x2a1a0a} />
        </mesh>
      ))}

      {/* ===== DIAMOND MARKERS ===== */}
      <DiamondMarkers />

      {/* ===== FLOOR ===== */}
      <mesh position={[0, -0.5, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <planeGeometry args={[10, 10]} />
        <meshStandardMaterial color={0x1a0f06} />
      </mesh>
    </group>
  )
}

/** A single cushion segment */
function Cushion({ position, size }: { position: [number, number, number]; size: [number, number, number] }) {
  return (
    <mesh position={position} castShadow receiveShadow>
      <boxGeometry args={size} />
      <meshStandardMaterial color={0x0a5e28} />
    </mesh>
  )
}

/** Diamond sight markers on rails */
function DiamondMarkers() {
  const diamonds: JSX.Element[] = []
  const halfL = TABLE_LENGTH / 2
  const halfW = TABLE_WIDTH / 2
  const markerY = RAIL_HEIGHT * 0.6 + FELT_THICKNESS
  const markerSize = 0.012

  // Markers along long sides (top and bottom)
  for (let i = 1; i <= 3; i++) {
    const x = (halfL / 4) * i
    // Top
    diamonds.push(
      <mesh key={`t-${i}`} position={[x, markerY, -(halfW + RAIL_THICKNESS * 0.5)]}>
        <sphereGeometry args={[markerSize, 8, 8]} />
        <meshStandardMaterial color={0xccccaa} metalness={0.8} roughness={0.2} />
      </mesh>,
      <mesh key={`t-${-i}`} position={[-x, markerY, -(halfW + RAIL_THICKNESS * 0.5)]}>
        <sphereGeometry args={[markerSize, 8, 8]} />
        <meshStandardMaterial color={0xccccaa} metalness={0.8} roughness={0.2} />
      </mesh>
    )
    // Bottom
    diamonds.push(
      <mesh key={`b-${i}`} position={[x, markerY, halfW + RAIL_THICKNESS * 0.5]}>
        <sphereGeometry args={[markerSize, 8, 8]} />
        <meshStandardMaterial color={0xccccaa} metalness={0.8} roughness={0.2} />
      </mesh>,
      <mesh key={`b-${-i}`} position={[-x, markerY, halfW + RAIL_THICKNESS * 0.5]}>
        <sphereGeometry args={[markerSize, 8, 8]} />
        <meshStandardMaterial color={0xccccaa} metalness={0.8} roughness={0.2} />
      </mesh>
    )
  }

  // Markers along short sides (left and right)
  for (let i = 1; i <= 2; i++) {
    const z = (halfW / 3) * i - halfW / 2
    diamonds.push(
      <mesh key={`l-${i}`} position={[-(halfL + RAIL_THICKNESS * 0.5), markerY, z]}>
        <sphereGeometry args={[markerSize, 8, 8]} />
        <meshStandardMaterial color={0xccccaa} metalness={0.8} roughness={0.2} />
      </mesh>
    )
    diamonds.push(
      <mesh key={`r-${i}`} position={[halfL + RAIL_THICKNESS * 0.5, markerY, z]}>
        <sphereGeometry args={[markerSize, 8, 8]} />
        <meshStandardMaterial color={0xccccaa} metalness={0.8} roughness={0.2} />
      </mesh>
    )
  }

  return <group>{diamonds}</group>
}
