import { useRef, useEffect, useCallback } from 'react'
import * as THREE from 'three'
import { useGameStore } from '../store/gameStore'
import { BALL_RADIUS, POCKET_POSITIONS, POCKET_RADIUS, TABLE_LENGTH, TABLE_WIDTH } from '../constants/table'
import { ALL_BALL_IDS, getRackPositions } from '../constants/balls'
import { MAX_IMPULSE, SETTLE_LINEAR_THRESHOLD, SETTLE_ANGULAR_THRESHOLD, SETTLE_FRAMES } from '../constants/physics'
import { useAimStore } from '../store/aimStore'
import { getBallGroup } from '../constants/balls'
import type { BallId, BallState } from '../types'
import * as CANNON from 'cannon-es'

// Shared physics world - singleton
let world: CANNON.World | null = null
const ballBodies: Map<number, CANNON.Body> = new Map()
const ballMeshes: Map<number, THREE.Mesh> = new Map()

// Contact tracking
let firstContactThisShot: BallId | null = null
let contactsThisShot: Set<number> = new Set()

export function getWorld(): CANNON.World {
  if (!world) {
    world = new CANNON.World()
    world.gravity.set(0, -9.81, 0)
    world.broadphase = new CANNON.SAPBroadphase(world)

    const ballBallMat = new CANNON.Material('ballBall')
    const ballFeltMat = new CANNON.Material('ballFelt')
    const ballCushionMat = new CANNON.Material('ballCushion')

    world.addContactMaterial(new CANNON.ContactMaterial(ballBallMat, ballBallMat, {
      friction: 0.05,
      restitution: 0.95,
    }))
    world.addContactMaterial(new CANNON.ContactMaterial(ballBallMat, ballFeltMat, {
      friction: 0.4,
      restitution: 0.1,
    }))
    world.addContactMaterial(new CANNON.ContactMaterial(ballBallMat, ballCushionMat, {
      friction: 0.1,
      restitution: 0.7,
    }))

    // Store materials for later use
    ;(world as any)._ballMat = ballBallMat
    ;(world as any)._feltMat = ballFeltMat
    ;(world as any)._cushionMat = ballCushionMat

    createTableBodies()
    createBallBodies()
  }
  return world
}

function createTableBodies() {
  const w = world!
  const feltMat = (w as any)._feltMat as CANNON.Material
  const cushionMat = (w as any)._cushionMat as CANNON.Material

  // Felt plane (ground)
  const feltBody = new CANNON.Body({
    mass: 0,
    material: feltMat,
    shape: new CANNON.Plane(),
  })
  feltBody.quaternion.setFromEuler(-Math.PI / 2, 0, 0)
  w.addBody(feltBody)

  const halfL = TABLE_LENGTH / 2
  const halfW = TABLE_WIDTH / 2
  const cushionH = 0.05

  // Cushion bodies
  const cushionDefs: { pos: [number, number, number]; size: [number, number, number] }[] = [
    // Long rails (split by side pockets)
    { pos: [-halfL / 2, cushionH / 2, -(halfW)], size: [halfL * 0.85, cushionH, 0.03] },
    { pos: [halfL / 2, cushionH / 2, -(halfW)], size: [halfL * 0.85, cushionH, 0.03] },
    { pos: [-halfL / 2, cushionH / 2, halfW], size: [halfL * 0.85, cushionH, 0.03] },
    { pos: [halfL / 2, cushionH / 2, halfW], size: [halfL * 0.85, cushionH, 0.03] },
    // Short rails
    { pos: [-(halfL), cushionH / 2, 0], size: [0.03, cushionH, TABLE_WIDTH * 0.75] },
    { pos: [halfL, cushionH / 2, 0], size: [0.03, cushionH, TABLE_WIDTH * 0.75] },
  ]

  for (const def of cushionDefs) {
    const body = new CANNON.Body({
      mass: 0,
      material: cushionMat,
      shape: new CANNON.Box(new CANNON.Vec3(def.size[0] / 2, def.size[1] / 2, def.size[2] / 2)),
    })
    body.position.set(def.pos[0], def.pos[1], def.pos[2])
    w.addBody(body)
  }
}

function createBallBodies() {
  const w = world!
  const ballMat = (w as any)._ballMat as CANNON.Material
  const positions = getRackPositions()

  for (const id of ALL_BALL_IDS) {
    const pos = positions[id as BallId]
    const body = new CANNON.Body({
      mass: 0.17,
      material: ballMat,
      shape: new CANNON.Sphere(BALL_RADIUS),
      linearDamping: 0.4,
      angularDamping: 0.4,
    })
    body.position.set(pos[0], BALL_RADIUS, pos[2])
    w.addBody(body)
    ballBodies.set(id, body)

    // Contact detection
    body.addEventListener('collide', (e: { body: CANNON.Body; contact: CANNON.ContactEquation }) => {
      const otherBody = e.body
      // Find which ball was hit
      for (const [otherId, otherBallBody] of ballBodies) {
        if (otherBallBody === otherBody && otherId !== id) {
          contactsThisShot.add(otherId)
          if (id === 0 && firstContactThisShot === null) {
            firstContactThisShot = otherId as BallId
          }
          break
        }
      }
    })
  }
}

export function getBallBody(id: number): CANNON.Body | undefined {
  return ballBodies.get(id)
}

export function registerBallMesh(id: number, mesh: THREE.Mesh) {
  ballMeshes.set(id, mesh)
}

export function unregisterBallMesh(id: number) {
  ballMeshes.delete(id)
}

export function resetPhysics() {
  if (world) {
    // Remove all bodies
    while (world.bodies.length > 0) {
      world.removeBody(world.bodies[0])
    }
    ballBodies.clear()
    ballMeshes.clear()
    world = null
    firstContactThisShot = null
    contactsThisShot.clear()
  }
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
  contactsThisShot.clear()
}

export function applyImpulseToCueBall(direction: THREE.Vector3, power: number) {
  const cueBody = ballBodies.get(0)
  if (!cueBody) return

  firstContactThisShot = null
  contactsThisShot.clear()

  const impulse = new CANNON.Vec3(
    direction.x * power * MAX_IMPULSE,
    0,
    direction.z * power * MAX_IMPULSE,
  )
  cueBody.wakeUp()
  cueBody.applyImpulse(impulse)
}

export function stepWorld(dt: number) {
  if (!world) return
  world.step(1 / 60, dt, 3)
}

export function syncMeshes(pocketedBalls: BallId[]) {
  for (const [id, body] of ballBodies) {
    if (pocketedBalls.includes(id as BallId)) continue
    const mesh = ballMeshes.get(id)
    if (mesh) {
      mesh.position.set(body.position.x, body.position.y, body.position.z)
      mesh.quaternion.set(
        body.quaternion.x,
        body.quaternion.y,
        body.quaternion.z,
        body.quaternion.w,
      )
    }
  }
}

export function checkPockets(): BallId[] {
  const pocketed: BallId[] = []
  for (const [id, body] of ballBodies) {
    for (const pocket of POCKET_POSITIONS) {
      const dx = body.position.x - pocket.x
      const dz = body.position.z - pocket.z
      const dist = Math.sqrt(dx * dx + dz * dz)
      if (dist < POCKET_RADIUS) {
        pocketed.push(id as BallId)
        break
      }
    }
  }
  return pocketed
}

export function removeBallFromPlay(id: BallId) {
  const body = ballBodies.get(id)
  if (body) {
    body.position.set(0, -1, 0) // Move below table
    body.velocity.setZero()
    body.angularVelocity.setZero()
    body.sleep()
  }
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

export function isSettled(): boolean {
  for (const [id, body] of ballBodies) {
    const lv = body.velocity.length()
    const av = body.angularVelocity.length()
    if (lv > SETTLE_LINEAR_THRESHOLD || av > SETTLE_ANGULAR_THRESHOLD) {
      return false
    }
  }
  return true
}

export function getFirstContact(): BallId | null {
  return firstContactThisShot
}

export function getCueBallPosition(): { x: number; z: number } {
  const cueBody = ballBodies.get(0)
  if (!cueBody) return { x: 0, z: 0 }
  return { x: cueBody.position.x, z: cueBody.position.z }
}

export function getBallStates(): BallState[] {
  const states: BallState[] = []
  for (const [id, body] of ballBodies) {
    states.push({
      id: id as BallId,
      position: [body.position.x, body.position.y, body.position.z],
      velocity: [body.velocity.x, body.velocity.y, body.velocity.z],
      angularVelocity: [body.angularVelocity.x, body.angularVelocity.y, body.angularVelocity.z],
      pocketed: false,
    })
  }
  return states
}
