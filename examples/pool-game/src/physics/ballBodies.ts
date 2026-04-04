import { useRef } from 'react'
import { useFrame, useThree } from '@react-three/fiber'
import * as THREE from 'three'
import { useGameStore } from '../store/gameStore'

// Track per-ball positions and velocities for settle detection and pocket detection
interface BallBodyData {
  position: THREE.Vector3
  velocity: THREE.Vector3
  angularVelocity: THREE.Vector3
  api: any
}

const ballBodies = new Map<number, BallBodyData>()
const pocketedThisShot = new Set<number>()
let firstContactThisShot: number | null = null
let cueBallApi: any = null

export function registerBallBody(id: number, api: any) {
  const data: BallBodyData = {
    position: new THREE.Vector3(),
    velocity: new THREE.Vector3(),
    angularVelocity: new THREE.Vector3(),
    api,
  }

  api.position.subscribe((v: [number, number, number]) => {
    data.position.set(v[0], v[1], v[2])
  })
  api.velocity.subscribe((v: [number, number, number]) => {
    data.velocity.set(v[0], v[1], v[2])
  })
  api.angularVelocity.subscribe((v: [number, number, number]) => {
    data.angularVelocity.set(v[0], v[1], v[2])
  })

  ballBodies.set(id, data)
  if (id === 0) cueBallApi = api
  return data
}

export function unregisterBallBody(id: number) {
  ballBodies.delete(id)
  if (id === 0) cueBallApi = null
}

export function getBallPosition(id: number): THREE.Vector3 | undefined {
  return ballBodies.get(id)?.position
}

export function getBallVelocity(id: number): THREE.Vector3 | undefined {
  return ballBodies.get(id)?.velocity
}

export function getCueBallApi() {
  return cueBallApi
}

export function getBallApi(id: number) {
  return ballBodies.get(id)?.api
}

export function getAllBallPositions(): Record<number, [number, number, number]> {
  const positions: Record<number, [number, number, number]> = {}
  ballBodies.forEach((data, id) => {
    positions[id] = [data.position.x, data.position.y, data.position.z]
  })
  return positions
}

export function resetShotTracking() {
  pocketedThisShot.clear()
  firstContactThisShot = null
}

export function recordPocket(id: number) {
  pocketedThisShot.add(id)
}

export function recordFirstContact(id: number) {
  if (firstContactThisShot === null) {
    firstContactThisShot = id
  }
}

export function getShotResult() {
  return {
    pocketed: Array.from(pocketedThisShot) as number[],
    firstContact: firstContactThisShot,
  }
}
