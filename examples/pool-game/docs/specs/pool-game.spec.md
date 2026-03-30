# Technical Specification: 3D 8-Ball Pool Game

## Tech Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Vite | ^6.x | Build tool and dev server |
| React | ^19.x | UI framework |
| TypeScript | ^5.x | Type safety |
| Three.js | ^0.170 | 3D rendering engine |
| @react-three/fiber | ^9.x | React renderer for Three.js |
| @react-three/drei | ^10.x | Three.js helpers (OrbitControls, etc.) |
| cannon-es | ^0.20 | Physics engine (rigid body simulation) |
| @react-three/cannon | ^6.x | React bindings for cannon-es |
| Zustand | ^5.x | State management |

## Architecture

### File Structure

```
examples/pool-game/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── docs/                          # Documentation (created in Pass 1)
│   ├── prd/
│   ├── specs/
│   ├── tasks/
│   ├── validation.md
│   └── review-checklist.md
└── src/
    ├── main.tsx                   # React entry point
    ├── App.tsx                    # Root component, wires Scene + UI
    ├── types/
    │   └── index.ts               # BallId, GamePhase, Player, BallGroup, Foul, ShotResult
    ├── constants/
    │   ├── table.ts               # Table dimensions, pocket positions, cushion geometry
    │   ├── physics.ts             # Friction, restitution, mass, gravity, materials
    │   └── balls.ts               # Ball colors, radius, rack positions, stripe mask
    ├── store/
    │   ├── gameStore.ts           # Zustand: game state machine + actions
    │   └── aimStore.ts            # Zustand: aim direction, power level, cue position
    ├── components/
    │   ├── Scene.tsx              # Canvas, lighting, camera, Physics provider
    │   ├── Table.tsx              # Table bed, rails, cushions, pockets (visual)
    │   ├── Ball.tsx               # Single ball mesh with procedural texture
    │   ├── Balls.tsx              # Renders all 16 balls from store state
    │   ├── CueStick.tsx           # Cue stick mesh, follows aim direction
    │   ├── AimLine.tsx            # Dashed line showing projected path + ghost ball
    │   ├── PowerMeter.tsx         # On-screen power bar (HTML overlay)
    │   └── GameUI.tsx             # HTML overlay: player info, pocketed balls, modals
    ├── physics/
    │   ├── PhysicsWorld.tsx       # Physics provider with configured materials
    │   ├── BallBody.tsx           # useSphere body for a single ball
    │   └── pocketDetection.ts     # Distance-based pocket check (runs each frame)
    ├── game-logic/
    │   ├── rules.ts               # 8-ball rules engine: validate shots, detect fouls
    │   ├── rack.ts                # Triangle rack position calculator
    │   └── shotController.ts      # Orchestrates shot: apply impulse → simulate → evaluate
    ├── hooks/
    │   ├── useAim.ts              # Computes aim direction from mouse position
    │   ├── usePower.ts            # Manages power charge on mouse hold
    │   ├── useSettleDetector.ts   # Detects when all balls have stopped moving
    │   └── useShotSequence.ts     # Full shot lifecycle hook
    └── utils/
        ├── vector.ts              # vec3 helpers: distance, normalize, reflect, lerp
        └── textures.ts            # Procedural canvas texture generator for balls + table
```

### Component Architecture

```
App
├── Scene (R3F Canvas)
│   ├── PhysicsWorld (R3C Physics provider)
│   │   ├── Table (static bodies: bed, cushions, rails)
│   │   ├── Balls (16x BallBody + Ball mesh)
│   │   └── CueStick (visual only, follows aim)
│   ├── AimLine (visual guide)
│   ├── Lights (ambient + directional + point)
│   └── Camera (OrbitControls + top-down toggle)
└── GameUI (HTML overlay)
    ├── PlayerIndicator
    ├── PowerMeter
    ├── PocketedBalls
    ├── FoulToast
    └── GameOverModal
```

### State Machine

```
IDLE ──[click to aim]──→ AIMING ──[mousedown]──→ POWER
  ↑                                                │
  │                                          [mouseup / release]
  │                                                ↓
  │                                         SIMULATING
  │                                                │
  │                                     [all balls settled]
  │                                                ↓
  └──────── EVALUATING ──[rules check]──→ next turn or GAME_OVER
```

| Phase | Description | Valid Actions |
|-------|-------------|---------------|
| `IDLE` | Waiting for current player to start aiming | Move mouse to position aim line |
| `AIMING` | Player is positioning their shot | Mouse move rotates aim, click starts power charge |
| `POWER` | Player is charging the power meter | Hold to charge, release to shoot |
| `SIMULATING` | Balls are in motion after a shot | No input (wait for settle) |
| `EVALUATING` | Checking rules: fouls, pocketed balls, game state | Automatic transition |
| `GAME_OVER` | Game has ended | Click "Play Again" to restart |

### Physics Design

#### Contact Materials

| Material Pair | Friction | Restitution | Description |
|---------------|----------|-------------|-------------|
| Ball–Ball | 0.05 | 0.95 | Nearly elastic collision |
| Ball–Felt | 0.4 | 0.1 | High friction for deceleration, low bounce |
| Ball–Cushion | 0.1 | 0.7 | Moderate bounce off rails |

#### Physics Bodies

| Body | Shape | Mass | Type | Notes |
|------|-------|------|------|-------|
| Ball | Sphere (r=0.0285m) | 0.17 kg | Dynamic | Standard billiard ball |
| Table bed | Box | — | Static | Flat surface with felt material |
| Cushion | Box (x6) | — | Static | Angled rail cushions |
| Pocket zone | — | — | — | No body; checked via distance code |

#### Pocket Detection

Pockets are not physics bodies. Instead, each frame during `SIMULATING`:
1. For each ball, compute distance to each of the 6 pocket centers
2. If distance < pocket radius (0.047m), flag ball as pocketed
3. Remove ball body from physics world
4. Animate ball dropping below table surface
5. After settle, feed pocketed balls to rules engine

#### Coordinate System

- Origin at center of table bed surface
- X-axis: width (short dimension)
- Z-axis: length (long dimension)
- Y-axis: up (height)
- Table dimensions: 2.24m x 1.12m (playing surface)

### Aiming System

1. **Raycasting**: Cast ray from camera through mouse position onto the table plane (Y=0)
2. **Direction vector**: `aimDir = normalize(raycastHit - cueBallPosition)`
3. **Guide line**: Draw a dashed line from cue ball in `aimDir` direction
4. **Ghost ball**: Cast a ray from cue ball along `aimDir`; if it intersects any ball, show ghost ball at contact point
5. **Cue stick**: Position behind cue ball opposite to `aimDir`, animate forward on strike

### Procedural Textures

All textures are generated at runtime using HTML Canvas — no external image files needed.

#### Ball Texture (512x256 per ball)
- White (solids) or white with colored band (stripes) background
- Number circle in center with ball number
- Stripe balls: color band covers middle 50% of UV space

#### Table Texture
- Felt: dark green with subtle noise grain
- Wood rails: brown with grain pattern
- Pocket holes: dark circles with subtle rim

### Camera System

| Mode | Description |
|------|-------------|
| Default | OrbitControls centered on table, constrained angle |
| Top-down | Orthographic view directly above table (toggle with T key) |
| Ball-in-hand | Camera follows cue ball for placement |

## Key Design Decisions

1. **Distance-based pockets over physics triggers** — More reliable than sensor bodies in cannon-es; avoids tunneling and false positives
2. **Procedural textures over image files** — Zero external dependencies; textures generate instantly at startup
3. **Zustand over Context/Redux** — Minimal boilerplate, excellent performance for game state that updates frequently during simulation
4. **Separate aim store from game store** — Aim state updates every frame (mouse movement); game state updates only on phase transitions
5. **State machine for game phases** — Clear, testable transitions prevent impossible states
6. **cannon-es over Ammo.js** — Lighter weight, pure JS, sufficient for billiards physics
