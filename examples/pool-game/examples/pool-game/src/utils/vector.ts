// src/utils/vector.ts
import { Vector3 as ThreeVector3 } from 'three';

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export function vec3Distance(a: Vec3, b: Vec3): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  const dz = a.z - b.z;
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

export function vec3Length(v: Vec3): number {
  return Math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z);
}

export function vec3Normalize(v: Vec3): Vec3 {
  const len = vec3Length(v);
  if (len === 0) return { x: 0, y: 0, z: 0 };
  return { x: v.x / len, y: v.y / len, z: v.z / len };
}

export function vec3Scale(v: Vec3, s: number): Vec3 {
  return { x: v.x * s, y: v.y * s, z: v.z * s };
}

export function vec3Add(a: Vec3, b: Vec3): Vec3 {
  return { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z };
}

export function vec3Sub(a: Vec3, b: Vec3): Vec3 {
  return { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z };
}

export function vec3Dot(a: Vec3, b: Vec3): number {
  return a.x * b.x + a.y * b.y + a.z * b.z;
}

export function vec3Cross(a: Vec3, b: Vec3): Vec3 {
  return {
    x: a.y * b.z - a.z * b.y,
    y: a.z * b.x - a.x * b.z,
    z: a.x * b.y - a.y * b.x,
  };
}

export function vec3Reflect(v: Vec3, normal: Vec3): Vec3 {
  const d = vec3Dot(v, normal);
  return {
    x: v.x - 2 * d * normal.x,
    y: v.y - 2 * d * normal.y,
    z: v.z - 2 * d * normal.z,
  };
}

export function vec3Lerp(a: Vec3, b: Vec3, t: number): Vec3 {
  return {
    x: a.x + (b.x - a.x) * t,
    y: a.y + (b.y - a.y) * t,
    z: a.z + (b.z - a.z) * t,
  };
}

export function vec3Angle(a: Vec3, b: Vec3): number {
  const na = vec3Normalize(a);
  const nb = vec3Normalize(b);
  const dot = Math.max(-1, Math.min(1, vec3Dot(na, nb)));
  return Math.acos(dot);
}

export function vec3FromArray(arr: [number, number, number]): Vec3 {
  return { x: arr[0], y: arr[1], z: arr[2] };
}

export function vec3ToArray(v: Vec3): [number, number, number] {
  return [v.x, v.y, v.z];
}

export function vec3Clone(v: Vec3): Vec3 {
  return { x: v.x, y: v.y, z: v.z };
}
