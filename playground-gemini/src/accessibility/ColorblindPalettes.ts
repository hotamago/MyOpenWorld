// Accessible Colorblind-Safe Color Maps and Matrix Transforms
// Based on Viridis, Cividis, and Plasma scientific palettes

export interface RGB {
  r: number;
  g: number;
  b: number;
}

export function hexToRgb(hex: string): RGB {
  const cleanHex = hex.replace('#', '');
  const bigint = parseInt(cleanHex, 16);
  return {
    r: (bigint >> 16) & 255,
    g: (bigint >> 8) & 255,
    b: bigint & 255,
  };
}

export function rgbToHex(rgb: RGB): string {
  const clamp = (v: number) => Math.max(0, Math.min(255, Math.round(v)));
  return '#' + [rgb.r, rgb.g, rgb.b].map((x) => clamp(x).toString(16).padStart(2, '0')).join('');
}

export function interpolateColor(c1: RGB, c2: RGB, factor: number): RGB {
  const f = Math.max(0, Math.min(1, factor));
  return {
    r: c1.r + (c2.r - c1.r) * f,
    g: c1.g + (c2.g - c1.g) * f,
    b: c1.b + (c2.b - c1.b) * f,
  };
}

export function sampleMultiStopGradient(stops: { pos: number; color: RGB }[], t: number): RGB {
  const clampedT = Math.max(0, Math.min(1, t));
  if (clampedT <= stops[0].pos) return stops[0].color;
  if (clampedT >= stops[stops.length - 1].pos) return stops[stops.length - 1].color;

  for (let i = 0; i < stops.length - 1; i++) {
    if (clampedT >= stops[i].pos && clampedT <= stops[i + 1].pos) {
      const localT = (clampedT - stops[i].pos) / (stops[i + 1].pos - stops[i].pos);
      return interpolateColor(stops[i].color, stops[i + 1].color, localT);
    }
  }
  return stops[stops.length - 1].color;
}

// 1. Viridis Palette (Optimized for continuous data & colorblindness)
export const VIRIDIS_STOPS: { pos: number; color: RGB }[] = [
  { pos: 0.0, color: { r: 68, g: 1, b: 84 } },     // #440154 Dark Purple
  { pos: 0.25, color: { r: 59, g: 82, b: 139 } },  // #3b528b Blue
  { pos: 0.5, color: { r: 33, g: 145, b: 140 } },  // #21918c Teal
  { pos: 0.75, color: { r: 94, g: 201, b: 98 } },  // #5ec962 Green
  { pos: 1.0, color: { r: 253, g: 231, b: 37 } },  // #fde725 Bright Yellow
];

// 2. Cividis Palette (Optimal for Deuteranopia & Protanopia)
export const CIVIDIS_STOPS: { pos: number; color: RGB }[] = [
  { pos: 0.0, color: { r: 0, g: 32, b: 77 } },     // Deep Navy
  { pos: 0.35, color: { r: 65, g: 79, b: 110 } },  // Muted Slate
  { pos: 0.65, color: { r: 124, g: 123, b: 120 } },// Neutral Silver
  { pos: 1.0, color: { r: 255, g: 234, b: 70 } },  // Solar Gold
];

// 3. Plasma Palette (Vibrant perceptually uniform heatmap)
export const PLASMA_STOPS: { pos: number; color: RGB }[] = [
  { pos: 0.0, color: { r: 13, g: 8, b: 135 } },    // Deep Indigo
  { pos: 0.3, color: { r: 126, g: 3, b: 168 } },   // Rich Violet
  { pos: 0.6, color: { r: 204, g: 71, b: 120 } },  // Magenta
  { pos: 0.85, color: { r: 248, g: 149, b: 64 } }, // Warm Amber
  { pos: 1.0, color: { r: 240, g: 249, b: 33 } },  // Electric Yellow
];

// 4. Arcane Mana Gradient (Mystic Cyan to Celestial Magenta)
export const MANA_STOPS: { pos: number; color: RGB }[] = [
  { pos: 0.0, color: { r: 10, g: 15, b: 35 } },    // Void Navy
  { pos: 0.3, color: { r: 20, g: 80, b: 150 } },   // Cobalt Blue
  { pos: 0.6, color: { r: 40, g: 200, b: 220 } },  // Radiant Cyan
  { pos: 0.85, color: { r: 160, g: 80, b: 250 } }, // Mystic Violet
  { pos: 1.0, color: { r: 255, g: 220, b: 255 } }, // Starlight White
];

// 5. Accessible Cool-to-Warm Temperature (Blue -> Cyan -> Pale Cream -> Orange -> Crimson)
export const COOLWARM_STOPS: { pos: number; color: RGB }[] = [
  { pos: 0.0, color: { r: 59, g: 76, b: 192 } },   // Polar Blue (-15°C)
  { pos: 0.25, color: { r: 142, g: 178, b: 255 } },// Crisp Light Blue (0°C)
  { pos: 0.5, color: { r: 221, g: 221, b: 221 } }, // Neutral Cream (15°C)
  { pos: 0.75, color: { r: 244, g: 154, b: 107 } },// Warm Amber (30°C)
  { pos: 1.0, color: { r: 180, g: 4, b: 38 } },    // Desert Heat (45°C)
];

// Colorblind Simulation Matrices (Brettel et al. and Vienot et al.)
export function applyColorblindSimulation(rgb: RGB, mode: string): RGB {
  if (mode === 'NORMAL') return rgb;

  const r = rgb.r / 255;
  const g = rgb.g / 255;
  const b = rgb.b / 255;

  let nr = r;
  let ng = g;
  let nb = b;

  if (mode === 'PROTANOPIA') {
    // Red-blind
    nr = 0.56667 * r + 0.43333 * g + 0.0 * b;
    ng = 0.55833 * r + 0.44167 * g + 0.0 * b;
    nb = 0.0 * r + 0.24167 * g + 0.75833 * b;
  } else if (mode === 'DEUTERANOPIA') {
    // Green-blind
    nr = 0.625 * r + 0.375 * g + 0.0 * b;
    ng = 0.7 * r + 0.3 * g + 0.0 * b;
    nb = 0.0 * r + 0.3 * g + 0.7 * b;
  } else if (mode === 'TRITANOPIA') {
    // Blue-blind
    nr = 0.95 * r + 0.05 * g + 0.0 * b;
    ng = 0.0 * r + 0.43333 * g + 0.56667 * b;
    nb = 0.0 * r + 0.475 * g + 0.525 * b;
  } else if (mode === 'HIGH_CONTRAST') {
    // High contrast monochrome / luminance boost
    const lum = 0.299 * r + 0.587 * g + 0.114 * b;
    const boost = lum > 0.5 ? Math.min(1.0, lum * 1.2) : lum * 0.8;
    nr = boost;
    ng = boost;
    nb = boost;
  }

  return {
    r: Math.round(nr * 255),
    g: Math.round(ng * 255),
    b: Math.round(nb * 255),
  };
}
