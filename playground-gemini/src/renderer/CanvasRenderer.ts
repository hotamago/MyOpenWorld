// Master 2D Canvas Renderer for My Open World

import {
  BiomeType,
  DayPhase,
  EntityState,
  OverlayType,
  Position,
  SpeciesType,
  TileCoord,
  TileData,
  WeatherType,
} from '../core/Types';
import { WorldMap } from '../world/WorldMap';
import { Entity } from '../entities/Entity';
import { Camera } from './Camera';
import { LightingSystem, PointLight } from './Lighting';
import { ParticleSystem } from './ParticleSystem';
import { WeatherRenderer } from './WeatherRenderer';
import { OverlayCalculator } from '../world/Overlays';
import { AccessibilityManager } from '../accessibility/AccessibilityManager';

export class CanvasRenderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;

  public camera: Camera;
  public lighting: LightingSystem;
  public particles: ParticleSystem;
  public weatherRenderer: WeatherRenderer;
  private overlayCalc: OverlayCalculator;
  private accessibilityManager: AccessibilityManager;

  public activeOverlay: OverlayType = OverlayType.NONE;
  public selectedEntity: Entity | null = null;
  public hoveredTile: TileCoord | null = null;

  private animTimer: number = 0;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d', { alpha: false })!;

    this.camera = new Camera(canvas.width, canvas.height);
    this.lighting = new LightingSystem();
    this.particles = new ParticleSystem();
    this.weatherRenderer = new WeatherRenderer();
    this.overlayCalc = new OverlayCalculator();
    this.accessibilityManager = AccessibilityManager.getInstance();

    this.handleResize();
  }

  public handleResize(): void {
    const dpr = window.devicePixelRatio || 1;
    const rect = this.canvas.getBoundingClientRect();
    const width = Math.floor(rect.width);
    const height = Math.floor(rect.height);

    if (width > 0 && height > 0) {
      this.canvas.width = width * dpr;
      this.canvas.height = height * dpr;
      this.ctx.resetTransform?.();
      this.ctx.scale(dpr, dpr);
      this.camera.resize(width, height);
      this.lighting.resize(width, height);
    }
  }

  public render(
    worldMap: WorldMap,
    entities: Entity[],
    deltaTimeSec: number,
    dayFraction: number,
    weather: WeatherType
  ): void {
    this.animTimer += deltaTimeSec;
    const ctx = this.ctx;
    const cam = this.camera;
    const tileSize = worldMap.tileSize;

    // Update systems
    cam.update(deltaTimeSec, tileSize);
    this.particles.update(deltaTimeSec);
    this.weatherRenderer.update(deltaTimeSec, weather);

    // Clear Screen
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, cam.viewportWidth, cam.viewportHeight);

    const bounds = cam.getVisibleTileBounds(tileSize);
    const pointLights: PointLight[] = [];

    // 1. Render Visible Tiles
    ctx.save();
    for (let ty = Math.max(0, bounds.minTy); ty <= Math.min(worldMap.height - 1, bounds.maxTy); ty++) {
      for (let tx = Math.max(0, bounds.minTx); tx <= Math.min(worldMap.width - 1, bounds.maxTx); tx++) {
        const tile = worldMap.getTile(tx, ty);
        if (!tile) continue;

        const screenPos = cam.worldToScreen(tx * tileSize, ty * tileSize);
        const scaledTileSize = tileSize * cam.zoom;

        // Base Terrain Tile
        this.renderTile(ctx, tile, screenPos.x, screenPos.y, scaledTileSize, cam.zoom);

        // Overlay Heatmap if active
        if (this.activeOverlay !== OverlayType.NONE) {
          const density = this.activeOverlay === OverlayType.POPULATION_DENSITY ? worldMap.getDensityAt(tx, ty) : 0;
          const overlayColor = this.overlayCalc.getTileOverlayColor(tile, this.activeOverlay, density);
          if (overlayColor) {
            ctx.fillStyle = overlayColor;
            ctx.globalAlpha = 0.72;
            ctx.fillRect(screenPos.x, screenPos.y, scaledTileSize + 0.5, scaledTileSize + 0.5);
            ctx.globalAlpha = 1.0;
          }
        }

        // Collect point lights from campfires, lanterns, shrines
        if (tile.decoration === 'campfire') {
          pointLights.push({
            x: tx * tileSize + tileSize / 2,
            y: ty * tileSize + tileSize / 2,
            radius: 80,
            color: '#ffa500',
            intensity: 0.9,
            flickerSpeed: 12,
          });
          // Emit subtle smoke occasionally
          if (Math.random() < 0.08) {
            this.particles.emitCampfireSmoke(tx * tileSize + tileSize / 2, ty * tileSize + tileSize / 2);
          }
        } else if (tile.decoration === 'mana_shrine') {
          pointLights.push({
            x: tx * tileSize + tileSize / 2,
            y: ty * tileSize + tileSize / 2,
            radius: 95,
            color: '#00f5ff',
            intensity: 0.95,
            flickerSpeed: 6,
          });
          if (Math.random() < 0.12) {
            this.particles.emitManaSparkle(tx * tileSize + tileSize / 2, ty * tileSize + tileSize / 2);
          }
        }
      }
    }
    ctx.restore();

    // 2. Render Path Preview for Selected Entity
    if (this.selectedEntity && this.selectedEntity.vitality.needs.health > 0) {
      this.renderEntityPath(ctx, this.selectedEntity, tileSize);
    }

    // 3. Render Entities (sorted by Y position for depth)
    const sortedEntities = [...entities].sort((a, b) => a.position.y - b.position.y);

    for (const entity of sortedEntities) {
      if (entity.vitality.needs.health <= 0) continue;

      const entityWorldX = entity.position.x * tileSize;
      const entityWorldY = entity.position.y * tileSize;

      // Add lights for wisps and blessed entities
      if (entity.species === SpeciesType.WISP) {
        pointLights.push({
          x: entityWorldX,
          y: entityWorldY,
          radius: 50,
          color: '#38bdf8',
          intensity: 0.85,
          flickerSpeed: 8,
        });
      } else if (entity.isBlessed) {
        pointLights.push({
          x: entityWorldX,
          y: entityWorldY,
          radius: 65,
          color: '#facc15',
          intensity: 0.9,
          flickerSpeed: 10,
        });
      }

      this.renderEntity(ctx, entity, entityWorldX, entityWorldY, cam.zoom);
    }

    // 4. Render Particle System (in-world effects)
    this.particles.render(ctx, cam);

    // 5. Ambient 2D Day/Night Lighting Pass
    this.lighting.renderLighting(ctx, cam, dayFraction, pointLights, this.animTimer);

    // 6. Dynamic Weather Overlay Pass
    this.weatherRenderer.render(ctx, cam, weather);

    // 7. Hovered Tile Cursor Outline
    if (this.hoveredTile && worldMap.isInBounds(this.hoveredTile.tx, this.hoveredTile.ty)) {
      const hScreen = cam.worldToScreen(this.hoveredTile.tx * tileSize, this.hoveredTile.ty * tileSize);
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.7)';
      ctx.lineWidth = 1.5;
      ctx.strokeRect(hScreen.x, hScreen.y, tileSize * cam.zoom, tileSize * cam.zoom);
    }
  }

  private renderTile(
    ctx: CanvasRenderingContext2D,
    tile: TileData,
    sx: number,
    sy: number,
    size: number,
    zoom: number
  ): void {
    // Fill base biome color
    ctx.fillStyle = tile.baseColor;
    ctx.fillRect(sx, sy, size + 0.5, size + 0.5);

    // Animated water ripples
    if (tile.biome === BiomeType.COASTAL_WATER || tile.biome === BiomeType.DEEP_OCEAN) {
      const wave = Math.sin(this.animTimer * 2.5 + tile.x * 0.5 + tile.y * 0.7) * 0.5 + 0.5;
      ctx.fillStyle = `rgba(255, 255, 255, ${0.08 + wave * 0.12})`;
      ctx.fillRect(sx, sy + size * 0.3 * wave, size, size * 0.2);
      return;
    }

    // Decorations & Structures
    if (tile.decoration) {
      this.renderDecoration(ctx, tile, sx, sy, size, zoom);
    }
  }

  private renderDecoration(
    ctx: CanvasRenderingContext2D,
    tile: TileData,
    sx: number,
    sy: number,
    size: number,
    zoom: number
  ): void {
    const cx = sx + size / 2;
    const cy = sy + size / 2;

    switch (tile.decoration) {
      case 'tree_oak':
      case 'tree_pine': {
        // Tree trunk & foliage
        ctx.fillStyle = '#78350f';
        ctx.fillRect(cx - 2 * zoom, cy, 4 * zoom, size * 0.4);

        ctx.fillStyle = tile.decoration === 'tree_pine' ? '#14532d' : '#15803d';
        ctx.beginPath();
        ctx.arc(cx, cy - 4 * zoom, size * 0.38, 0, Math.PI * 2);
        ctx.fill();
        break;
      }

      case 'berry_bush': {
        // Bush
        ctx.fillStyle = '#166534';
        ctx.beginPath();
        ctx.arc(cx, cy + 2 * zoom, size * 0.3, 0, Math.PI * 2);
        ctx.fill();

        // Red Berries if ripe
        if (tile.resource && tile.resource.amount > 0) {
          ctx.fillStyle = '#ef4444';
          ctx.beginPath();
          ctx.arc(cx - 3 * zoom, cy, 2 * zoom, 0, Math.PI * 2);
          ctx.arc(cx + 3 * zoom, cy + 1 * zoom, 2 * zoom, 0, Math.PI * 2);
          ctx.arc(cx, cy - 3 * zoom, 2 * zoom, 0, Math.PI * 2);
          ctx.fill();
        }
        break;
      }

      case 'wheat_stalk': {
        // Wheat stalks with wind sway
        const sway = Math.sin(this.animTimer * 3 + tile.x) * 2 * zoom;
        const stage = tile.resource?.growthStage ?? 2;
        ctx.fillStyle = stage === 2 ? '#f59e0b' : stage === 1 ? '#84cc16' : '#65a30d';

        ctx.lineWidth = 1.5 * zoom;
        ctx.strokeStyle = ctx.fillStyle;
        ctx.beginPath();
        ctx.moveTo(cx - 4 * zoom, sy + size);
        ctx.lineTo(cx - 4 * zoom + sway, sy + size * 0.3);
        ctx.moveTo(cx, sy + size);
        ctx.lineTo(cx + sway, sy + size * 0.2);
        ctx.moveTo(cx + 4 * zoom, sy + size);
        ctx.lineTo(cx + 4 * zoom + sway, sy + size * 0.3);
        ctx.stroke();
        break;
      }

      case 'campfire': {
        // Campfire logs
        ctx.fillStyle = '#451a03';
        ctx.fillRect(cx - 6 * zoom, cy + 2 * zoom, 12 * zoom, 3 * zoom);

        // Animated flame
        const flameHeight = (6 + Math.sin(this.animTimer * 14) * 2.5) * zoom;
        ctx.fillStyle = '#f97316';
        ctx.beginPath();
        ctx.arc(cx, cy - flameHeight / 2, 4 * zoom, 0, Math.PI * 2);
        ctx.fill();

        ctx.fillStyle = '#fde047';
        ctx.beginPath();
        ctx.arc(cx, cy - flameHeight / 3, 2.2 * zoom, 0, Math.PI * 2);
        ctx.fill();
        break;
      }

      case 'water_well': {
        ctx.fillStyle = '#64748b';
        ctx.fillRect(cx - 7 * zoom, cy - 7 * zoom, 14 * zoom, 14 * zoom);
        ctx.fillStyle = '#0284c7';
        ctx.beginPath();
        ctx.arc(cx, cy, 4 * zoom, 0, Math.PI * 2);
        ctx.fill();
        break;
      }

      case 'mana_shrine': {
        // Glowing Obelisk / Monolith
        ctx.fillStyle = '#475569';
        ctx.fillRect(cx - 6 * zoom, cy - 10 * zoom, 12 * zoom, 20 * zoom);

        // Floating pulsing crystal
        const floatY = Math.sin(this.animTimer * 4) * 3 * zoom;
        ctx.fillStyle = '#00f5ff';
        ctx.shadowColor = '#00f5ff';
        ctx.shadowBlur = 8 * zoom;
        ctx.beginPath();
        ctx.arc(cx, cy - 6 * zoom + floatY, 4 * zoom, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;
        break;
      }

      case 'cottage_roof': {
        ctx.fillStyle = '#991b1b'; // Red roof tiles
        ctx.fillRect(sx, sy, size, size);
        ctx.fillStyle = '#7f1d1d';
        ctx.fillRect(sx + 2 * zoom, sy + 2 * zoom, size - 4 * zoom, size - 4 * zoom);
        break;
      }

      case 'cottage_door': {
        ctx.fillStyle = '#fde047'; // Lit interior doorway
        ctx.fillRect(cx - 4 * zoom, cy - 6 * zoom, 8 * zoom, 12 * zoom);
        break;
      }

      case 'ore_vein': {
        ctx.fillStyle = '#e2e8f0';
        ctx.beginPath();
        ctx.arc(cx - 3 * zoom, cy - 2 * zoom, 3 * zoom, 0, Math.PI * 2);
        ctx.arc(cx + 3 * zoom, cy + 2 * zoom, 3.5 * zoom, 0, Math.PI * 2);
        ctx.fill();
        break;
      }

      default: {
        if (tile.decoration && tile.decoration.startsWith('flower_')) {
          const color = tile.decoration.split('_')[1];
          ctx.fillStyle = color === 'red' ? '#ef4444' : color === 'yellow' ? '#eab308' : '#3b82f6';
          ctx.beginPath();
          ctx.arc(cx, cy, 2.5 * zoom, 0, Math.PI * 2);
          ctx.fill();
        }
      }
    }
  }

  private renderEntity(
    ctx: CanvasRenderingContext2D,
    entity: Entity,
    worldX: number,
    worldY: number,
    zoom: number
  ): void {
    const screenPos = this.camera.worldToScreen(worldX, worldY);
    const sx = screenPos.x;
    const sy = screenPos.y;

    // Walking bounce offset
    const bounce = Math.abs(Math.sin(entity.walkAnimCycle)) * 3 * zoom;
    const ey = sy - bounce;

    const isSelected = this.selectedEntity?.id === entity.id;

    // 1. Selection Ring
    if (isSelected) {
      ctx.save();
      ctx.strokeStyle = '#38bdf8';
      ctx.lineWidth = 2.0;
      ctx.setLineDash([4, 4]);
      ctx.lineDashOffset = -this.animTimer * 15;
      ctx.beginPath();
      ctx.ellipse(sx, sy + 6 * zoom, 14 * zoom, 7 * zoom, 0, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }

    // 2. Render Species Body
    ctx.save();

    if (entity.species === SpeciesType.WISP) {
      // Glowing floating wisp
      const wispFloat = Math.sin(this.animTimer * 5 + entity.position.x) * 4 * zoom;
      ctx.fillStyle = '#38bdf8';
      ctx.shadowColor = '#00f5ff';
      ctx.shadowBlur = 10 * zoom;
      ctx.beginPath();
      ctx.arc(sx, ey - 8 * zoom + wispFloat, 6 * zoom, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    } else if (entity.species === SpeciesType.DEER) {
      // Deer Body
      ctx.fillStyle = entity.config.spriteColor;
      ctx.beginPath();
      ctx.ellipse(sx, ey - 6 * zoom, 9 * zoom, 6 * zoom, 0, 0, Math.PI * 2);
      ctx.fill();

      // Head & Antlers
      const headX = entity.facingDirection === 'right' ? sx + 7 * zoom : sx - 7 * zoom;
      ctx.beginPath();
      ctx.arc(headX, ey - 12 * zoom, 4 * zoom, 0, Math.PI * 2);
      ctx.fill();

      // Antlers
      ctx.strokeStyle = '#92400e';
      ctx.lineWidth = 1.5 * zoom;
      ctx.beginPath();
      ctx.moveTo(headX, ey - 15 * zoom);
      ctx.lineTo(headX + (entity.facingDirection === 'right' ? 3 : -3) * zoom, ey - 20 * zoom);
      ctx.stroke();
    } else if (entity.species === SpeciesType.WOLF) {
      // Wolf Body
      ctx.fillStyle = '#4a5568';
      ctx.beginPath();
      ctx.ellipse(sx, ey - 5 * zoom, 9 * zoom, 5 * zoom, 0, 0, Math.PI * 2);
      ctx.fill();

      const headX = entity.facingDirection === 'right' ? sx + 7 * zoom : sx - 7 * zoom;
      ctx.beginPath();
      ctx.arc(headX, ey - 9 * zoom, 4.5 * zoom, 0, Math.PI * 2);
      ctx.fill();
    } else {
      // Humanoid (Human, Elf, Dwarf)
      // Body / Tunic
      ctx.fillStyle = entity.outfitColor;
      ctx.beginPath();
      ctx.roundRect(sx - 5 * zoom, ey - 10 * zoom, 10 * zoom, 10 * zoom, 3 * zoom);
      ctx.fill();

      // Head / Skin
      ctx.fillStyle = entity.skinColor;
      ctx.beginPath();
      ctx.arc(sx, ey - 14 * zoom, 4.5 * zoom, 0, Math.PI * 2);
      ctx.fill();

      // Hair
      ctx.fillStyle = entity.hairColor;
      ctx.beginPath();
      ctx.arc(sx, ey - 16 * zoom, 4.5 * zoom, Math.PI, Math.PI * 2);
      ctx.fill();

      // Elven long ears
      if (entity.species === SpeciesType.ELF) {
        ctx.fillStyle = entity.skinColor;
        ctx.beginPath();
        ctx.moveTo(sx - 4 * zoom, ey - 14 * zoom);
        ctx.lineTo(sx - 7 * zoom, ey - 17 * zoom);
        ctx.lineTo(sx - 3 * zoom, ey - 13 * zoom);
        ctx.moveTo(sx + 4 * zoom, ey - 14 * zoom);
        ctx.lineTo(sx + 7 * zoom, ey - 17 * zoom);
        ctx.lineTo(sx + 3 * zoom, ey - 13 * zoom);
        ctx.fill();
      }

      // Dwarven beard
      if (entity.species === SpeciesType.DWARF) {
        ctx.fillStyle = entity.hairColor;
        ctx.beginPath();
        ctx.roundRect(sx - 3.5 * zoom, ey - 12 * zoom, 7 * zoom, 5 * zoom, 2 * zoom);
        ctx.fill();
      }
    }

    ctx.restore();

    // 3. Overhead Mini Status Gauges
    const barW = 20 * zoom;
    const barH = 3 * zoom;
    const barY = ey - 22 * zoom;

    // Mini Health Bar (if damaged)
    if (entity.vitality.needs.health < 95) {
      ctx.fillStyle = 'rgba(0, 0, 0, 0.6)';
      ctx.fillRect(sx - barW / 2, barY, barW, barH);
      ctx.fillStyle = '#22c55e';
      ctx.fillRect(sx - barW / 2, barY, barW * (entity.vitality.needs.health / 100), barH);
    }

    // Mini State Icon
    if (entity.state === EntityState.SLEEPING) {
      if (Math.random() < 0.05) {
        this.particles.emitSleepZzz(worldX, worldY);
      }
    } else if (entity.state === EntityState.EATING) {
      ctx.font = `${Math.round(11 * zoom)}px sans-serif`;
      ctx.fillText('🍴', sx - 5 * zoom, barY - 2 * zoom);
    }

    // 4. Floating Speech Bubble
    if (entity.currentSpeech) {
      this.renderSpeechBubble(ctx, sx, ey - 28 * zoom, entity.currentSpeech, zoom);
    }
  }

  private renderSpeechBubble(
    ctx: CanvasRenderingContext2D,
    x: number,
    y: number,
    text: string,
    zoom: number
  ): void {
    ctx.save();
    ctx.font = `${Math.max(10, Math.round(11 * zoom))}px sans-serif`;
    const metrics = ctx.measureText(text);
    const padding = 6 * zoom;
    const bw = metrics.width + padding * 2;
    const bh = 18 * zoom;

    const bx = x - bw / 2;
    const by = y - bh;

    // Bubble Background
    ctx.fillStyle = 'rgba(15, 23, 42, 0.92)';
    ctx.strokeStyle = '#38bdf8';
    ctx.lineWidth = 1.2;
    ctx.beginPath();
    ctx.roundRect(bx, by, bw, bh, 6 * zoom);
    ctx.fill();
    ctx.stroke();

    // Little triangle tail
    ctx.beginPath();
    ctx.moveTo(x - 4 * zoom, by + bh);
    ctx.lineTo(x, by + bh + 4 * zoom);
    ctx.lineTo(x + 4 * zoom, by + bh);
    ctx.fill();

    // Text
    ctx.fillStyle = '#f8fafc';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(text, x, by + bh / 2);

    ctx.restore();
  }

  private renderEntityPath(ctx: CanvasRenderingContext2D, entity: Entity, tileSize: number): void {
    const path = entity.ai.getPath();
    if (path.length === 0) return;

    ctx.save();
    ctx.strokeStyle = 'rgba(56, 189, 248, 0.6)';
    ctx.lineWidth = 2.0;
    ctx.setLineDash([4, 4]);

    ctx.beginPath();
    const startScreen = this.camera.worldToScreen(entity.position.x * tileSize, entity.position.y * tileSize);
    ctx.moveTo(startScreen.x, startScreen.y);

    for (const step of path) {
      const stepScreen = this.camera.worldToScreen(
        step.tx * tileSize + tileSize / 2,
        step.ty * tileSize + tileSize / 2
      );
      ctx.lineTo(stepScreen.x, stepScreen.y);
    }
    ctx.stroke();

    // Destination Pin
    const end = path[path.length - 1];
    const endScreen = this.camera.worldToScreen(
      end.tx * tileSize + tileSize / 2,
      end.ty * tileSize + tileSize / 2
    );
    ctx.fillStyle = '#38bdf8';
    ctx.beginPath();
    ctx.arc(endScreen.x, endScreen.y, 4 * this.camera.zoom, 0, Math.PI * 2);
    ctx.fill();

    ctx.restore();
  }
}
