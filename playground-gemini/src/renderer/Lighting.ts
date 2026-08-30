// Dynamic 2D Ambient Day/Night Lighting and Point-Light System

import { DayPhase, Position } from '../core/Types';
import { Camera } from './Camera';

export interface PointLight {
  x: number; // World pixel coordinate
  y: number;
  radius: number;
  color: string;
  intensity: number;
  flickerSpeed?: number;
}

export class LightingSystem {
  private ambientCanvas: HTMLCanvasElement;
  private ambientCtx: CanvasRenderingContext2D;

  constructor() {
    this.ambientCanvas = document.createElement('canvas');
    this.ambientCtx = this.ambientCanvas.getContext('2d')!;
  }

  public resize(width: number, height: number): void {
    this.ambientCanvas.width = width;
    this.ambientCanvas.height = height;
  }

  public renderLighting(
    mainCtx: CanvasRenderingContext2D,
    camera: Camera,
    dayFraction: number, // 0.0 to 1.0 (0 = midnight, 0.5 = noon)
    pointLights: PointLight[],
    currentTimeSec: number
  ): void {
    const w = camera.viewportWidth;
    const h = camera.viewportHeight;

    if (this.ambientCanvas.width !== w || this.ambientCanvas.height !== h) {
      this.resize(w, h);
    }

    const ctx = this.ambientCtx;
    ctx.clearRect(0, 0, w, h);

    // 1. Compute ambient darkness color and opacity based on time of day
    const { color, alpha } = this.getAmbientLighting(dayFraction);

    if (alpha <= 0.02 && pointLights.length === 0) {
      return; // Midday, fully bright!
    }

    // Fill darkness layer
    ctx.fillStyle = color;
    ctx.globalAlpha = alpha;
    ctx.fillRect(0, 0, w, h);

    // 2. Carve out point lights with radial gradients
    ctx.globalCompositeOperation = 'destination-out';

    for (const light of pointLights) {
      const screenPos = camera.worldToScreen(light.x, light.y);
      const screenRadius = light.radius * camera.zoom;

      // Skip lights completely outside viewport
      if (
        screenPos.x + screenRadius < 0 ||
        screenPos.x - screenRadius > w ||
        screenPos.y + screenRadius < 0 ||
        screenPos.y - screenRadius > h
      ) {
        continue;
      }

      // Add gentle flicker
      const flicker = light.flickerSpeed
        ? Math.sin(currentTimeSec * light.flickerSpeed + light.x) * 0.08 + 0.95
        : 1.0;
      const finalRadius = screenRadius * flicker;

      const grad = ctx.createRadialGradient(
        screenPos.x,
        screenPos.y,
        0,
        screenPos.x,
        screenPos.y,
        finalRadius
      );

      grad.addColorStop(0, `rgba(0, 0, 0, ${light.intensity * 0.95})`);
      grad.addColorStop(0.5, `rgba(0, 0, 0, ${light.intensity * 0.55})`);
      grad.addColorStop(1, 'rgba(0, 0, 0, 0)');

      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(screenPos.x, screenPos.y, finalRadius, 0, Math.PI * 2);
      ctx.fill();
    }

    // Reset composite operation
    ctx.globalCompositeOperation = 'source-over';
    ctx.globalAlpha = 1.0;

    // Draw ambient dark layer onto main canvas
    mainCtx.drawImage(this.ambientCanvas, 0, 0);

    // 3. Add warm colored light glow overlay
    mainCtx.save();
    mainCtx.globalCompositeOperation = 'screen';

    for (const light of pointLights) {
      const screenPos = camera.worldToScreen(light.x, light.y);
      const screenRadius = light.radius * 0.7 * camera.zoom;

      if (
        screenPos.x + screenRadius < 0 ||
        screenPos.x - screenRadius > w ||
        screenPos.y + screenRadius < 0 ||
        screenPos.y - screenRadius > h
      ) {
        continue;
      }

      const grad = mainCtx.createRadialGradient(
        screenPos.x,
        screenPos.y,
        0,
        screenPos.x,
        screenPos.y,
        screenRadius
      );
      grad.addColorStop(0, light.color);
      grad.addColorStop(1, 'transparent');

      mainCtx.fillStyle = grad;
      mainCtx.beginPath();
      mainCtx.arc(screenPos.x, screenPos.y, screenRadius, 0, Math.PI * 2);
      mainCtx.fill();
    }

    mainCtx.restore();
  }

  private getAmbientLighting(dayFraction: number): { color: string; alpha: number } {
    // dayFraction: 0.0 (midnight) -> 0.25 (6am Dawn) -> 0.5 (noon) -> 0.75 (6pm Dusk) -> 1.0 (midnight)
    const hour = dayFraction * 24;

    if (hour >= 8 && hour < 17) {
      // Day (08:00 - 17:00): Full sunlight
      return { color: '#000000', alpha: 0.0 };
    } else if (hour >= 5 && hour < 8) {
      // Dawn (05:00 - 08:00): Transitions from dark blue to soft pink-gold
      const progress = (hour - 5) / 3;
      return { color: '#2a1a3a', alpha: (1 - progress) * 0.65 };
    } else if (hour >= 17 && hour < 20) {
      // Dusk (17:00 - 20:00): Transitions from golden amber to deep twilight
      const progress = (hour - 17) / 3;
      return { color: '#1a102f', alpha: progress * 0.68 };
    } else {
      // Night (20:00 - 05:00): Deep dark midnight blue
      return { color: '#0a0d20', alpha: 0.78 };
    }
  }
}
