import type { BallId } from '../types';
import { BALL_COLORS } from '../constants/balls';
import { isStripe } from '../types';

// ─────────────────────────────────────────────────────────────────────────────
// T24: Enhanced texture resolution 1024×512 for sharper ball visuals
// ─────────────────────────────────────────────────────────────────────────────
const TEXTURE_WIDTH = 1024;
const TEXTURE_HEIGHT = 512;

/**
 * Generate a procedural canvas texture for a billiard ball (T24 enhanced).
 *
 * - Cue ball (id 0): plain white with subtle specular highlight
 * - Solid balls (1-7): solid color fill + white number circle
 * - Stripe balls (9-15): white background + colored band middle 50% + white number circle
 * - Eight ball (8): solid black + white number circle
 *
 * All balls get a baked-in subtle specular highlight for visual polish.
 * Numbers are anti-aliased via 2× downscale rendering for crisp edges.
 */
export function generateBallTexture(id: BallId): HTMLCanvasElement {
  // Render at 2× resolution then downscale for anti-aliased number text
  const scale = 2;
  const rw = TEXTURE_WIDTH * scale;
  const rh = TEXTURE_HEIGHT * scale;

  const renderCanvas = document.createElement('canvas');
  renderCanvas.width = rw;
  renderCanvas.height = rh;
  const ctx = renderCanvas.getContext('2d')!;

  // ── Base fill ──────────────────────────────────────────────────────────
  if (id === 0) {
    // Cue ball: plain white
    ctx.fillStyle = '#F8F8FF';
    ctx.fillRect(0, 0, rw, rh);
  } else {
    const color = BALL_COLORS[id];
    const stripe = isStripe(id);

    if (stripe) {
      // Stripe ball: white background with colored band in middle 50%
      ctx.fillStyle = '#FFFFFF';
      ctx.fillRect(0, 0, rw, rh);

      const bandTop = rh * 0.25;
      const bandBottom = rh * 0.75;
      ctx.fillStyle = color;
      ctx.fillRect(0, bandTop, rw, bandBottom - bandTop);

      // Soft feathered edges on stripe band for anti-aliased look
      const feather = rh * 0.02;
      const gradTop = ctx.createLinearGradient(0, bandTop - feather, 0, bandTop + feather);
      gradTop.addColorStop(0, 'rgba(255,255,255,1)');
      gradTop.addColorStop(1, 'rgba(255,255,255,0)');
      ctx.fillStyle = gradTop;
      ctx.fillRect(0, bandTop - feather, rw, feather * 2);

      const gradBot = ctx.createLinearGradient(0, bandBottom - feather, 0, bandBottom + feather);
      gradBot.addColorStop(0, 'rgba(255,255,255,0)');
      gradBot.addColorStop(1, 'rgba(255,255,255,1)');
      ctx.fillStyle = gradBot;
      ctx.fillRect(0, bandBottom - feather, rw, feather * 2);
    } else {
      // Solid ball: entire surface is the ball color
      ctx.fillStyle = color;
      ctx.fillRect(0, 0, rw, rh);
    }
  }

  // ── Number circle & text ───────────────────────────────────────────────
  if (id !== 0) {
    const cx = rw / 2;
    const cy = rh / 2;
    const circleRadius = 56 * scale; // proportional to larger canvas

    // White circle background
    ctx.beginPath();
    ctx.arc(cx, cy, circleRadius, 0, Math.PI * 2);
    ctx.fillStyle = '#FFFFFF';
    ctx.fill();
    ctx.strokeStyle = '#444444';
    ctx.lineWidth = 1.5 * scale;
    ctx.stroke();

    // Anti-aliased number text (rendered large, will be downscaled)
    ctx.fillStyle = '#000000';
    ctx.font = `bold ${54 * scale}px Arial, sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(String(id), cx, cy + 2 * scale);
  }

  // ── Baked specular highlight ───────────────────────────────────────────
  // Subtle specular highlight in the upper-left region of the texture,
  // simulating a light reflection baked into the surface.
  const specX = rw * 0.33;
  const specY = rh * 0.33;
  const specRadius = rw * 0.28;
  const specular = ctx.createRadialGradient(specX, specY, 0, specX, specY, specRadius);
  specular.addColorStop(0, 'rgba(255, 255, 255, 0.18)');
  specular.addColorStop(0.4, 'rgba(255, 255, 255, 0.06)');
  specular.addColorStop(1, 'rgba(255, 255, 255, 0)');
  ctx.fillStyle = specular;
  ctx.fillRect(0, 0, rw, rh);

  // ── Downscale to final resolution for anti-aliased result ──────────────
  const finalCanvas = document.createElement('canvas');
  finalCanvas.width = TEXTURE_WIDTH;
  finalCanvas.height = TEXTURE_HEIGHT;
  const fctx = finalCanvas.getContext('2d')!;
  fctx.imageSmoothingEnabled = true;
  fctx.imageSmoothingQuality = 'high';
  fctx.drawImage(renderCanvas, 0, 0, TEXTURE_WIDTH, TEXTURE_HEIGHT);

  return finalCanvas;
}

// ─────────────────────────────────────────────────────────────────────────────
// T25: Felt noise texture – canvas-generated subtle grain pattern
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Generate a felt texture for the table bed.
 * Dark green base with layered subtle noise grain for a realistic cloth look.
 * Uses a larger canvas (1024×1024) for finer grain detail.
 */
export function generateFeltTexture(): HTMLCanvasElement {
  const size = 1024;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;

  // Base green
  ctx.fillStyle = '#0d6b2e';
  ctx.fillRect(0, 0, size, size);

  // Layer 1: coarse noise for fabric weave impression
  const imageData = ctx.getImageData(0, 0, size, size);
  const data = imageData.data;
  for (let i = 0; i < data.length; i += 4) {
    const noise = (Math.random() - 0.5) * 12;
    data[i] = Math.max(0, Math.min(255, data[i] + noise));
    data[i + 1] = Math.max(0, Math.min(255, data[i + 1] + noise));
    data[i + 2] = Math.max(0, Math.min(255, data[i + 2] + noise));
  }
  ctx.putImageData(imageData, 0, 0);

  // Layer 2: subtle horizontal fibre lines (felt cloth direction)
  ctx.globalAlpha = 0.04;
  ctx.strokeStyle = '#000000';
  for (let y = 0; y < size; y += 2 + Math.random() * 3) {
    ctx.lineWidth = 0.5 + Math.random() * 0.5;
    ctx.beginPath();
    ctx.moveTo(0, y);
    let x = 0;
    while (x < size) {
      x += 8 + Math.random() * 16;
      ctx.lineTo(x, y + (Math.random() - 0.5) * 1.5);
    }
    ctx.stroke();
  }
  ctx.globalAlpha = 1.0;

  // Layer 3: very subtle vertical cross-fibres
  ctx.globalAlpha = 0.02;
  for (let x = 0; x < size; x += 3 + Math.random() * 4) {
    ctx.lineWidth = 0.3 + Math.random() * 0.4;
    ctx.beginPath();
    ctx.moveTo(x, 0);
    let y = 0;
    while (y < size) {
      y += 8 + Math.random() * 16;
      ctx.lineTo(x + (Math.random() - 0.5) * 1.0, y);
    }
    ctx.stroke();
  }
  ctx.globalAlpha = 1.0;

  return canvas;
}

// ─────────────────────────────────────────────────────────────────────────────
// T25: Wood grain texture – brown gradient with dark grain lines
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Generate a wood grain texture for the rails.
 * Rich brown gradient base with darker grain lines and subtle variation
 * for a realistic polished wood appearance.
 */
export function generateWoodTexture(): HTMLCanvasElement {
  const size = 512;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d')!;

  // Base brown gradient (slight vertical variation for depth)
  const baseGrad = ctx.createLinearGradient(0, 0, 0, size);
  baseGrad.addColorStop(0, '#6B3A20');
  baseGrad.addColorStop(0.3, '#5C3317');
  baseGrad.addColorStop(0.7, '#4E2B14');
  baseGrad.addColorStop(1, '#5A3018');
  ctx.fillStyle = baseGrad;
  ctx.fillRect(0, 0, size, size);

  // Subtle horizontal colour variation
  ctx.globalAlpha = 0.06;
  for (let x = 0; x < size; x += 20 + Math.random() * 40) {
    const w = 10 + Math.random() * 30;
    const lightness = Math.random() > 0.5 ? '#8B5E3C' : '#3A1F0D';
    ctx.fillStyle = lightness;
    ctx.fillRect(x, 0, w, size);
  }
  ctx.globalAlpha = 1.0;

  // Primary dark grain lines (the main visible grain)
  ctx.globalAlpha = 0.18;
  for (let y = 0; y < size; y += 3 + Math.random() * 5) {
    ctx.strokeStyle = `rgba(0, 0, 0, ${0.1 + Math.random() * 0.15})`;
    ctx.lineWidth = 0.5 + Math.random() * 1.2;
    ctx.beginPath();
    ctx.moveTo(0, y);
    let x = 0;
    while (x < size) {
      x += 8 + Math.random() * 20;
      ctx.lineTo(x, y + (Math.random() - 0.5) * 2.5);
    }
    ctx.stroke();
  }
  ctx.globalAlpha = 1.0;

  // Occasional knot / wider grain feature
  ctx.globalAlpha = 0.08;
  for (let k = 0; k < 3; k++) {
    const kx = Math.random() * size;
    const ky = Math.random() * size;
    const kr = 5 + Math.random() * 10;
    const knotGrad = ctx.createRadialGradient(kx, ky, 0, kx, ky, kr);
    knotGrad.addColorStop(0, '#2A1508');
    knotGrad.addColorStop(1, 'rgba(42, 21, 8, 0)');
    ctx.fillStyle = knotGrad;
    ctx.fillRect(kx - kr, ky - kr, kr * 2, kr * 2);
  }
  ctx.globalAlpha = 1.0;

  // Fine noise over the top for micro-texture
  const imgData = ctx.getImageData(0, 0, size, size);
  const px = imgData.data;
  for (let i = 0; i < px.length; i += 4) {
    const n = (Math.random() - 0.5) * 8;
    px[i] = Math.max(0, Math.min(255, px[i] + n));
    px[i + 1] = Math.max(0, Math.min(255, px[i + 1] + n));
    px[i + 2] = Math.max(0, Math.min(255, px[i + 2] + n));
  }
  ctx.putImageData(imgData, 0, 0);

  return canvas;
}
