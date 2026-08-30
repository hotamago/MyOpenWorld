// Seedable 2D Simplex and Perlin Noise Generator

export class SimplexNoise {
  private p: Uint8Array;
  private perm: Uint8Array;
  private permMod12: Uint8Array;

  // Skewing and unskewing factors for 2D
  private static readonly F2 = 0.5 * (Math.sqrt(3.0) - 1.0);
  private static readonly G2 = (3.0 - Math.sqrt(3.0)) / 6.0;

  // 2D Simplex gradients
  private static readonly grad3 = new Float32Array([
    1, 1, 0,
    -1, 1, 0,
    1, -1, 0,
    -1, -1, 0,
    1, 0, 1,
    -1, 0, 1,
    1, 0, -1,
    -1, 0, -1,
    0, 1, 1,
    0, -1, 1,
    0, 1, -1,
    0, -1, -1,
  ]);

  constructor(seed: number = 42) {
    this.p = new Uint8Array(256);
    this.perm = new Uint8Array(512);
    this.permMod12 = new Uint8Array(512);

    // Mulberry32 PRNG
    let s = seed | 0;
    const rng = () => {
      s = (s + 0x6d2b79f5) | 0;
      let t = Math.imul(s ^ (s >>> 15), 1 | s);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };

    for (let i = 0; i < 256; i++) {
      this.p[i] = i;
    }

    // Shuffle
    for (let i = 255; i > 0; i--) {
      const r = Math.floor(rng() * (i + 1));
      const temp = this.p[i];
      this.p[i] = this.p[r];
      this.p[r] = temp;
    }

    for (let i = 0; i < 512; i++) {
      this.perm[i] = this.p[i & 255];
      this.permMod12[i] = this.perm[i] % 12;
    }
  }

  public noise2D(xin: number, yin: number): number {
    let n0 = 0;
    let n1 = 0;
    let n2 = 0;

    // Skew input space to determine simplex cell
    const s = (xin + yin) * SimplexNoise.F2;
    const i = Math.floor(xin + s);
    const j = Math.floor(yin + s);
    const t = (i + j) * SimplexNoise.G2;

    // Unskew cell origin back to (x, y) space
    const X0 = i - t;
    const Y0 = j - t;
    const x0 = xin - X0; // Distance from cell origin
    const y0 = yin - Y0;

    // Determine which simplex triangle we are in
    let i1 = 0;
    let j1 = 0;
    if (x0 > y0) {
      i1 = 1;
      j1 = 0;
    } else {
      i1 = 0;
      j1 = 1;
    }

    // Offsets for middle and last corners
    const x1 = x0 - i1 + SimplexNoise.G2;
    const y1 = y0 - j1 + SimplexNoise.G2;
    const x2 = x0 - 1.0 + 2.0 * SimplexNoise.G2;
    const y2 = y0 - 1.0 + 2.0 * SimplexNoise.G2;

    // Work out the hashed gradient indices of the three simplex corners
    const ii = i & 255;
    const jj = j & 255;
    const gi0 = this.permMod12[ii + this.perm[jj]];
    const gi1 = this.permMod12[ii + i1 + this.perm[jj + j1]];
    const gi2 = this.permMod12[ii + 1 + this.perm[jj + 1]];

    // Calculate contribution from three corners
    let t0 = 0.5 - x0 * x0 - y0 * y0;
    if (t0 >= 0) {
      t0 *= t0;
      const gIndex = gi0 * 3;
      n0 = t0 * t0 * (SimplexNoise.grad3[gIndex] * x0 + SimplexNoise.grad3[gIndex + 1] * y0);
    }

    let t1 = 0.5 - x1 * x1 - y1 * y1;
    if (t1 >= 0) {
      t1 *= t1;
      const gIndex = gi1 * 3;
      n1 = t1 * t1 * (SimplexNoise.grad3[gIndex] * x1 + SimplexNoise.grad3[gIndex + 1] * y1);
    }

    let t2 = 0.5 - x2 * x2 - y2 * y2;
    if (t2 >= 0) {
      t2 *= t2;
      const gIndex = gi2 * 3;
      n2 = t2 * t2 * (SimplexNoise.grad3[gIndex] * x2 + SimplexNoise.grad3[gIndex + 1] * y2);
    }

    // Result is scaled to return values in interval [-1, 1]
    return 70.0 * (n0 + n1 + n2);
  }

  // Fractal Brownian Motion (fBm)
  public fbm(
    x: number,
    y: number,
    octaves: number = 4,
    lacunarity: number = 2.0,
    gain: number = 0.5
  ): number {
    let total = 0;
    let frequency = 1.0;
    let amplitude = 1.0;
    let maxValue = 0;

    for (let i = 0; i < octaves; i++) {
      total += this.noise2D(x * frequency, y * frequency) * amplitude;
      maxValue += amplitude;
      frequency *= lacunarity;
      amplitude *= gain;
    }

    return total / maxValue; // Normalized to [-1, 1]
  }

  // Domain Warping for organic natural landscapes
  public warpedFbm(x: number, y: number, octaves: number = 4): number {
    const qx = this.fbm(x, y, 2);
    const qy = this.fbm(x + 5.2, y + 1.3, 2);

    const rx = this.fbm(x + 4.0 * qx + 1.7, y + 4.0 * qy + 9.2, 2);
    const ry = this.fbm(x + 4.0 * qx + 8.3, y + 4.0 * qy + 2.8, 2);

    return this.fbm(x + 4.0 * rx, y + 4.0 * ry, octaves);
  }
}
