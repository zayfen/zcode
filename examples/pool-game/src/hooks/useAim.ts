import { useFrame, useThree } from '@react-three/fiber'
import { useRef, useEffect, useCallback } from 'react'
import * as THREE from 'three'
import { useGameStore } from '../store/gameStore'

interface AimState {
  direction: THREE.Vector3
  power: number
  isCharging: boolean
  hitPoint: THREE.Vector3 | null
}

const aimState: AimState = {
  direction: new THREE.Vector3(0, 0, -1),
  power: 0,
  isCharging: false,
  hitPoint: null,
}

export function useAim() {
  const { camera, raycaster, pointer } = useThree()
  const phase = useGameStore(s => s.phase)
  const tablePlane = useRef(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0))
  const charging = useRef(false)
  const chargeStart = useRef(0)

  const updateAim = useCallback(() => {
    if (phase !== 'AIMING' && phase !== 'POWER') return

    const ray = raycaster
    // Cast ray from camera through mouse position
    ray.setFromCamera(pointer, camera)

    const intersectPoint = new THREE.Vector3()
    ray.ray.intersectPlane(tablePlane.current, intersectPoint)

    if (intersectPoint) {
      aimState.hitPoint = intersectPoint.clone()
    }
  }, [camera, raycaster, pointer, phase])

  return { aimState, updateAim, charging, chargeStart }
}

export { aimState }
