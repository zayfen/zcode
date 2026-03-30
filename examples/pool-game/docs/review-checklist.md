# Code Review Checklist: 3D 8-Ball Pool Game

## TypeScript & Code Quality

- [ ] TypeScript strict mode enabled (`"strict": true` in tsconfig.json)
- [ ] No `any` types — all variables, parameters, and returns are properly typed
- [ ] No unused imports or variables
- [ ] No `console.log` statements in production code (debug logs removed)
- [ ] ESLint passes with zero warnings (`npm run lint` if configured)
- [ ] All components use named exports (consistent style)
- [ ] No magic numbers — all constants defined in `src/constants/`

## Physics Accuracy

- [ ] Ball dimensions match real billiards (radius 28.5mm / 0.0285m)
- [ ] Ball mass matches real billiards (170g / 0.17kg)
- [ ] Table proportions match regulation tables (2:1 ratio playing surface)
- [ ] Pocket positions at 6 standard locations (4 corners + 2 side)
- [ ] Ball-ball restitution near 0.95 (nearly elastic)
- [ ] Felt friction causes natural deceleration
- [ ] Cushion restitution produces realistic rebounds
- [ ] Physics solver iterations high enough to prevent tunneling

## Game Logic

- [ ] State machine covers all 6 phases: IDLE, AIMING, POWER, SIMULATING, EVALUATING, GAME_OVER
- [ ] No invalid state transitions are possible
- [ ] 8-ball rules correctly enforced:
  - [ ] Groups assigned on first legal pocket after break
  - [ ] Must hit own group ball first (after assignment)
  - [ ] Scratch → ball-in-hand for opponent
  - [ ] Wrong ball first-contact → foul
  - [ ] No rail contact after hit → foul
  - [ ] 8-ball pocketed early → loss
  - [ ] Scratch on 8-ball → loss
  - [ ] 8-ball pocketed legally after clearing group → win
- [ ] Break shot handled correctly (special rules)
- [ ] Ball-in-hand placement restricted to table surface

## Component Architecture

- [ ] Scene component manages Canvas, camera, and lighting
- [ ] Physics world is a single provider wrapping all physics bodies
- [ ] Ball component is pure presentation (no physics logic)
- [ ] BallBody component handles physics (useSphere hook)
- [ ] GameUI is HTML overlay (not 3D) for text and controls
- [ ] State management separated: gameStore (state machine) vs aimStore (real-time input)
- [ ] No prop drilling beyond 2 levels — use stores for deep state

## Performance

- [ ] `useFrame` hooks do minimal computation per frame
- [ ] Aim direction update is lightweight (no raycasting against all objects)
- [ ] Pocket detection uses early-out (distance squared comparison before sqrt)
- [ ] Ball meshes share geometry (instanced or cloned)
- [ ] Textures generated once at startup (not per frame)
- [ ] Shadow map resolution is appropriate (2048x2048 max)
- [ ] Physics world uses broadphase optimization
- [ ] React re-renders minimized during SIMULATING phase

## Accessibility

- [ ] Keyboard controls documented and functional (R, T, U keys)
- [ ] Color-blind consideration: balls have numbers, not just colors
- [ ] Power meter has text percentage in addition to color gradient
- [ ] Game state announcements could work with screen readers (aria labels on UI overlay)
- [ ] Focus management: game area is focusable for keyboard input

## Security & Build

- [ ] No external image/font dependencies (all procedural)
- [ ] No API calls or network requests
- [ ] No eval() or dynamic code execution
- [ ] No user-generated content injection risks
- [ ] `npm run build` produces clean production bundle
- [ ] No source maps in production build
- [ ] Dependencies pinned to major versions in package.json

## File Organization

- [ ] Each file under 300 lines (split if larger)
- [ ] Components are in `components/`, not scattered
- [ ] Business logic in `game-logic/`, not in components
- [ ] Constants centralized in `constants/`, not inline
- [ ] Hooks in `hooks/`, not in components
- [ ] Utility functions in `utils/`, not duplicated
