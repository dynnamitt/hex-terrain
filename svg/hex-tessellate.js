import * as THREE from 'three';

const centroidXZ = (corners) => {
  let cx = 0, cz = 0;
  for (const [x, z] of corners) { cx += x; cz += z; }
  return [cx / corners.length, cz / corners.length];
};

const finish = (positions) => {
  const geom = new THREE.BufferGeometry();
  geom.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
  geom.computeVertexNormals();
  return geom;
};

export function fanCentroid(corners, y) {
  const [cx, cz] = centroidXZ(corners);
  const positions = [];
  for (let i = 0; i < 6; i++) {
    const [ax, az] = corners[i];
    const [bx, bz] = corners[(i + 1) % 6];
    positions.push(cx, y, cz, bx, y, bz, ax, y, az);
  }
  return finish(positions);
}

export function threeRhombi(corners, y) {
  const [cx, cz] = centroidXZ(corners);
  const positions = [];
  for (const [i, j, k] of [[0, 1, 2], [2, 3, 4], [4, 5, 0]]) {
    const [ax, az] = corners[i];
    const [bx, bz] = corners[j];
    const [dx, dz] = corners[k];
    positions.push(cx, y, cz, ax, y, az, bx, y, bz);
    positions.push(cx, y, cz, bx, y, bz, dx, y, dz);
  }
  return finish(positions);
}

export function shapeEarcut(corners, y) {
  const shape = new THREE.Shape(corners.map(([x, z]) => new THREE.Vector2(x, z)));
  const geom = new THREE.ShapeGeometry(shape);
  geom.rotateX(-Math.PI / 2);
  geom.translate(0, y, 0);
  geom.computeVertexNormals();
  return geom;
}

export function loopSubdivide(baseGeom, levels) {
  let positions = Array.from(baseGeom.attributes.position.array);
  for (let level = 0; level < levels; level++) {
    const next = [];
    for (let i = 0; i < positions.length; i += 9) {
      const ax = positions[i],     ay = positions[i + 1], az = positions[i + 2];
      const bx = positions[i + 3], by = positions[i + 4], bz = positions[i + 5];
      const cx = positions[i + 6], cy = positions[i + 7], cz = positions[i + 8];
      const mabx = (ax + bx) * 0.5, maby = (ay + by) * 0.5, mabz = (az + bz) * 0.5;
      const mbcx = (bx + cx) * 0.5, mbcy = (by + cy) * 0.5, mbcz = (bz + cz) * 0.5;
      const mcax = (cx + ax) * 0.5, mcay = (cy + ay) * 0.5, mcaz = (cz + az) * 0.5;
      next.push(
        ax, ay, az,    mabx, maby, mabz,  mcax, mcay, mcaz,
        mabx, maby, mabz,  bx, by, bz,    mbcx, mbcy, mbcz,
        mcax, mcay, mcaz,  mbcx, mbcy, mbcz,  cx, cy, cz,
        mabx, maby, mabz,  mbcx, mbcy, mbcz,  mcax, mcay, mcaz,
      );
    }
    positions = next;
  }
  return finish(positions);
}

export function triCount(geom) {
  return geom.attributes.position.count / 3;
}
