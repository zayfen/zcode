// src/components/Table.tsx
import React from 'react';
import * as THREE from 'three';
import {
  TABLE_LENGTH,
  TABLE_WIDTH,
  TABLE_HEIGHT,
  RAIL_HEIGHT,
  RAIL_WIDTH,
  CUSHION_HEIGHT,
  CUSHION_WIDTH,
  POCKET_RADIUS,
  POCKET_POSITIONS,
  BALL_RADIUS,
} from '../constants/table';

function FeltBed() {
  return (
    <mesh position={[0, TABLE_HEIGHT, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
      <planeGeometry args={[TABLE_WIDTH, TABLE_LENGTH]} />
      <meshStandardMaterial color="#0a7e1a" roughness={0.8} />
    </mesh>
  );
}

function Rail({ position, size }: { position: [number, number, number]; size: [number, number, number] }) {
  return (
    <mesh position={position} castShadow receiveShadow>
      <boxGeometry args={size} />
      <meshStandardMaterial color="#5D3A1A" roughness={0.4} />
    </mesh>
  );
}

function Cushion({ position, size }: { position: [number, number, number]; size: [number, number, number] }) {
  return (
    <mesh position={position}>
      <boxGeometry args={size} />
      <meshStandardMaterial color="#0d8c23" roughness={0.7} />
    </mesh>
  );
}

function Pocket({ position }: { position: [number, number, number] }) {
  return (
    <mesh position={[position[0], position[1] + 0.002, position[2]]} rotation={[-Math.PI / 2, 0, 0]}>
      <circleGeometry args={[POCKET_RADIUS, 32]} />
      <meshStandardMaterial color="#111111" />
    </mesh>
  );
}

function DiamondMarker({ position }: { position: [number, number, number] }) {
  return (
    <mesh position={position}>
      <circleGeometry args={[0.008, 4]} />
      <meshStandardMaterial color="#C4A76C" />
    </mesh>
  );
}

export default function Table() {
  const halfL = TABLE_LENGTH / 2;
  const halfW = TABLE_WIDTH / 2;
  const rh = RAIL_HEIGHT;
  const rw = RAIL_WIDTH;
  const tableY = TABLE_HEIGHT;

  return (
    <group>
      {/* Felt bed */}
      <FeltBed />

      {/* Visible felt bed mesh (double-sided for the visual) */}
      <mesh position={[0, tableY + 0.001, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <planeGeometry args={[TABLE_WIDTH - CUSHION_WIDTH * 2, TABLE_LENGTH - CUSHION_WIDTH * 2]} />
        <meshStandardMaterial color="#0a7e1a" roughness={0.8} />
      </mesh>

      {/* Outer frame/border */}
      <mesh position={[0, tableY - 0.02, 0]} receiveShadow>
        <boxGeometry args={[TABLE_WIDTH + rw * 2, 0.04, TABLE_LENGTH + rw * 2]} />
        <meshStandardMaterial color="#3D2410" roughness={0.3} />
      </mesh>

      {/* Table legs (4 corners) */}
      {[
        [-halfW - rw / 2, tableY / 2 - 0.02, -halfL - rw / 2],
        [halfW + rw / 2, tableY / 2 - 0.02, -halfL - rw / 2],
        [-halfW - rw / 2, tableY / 2 - 0.02, halfL + rw / 2],
        [halfW + rw / 2, tableY / 2 - 0.02, halfL + rw / 2],
      ].map((pos, i) => (
        <mesh key={`leg-${i}`} position={pos as [number, number, number]} castShadow>
          <boxGeometry args={[0.06, tableY - 0.04, 0.06]} />
          <meshStandardMaterial color="#3D2410" roughness={0.3} />
        </mesh>
      ))}

      {/* Wooden Rails */}
      <Rail position={[0, tableY + rh / 2, -halfL - rw / 2]} size={[TABLE_WIDTH + rw * 2, rh, rw]} />
      <Rail position={[0, tableY + rh / 2, halfL + rw / 2]} size={[TABLE_WIDTH + rw * 2, rh, rw]} />
      <Rail position={[-halfW - rw / 2, tableY + rh / 2, 0]} size={[rw, rh, TABLE_LENGTH + rw * 2]} />
      <Rail position={[halfW + rw / 2, tableY + rh / 2, 0]} size={[rw, rh, TABLE_LENGTH + rw * 2]} />

      {/* Cushions (inner bouncing surfaces) */}
      {/* Short cushion (negative Z) - top rail solid */}
      <Cushion
        position={[0, tableY + CUSHION_HEIGHT / 2, -halfL + CUSHION_WIDTH / 2]}
        size={[TABLE_WIDTH - POCKET_RADIUS * 2.5, CUSHION_HEIGHT, CUSHION_WIDTH]}
      />

      {/* Short cushion (positive Z) - bottom rail solid */}
      <Cushion
        position={[0, tableY + CUSHION_HEIGHT / 2, halfL - CUSHION_WIDTH / 2]}
        size={[TABLE_WIDTH - POCKET_RADIUS * 2.5, CUSHION_HEIGHT, CUSHION_WIDTH]}
      />

      {/* Long cushion Left - split for side pocket at Z=0 */}
      <Cushion
        position={[-halfW + CUSHION_WIDTH / 2, tableY + CUSHION_HEIGHT / 2, -halfL / 2 - CUSHION_WIDTH / 4]}
        size={[CUSHION_WIDTH, CUSHION_HEIGHT, halfL - POCKET_RADIUS * 1.5]}
      />
      <Cushion
        position={[-halfW + CUSHION_WIDTH / 2, tableY + CUSHION_HEIGHT / 2, halfL / 2 + CUSHION_WIDTH / 4]}
        size={[CUSHION_WIDTH, CUSHION_HEIGHT, halfL - POCKET_RADIUS * 1.5]}
      />

      {/* Long cushion Right - split for side pocket at Z=0 */}
      <Cushion
        position={[halfW - CUSHION_WIDTH / 2, tableY + CUSHION_HEIGHT / 2, -halfL / 2 - CUSHION_WIDTH / 4]}
        size={[CUSHION_WIDTH, CUSHION_HEIGHT, halfL - POCKET_RADIUS * 1.5]}
      />
      <Cushion
        position={[halfW - CUSHION_WIDTH / 2, tableY + CUSHION_HEIGHT / 2, halfL / 2 + CUSHION_WIDTH / 4]}
        size={[CUSHION_WIDTH, CUSHION_HEIGHT, halfL - POCKET_RADIUS * 1.5]}
      />

      {/* Pockets */}
      {POCKET_POSITIONS.map((pos, i) => (
        <Pocket key={`pocket-${i}`} position={pos} />
      ))}

      {/* Diamond markers on rails */}
      {generateDiamondPositions().map((pos, i) => (
        <DiamondMarker key={`diamond-${i}`} position={pos} />
      ))}

      {/* Floor */}
      <mesh position={[0, -0.01, 0]} rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <planeGeometry args={[10, 10]} />
        <meshStandardMaterial color="#2a1810" roughness={0.9} />
      </mesh>
    </group>
  );
}

function generateDiamondPositions(): [number, number, number][] {
  const markers: [number, number, number][] = [];
  const halfL = TABLE_LENGTH / 2;
  const halfW = TABLE_WIDTH / 2;
  const y = TABLE_HEIGHT + RAIL_HEIGHT + 0.001;

  // Long rail diamonds (Z axis) - 7 diamonds per rail
  for (let i = 1; i <= 7; i++) {
    const z = -halfL + (TABLE_LENGTH * i) / 8;
    markers.push([halfW + RAIL_WIDTH / 2, y, z]);
    markers.push([-halfW - RAIL_WIDTH / 2, y, z]);
  }

  // Short rail diamonds (X axis) - 3 diamonds per rail
  for (let i = 1; i <= 3; i++) {
    const x = -halfW + (TABLE_WIDTH * i) / 4;
    markers.push([x, y, -halfL - RAIL_WIDTH / 2]);
    markers.push([x, y, halfL + RAIL_WIDTH / 2]);
  }

  return markers;
}
