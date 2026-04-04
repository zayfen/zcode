import { useRef, useEffect, useMemo, useCallback } from 'react'
import * as THREE from 'three'
import { useFrame, useThree } from '@react-three/fiber'
import { BALL_RADIUS, TABLE_LENGTH, TABLE_WIDTH, POCKET_POSITIONS, POCKET_RADIUS, TABLE_HEIGHT } from '../constants/table'
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
let newlyPocketedThisShot: BallId[] = []

function getOrCreateWorld(): CANNON.World {
  if (world) return world

  world = new CANNON.World()
  world.gravity.set(0, -9.81, 0)
  world.broadphase = new CANNON.SAPBroadphase(world)
  ;(world.solver as CANNON.GSSolver).iterations = 10

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
  ground.position.set(0, TABLE_HEIGHT, 0)
  ground.quaternion.setFromEuler(-Math.PI / 2, 0, 0)
  world.addBody(ground)

  // Cushions
  const halfL = TABLE_LENGTH / 2
  const halfW = TABLE_WIDTH / 2
  const ch = 0.1
  const ct = 0.15

  const cushionDefs: { pos: [number, number, number]; size: [number, number, number] }[] = [
    // Long rails (Left, split at middle)
    { pos: [-halfW - ct/2 + 0.05, TABLE_HEIGHT + ch / 2, -halfL / 2], size: [ct, ch, halfL * 0.85] },
    { pos: [-halfW - ct/2 + 0.05, TABLE_HEIGHT + ch / 2, halfL / 2], size: [ct, ch, halfL * 0.85] },
    // Long rails (Right, split at middle)
    { pos: [halfW + ct/2 - 0.05, TABLE_HEIGHT + ch / 2, -halfL / 2], size: [ct, ch, halfL * 0.85] },
    { pos: [halfW + ct/2 - 0.05, TABLE_HEIGHT + ch / 2, halfL / 2], size: [ct, ch, halfL * 0.85] },
    // Short rails (Top/Bottom, solid)
    { pos: [0, TABLE_HEIGHT + ch / 2, -halfL - ct/2 + 0.05], size: [halfW * 0.85, ch, ct] },
    { pos: [0, TABLE_HEIGHT + ch / 2, halfL + ct/2 - 0.05], size: [halfW * 0.85, ch, ct] },
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
    body.position.set(pos.x, pos.y, pos.z)
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
    body.position.set(pos.x, pos.y, pos.z)
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
    Math.max(-halfW, Math.min(halfW, x)),
    TABLE_HEIGHT + BALL_RADIUS,
    Math.max(-halfL, Math.min(halfL, z)),
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
        if (state.pocketedBalls.includes(id as BallId) || newlyPocketedThisShot.includes(id as BallId)) continue
        for (const pocket of POCKET_POSITIONS) {
          const dx = body.position.x - pocket[0]
          const dz = body.position.z - pocket[2]
          const dist = Math.sqrt(dx * dx + dz * dz)
          if (dist < POCKET_RADIUS) {
            newlyPocketedThisShot.push(id as BallId)
            // Move ball below table
            body.position.set(0, -1, 0)
            body.velocity.setZero()
            body.angularVelocity.setZero()
            body.sleep()
            break
          }
        }
      }

      // Settle detection & state sync
      let allSettled = true
      for (const [id, body] of ballBodies) {
        state.updateBallPosition(id, [body.position.x, body.position.y, body.position.z])

        const lv = body.velocity.length()
        const av = body.angularVelocity.length()
        if (lv > SETTLE_LINEAR_THRESHOLD || av > SETTLE_ANGULAR_THRESHOLD) {
          allSettled = false
        }
      }

      if (allSettled) {
        settledFrames.current++
        if (settledFrames.current >= 10) {
          settledFrames.current = 0
          
          let foul: Foul = null;
          if (newlyPocketedThisShot.includes(0)) foul = 'SCRATCH';
          else if (firstContactThisShot === null) foul = 'NO_BALL_HIT';

          state.evaluateShot(newlyPocketedThisShot, firstContactThisShot, foul)
          aimState.resetShot()
          firstContactThisShot = null
        }
      } else {
        settledFrames.current = 0
      }
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
            state.setBallInHand(null)
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
        newlyPocketedThisShot = []
        settledFrames.current = 0

        // Save snapshot for undo
        state.takeSnapshot()

        // Apply impulse to cue ball
        const cueBody = ballBodies.get(0)
        if (cueBody) {
          const dirVec = new THREE.Vector3(aimState.direction.x, aimState.direction.y, aimState.direction.z)
          const dir = dirVec.clone().normalize()
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
          state.undo()
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
