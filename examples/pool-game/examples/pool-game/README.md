# 3D 8-Ball Pool Game

A fully interactive 3D 8-ball pool game built with React, Three.js, and Cannon.js physics.

## Tech Stack

- **Vite** + **React** + **TypeScript** — Build tooling and UI framework
- **Three.js** + **@react-three/fiber** + **@react-three/drei** — 3D rendering
- **Cannon.js** (`cannon-es` + `@react-three/cannon`) — Physics simulation
- **Zustand** — State management

## Setup

```bash
# Install dependencies
npm install

# Start development server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Controls

| Action | Control |
|--------|---------|
| Aim | Move mouse |
| Charge power | Hold left mouse button |
| Shoot | Release left mouse button |
| Toggle camera view | Press **T** |
| Undo last shot | Press **U** |
| Reset game (after game over) | Press **R** |
| Place cue ball (after foul) | Click on table |

## Features

- **Full 8-ball rules**: Group assignment (solids/stripes), fouls, legal 8-ball shots, win/loss conditions
- **Realistic physics**: Ball-ball collisions, cushion rebounds, friction, pocket detection
- **3D rendering**: Procedural ball textures with numbers, shadows, realistic table with felt, rails, cushions, diamond markers, and pockets
- **Game UI**: Player indicators, power meter, foul notifications, pocketed balls display, game over modal
- **Aiming system**: Cue stick, aim line, power charging with visual meter
- **Camera**: Orbit controls + top-down toggle
- **Ball-in-hand**: Place cue ball anywhere after a foul
- **Undo**: Revert last shot (one step)

## Project Structure

```
src/
├── components/    # React components (Scene, Table, Ball, CueStick, GameUI, etc.)
├── constants/     # Table dimensions, physics constants, ball colors
├── game-logic/    # 8-ball rules engine
├── hooks/         # Aim, settle detection, power hooks
├── physics/       # Physics world provider, pocket detection
├── store/         # Zustand stores (game state, aim state)
├── types/         # TypeScript type definitions
└── utils/         # Vector math, texture generation
```

## Rules Summary

- Standard 8-ball: pocket all balls in your group (solids 1-7 or stripes 9-15), then pocket the 8-ball
- First player to legally pocket a ball claims that group
- Fouls (scratch, wrong ball first, no contact) give ball-in-hand to opponent
- Pocketing the 8-ball early or on a foul results in a loss
