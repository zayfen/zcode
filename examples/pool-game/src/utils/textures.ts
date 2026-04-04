import * as THREE from 'three';
import type { BallId } from '../types';
import { BALL_COLORS } from '../constants/balls';

/** Generate a procedural texture for a billiard ball */
export function generateBallTexture(id: BallId): THREE.CanvasTexture {
  const width = 1024;
  const height = 512;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;

  const color = BALL_COLORS[id];

  if (id === 0) {
    // Cue ball - white with subtle gloss
    ctx.fillStyle = '#f8f8f8';
    ctx.fillRect(0, 0, width, height);
    // Subtle specular highlight
    const grad = ctx.createRadialGradient(width * 0.35, height * 0.35, 0, width * 0.5, height * 0.5, width * 0.5);
    grad.addColorStop(0, 'rgba(255,255,255,0.3)');
    grad.addColorStop(1, 'rgba(255,255,255,0)');
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, width, height);
  } else if (id >= 1 && id <= 8) {
    // Solid balls - fill with color, white number circle
    ctx.fillStyle = color;
    ctx.fillRect(0, 0, width, height);

    // Number circle (white background)
    drawNumberCircle(ctx, width / 2, height / 2, id, width);
  } else {
    // Stripe balls (9-15) - white background, colored stripe in middle 50%
    ctx.fillStyle = '#f8f8f8';
    ctx.fillRect(0, 0, width, height);

    // Colored stripe band in the middle
    const stripeTop = height * 0.25;
    const stripeBottom = height * 0.75;
    ctx.fillStyle = color;
    ctx.fillRect(0, stripeTop, width, stripeBottom - stripeTop);

    // Anti-aliased stripe edges
    const edgeGrad1 = ctx.createLinearGradient(0, stripeTop - 4, 0, stripeTop + 4);
    edgeGrad1.addColorStop(0, '#f8f8f8');
    edgeGrad1.addColorStop(1, color);
    ctx.fillStyle = edgeGrad1;
    ctx.fillRect(0, stripeTop - 4, width, 8);

    const edgeGrad2 = ctx.createLinearGradient(0, stripeBottom - 4, 0, stripeBottom + 4);
    edgeGrad2.addColorStop(0, color);
    edgeGrad2.addColorStop(1, '#f8f8f8');
    ctx.fillStyle = edgeGrad2;
    ctx.fillRect(0, stripeBottom - 4, width, 8);

    // Number circle
    drawNumberCircle(ctx, width / 2, height / 2, id, width);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

function drawNumberCircle(
  ctx: CanvasRenderingContext2D,
  cx: number,
  cy: number,
  number: number,
  canvasWidth: number
) {
  const radius = canvasWidth * 0.08;

  // White circle background
  ctx.beginPath();
  ctx.arc(cx, cy, radius, 0, Math.PI * 2);
  ctx.fillStyle = '#ffffff';
  ctx.fill();
  ctx.strokeStyle = '#cccccc';
  ctx.lineWidth = 2;
  ctx.stroke();

  // Number text
  ctx.fillStyle = '#000000';
  ctx.font = `bold ${radius * 1.2}px Arial, sans-serif`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(number), cx, cy + 1);
}

/** Generate felt texture with subtle noise */
export function generateFeltTexture(): THREE.CanvasTexture {
  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;

  // Base green
  ctx.fillStyle = '#0d6b2e';
  ctx.fillRect(0, 0, size, size);

  // Add noise grain
  const imageData = ctx.getImageData(0, 0, size, size);
  for (let i = 0; i < imageData.data.length; i += 4) {
    const noise = (Math.random() - 0.5) * 12;
    imageData.data[i] = Math.max(0, Math.min(255, imageData.data[i] + noise));
    imageData.data[i + 1] = Math.max(0, Math.min(255, imageData.data[i + 1] + noise));
    imageData.data[i + 2] = Math.max(0, Math.min(255, imageData.data[i + 2] + noise));
  }
  ctx.putImageData(imageData, 0, 0);

  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(4, 4);
  return texture;
}

/** Generate wood grain texture */
export function generateWoodTexture(): THREE.CanvasTexture {
  const width = 256;
  const height = 256;
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d')!;

  // Base brown
  ctx.fillStyle = '#5c3a1e';
  ctx.fillRect(0, 0, width, height);

  // Wood grain lines
  ctx.strokeStyle = 'rgba(30, 15, 5, 0.3)';
  ctx.lineWidth = 1;
  for (let y = 0; y < height; y += 3 + Math.random() * 4) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    let x = 0;
    while (x < width) {
      x += 5 + Math.random() * 10;
      ctx.lineTo(x, y + (Math.random() - 0.5) * 2);
    }
    ctx.stroke();
  }

  const texture = new THREE.CanvasTexture(canvas);
  return texture;
}
