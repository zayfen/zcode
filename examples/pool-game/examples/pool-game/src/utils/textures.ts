// src/utils/textures.ts
import * as THREE from 'three';
import { BALL_COLORS, isStripe } from '../constants/balls';
import { BALL_RADIUS } from '../constants/table';

const TEXTURE_WIDTH = 1024;
const TEXTURE_HEIGHT = 512;

export function generateBallTexture(ballId: number): THREE.CanvasTexture {
  const canvas = document.createElement('canvas');
  canvas.width = TEXTURE_WIDTH;
  canvas.height = TEXTURE_HEIGHT;
  const ctx = canvas.getContext('2d')!;

  const color = BALL_COLORS[ballId] || '#FFFFFF';

  if (ballId === 0) {
    // Cue ball - plain white with subtle sheen
    ctx.fillStyle = '#F8F8F8';
    ctx.fillRect(0, 0, TEXTURE_WIDTH, TEXTURE_HEIGHT);
    // Add subtle gradient for 3D effect
    const grad = ctx.createRadialGradient(
      TEXTURE_WIDTH * 0.35, TEXTURE_HEIGHT * 0.35, 0,
      TEXTURE_WIDTH * 0.5, TEXTURE_HEIGHT * 0.5, TEXTURE_WIDTH * 0.5
    );
    grad.addColorStop(0, 'rgba(255,255,255,0.3)');
    grad.addColorStop(1, 'rgba(0,0,0,0.05)');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, TEXTURE_WIDTH, TEXTURE_HEIGHT);
  } else if (isStripe(ballId)) {
    // Stripe ball - white background with colored band in middle 50%
    ctx.fillStyle = '#FFFFFF';
    ctx.fillRect(0, 0, TEXTURE_WIDTH, TEXTURE_HEIGHT);
    // Color band
    ctx.fillStyle = color;
    ctx.fillRect(0, TEXTURE_HEIGHT * 0.25, TEXTURE_WIDTH, TEXTURE_HEIGHT * 0.5);
    // Number circle
    drawNumberCircle(ctx, ballId, TEXTURE_WIDTH / 2, TEXTURE_HEIGHT / 2, TEXTURE_HEIGHT * 0.12);
  } else {
    // Solid ball - filled with color
    ctx.fillStyle = color;
    ctx.fillRect(0, 0, TEXTURE_WIDTH, TEXTURE_HEIGHT);
    // Number circle
    drawNumberCircle(ctx, ballId, TEXTURE_WIDTH / 2, TEXTURE_HEIGHT / 2, TEXTURE_HEIGHT * 0.12);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function drawNumberCircle(
  ctx: CanvasRenderingContext2D,
  ballId: number,
  cx: number,
  cy: number,
  radius: number
) {
  // White circle
  ctx.beginPath();
  ctx.arc(cx, cy, radius, 0, Math.PI * 2);
  ctx.fillStyle = '#FFFFFF';
  ctx.fill();

  // Number text
  ctx.fillStyle = '#000000';
  ctx.font = `bold ${radius * 1.4}px Arial`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(ballId), cx, cy + radius * 0.05);
}

// Cache textures
const textureCache = new Map<number, THREE.CanvasTexture>();

export function getBallTexture(ballId: number): THREE.CanvasTexture {
  if (!textureCache.has(ballId)) {
    textureCache.set(ballId, generateBallTexture(ballId));
  }
  return textureCache.get(ballId)!;
}
