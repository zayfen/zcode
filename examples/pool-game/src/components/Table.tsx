import { useMemo } from 'react';
import * as THREE from 'three';
import {
  TABLE_LENGTH,
  TABLE_WIDTH,
  RAIL_HEIGHT,
  RAIL_WIDTH,
  HALF_LENGTH,
  HALF_WIDTH,
  POCKET_POSITIONS,
  POCKET_RADIUS,
  CUSHION_THICKNESS,
  CUSHION_HEIGHT,
  FLOOR_Y,
} from '../constants/table';
import { generateFeltTexture, generateWoodTexture } from '../utils/textures';

/**
 * Standard diamond sight positions on a pool table:
 * - Each long rail has 7 diamonds (3 per half + center)
 * - Each short rail has 3 diamonds
 * - Evenly spaced along the playing surface edge
 */

/** Diamond sight marker component */
function DiamondSight({
  position,
  rotation,
}: {
  position: [number, number, number];
  rotation?: [number, number, number];
}) {
  // Small diamond shape (~8mm tall) on the rail top surface
  const d = 0.008; // diamond half-size
  return (
    <mesh position={position} rotation={rotation} renderOrder={1}>
      {/* Use a small rotated plane with diamond-shaped geometry approximated by a box */}
      <boxGeometry args={[d * 2, 0.002, d * 2]} />
      <meshStandardMaterial
        color="#c4a265"
        roughness={0.4}
        metalness={0.2}
      />
    </mesh>
  );
}

export default function Table() {
  const feltTexture = useMemo(() => {
    const canvas = generateFeltTexture();
    const tex = new THREE.CanvasTexture(canvas);
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
    tex.repeat.set(4, 8);
    return tex;
  }, []);

  const woodTexture = useMemo(() => {
    const canvas = generateWoodTexture();
    const tex = new THREE.CanvasTexture(canvas);
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
    return tex;
  }, []);

  // Long rail half-length (between side-pocket gap and corner-pocket gap)
  const longSegmentLength = HALF_LENGTH - RAIL_WIDTH * 1.2;
  // Short rail full length
  const shortRailLength = TABLE_WIDTH;

  // ── Diamond sight positions ──
  // Long rails: 7 diamonds evenly spaced along the Z-axis playing surface
  // Short rails: 3 diamonds evenly spaced along the X-axis playing surface
  const diamondY = RAIL_HEIGHT + 0.001; // sit on top of rail
  const longRailInset = RAIL_WIDTH * 0.35; // how far into the rail from edge

  // Long rail diamond Z positions (7 marks: divide playing length into 8 segments)
  const longDiamondZs = useMemo(() => {
    const positions: number[] = [];
    const segCount = 8;
    for (let i = 1; i < segCount; i++) {
      positions.push(-HALF_LENGTH + (TABLE_LENGTH / segCount) * i);
    }
    return positions;
  }, []);

  // Short rail diamond X positions (3 marks: divide playing width into 4 segments)
  const shortDiamondXs = useMemo(() => {
    const positions: number[] = [];
    const segCount = 4;
    for (let i = 1; i < segCount; i++) {
      positions.push(-HALF_WIDTH + (TABLE_WIDTH / segCount) * i);
    }
    return positions;
  }, []);

  return (
    <group>
      {/* ── Table Bed (green felt surface) ── */}
      <mesh position={[0, -0.005, 0]} receiveShadow>
        <boxGeometry args={[TABLE_WIDTH, 0.01, TABLE_LENGTH]} />
        <meshStandardMaterial color="#0d6b2e" map={feltTexture} roughness={0.9} />
      </mesh>

      {/* ── Wooden frame beneath bed ── */}
      <mesh position={[0, -0.025, 0]} receiveShadow>
        <boxGeometry
          args={[
            TABLE_WIDTH + RAIL_WIDTH * 2,
            0.03,
            TABLE_LENGTH + RAIL_WIDTH * 2,
          ]}
        />
        <meshStandardMaterial color="#3b1f0b" map={woodTexture} roughness={0.7} />
      </mesh>

      {/* ── Wooden Rails ── */}

      {/* Left rail – two segments (gap at middle for side pocket) */}
      <RailSegment
        position={[
          -HALF_WIDTH - RAIL_WIDTH / 2,
          RAIL_HEIGHT / 2,
          -HALF_LENGTH / 2 - RAIL_WIDTH / 4,
        ]}
        size={[RAIL_WIDTH, RAIL_HEIGHT, longSegmentLength]}
        texture={woodTexture}
      />
      <RailSegment
        position={[
          -HALF_WIDTH - RAIL_WIDTH / 2,
          RAIL_HEIGHT / 2,
          HALF_LENGTH / 2 + RAIL_WIDTH / 4,
        ]}
        size={[RAIL_WIDTH, RAIL_HEIGHT, longSegmentLength]}
        texture={woodTexture}
      />

      {/* Right rail – two segments (gap at middle for side pocket) */}
      <RailSegment
        position={[
          HALF_WIDTH + RAIL_WIDTH / 2,
          RAIL_HEIGHT / 2,
          -HALF_LENGTH / 2 - RAIL_WIDTH / 4,
        ]}
        size={[RAIL_WIDTH, RAIL_HEIGHT, longSegmentLength]}
        texture={woodTexture}
      />
      <RailSegment
        position={[
          HALF_WIDTH + RAIL_WIDTH / 2,
          RAIL_HEIGHT / 2,
          HALF_LENGTH / 2 + RAIL_WIDTH / 4,
        ]}
        size={[RAIL_WIDTH, RAIL_HEIGHT, longSegmentLength]}
        texture={woodTexture}
      />

      {/* Head rail (near end, Z negative) */}
      <RailSegment
        position={[0, RAIL_HEIGHT / 2, -HALF_LENGTH - RAIL_WIDTH / 2]}
        size={[shortRailLength, RAIL_HEIGHT, RAIL_WIDTH]}
        texture={woodTexture}
      />

      {/* Foot rail (far end, Z positive) */}
      <RailSegment
        position={[0, RAIL_HEIGHT / 2, HALF_LENGTH + RAIL_WIDTH / 2]}
        size={[shortRailLength, RAIL_HEIGHT, RAIL_WIDTH]}
        texture={woodTexture}
      />

      {/* ── Diamond Sight Markers ── */}

      {/* Left rail diamonds */}
      {longDiamondZs.map((z, i) => (
        <DiamondSight
          key={`left-diamond-${i}`}
          position={[-HALF_WIDTH - longRailInset, diamondY, z]}
          rotation={[0, Math.PI / 4, 0]}
        />
      ))}

      {/* Right rail diamonds */}
      {longDiamondZs.map((z, i) => (
        <DiamondSight
          key={`right-diamond-${i}`}
          position={[HALF_WIDTH + longRailInset, diamondY, z]}
          rotation={[0, Math.PI / 4, 0]}
        />
      ))}

      {/* Head rail diamonds */}
      {shortDiamondXs.map((x, i) => (
        <DiamondSight
          key={`head-diamond-${i}`}
          position={[x, diamondY, -HALF_LENGTH - RAIL_WIDTH * 0.35]}
          rotation={[0, Math.PI / 4, 0]}
        />
      ))}

      {/* Foot rail diamonds */}
      {shortDiamondXs.map((x, i) => (
        <DiamondSight
          key={`foot-diamond-${i}`}
          position={[x, diamondY, HALF_LENGTH + RAIL_WIDTH * 0.35]}
          rotation={[0, Math.PI / 4, 0]}
        />
      ))}

      {/* ── Cushions (green rubber bumpers, slightly angled inward) ── */}

      {/* Head-end left cushion */}
      <CushionSegment
        position={[
          -HALF_WIDTH / 2 - RAIL_WIDTH * 0.3,
          CUSHION_HEIGHT / 2,
          -HALF_LENGTH + CUSHION_THICKNESS / 2 + 0.005,
        ]}
        size={[HALF_WIDTH - RAIL_WIDTH * 1.4, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />

      {/* Head-end right cushion */}
      <CushionSegment
        position={[
          HALF_WIDTH / 2 + RAIL_WIDTH * 0.3,
          CUSHION_HEIGHT / 2,
          -HALF_LENGTH + CUSHION_THICKNESS / 2 + 0.005,
        ]}
        size={[HALF_WIDTH - RAIL_WIDTH * 1.4, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />

      {/* Foot-end left cushion */}
      <CushionSegment
        position={[
          -HALF_WIDTH / 2 - RAIL_WIDTH * 0.3,
          CUSHION_HEIGHT / 2,
          HALF_LENGTH - CUSHION_THICKNESS / 2 - 0.005,
        ]}
        size={[HALF_WIDTH - RAIL_WIDTH * 1.4, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />

      {/* Foot-end right cushion */}
      <CushionSegment
        position={[
          HALF_WIDTH / 2 + RAIL_WIDTH * 0.3,
          CUSHION_HEIGHT / 2,
          HALF_LENGTH - CUSHION_THICKNESS / 2 - 0.005,
        ]}
        size={[HALF_WIDTH - RAIL_WIDTH * 1.4, CUSHION_HEIGHT, CUSHION_THICKNESS]}
      />

      {/* Left side cushion */}
      <CushionSegment
        position={[
          -HALF_WIDTH + CUSHION_THICKNESS / 2 + 0.005,
          CUSHION_HEIGHT / 2,
          0,
        ]}
        size={[
          CUSHION_THICKNESS,
          CUSHION_HEIGHT,
          TABLE_LENGTH - RAIL_WIDTH * 3,
        ]}
      />

      {/* Right side cushion */}
      <CushionSegment
        position={[
          HALF_WIDTH - CUSHION_THICKNESS / 2 - 0.005,
          CUSHION_HEIGHT / 2,
          0,
        ]}
        size={[
          CUSHION_THICKNESS,
          CUSHION_HEIGHT,
          TABLE_LENGTH - RAIL_WIDTH * 3,
        ]}
      />

      {/* ── Pockets (6 dark circles) ── */}
      {POCKET_POSITIONS.map((pos, i) => (
        <group key={i} position={[pos[0], 0.001, pos[2]]}>
          {/* Pocket hole */}
          <mesh rotation={[-Math.PI / 2, 0, 0]}>
            <circleGeometry args={[POCKET_RADIUS, 32]} />
            <meshStandardMaterial color="#050505" roughness={1} />
          </mesh>
          {/* Pocket rim */}
          <mesh rotation={[-Math.PI / 2, 0, 0]}>
            <ringGeometry args={[POCKET_RADIUS, POCKET_RADIUS + 0.006, 32]} />
            <meshStandardMaterial color="#2a1506" roughness={0.6} />
          </mesh>
        </group>
      ))}

      {/* ── Dark floor plane beneath table ── */}
      <mesh
        rotation={[-Math.PI / 2, 0, 0]}
        position={[0, FLOOR_Y, 0]}
        receiveShadow
      >
        <planeGeometry args={[12, 14]} />
        <meshStandardMaterial color="#0a0604" roughness={0.95} />
      </mesh>

      {/* ── Table legs (4 cylindrical legs) ── */}
      <TableLeg position={[-HALF_WIDTH - 0.01, FLOOR_Y / 2, -HALF_LENGTH - 0.01]} />
      <TableLeg position={[HALF_WIDTH + 0.01, FLOOR_Y / 2, -HALF_LENGTH - 0.01]} />
      <TableLeg position={[-HALF_WIDTH - 0.01, FLOOR_Y / 2, HALF_LENGTH + 0.01]} />
      <TableLeg position={[HALF_WIDTH + 0.01, FLOOR_Y / 2, HALF_LENGTH + 0.01]} />
    </group>
  );
}

/* ────────────────────────────────────────────────────────────────────────────
   Sub-components
   ──────────────────────────────────────────────────────────────────────────── */

function RailSegment({
  position,
  size,
  texture,
}: {
  position: [number, number, number];
  size: [number, number, number];
  texture: THREE.Texture;
}) {
  return (
    <mesh position={position} castShadow receiveShadow>
      <boxGeometry args={size} />
      <meshStandardMaterial map={texture} color="#5C3317" roughness={0.65} />
    </mesh>
  );
}

function CushionSegment({
  position,
  size,
}: {
  position: [number, number, number];
  size: [number, number, number];
}) {
  return (
    <mesh position={position}>
      <boxGeometry args={size} />
      <meshStandardMaterial color="#0a8a3a" roughness={0.85} />
    </mesh>
  );
}

function TableLeg({
  position,
}: {
  position: [number, number, number];
}) {
  const legHeight = Math.abs(position[1]) * 2;
  return (
    <mesh position={position} castShadow receiveShadow>
      <cylinderGeometry args={[0.03, 0.035, legHeight, 12]} />
      <meshStandardMaterial color="#3b1f0b" roughness={0.7} />
    </mesh>
  );
}
