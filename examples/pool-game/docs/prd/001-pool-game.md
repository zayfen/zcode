# PRD: 3D 8-Ball Pool Game

## Overview

A browser-based 3D 8-ball pool game for two players, built with Three.js and React. The game enforces standard 8-ball rules with realistic physics, intuitive aiming controls, and visual polish suitable for a demo of zcode's pipeline system.

## Goals

1. **Playable 8-ball pool** — Two players take turns shooting on a regulation-style table with full rule enforcement.
2. **Realistic physics** — Ball-to-ball collisions, cushion rebounds, friction deceleration, and pocket detection behave predictably and realistically.
3. **Intuitive controls** — Mouse-based aiming with a visual guide line, click-and-drag power meter, and clear turn indicators.
4. **Visual quality** — Procedurally generated textures, proper lighting, shadows, and a polished UI overlay showing game state.
5. **Pipeline demo** — Serve as a complete, non-trivial example of zcode's Cognition → Planning → Execution → Verification → Delivery pipeline.

## Features

### Core Gameplay

| Feature | Description |
|---------|-------------|
| Table | Regulation-proportioned table with 6 pockets, felt surface, wooden rails, and cushions |
| Balls | 16 balls: cue ball (white), 7 solids (1–7), 8-ball (black), 7 stripes (9–15) |
| Physics | cannon-es rigid body simulation with ball-ball, ball-felt, and ball-cushion contact materials |
| Aiming | Mouse-driven aim direction with a dashed guide line showing the projected cue ball path and first-contact ghost ball |
| Power | Click-and-hold power meter (0–100%) that maps to cue strike impulse |
| Shooting | Cue stick animation on strike, camera shake on break |
| Pocketing | Distance-based pocket detection with pocket animations (balls drop below table surface) |
| Turns | Alternating two-player turns with clear "Player 1 / Player 2" indicator |
| Fouls | Scratch (cue ball pocketed), no-rail contact, wrong ball first-contact — opponent gets ball-in-hand |
| 8-Ball Rules | Assign solids/stripes on first legal pocket after break; 8-ball must be last; pocketing 8-ball early = loss |
| Win/Loss | Win by legally pocketing the 8-ball after clearing assigned group. Loss by pocketing 8-ball early or scratching on the 8-ball shot |

### Controls

| Input | Action |
|-------|--------|
| Mouse move | Aim cue stick |
| Left click + hold | Charge power meter |
| Left click release | Strike cue ball |
| R key | Reset cue ball position (ball-in-hand mode) |
| T key | Toggle top-down camera view |
| U key | Undo last shot (if enabled) |

### UI Overlay

- Current player indicator with assigned ball group (solids/stripes/none)
- Power meter bar (fills during charge)
- Pocketed balls display (separated by player)
- Foul notification toast
- Game over modal with winner announcement and "Play Again" button
- FPS counter (debug)

## Acceptance Criteria

- [ ] **AC-1**: Game launches in browser via `npm run dev` with no console errors
- [ ] **AC-2**: All 16 balls render at correct triangular rack positions with accurate colors and numbering
- [ ] **AC-3**: Cue ball can be aimed in any direction using mouse movement, with a visible aim line
- [ ] **AC-4**: Power meter charges while mouse is held and applies proportional impulse on release
- [ ] **AC-5**: Balls collide realistically — elastic collisions preserve momentum direction
- [ ] **AC-6**: Balls decelerate naturally via felt friction and come to rest
- [ ] **AC-7**: Balls pocket correctly when entering pocket zones (distance threshold)
- [ ] **AC-8**: Standard 8-ball rules enforced: group assignment, legal shots, fouls, win/loss conditions
- [ ] **AC-9**: Two players alternate turns with correct state transitions
- [ ] **AC-10**: Game detects and announces a winner with a game-over screen
- [ ] **AC-11**: `npm run build` produces a production bundle with no errors
- [ ] **AC-12**: Game runs at 60fps on modern hardware (Chrome, Firefox, Safari)
- [ ] **AC-13**: No external image dependencies — all textures procedurally generated
- [ ] **AC-14**: Keyboard controls work for aiming, camera toggle, and ball-in-hand placement

## Non-Goals

- Single-player mode with AI opponent
- Online multiplayer
- Sound effects or music
- Mobile/touch support
- Save/load game state
- Tournament or scoring system
