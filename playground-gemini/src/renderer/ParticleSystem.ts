// Particle System for Atmospheric and Entity Effects

import { Camera } from './Camera';

export interface Particle {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  color: string;
  alpha: number;
  life: number;
  maxLife: number;
  type: 'text' | 'circle' | 'sparkle' | 'smoke';
  text?: string;
}

export class ParticleSystem {
  private particles: Particle[] = [];

  public emitSleepZzz(worldX: number, worldY: number): void {
    this.particles.push({
      x: worldX + (Math.random() - 0.5) * 6,
      y: worldY - 12,
      vx: (Math.random() - 0.5) * 4 + 2,
      vy: -14 - Math.random() * 8,
      size: 11 + Math.random() * 4,
      color: '#cbd5e0',
      alpha: 1.0,
      life: 2.0,
      maxLife: 2.0,
      type: 'text',
      text: 'Z',
    });
  }

  public emitHeart(worldX: number, worldY: number): void {
    this.particles.push({
      x: worldX + (Math.random() - 0.5) * 8,
      y: worldY - 16,
      vx: (Math.random() - 0.5) * 5,
      vy: -16 - Math.random() * 8,
      size: 14,
      color: '#f56565',
      alpha: 1.0,
      life: 1.8,
      maxLife: 1.8,
      type: 'text',
      text: '❤️',
    });
  }

  public emitManaSparkle(worldX: number, worldY: number): void {
    this.particles.push({
      x: worldX + (Math.random() - 0.5) * 16,
      y: worldY + (Math.random() - 0.5) * 16,
      vx: (Math.random() - 0.5) * 12,
      vy: -10 - Math.random() * 12,
      size: 2 + Math.random() * 3,
      color: Math.random() < 0.5 ? '#63b3ed' : '#b794f4',
      alpha: 0.9,
      life: 1.2,
      maxLife: 1.2,
      type: 'sparkle',
    });
  }

  public emitCampfireSmoke(worldX: number, worldY: number): void {
    this.particles.push({
      x: worldX + (Math.random() - 0.5) * 4,
      y: worldY - 8,
      vx: (Math.random() - 0.5) * 6 + 1,
      vy: -18 - Math.random() * 10,
      size: 4 + Math.random() * 6,
      color: '#718096',
      alpha: 0.6,
      life: 2.2,
      maxLife: 2.2,
      type: 'smoke',
    });
  }

  public emitDivineBlessingRays(worldX: number, worldY: number): void {
    for (let i = 0; i < 16; i++) {
      const angle = (i / 16) * Math.PI * 2;
      const speed = 25 + Math.random() * 20;
      this.particles.push({
        x: worldX,
        y: worldY - 10,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        size: 3 + Math.random() * 4,
        color: '#f6e05e',
        alpha: 1.0,
        life: 1.4,
        maxLife: 1.4,
        type: 'sparkle',
      });
    }
  }

  public update(deltaTimeSec: number): void {
    for (let i = this.particles.length - 1; i >= 0; i--) {
      const p = this.particles[i];
      p.life -= deltaTimeSec;
      if (p.life <= 0) {
        this.particles.splice(i, 1);
        continue;
      }

      p.x += p.vx * deltaTimeSec;
      p.y += p.vy * deltaTimeSec;
      p.alpha = Math.max(0, p.life / p.maxLife);

      if (p.type === 'smoke') {
        p.size += deltaTimeSec * 4; // Smoke expands
      }
    }
  }

  public render(ctx: CanvasRenderingContext2D, camera: Camera): void {
    for (const p of this.particles) {
      const screenPos = camera.worldToScreen(p.x, p.y);

      // Skip if offscreen
      if (
        screenPos.x < -20 ||
        screenPos.x > camera.viewportWidth + 20 ||
        screenPos.y < -20 ||
        screenPos.y > camera.viewportHeight + 20
      ) {
        continue;
      }

      ctx.save();
      ctx.globalAlpha = p.alpha;

      if (p.type === 'text' && p.text) {
        ctx.font = `bold ${Math.round(p.size * camera.zoom)}px sans-serif`;
        ctx.fillStyle = p.color;
        ctx.textAlign = 'center';
        ctx.fillText(p.text, screenPos.x, screenPos.y);
      } else if (p.type === 'sparkle') {
        ctx.fillStyle = p.color;
        ctx.shadowColor = p.color;
        ctx.shadowBlur = 6 * camera.zoom;
        ctx.beginPath();
        ctx.arc(screenPos.x, screenPos.y, p.size * camera.zoom, 0, Math.PI * 2);
        ctx.fill();
      } else if (p.type === 'smoke') {
        ctx.fillStyle = p.color;
        ctx.beginPath();
        ctx.arc(screenPos.x, screenPos.y, p.size * camera.zoom, 0, Math.PI * 2);
        ctx.fill();
      }

      ctx.restore();
    }
  }
}
