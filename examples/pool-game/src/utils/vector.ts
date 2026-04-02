/**
 * 3D vector utilities operating on [x, y, z] tuples.
 *
 * These pure functions avoid allocating class instances and are
 * compatible with Three.js, cannon-es, and our own constants.
 */
export type Vec3Tuple = [number, number, number];

// ---------------------------------------------------------------------------
// Creation / conversion helpers
// ---------------------------------------------------------------------------

/** Create a Vec3Tuple from individual components. */
export function vec3(x: number, y: number, z: number): Vec3Tuple {
  return [x, y, z];
}

/** Zero vector constant (immutable – treat as read-only). */
export const VEC3_ZERO: Readonly<Vec3Tuple> = [0, 0, 0];

/** Unit vectors along each axis. */
export const VEC3_UP: Readonly<Vec3Tuple> = [0, 1, 0];
export const VEC3_RIGHT: Readonly<Vec3Tuple> = [1, 0, 0];
export const VEC3_FORWARD: Readonly<Vec3Tuple> = [0, 0, 1];

// ---------------------------------------------------------------------------
// Basic operations
// ---------------------------------------------------------------------------

/** Euclidean distance between two points. */
export function distance(a: Vec3Tuple, b: Vec3Tuple): number {
  const dx = a[0] - b[0];
  const dy = a[1] - b[1];
  const dz = a[2] - b[2];
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

/** Squared distance (avoids the sqrt when only comparisons are needed). */
export function distanceSq(a: Vec3Tuple, b: Vec3Tuple): number {
  const dx = a[0] - b[0];
  const dy = a[1] - b[1];
  const dz = a[2] - b[2];
  return dx * dx + dy * dy + dz * dz;
}

/** Length (magnitude) of a vector. */
export function length(v: Vec3Tuple): number {
  return Math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
}

/** Squared length. */
export function lengthSq(v: Vec3Tuple): number {
  return v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
}

/** Return the unit-length copy of `v`. Returns [0,0,0] for zero-length input. */
export function normalize(v: Vec3Tuple): Vec3Tuple {
  const len = Math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
  if (len === 0) return [0, 0, 0];
  return [v[0] / len, v[1] / len, v[2] / len];
}

/** Component-wise subtraction: a − b. */
export function sub(a: Vec3Tuple, b: Vec3Tuple): Vec3Tuple {
  return [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
}

/** Component-wise addition: a + b. */
export function add(a: Vec3Tuple, b: Vec3Tuple): Vec3Tuple {
  return [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
}

/** Scalar multiplication. */
export function scale(v: Vec3Tuple, s: number): Vec3Tuple {
  return [v[0] * s, v[1] * s, v[2] * s];
}

/** Negate a vector. */
export function negate(v: Vec3Tuple): Vec3Tuple {
  return [-v[0], -v[1], -v[2]];
}

/** Component-wise multiplication (Hadamard product). */
export function multiply(a: Vec3Tuple, b: Vec3Tuple): Vec3Tuple {
  return [a[0] * b[0], a[1] * b[1], a[2] * b[2]];
}

// ---------------------------------------------------------------------------
// Interpolation / reflection
// ---------------------------------------------------------------------------

/** Linear interpolation between `a` and `b` by factor `t` ∈ [0, 1]. */
export function lerp(a: Vec3Tuple, b: Vec3Tuple, t: number): Vec3Tuple {
  return [
    a[0] + (b[0] - a[0]) * t,
    a[1] + (b[1] - a[1]) * t,
    a[2] + (b[2] - a[2]) * t,
  ];
}

/**
 * Reflect vector `v` about `normal`.
 * Assumes `normal` is unit-length.
 */
export function reflect(v: Vec3Tuple, normal: Vec3Tuple): Vec3Tuple {
  const d = dot(v, normal);
  return [
    v[0] - 2 * d * normal[0],
    v[1] - 2 * d * normal[1],
    v[2] - 2 * d * normal[2],
  ];
}

// ---------------------------------------------------------------------------
// Products
// ---------------------------------------------------------------------------

/** Dot product. */
export function dot(a: Vec3Tuple, b: Vec3Tuple): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

/** Cross product. */
export function cross(a: Vec3Tuple, b: Vec3Tuple): Vec3Tuple {
  return [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
}

/** Angle in radians between two vectors. Returns 0 if either is zero-length. */
export function angle(a: Vec3Tuple, b: Vec3Tuple): number {
  const lenA = Math.sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]);
  const lenB = Math.sqrt(b[0] * b[0] + b[1] * b[1] + b[2] * b[2]);
  if (lenA === 0 || lenB === 0) return 0;
  const cosAngle = Math.max(-1, Math.min(1, dot(a, b) / (lenA * lenB)));
  return Math.acos(cosAngle);
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/** Check strict equality of two vectors. */
export function equals(a: Vec3Tuple, b: Vec3Tuple): boolean {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2];
}

/** Check approximate equality within an epsilon. */
export function approxEquals(a: Vec3Tuple, b: Vec3Tuple, eps = 1e-6): boolean {
  return (
    Math.abs(a[0] - b[0]) < eps &&
    Math.abs(a[1] - b[1]) < eps &&
    Math.abs(a[2] - b[2]) < eps
  );
}

/**
 * Clamp each component of `v` to the range [min, max].
 */
export function clamp(v: Vec3Tuple, min: number, max: number): Vec3Tuple {
  return [
    Math.max(min, Math.min(max, v[0])),
    Math.max(min, Math.min(max, v[1])),
    Math.max(min, Math.min(max, v[2])),
  ];
}

/**
 * Project vector `a` onto vector `b`.
 */
export function project(a: Vec3Tuple, b: Vec3Tuple): Vec3Tuple {
  const lenSqB = lengthSq(b);
  if (lenSqB === 0) return [0, 0, 0];
  const scalar = dot(a, b) / lenSqB;
  return scale(b, scalar);
}

// ─────────────────────────────────────────────────────────────────────────────
// Backwards-compatible aliases (old naming convention)
// ─────────────────────────────────────────────────────────────────────────────
/** @deprecated Use {@link distance} */
export const vec3Distance = distance;
/** @deprecated Use {@link normalize} */
export const vec3Normalize = normalize;
/** @deprecated Use {@link sub} */
export const vec3Sub = sub;
/** @deprecated Use {@link add} */
export const vec3Add = add;
/** @deprecated Use {@link scale} */
export const vec3Scale = scale;
/** @deprecated Use {@link lerp} */
export const vec3Lerp = lerp;
/** @deprecated Use {@link reflect} */
export const vec3Reflect = reflect;
/** @deprecated Use {@link dot} */
export const vec3Dot = dot;
/** @deprecated Use {@link cross} */
export const vec3Cross = cross;
/** @deprecated Use {@link angle} */
export const vec3Angle = angle;
/** @deprecated Use {@link length} */
export const vec3Length = length;

/**
 * Returns a string representation for debugging.
 */
export function toString(v: Vec3Tuple, precision = 4): string {
  return `(${v[0].toFixed(precision)}, ${v[1].toFixed(precision)}, ${v[2].toFixed(precision)})`;
}
