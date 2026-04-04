import { useRef, useCallback, useEffect } from 'react'
import { useFrame, useThree } from '@react-three/fiber'
import * as THREE from 'three'
import { useGameStore } from '../store/gameStore'
import { useAimStore } from '../store/aimStore'
import { getPhysicsWorld } from '../physics/physicsEngine'
import { BALL_RADIUS, TABLE_LENGTH, TABLE_WIDTH } from '../constants/table'
import { MAX_IMPULSE, POWER_CHARGE_DURATION } from '../constants/physics'
import { vec3XZDistance } from '../utils/vector'
import { POCKET_CENTERS, POCKET_RADIUS } from '../constants/table'
import type { BallId, BallState, Foul } from '../types'

/** Hook that orchestrates aiming, power, shooting, and physics stepping */
export function useShotSequence(getBallPositions: () => Map<number, THREE.Vector3>) {
  const phase = useGameStore(s => s.phase)
  const currentPlayer = useGameStore(s => s.currentPlayer)
  const startPower = useGameStore(s => s.startPower)
  const shoot = useGameStore(s => s.shoot)
  const evaluateShot = useGameStore(s => s.evaluateShot)
  const setFoul = useGameStore(s => s.setFoul)
  const nextTurn = useGameStore(s => s.nextTurn)
  const setGameOver = useGameStore(s => s.setGameOver)
  const pocketBall = useGameStore(s => s.pocketBall)
  const assignGroups = useGameStore(s => s.assignGroups)
  const playerGroups = useGameStore(s => s.playerGroups)
  const pocketedBalls = useGameStore(s => s.pocketedBalls)
  const groupsAssigned = useGameStore(s => s.groupsAssigned)
  const breakShot = useGameStore(s => s.breakShot)
  const setBreakShot = useGameStore(s => s.setBreakShot)
  const saveSnapshot = useGameStore(s => s.saveSnapshot)
  const setBallInHand = useGameStore(s => s.setBallInHand)

  const aimDir = useAimStore(s => s.direction)
  const aimPower = useAimStore(s => s.power)
  const setPower = useAimStore(s => s.setPower)
  const setIsCharging = useAimStore(s => s.setIsCharging)
  const aimReset = useAimStore(s => s.reset)

  const { camera, raycaster, pointer } = useThree()
  const tablePlane = useRef(new THREE.Plane(new THREE.Vector3(0, 1, 0), -BALL_RADIUS))
  const chargeStart = useRef(0)
  const shotActive = useRef(false)
  const preShotBalls = useRef<BallId[]>([])
  const firstContact = useRef<BallId | null>(null)
  const settledFrames = useRef(0)

  // Track mouse for aiming
  useFrame(() => {
    if (phase === 'AIMING') {
      raycaster.setFromCamera(pointer, camera)
      const hit = new THREE.Vector3()
      raycaster.ray.intersectPlane(tablePlane.current, hit)

      if (hit) {
        const positions = getBallPositions()
        const cuePos = positions.get(0)
        if (cuePos) {
          const dir = new THREE.Vector3().subVectors(hit, cuePos)
          dir.y = 0
          if (dir.length() > 0.001) {
            dir.normalize()
            useAimStore.getState().setDirection(dir)
          }
        }
      }
    }

    if (phase === 'POWER') {
      const elapsed = (performance.now() - chargeStart.current) / 1000
      const pwr = Math.min(elapsed / POWER_CHARGE_DURATION, 1)
      setPower(pwr)
    }

    if (phase === 'SIMULATING') {
      const world = getPhysicsWorld()
      if (world) {
        world.step(1 / 60)
      }

      // Check pocketing
      const positions = getBallPositions()
      positions.forEach((pos, id) => {
        for (const pocket of POCKET_CENTERS) {
          const dist = vec3XZDistance(pos, pocket)
          if (dist < POCKET_RADIUS) {
            if (!preShotBalls.current.includes(id as BallId) || 
                !useGameStore.getState().pocketedBalls.includes(id as BallId)) {
              pocketBall(id as BallId)
            }
          }
        }
      })

      // Check settle
      let allSettled = true
      const world2 = getPhysicsWorld()
      if (world2) {
        const bodies = world2.bodies
        for (const body of bodies) {
          const lv = body.velocity
          const av = body.angularVelocity
          if (Math.abs(lv.x) > 0.001 || Math.abs(lv.y) > 0.001 || Math.abs(lv.z) > 0.001 ||
              Math.abs(av.x) > 0.01 || Math.abs(av.y) > 0.01 || Math.abs(av.z) > 0.01) {
            allSettled = false
            break
          }
        }
      }

      if (allSettled) {
        settledFrames.current++
        if (settledFrames.current >= 10) {
          evaluateShot()
        }
      } else {
        settledFrames.current = 0
      }
    }
  })

  // Handle evaluate
  useEffect(() => {
    if (phase !== 'EVALUATING') return

    const state = useGameStore.getState()
    const newlyPocketed = state.pocketedBalls.filter(b => !preShotBalls.current.includes(b))
    const cuePocketed = newlyPocketed.includes(0)

    let foul: Foul = null
    if (cuePocketed) {
      foul = 'SCRATCH'
    } else if (!firstContact.current && !breakShot) {
      foul = 'NO_BALL_HIT'
    }

    const pocketedNonCue = newlyPocketed.filter(b => b !== 0)

    // Group assignment
    let groupAssigned = false
    if (!groupsAssigned && pocketedNonCue.length > 0) {
      const firstPocketed = pocketedNonCue[0]
      if (firstPocketed >= 1 && firstPocketed <= 7) {
        assignGroups(currentPlayer, 'solids')
        groupAssigned = true
      } else if (firstPocketed >= 9 && firstPocketed <= 15) {
        assignGroups(currentPlayer, 'stripes')
        groupAssigned = true
      }
    }

    // Check 8-ball
    if (newlyPocketed.includes(8)) {
      const myGroup = playerGroups[currentPlayer]
      const myBalls = myGroup === 'solids'
        ? [1,2,3,4,5,6,7] as BallId[]
        : myGroup === 'stripes'
        ? [9,10,11,12,13,14,15] as BallId[]
        : []
      const allCleared = myBalls.every(b => state.pocketedBalls.includes(b))

      if (allCleared && !foul) {
        setGameOver(currentPlayer)
      } else {
        setGameOver(currentPlayer === 1 ? 2 : 1)
      }
      return
    }

    if (foul) {
      setFoul(foul)
      const opponent = currentPlayer === 1 ? 2 : 1
      setBallInHand(true)
      nextTurn(opponent as 1 | 2)
    } else if (pocketedNonCue.length > 0) {
      // Player continues
      useGameStore.getState().setPhase('IDLE')
    } else {
      nextTurn((currentPlayer === 1 ? 2 : 1) as 1 | 2)
    }

    setBreakShot(false)
    aimReset()
  }, [phase])

  const onMouseDown = useCallback(() => {
    if (phase === 'AIMING') {
      startPower()
      chargeStart.current = performance.now()
      setIsCharging(true)
    }
  }, [phase])

  const onMouseUp = useCallback(() => {
    if (phase === 'POWER') {
      const positions = getBallPositions()
      const state = useGameStore.getState()
      preShotBalls.current = [...state.pocketedBalls]
      firstContact.current = null

      // Save snapshot for undo
      const ballStates: BallState[] = []
      positions.forEach((pos, id) => {
        ballStates.push({
          id: id as BallId,
          position: [pos.x, pos.y, pos.z],
          velocity: [0, 0, 0],
          angularVelocity: [0, 0, 0],
          pocketed: state.pocketedBalls.includes(id as BallId),
        })
      })
      saveSnapshot(ballStates)

      // Apply impulse to cue ball
      const world = getPhysicsWorld()
      if (world) {
        const bodies = world.bodies
        for (const body of bodies) {
          if (body.userData?.ballId === 0) {
            const dir = aimDir.clone().normalize()
            const impulse = dir.multiplyScalar(aimPower * MAX_IMPULSE)
            body.applyImpulse(new (await import('cannon-es')).Vec3(impulse.x, 0, impulse.z))
            break
          }
        }
      }

      shoot()
      setIsCharging(false)
      setPower(0)
    }
  }, [phase, aimDir, aimPower])

  return { onMouseDown, onMouseUp }
}
