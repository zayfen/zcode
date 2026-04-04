import * as THREE from 'three';

/** Euclidean distance between two Vector3-like objects */
export function vec3Distance(
  a: THREE.Vector3 | [number, number, number],
  b: THREE.Vector3 | [number, number, number]
): number {
  const ax = Array.isArray(a) ? a[0] : a.x;
  const ay = Array.isArray(a) ? a[1] : a.y;
  const az = Array.isArray(a) ? a[2] : a.z;
  const bx = Array.isArray(b) ? b[0] : b.x;
  const by = Array.isArray(b) ? b[1] : b.y;
  const bz = Array.isArray(b) ? b[2] : b.z;
  return Math.sqrt((ax - bx) ** 2 + (ay - by) ** 2 + (az - bz) ** 2);
}

/** Return a normalized copy of a Vector3-like */
export function vec3Normalize(v: THREE.Vector3): THREE.Vector3 {
  return v.clone().normalize();
}

/** Reflect vector v off surface with given normal */
export function vec3Reflect(v: THREE.Vector3, normal: THREE.Vector3): THREE.Vector3 {
  return v.clone().reflect(normal);
}

/** Linear interpolation between two Vector3 values */
export function vec3Lerp(a: THREE.Vector3, b: THREE.Vector3, t: number): THREE.Vector3 {
  return new THREE.Vector3().lerpVectors(a, b, t);
}

/** Angle in radians between two vectors */
export function vec3Angle(a: THREE.Vector3, b: THREE.Vector3): number {
  return a.angleTo(b);
}

/** Get XZ distance (horizontal plane) */
export function vec3XZDistance(
  a: THREE.Vector3 | [number, number, number],
  b: THREE.Vector3 | [number, number, number]
): number {
  const ax = Array.isArray(a) ? a[0] : a.x;
  const az = Array.isArray(a) ? a[2] : a.z;
  const bx = Array.isArray(b) ? b[0] : b.x;
  const bz = Array.isArray(b) ? b[2] : b.z;
  return Math.sqrt((ax - bx) ** 2 + (az - bz) ** 2);
}
