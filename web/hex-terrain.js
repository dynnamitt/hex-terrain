import * as THREE from 'three';

const keyOf = (v) => `${v[0]},${v[1]},${v[2]}`;

/**
 * Builds a single indexed `THREE.BufferGeometry` from quad and tri faces,
 * welding (deduplicating) shared vertex positions so neighboring faces meet
 * seamlessly and `computeVertexNormals()` produces continuous shading across
 * the seam.
 *
 * Each unique `[x, y, z]` becomes one vertex in the position buffer; faces
 * become triangle indices into that buffer:
 *   - Quads `[a, b, c, d]` are triangulated as `(a, b, c)` + `(a, c, d)`
 *     (CCW fan from `a`).
 *   - Tris `[a, b, c]` are emitted as-is.
 *
 * Welding is exact: vertices are matched by stringified coordinates, so
 * inputs must already share identical floats at seams (no epsilon merge).
 *
 * @param {Array<[Vec3, Vec3, Vec3, Vec3]>} quads - CCW quads, each four
 *   `[x, y, z]` corners.
 * @param {Array<[Vec3, Vec3, Vec3]>} [tris=[]] - CCW triangles, each three
 *   `[x, y, z]` corners. Used for 3-hex junction gaps and (optionally) hex
 *   face fans.
 * @returns {THREE.BufferGeometry} Indexed geometry with `position` attribute,
 *   computed vertex normals, and computed bounding box.
 *
 * @typedef {[number, number, number]} Vec3
 */
export function weldedMesh(quads, tris = []) {
  const positions = [];
  const indices = [];
  const lookup = new Map();

  const indexOf = (v) => {
    const k = keyOf(v);
    let idx = lookup.get(k);
    if (idx === undefined) {
      idx = positions.length / 3;
      positions.push(v[0], v[1], v[2]);
      lookup.set(k, idx);
    }
    return idx;
  };

  for (const [a, b, c, d] of quads) {
    const ia = indexOf(a), ib = indexOf(b), ic = indexOf(c), id = indexOf(d);
    indices.push(ia, ib, ic, ia, ic, id);
  }
  for (const [a, b, c] of tris) {
    indices.push(indexOf(a), indexOf(b), indexOf(c));
  }

  const geom = new THREE.BufferGeometry();
  geom.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geom.setIndex(indices);
  geom.computeVertexNormals();
  geom.computeBoundingBox();
  return geom;
}

export function vertexCount(geom) {
  return geom.attributes.position.count;
}

export function triangleCount(geom) {
  return geom.index.count / 3;
}
