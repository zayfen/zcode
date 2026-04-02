# Coding Specification

## Tech Stack

| Technology | Version | Purpose |
|-----------|---------|---------|
| Vite | ^8.x | Build tool and dev server |
| React | ^19.x | UI framework |
| TypeScript | ^5.x | Type safety |
| Three.js | ^0.183 | 3D rendering engine |
| @react-three/fiber | ^9.x | React renderer for Three.js |
| @react-three/drei | ^10.x | Three.js helpers (OrbitControls, etc.) |
| cannon-es | ^0.20 | Physics engine (rigid body simulation) |
| @react-three/cannon | ^6.x | React bindings for cannon-es |
| Zustand | ^5.x | State management |

## File Structure

```
examples/pool-game/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
├── docs/
│   ├── prd/
│   ├── specs/
│   ├── tasks/
│   ├── validation.md
│   └── review-checklist.md
└── src/
    ├── main.tsx
    ├── App.tsx
    ├── types/
    │   └── index.ts
    ├── constants/
    │   ├── table.ts
    │   ├── physics.ts
    │   └── balls.ts
    ├── store/
    │   ├── gameStore.ts
    │   └── aimStore.ts
    ├── components/
    │   ├── Scene.tsx
    │   ├── Table.tsx
    │   ├── Ball.tsx
    │   ├── Balls.tsx
    │   ├── CueStick.tsx
    │   ├── AimLine.tsx
    │   ├── PowerMeter.tsx
    │   └── GameUI.tsx
    ├── physics/
    │   ├── PhysicsWorld.tsx
    │   ├── BallBody.tsx
    │   └── pocketDetection.ts
    ├── game-logic/
    │   ├── rules.ts
    │   └── rack.ts
    ├── hooks/
    │   ├── useAim.ts
    │   ├── usePower.ts
    │   ├── useSettleDetector.ts
    │   └── useShotSequence.ts
    └── utils/
        ├── vector.ts
        └── textures.ts
```
