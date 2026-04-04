import { useRef, useEffect, useMemo, useCallback } from 'react'
import * as THREE from 'three'
import { useFrame, useThree } from '@react-three/fiber'
import { BALL_RADIUS, TABLE_LENGTH, TABLE_WIDTH, POCKET_POSITIONS, POCKET_RADIUS } from '../constants/table'
import { ALL_BALL_IDS, getRackPositions, BALL_COLORS, isStripe } from '../constants/balls'
import { BALL_MASS, LINEAR_DAMPING, ANGULAR_DAMPING, MAX_IMPULSE, SETTLE_LINEAR_THRESHOLD, SETTLE_ANGULAR_THRESHOLD } from '../constants/physics'
import { useGameStore } from '../store/gameStore'
import { useAimStore } from '../store/aimStore'
import { evaluateShotResult } from '../game-logic/rules'
import { getBallGroup } from '../constants/balls'
import { generateBallTexture } from '../utils/textures'
import type { BallId, BallState, Foul } from '../types'
import * as CANNON from 'cannon-es'

// ─── Physics World Singleton ──────────────────────────────────────────────

let world: CANNON.World | null = null
const ballBodies = new Map<number, CANNON.Body>()
let firstContactThisShot: BallId | null = null
let preShotPocketed: BallId[] = []

function getOrCreateWorld(): CANNON.World {
  if (world) return world

  world = new CANNON.World()
  world.gravity.set(0, -9.81, 0)
  world.broadphase = new CANNON.SAPBroadphase(world)
  world.solver.iterations = 10

  const ballMat = new CANNON.Material('ball')
  const feltMat = new CANNON.Material('felt')
  const cushionMat = new CANNON.Material('cushion')

  world.addContactMaterial(new CANNON.ContactMaterial(ballMat, ballMat, {
    friction: 0.05,
    restitution: 0.95,
  }))
  world.addContactMaterial(new CANNON.ContactMaterial(ballMat, feltMat, {
    friction: 0.4,
    restitution: 0.1,
  }))
  world.addContactMaterial(new CANNON.ContactMaterial(ballMat, cushionMat, {
    friction: 0.1,
    restitution: 0.7,
  }))

  // Ground plane (felt)
  const ground = new CANNON.Body({
    mass: 0,
    material: feltMat,
    shape: new CANNON.Plane(),
  })
  ground.quaternion.setFromEuler(-Math.PI / 2, 0, 0)
  world.addBody(ground)

  // Cushions
  const halfL = TABLE_LENGTH / 2
  const halfW = TABLE_WIDTH / 2
  const ch = 0.035
  const ct = 0.03

  const cushionDefs: { pos: [number, number, number]; size: [number, number, number] }[] = [
    { pos: [-halfL / 2, ch / 2, -(halfW)], size: [halfL * 0.85, ch, ct] },
    { pos: [halfL / 2, ch / 2, -(halfW)], size: [halfL * 0.85, ch, ct] },
    { pos: [-halfL / 2, ch / 2, halfW], size: [halfL * 0.85, ch, ct] },
    { pos: [halfL / 2, ch / 2, halfW], size: [halfL * 0.85, ch, ct] },
    { pos: [-(halfL), ch / 2, 0], size: [ct, ch, TABLE_WIDTH * 0.75] },
    { pos: [halfL, ch / 2, 0], size: [ct, ch, TABLE_WIDTH * 0.75] },
  ]

  for (const def of cushionDefs) {
    const body = new CANNON.Body({
      mass: 0,
      material: cushionMat,
      shape: new CANNON.Box(new CANNON.Vec3(def.size[0] / 2, def.size[1] / 2, def.size[2] / 2)),
    })
    body.position.set(def.pos[0], def.pos[1], def.pos[2])
    world.addBody(body)
  }

  // Ball bodies
  const positions = getRackPositions()
  for (const id of ALL_BALL_IDS) {
    const pos = positions[id as BallId]
    const body = new CANNON.Body({
      mass: BALL_MASS,
      material: ballMat,
      shape: new CANNON.Sphere(BALL_RADIUS),
      linearDamping: LINEAR_DAMPING,
      angularDamping: ANGULAR_DAMPING,
    })
    body.position.set(pos[0], BALL_RADIUS, pos[2])
    world.addBody(body)
    ballBodies.set(id, body)

    // Contact tracking for cue ball
    body.addEventListener('collide', (e: { body: CANNON.Body }) => {
      const otherBody = e.body
      for (const [otherId, otherBallBody] of ballBodies) {
        if (otherBallBody === otherBody && otherId !== id) {
          if (id === 0 && firstContactThisShot === null) {
            firstContactThisShot = otherId as BallId
          }
          break
        }
      }
    })
  }

  return world
}

export function resetPhysicsWorld() {
  if (!world) return
  while (world.bodies.length > 0) {
    world.removeBody(world.bodies[0])
  }
  ballBodies.clear()
  world = null
  firstContactThisShot = null
}

export function resetBallPositions() {
  const positions = getRackPositions()
  for (const [id, body] of ballBodies) {
    const pos = positions[id as BallId]
    body.position.set(pos[0], BALL_RADIUS, pos[2])
    body.velocity.setZero()
    body.angularVelocity.setZero()
    body.wakeUp()
  }
  firstContactThisShot = null
}

export function placeCueBall(x: number, z: number) {
  const cueBody = ballBodies.get(0)
  if (!cueBody) return
  const halfL = TABLE_LENGTH / 2 - BALL_RADIUS
  const halfW = TABLE_WIDTH / 2 - BALL_RADIUS
  cueBody.position.set(
    Math.max(-halfL, Math.min(halfL, x)),
    BALL_RADIUS,
    Math.max(-halfW, Math.min(halfW, z)),
  )
  cueBody.velocity.setZero()
  cueBody.angularVelocity.setZero()
  cueBody.wakeUp()
}

// ─── GameController Component ─────────────────────────────────────────────

export default function GameController() {
  const phase = useGameStore(s => s.phase)
  const { camera, raycaster, pointer } = useThree()
  const tablePlane = useRef(new THREE.Plane(new THREE.Vector3(0, 1, 0), -BALL_RADIUS))
  const chargeStart = useRef(0)
  const settledFrames = useRef(0)
  const shotDone = useRef(false)

  // Ensure physics world exists
  const worldRef = useRef<CANNON.World | null>(null)

  useFrame(() => {
    const w = getOrCreateWorld()
    worldRef.current = w

    const state = useGameStore.getState()
    const aimState = useAimStore.getState()

    // ── AIMING: update aim direction from mouse ──
    if (state.phase === 'AIMING') {
      raycaster.setFromCamera(pointer, camera)
      const hit = new THREE.Vector3()
      raycaster.ray.intersectPlane(tablePlane.current, hit)
      if (hit) {
        const cueBody = ballBodies.get(0)
        if (cueBody) {
          const dir = new THREE.Vector3(
            hit.x - cueBody.position.x,
            0,
            hit.z - cueBody.position.z,
          )
          if (dir.length() > 0.001) {
            dir.normalize()
            aimState.setDirection(dir)
          }
        }
      }
    }

    // ── POWER: charge up ──
    if (state.phase === 'POWER') {
      const elapsed = (performance.now() - chargeStart.current) / 1000
      const pwr = Math.min(elapsed / 2, 1) // 2 seconds max
      aimState.setPower(pwr)
    }

    // ── SIMULATING: step physics, detect pockets, settle ──
    if (state.phase === 'SIMULATING') {
      w.step(1 / 60)

      // Pocket detection
      for (const [id, body] of ballBodies) {
        if (state.pocketedBalls.includes(id as BallId)) continue
        for (const pocket of POCKET_POSITIONS) {
          const dx = body.position.x - pocket.x
          const dz = body.position.z - pocket.z
          const dist = Math.sqrt(dx * dx + dz * dz)
          if (dist < POCKET_RADIUS) {
            state.pocketBall(id as BallId)
            // Move ball below table
            body.position.set(0, -1, 0)
            body.velocity.setZero()
            body.angularVelocity.setZero()
            body.sleep()
            break
          }
        }
      }

      // Settle detection
      let allSettled = true
      for (const [, body] of ballBodies) {
        const lv = body.velocity.length()
        const av = body.angularVelocity.length()
        if (lv > SETTLE_LINEAR_THRESHOLD || av > SETTLE_ANGULAR_THRESHOLD) {
          allSettled = false
          break
        }
      }

      if (allSettled) {
        settledFrames.current++
        if (settledFrames.current >= 10) {
          settledFrames.current = 0
          state.evaluateShot()
        }
      } else {
        settledFrames.current = 0
      }
    }

    // ── EVALUATING: run rules engine ──
    if (state.phase === 'EVALUATING' && !shotDone.current) {
      shotDone.current = true

      const newlyPocketed = state.pocketedBalls.filter(b => !preShotPocketed.includes(b))
      const cuePocketed = newlyPocketed.includes(0 as BallId)

      const shotData = {
        pocketed: newlyPocketed,
        firstContact: firstContactThisShot,
        cueBallPocketed: cuePocketed,
        cueBallStopped: { x: 0, y: 0, z: 0 },
      }

      const evaluation = evaluateShotResult(shotData, {
        phase: state.phase,
        currentPlayer: state.currentPlayer,
        playerGroups: state.playerGroups,
        pocketedBalls: state.pocketedBalls,
        scores: state.scores,
        foul: state.foul,
        winner: state.winner,
        ballInHand: state.ballInHand,
        ballInHandPosition: state.ballInHandPosition,
        breakShot: state.breakShot,
        groupsAssigned: state.groupsAssigned,
      })

      // Assign groups if needed
      if (evaluation.ballGroupAssigned && !state.groupsAssigned) {
        const firstPocketedNonCue = newlyPocketed.find(b => b !== 0 && b !== 8)
        if (firstPocketedNonCue !== undefined) {
          const group = getBallGroup(firstPocketedNonCue)
          if (group) {
            state.assignGroups(state.currentPlayer, group)
          }
        }
      }

      if (evaluation.gameOver) {
        state.setGameOver(evaluation.winner!)
      } else if (evaluation.foul) {
        state.setFoul(evaluation.foul)
        state.setBallInHand(true)
        state.nextTurn(evaluation.nextPlayer)
      } else {
        state.nextTurn(evaluation.nextPlayer)
      }

      state.setBreakShot(false)
      aimState.reset()
      firstContactThisShot = null
    }

    if (phase !== 'EVALUATING') {
      shotDone.current = false
    }
  })

  // Mouse handlers
  useEffect(() => {
    const handleMouseDown = (e: MouseEvent) => {
      const state = useGameStore.getState()
      if (state.phase === 'IDLE' && !state.ballInHand) {
        state.setPhase('AIMING')
      }
      if (state.phase === 'AIMING') {
        state.startPower()
        chargeStart.current = performance.now()
      }
      // Ball in hand placement
      if (state.phase === 'IDLE' && state.ballInHand) {
        const raycaster = new THREE.Raycaster()
        const pointer = new THREE.Vector2(
          (e.clientX / window.innerWidth) * 2 - 1,
          -(e.clientY / window.innerHeight) * 2 + 1,
        )
        const camera = (window as any).__threeCamera as THREE.Camera
        if (camera) {
          raycaster.setFromCamera(pointer, camera)
          const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -BALL_RADIUS)
          const hit = new THREE.Vector3()
          raycaster.ray.intersectPlane(plane, hit)
          if (hit) {
            placeCueBall(hit.x, hit.z)
            state.setBallInHand(false)
          }
        }
      }
    }

    const handleMouseUp = () => {
      const state = useGameStore.getState()
      const aimState = useAimStore.getState()
      if (state.phase === 'POWER') {
        // Save pre-shot state
        preShotPocketed = [...state.pocketedBalls]
        firstContactThisShot = null
        settledFrames.current = 0

        // Save snapshot for undo
        const ballStates: BallState[] = []
        for (const [id, body] of ballBodies) {
          ballStates.push({
            id: id as BallId,
            position: [body.position.x, body.position.y, body.position.z],
            velocity: [body.velocity.x, body.velocity.y, body.velocity.z],
            angularVelocity: [body.angularVelocity.x, body.angularVelocity.y, body.angularVelocity.z],
            pocketed: state.pocketedBalls.includes(id as BallId),
          })
        }
        state.saveSnapshot(ballStates)

        // Apply impulse to cue ball
        const cueBody = ballBodies.get(0)
        if (cueBody) {
          const dir = aimState.direction.clone().normalize()
          const impulse = new CANNON.Vec3(
            dir.x * aimState.power * MAX_IMPULSE,
            0,
            dir.z * aimState.power * MAX_IMPULSE,
          )
          cueBody.wakeUp()
          cueBody.applyImpulse(impulse)
        }

        state.shoot()
        aimState.setIsCharging(false)
        aimState.setPower(0)
      }
    }

    // Undo handler
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'u' || e.key === 'U') {
        const state = useGameStore.getState()
        if (state.phase === 'IDLE') {
          const snapshot = state.undo()
          if (snapshot) {
            resetBallPositions()
          }
        }
      }
    }

    window.addEventListener('mousedown', handleMouseDown)
    window.addEventListener('mouseup', handleMouseUp)
    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener('mousedown', handleMouseDown)
      window.removeEventListener('mouseup', handleMouseUp)
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [])

  // Store camera reference for ball-in-hand placement
  const { camera: cam } = useThree()
  useEffect(() => {
    ;(window as any).__threeCamera = cam
  }, [cam])

  return null
}

// ─── Export helpers for other components ──────────────────────────────────

export function getCueBallPosition(): THREE.Vector3 {
  const cueBody = ballBodies.get(0)
  if (!cueBody) return new THREE.Vector3(0, BALL_RADIUS, 0)
  return new THREE.Vector3(cueBody.position.x, cueBody.position.y, cueBody.position.z)
}

export function getBallPosition(id: number): THREE.Vector3 | null {
  const body = ballBodies.get(id)
  if (!body) return null
  return new THREE.Vector3(body.position.x, body.position.y, body.position.z)
}

export function getBallQuaternion(id: number): THREE.Quaternion | null {
  const body = ballBodies.get(id)
  if (!body) return null
  return new THREE.Quaternion(body.quaternion.x, body.quaternion.y, body.quaternion.z, body.quaternion.w)
}

export function isBallPocketed(id: number, pocketedBalls: BallId[]): boolean {
  return pocketedBalls.includes(id as BallId)
}
