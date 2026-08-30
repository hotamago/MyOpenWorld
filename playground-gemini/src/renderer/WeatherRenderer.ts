// Dynamic Weather and Environmental Effects Renderer

import { WeatherType } from '../core/Types';
import { Camera } from './Camera';

interface RainDrop {
  x: number;
  y: number;
  length: number;
  speed: number;
}

interface MistCloud {
  x: number;
  y: number;
  radius: number;
  speed: number;
  alpha: number;
}

export class WeatherRenderer {
  private raindrops: RainDrop[] = [];
  private mistClouds: MistCloud[] = [];
  private lightningAlpha: number = 0;

  constructor() {
    // Initialize rain drops
    for (let i = 0; i < 180; i++) {
      this.raindrops.push({
        x: Math.random() * 1200,
        y: Math.random() * 800,
        length: 12 + Math.random() * 10,
        speed: 400 + Math.random() * 200,
      });
    }

    // Initialize mist clouds
    for (let i = 0; i < 12; i++) {
      this.mistClouds.push({
        x: Math.random() * 1400,
        y: Math.random() * 900,
        radius: 80 + Math.random() * 100,
        speed: 10 + Math.random() * 15,
        alpha: 0.15 + Math.random() * 0.15,
      });
    }
  }

  public update(deltaTimeSec: number, weather: WeatherType): void {
    if (weather === WeatherType.RAIN) {
      for (const drop of this.raindrops) {
        drop.y += drop.speed * deltaTimeSec;
        drop.x -= (drop.speed * 0.25) * deltaTimeSec; // Wind slant

        if (drop.y > 900) {
          drop.y = -20;
          drop.x = Math.random() * 1400;
        }
      }
    } else if (weather === WeatherType.MIST) {
      for (const cloud of this.mistClouds) {
        cloud.x += cloud.speed * deltaTimeSec;
        if (cloud.x - cloud.radius > 1400) {
          cloud.x = -cloud.radius;
          cloud.y = Math.random() * 900;
        }
      }
    } else if (weather === WeatherType.MANA_STORM) {
      // Occasional lightning flash
      if (Math.random() < 0.015) {
        this.lightningAlpha = 0.6;
      }
      if (this.lightningAlpha > 0) {
        this.lightningAlpha = Math.max(0, this.lightningAlpha - deltaTimeSec * 2.5);
      }
    }
  }

  public render(ctx: CanvasRenderingContext2D, camera: Camera, weather: WeatherType): void {
    const w = camera.viewportWidth;
    const h = camera.viewportHeight;

    if (weather === WeatherType.RAIN) {
      ctx.save();
      ctx.strokeStyle = 'rgba(174, 214, 241, 0.6)';
      ctx.lineWidth = 1.5;

      ctx.beginPath();
      for (const drop of this.raindrops) {
        const sx = drop.x % (w + 100);
        const sy = drop.y % (h + 100);
        ctx.moveTo(sx, sy);
        ctx.lineTo(sx - 4, sy + drop.length);
      }
      ctx.stroke();
      ctx.restore();
    } else if (weather === WeatherType.MIST) {
      ctx.save();
      for (const cloud of this.mistClouds) {
        const sx = cloud.x % (w + cloud.radius * 2) - cloud.radius;
        const sy = cloud.y % (h + cloud.radius * 2) - cloud.radius;

        const grad = ctx.createRadialGradient(sx, sy, 0, sx, sy, cloud.radius);
        grad.addColorStop(0, `rgba(226, 232, 240, ${cloud.alpha})`);
        grad.addColorStop(1, 'rgba(226, 232, 240, 0)');

        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(sx, sy, cloud.radius, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.restore();
    } else if (weather === WeatherType.MANA_STORM) {
      if (this.lightningAlpha > 0.02) {
        ctx.save();
        ctx.fillStyle = `rgba(180, 130, 255, ${this.lightningAlpha})`;
        ctx.fillRect(0, 0, w, h);
        ctx.restore();
      }
    }
  }
}
