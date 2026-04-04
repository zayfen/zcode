import { useRef, useEffect, useMemo, useCallback } from 'react'
import * as THREE from 'three'
import { BALL_RADIUS } from '../constants/table'
import { BALL_COLORS, isStripe } from '../constants/balls'
import type { BallId } from '../types'

interface BallProps {
  id: BallId
  position: [number, number, number]
  pocketed?: boolean
  meshRef?: React.RefObject<THREE.Mesh | null>
}

export default function Ball({ id, position, pocketed = false, meshRef: externalRef }: BallProps) {
  const internalRef = useRef<THREE.Mesh>(null)
  const meshRef = externalRef || internalRef
  const texture = useMemo(() => generateBallTexture(id), [id])

  useEffect(() => {
    if (meshRef.current) {
      meshRef.current.position.set(position[0], position[1], position[2])
    }
  }, [position])

  if (pocketed) return null

  return (
    <mesh
      ref={meshRef}
      position={position}
      castShadow
      receiveShadow
    >
      <sphereGeometry args={[BALL_RADIUS, 32, 32]} />
      <meshStandardMaterial
        map={texture}
        roughness={0.15}
        metalness={0.05}
      />
    </mesh>
  )
}

/** Generate a procedural texture for a billiard ball */
function generateBallTexture(id: BallId): THREE.CanvasTexture {
  const canvas = document.createElement('canvas')
  canvas.width = 1024
  canvas.height = 512
  const ctx = canvas.getContext('2d')!
  const color = BALL_COLORS[id]

  if (id === 0) {
    // Cue ball — plain white with subtle sheen
    ctx.fillStyle = '#f5f5f0'
    ctx.fillRect(0, 0, 1024, 512)
    const grad = ctx.createRadialGradient(400, 200, 20, 512, 256, 400)
    grad.addColorStop(0, 'rgba(255,255,255,0.4)')
    grad.addColorStop(1, 'rgba(255,255,255,0)')
    ctx.fillStyle = grad
    ctx.fillRect(0, 0, 1024, 512)
  } else if (id === 8) {
    // 8-ball: black with white number circle
    ctx.fillStyle = '#111111'
    ctx.fillRect(0, 0, 1024, 512)
    drawNumberCircle(ctx, 512, 256, 8)
  } else if (isStripe(id)) {
    // Stripe ball: white background, colored band in middle 50%
    ctx.fillStyle = '#ffffff'
    ctx.fillRect(0, 0, 1024, 512)
    ctx.fillStyle = color
    ctx.fillRect(0, 128, 1024, 256)
    drawNumberCircle(ctx, 512, 256, id)
  } else {
    // Solid ball: fully colored with white number circle
    ctx.fillStyle = color
    ctx.fillRect(0, 0, 1024, 512)
    drawNumberCircle(ctx, 512, 256, id)
  }

  const texture = new THREE.CanvasTexture(canvas)
  texture.colorSpace = THREE.SRGBColorSpace
  return texture
}

function drawNumberCircle(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  number: number,
) {
  const radius = 60
  ctx.beginPath()
  ctx.arc(x, y, radius, 0, Math.PI * 2)
  ctx.fillStyle = '#ffffff'
  ctx.fill()
  ctx.strokeStyle = '#cccccc'
  ctx.lineWidth = 2
  ctx.stroke()

  ctx.fillStyle = '#000000'
  ctx.font = 'bold 70px Arial, sans-serif'
  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.fillText(String(number), x, y + 2)
}
