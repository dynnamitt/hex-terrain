import * as THREE from 'three';

const keyOf = (v) => `${v[0]},${v[1]},${v[2]}`;

export function weldedMeshFromQuads(quads, tris = []) {
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

export function hexFaceQuads(hexes) {
  const quads = [];
  for (const h of hexes) {
    const [cx, cz] = h.center;
    const y = h.height;
    const C = [cx, y, cz];
    for (const [i, j] of [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 0]]) {
      const [ax, az] = h.corners[i];
      const [bx, bz] = h.corners[j];
      quads.push([C, [ax, y, az], [bx, y, bz], C]);
    }
  }
  return quads;
}

export function hexFaceTris(hexes) {
  const tris = [];
  for (const h of hexes) {
    const [cx, cz] = h.center;
    const y = h.height;
    const C = [cx, y, cz];
    for (let i = 0; i < 6; i++) {
      const [ax, az] = h.corners[i];
      const [bx, bz] = h.corners[(i + 1) % 6];
      tris.push([C, [ax, y, az], [bx, y, bz]]);
    }
  }
  return tris;
}

export function vertexCount(geom) {
  return geom.attributes.position.count;
}

export function triangleCount(geom) {
  return geom.index.count / 3;
}
