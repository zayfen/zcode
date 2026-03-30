# Task Breakdown: 3D 8-Ball Pool Game

## Task Overview

29 tasks across 9 phases. Tasks are ordered by dependency — earlier phases must complete before later ones begin.

## Phase 1: Scaffolding

### T01: Initialize Vite Project

- **Description**: Create Vite + React + TypeScript project at `examples/pool-game/`
- **Actions**:
  - Run `npm create vite@latest pool-game -- --template react-ts` in `examples/`
  - Verify `npm run dev` and `npm run build` work
  - Clean default boilerplate (remove App.css, assets/)
- **Dependencies**: None
- **Acceptance**: `npm run dev` starts without errors, `npm run build` produces a bundle

### T02: Install 3D Dependencies

- **Description**: Install all required packages
- **Actions**:
  - `npm install three @react-three/fiber @react-three/drei cannon-es @react-three/cannon zustand`
  - `npm install -D @types/three`
- **Dependencies**: T01
- **Acceptance**: `npm run build` succeeds with all packages installed

### T03: Create Directory Structure + Type Definitions

- **Description**: Create all source directories and define TypeScript types
- **Actions**:
  - Create directories: `types/`, `constants/`, `store/`, `components/`, `physics/`, `game-logic/`, `hooks/`, `utils/`
  - Define types in `src/types/index.ts`:
    ```typescript
    type BallId = 0 | 1 | 2 | ... | 15;  // 0 = cue
    type BallGroup = 'solids' | 'stripes' | null;
    type GamePhase = 'IDLE' | 'AIMING' | 'POWER' | 'SIMULATING' | 'EVALUATING' | 'GAME_OVER';
    type Player = 1 | 2;
    type Foul = 'SCRATCH' | 'NO_RAIL_CONTACT' | 'WRONG_BALL_FIRST' | 'NO_BALL_HIT' | null;
    interface ShotResult {
      pocketed: BallId[];
      firstContact: BallId | null;
      cueBallStopped: { x: number; y: number; z: number };
      foul: Foul;
    }
    ```
- **Dependencies**: T01
- **Acceptance**: TypeScript compiles without errors

---

## Phase 2: Constants

### T04: Table Dimensions + Physics Constants

- **Description**: Define all physical constants in `src/constants/table.ts` and `src/constants/physics.ts`
- **Actions**:
  - `table.ts`: TABLE_LENGTH (2.24m), TABLE_WIDTH (1.12m), BALL_RADIUS (0.0285m), POCKET_RADIUS (0.047m), cushion positions, pocket center positions (6 pockets)
  - `physics.ts`: GRAVITY, BALL_MASS (0.17kg), friction/restitution for ball-ball (0.05/0.95), ball-felt (0.4/0.1), ball-cushion (0.1/0.7), LINEAR_DAMPING, ANGULAR_DAMPING
- **Dependencies**: T03
- **Acceptance**: Constants exported and used consistently

### T05: Ball Colors + Rack Positions

- **Description**: Define ball appearance and initial positions in `src/constants/balls.ts`
- **Actions**:
  - Ball colors: map of BallId → hex color (standard billiard colors)
  - Rack layout: triangle formation with 8-ball at center
  - Ball group assignment: solids (1-7), eight (8), stripes (9-15)
- **Dependencies**: T03
- **Acceptance**: 15 object balls in correct triangle + cue ball at head position

### T06: Vector Math Utilities

- **Description**: Create `src/utils/vector.ts` with vector helpers
- **Actions**:
  - `vec3Distance(a, b)`: Euclidean distance
  - `vec3Normalize(v)`: Unit vector
  - `vec3Reflect(v, normal)`: Reflection for cushion bouncing
  - `vec3Lerp(a, b, t)`: Linear interpolation
  - `vec3Angle(a, b)`: Angle between vectors
- **Dependencies**: T03
- **Acceptance**: All functions typed and exported

---

## Phase 3: Rendering

### T07: Scene + Lighting Setup

- **Description**: Create `src/components/Scene.tsx` with R3F Canvas, camera, and lighting
- **Actions**:
  - R3F `<Canvas>` with shadow map enabled
  - Ambient light (soft fill)
  - Directional light (main, casts shadows, positioned above table)
  - Two point lights at table ends (even illumination)
  - Camera: perspective, 45° FOV, positioned at slight angle above table
  - OrbitControls: constrained to not go below table
- **Dependencies**: T02, T04
- **Acceptance**: Scene renders with proper lighting; camera can orbit around table

### T08: Table Components

- **Description**: Create `src/components/Table.tsx` with felt bed, rails, cushions, and pockets
- **Actions**:
  - Felt bed: flat box mesh with green material
  - Wooden rails: 4 outer rail meshes with brown material
  - Cushions: 6 inner cushion meshes (angled)
  - Pockets: 6 dark circular depressions at corner and side positions
  - All geometry uses constants from T04
- **Dependencies**: T04, T07
- **Acceptance**: Table looks correct with all elements at right positions

### T09: Ball Component + Procedural Textures

- **Description**: Create `src/components/Ball.tsx` and `src/utils/textures.ts`
- **Actions**:
  - `textures.ts`: Canvas-based texture generator
    - Generate 512x256 texture per ball
    - Solid balls: fill with ball color, white number circle
    - Stripe balls: white background, colored band in middle 50%, number circle
    - Cue ball: plain white
  - `Ball.tsx`: Sphere mesh with generated texture, correct radius
- **Dependencies**: T05, T07
- **Acceptance**: All 16 balls render with correct colors and numbers

### T10: App Wiring + Balls Container

- **Description**: Wire Scene, Table, and Balls together in `App.tsx`
- **Actions**:
  - `src/components/Balls.tsx`: Maps over ball IDs, renders `<Ball>` for each
  - Update `App.tsx`: Scene → Table + Balls, basic HTML overlay placeholder
  - Verify initial rack positions render correctly
- **Dependencies**: T08, T09
- **Acceptance**: All 16 balls visible on table at correct positions; `npm run dev` works

---

## Phase 4: Physics

### T11: Physics World Provider

- **Description**: Create `src/physics/PhysicsWorld.tsx`
- **Actions**:
  - Wrap children in `@react-three/cannon` `<Physics>` provider
  - Configure gravity: (0, -9.81, 0)
  - Define 3 contact materials (ball-ball, ball-felt, ball-cushion)
  - Set broadphase, solver iterations for accuracy
- **Dependencies**: T07, T04
- **Acceptance**: Physics provider wraps scene without errors

### T12: Ball Physics Bodies

- **Description**: Create `src/physics/BallBody.tsx` and update Ball component
- **Actions**:
  - `BallBody.tsx`: `useSphere` hook for each ball with correct mass, material, damping
  - Connect ball body position/rotation to Ball mesh
  - Static bodies for table bed and cushions using `usePlane`/`useBox`
  - Subscribe to ball body positions for pocket detection
- **Dependencies**: T11, T09
- **Acceptance**: Balls sit on table surface; pushing one causes it to roll and stop

### T13: Pocket Detection

- **Description**: Create `src/physics/pocketDetection.ts`
- **Actions**:
  - Function that takes ball positions + pocket positions
  - Returns list of pocketed ball IDs (distance < POCKET_RADIUS)
  - Integrate into useFrame loop during SIMULATING phase
  - When pocketed: remove body from physics world, animate ball below table
- **Dependencies**: T12, T04
- **Acceptance**: Balls that enter pocket zones are removed from play

---

## Phase 5: Game State

### T14: Zustand Game Store

- **Description**: Create `src/store/gameStore.ts`
- **Actions**:
  - State: `phase`, `currentPlayer`, `ballGroups` (which player has solids/stripes), `pocketedBalls`, `scores`, `foul`, `winner`
  - Actions: `startAiming()`, `startPower()`, `shoot()`, `setSimulating()`, `evaluateShot()`, `nextTurn()`, `setGameOver()`, `resetGame()`
  - State machine transitions with validation
  - `ballInHand` flag + position for foul recovery
- **Dependencies**: T03
- **Acceptance**: Store transitions through all phases correctly

### T15: 8-Ball Rules Engine

- **Description**: Create `src/game-logic/rules.ts`
- **Actions**:
  - `evaluateShotResult(shot: ShotResult, state: GameState)`:
    - Detect fouls: scratch, no ball hit, wrong ball first-contact, no rail after contact
    - Assign ball groups on first legal pocket (if unassigned)
    - Determine if shot was legal
    - Check for game over (8-ball pocketed legally or illegally)
  - Return: `{ foul, nextPlayer, pocketed, gameOver, winner }`
- **Dependencies**: T14
- **Acceptance**: All 8-ball rule cases handled correctly

### T16: Settle Detector Hook

- **Description**: Create `src/hooks/useSettleDetector.ts`
- **Actions**:
  - `useFrame` hook that checks if all ball bodies have velocity < threshold
  - Threshold: linear velocity < 0.001, angular velocity < 0.01
  - Debounce: must be below threshold for 10 consecutive frames
  - When settled: trigger EVALUATING phase transition
- **Dependencies**: T12, T14
- **Acceptance**: Correctly detects when all balls have stopped moving

---

## Phase 6: Aiming System

### T17: Aim Store + useAim Hook

- **Description**: Create `src/store/aimStore.ts` and `src/hooks/useAim.ts`
- **Actions**:
  - `aimStore`: `direction` (Vec3), `power` (0–1), `isCharging`
  - `useAim`: Raycast from camera through mouse onto table plane
  - Compute aim direction from cue ball to ray hit point
  - Update aim store direction on every mouse move (during AIMING phase)
- **Dependencies**: T14
- **Acceptance**: Aim direction updates smoothly with mouse movement

### T18: CueStick + AimLine Components

- **Description**: Create `src/components/CueStick.tsx` and `src/components/AimLine.tsx`
- **Actions**:
  - `CueStick.tsx`: Cylinder mesh positioned behind cue ball, rotated to match aim direction
  - Pull-back animation during POWER phase (retract proportional to power)
  - Strike animation: quick forward motion on shoot
  - `AimLine.tsx`: Dashed line from cue ball along aim direction (max 2m)
  - Ghost ball: if aim line intersects a target ball, show translucent sphere at contact point
- **Dependencies**: T17, T07
- **Acceptance**: Cue stick follows mouse; aim line shows projected path

### T19: Power + Shot Hooks

- **Description**: Create `src/hooks/usePower.ts` and `src/hooks/useShotSequence.ts`
- **Actions**:
  - `usePower`: On mousedown → start charging (power increases 0→1 over 2 seconds max)
  - On mouseup → fire shot
  - `useShotSequence`: Orchestrates full shot:
    1. Apply impulse to cue ball: `direction * power * MAX_IMPULSE`
    2. Transition to SIMULATING
    3. Wait for settle detector
    4. Collect shot result (pocketed balls, first contact)
    5. Call rules engine
    6. Transition to next phase
- **Dependencies**: T17, T16, T15
- **Acceptance**: Full shot cycle works: aim → charge → shoot → simulate → evaluate

### T20: PowerMeter Component

- **Description**: Create `src/components/PowerMeter.tsx`
- **Actions**:
  - HTML overlay bar that fills based on `aimStore.power`
  - Color gradient: green (low) → yellow (mid) → red (high)
  - Only visible during POWER phase
  - Positioned at bottom-center of screen
- **Dependencies**: T17
- **Acceptance**: Power meter fills while holding mouse, empties on release

---

## Phase 7: UI Overlay

### T21: GameUI Component

- **Description**: Create `src/components/GameUI.tsx`
- **Actions**:
  - Player indicator: "Player 1 (Solids)" / "Player 2 (Stripes)" with highlight for current
  - Pocketed balls display: small ball icons grouped by player
  - Foul toast: animated notification that appears for 3 seconds on foul
  - Game over modal: winner announcement + "Play Again" button
  - Phase indicator: subtle text showing current game phase
- **Dependencies**: T14, T15
- **Acceptance**: All UI elements display correctly and update with game state

### T22: Ball-in-Hand Mode

- **Description**: Implement ball-in-hand placement after fouls
- **Actions**:
  - When foul detected, set `ballInHand` flag and transition to IDLE
  - In IDLE with `ballInHand`: mouse click on table sets cue ball position
  - Constrain placement to table surface bounds
  - Visual: translucent cue ball follows mouse until placed
  - Keyboard shortcut R to confirm placement
- **Dependencies**: T14, T21
- **Acceptance**: After a foul, current player can place cue ball anywhere on table

### T23: Top-Down Camera Toggle

- **Description**: Add top-down camera view
- **Actions**:
  - T key toggles between orbit and top-down views
  - Top-down: orthographic camera directly above table center
  - Smooth transition between views (lerp over 0.5s)
  - Store camera mode in aim store
- **Dependencies**: T07
- **Acceptance**: T key switches views smoothly

---

## Phase 8: Polish

### T24: Enhanced Ball Textures

- **Description**: Improve procedural ball textures
- **Actions**:
  - Higher resolution (1024x512) with anti-aliased number rendering
  - Subtle specular highlight baked into texture
  - Stripe boundary anti-aliasing
  - Cue ball: slight off-white with visible glossiness
- **Dependencies**: T09
- **Acceptance**: Balls look polished and professional

### T25: Table Textures + Environment

- **Description**: Improve table visuals
- **Actions**:
  - Felt texture: canvas-generated with subtle noise/grain
  - Wood grain texture for rails: brown gradient with dark grain lines
  - Diamond markers on rails (sight dots)
  - Floor plane beneath table (dark wood)
- **Dependencies**: T08
- **Acceptance**: Table has realistic textures and looks polished

### T26: Shadows + Post-Processing

- **Description**: Add shadow mapping and optional post-processing
- **Actions**:
  - Configure shadow map: PCFSoftShadowMap, 2048x2048 resolution
  - Balls cast shadows on felt
  - Directional light shadow camera frustum covers table area
  - Optional: SSAO for subtle ambient occlusion
  - Optional: bloom on cue ball highlight during aiming
- **Dependencies**: T07, T08, T09
- **Acceptance**: Shadows render correctly; no shadow acne or peter-panning

---

## Phase 9: Integration

### T27: End-to-End Flow Integration

- **Description**: Wire all systems together for a complete game flow
- **Actions**:
  - Full game loop: break → turns → group assignment → 8-ball shot → win/loss
  - Integration test: play through a complete game automatically
  - Handle edge cases: break scratches, 8-ball on break, no group assignment
  - Fix any timing issues between physics and state transitions
- **Dependencies**: T20, T21, T22
- **Acceptance**: Complete game playable from break to win/loss

### T28: Undo + Final Polish

- **Description**: Add undo support and final polish
- **Actions**:
  - U key triggers undo: restore previous ball positions and game state
  - Snapshot ball positions + game state before each shot
  - Limit undo to 1 step
  - Smooth cue stick retraction animation
  - Ball pocket animation (sink below table, fade out)
  - Clean up any console warnings
- **Dependencies**: T27
- **Acceptance**: Undo works correctly; no visual glitches

### T29: README + Final Verification

- **Description**: Write project README and run final validation
- **Actions**:
  - `README.md`: project overview, tech stack, setup instructions, controls reference, screenshots placeholder
  - Run all validation criteria from `docs/validation.md`
  - Verify `npm run build` produces clean production bundle
  - Test in Chrome, Firefox, Safari (if available)
- **Dependencies**: T28
- **Acceptance**: README complete; all validation gates pass

---

## Dependency Graph

```
T01 ──→ T02 ──→ T03 ──┬──→ T04 ──→ T07 ──→ T08 ──→ T10
                       ├──→ T05 ──→ T09 ──┘
                       └──→ T06
                                  T07 ──→ T11 ──→ T12 ──→ T13
                                            T04 ──→ T11
                                            T09 ──→ T12
                                  T12 ──→ T16
                                  T03 ──→ T14 ──→ T15
                                  T14 ──→ T17 ──→ T18
                                            T17 ──→ T19 ──→ T27 ──→ T28 ──→ T29
                                            T16 ──→ T19
                                            T15 ──→ T19
                                            T17 ──→ T20
                                            T14 ──→ T21 ──→ T22
                                            T07 ──→ T23
                                            T09 ──→ T24
                                            T08 ──→ T25
                                            T07 ──→ T26
                                            T20 ──→ T27
                                            T21 ──→ T27
                                            T22 ──→ T27
```

## Estimated Complexity

| Phase | Tasks | Complexity | Risk |
|-------|-------|-----------|------|
| 1. Scaffolding | T01–T03 | Low | Low |
| 2. Constants | T04–T06 | Low | Low |
| 3. Rendering | T07–T10 | Medium | Low |
| 4. Physics | T11–T13 | Medium | **Medium** — physics tuning |
| 5. Game State | T14–T16 | Medium | **Medium** — rule edge cases |
| 6. Aiming | T17–T20 | High | **Medium** — raycast precision |
| 7. UI | T21–T23 | Low | Low |
| 8. Polish | T24–T26 | Medium | Low |
| 9. Integration | T27–T29 | High | **High** — cross-system timing |
