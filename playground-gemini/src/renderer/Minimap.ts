// Interactive Real-Time World Minimap

import { SpeciesType } from '../core/Types';
import { WorldMap } from '../world/WorldMap';
import { Entity } from '../entities/Entity';
import { Camera } from './Camera';
import { EventBus, Events } from '../core/EventBus';

export class Minimap {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private terrainCache: HTMLCanvasElement;
  private isInteracting: boolean = false;
  private eventBus = EventBus.getInstance();

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d')!;
    this.terrainCache = document.createElement('canvas');

    this.setupInteractions();
  }

  public bakeTerrain(worldMap: WorldMap): void {
    this.terrainCache.width = worldMap.width;
    this.terrainCache.height = worldMap.height;
    const tCtx = this.terrainCache.getContext('2d')!;

    const imgData = tCtx.createImageData(worldMap.width, worldMap.height);
    const data = imgData.data;

    for (let y = 0; y < worldMap.height; y++) {
      for (let x = 0; x < worldMap.width; x++) {
        const tile = worldMap.getTile(x, y);
        if (!tile) continue;

        const idx = (y * worldMap.width + x) * 4;
        const hex = tile.baseColor.replace('#', '');
        const bigint = parseInt(hex, 16);

        data[idx] = (bigint >> 16) & 255;
        data[idx + 1] = (bigint >> 8) & 255;
        data[idx + 2] = bigint & 255;
        data[idx + 3] = 255;
      }
    }

    tCtx.putImageData(imgData, 0, 0);
  }

  public render(worldMap: WorldMap, entities: Entity[], camera: Camera): void {
    const w = this.canvas.width;
    const h = this.canvas.height;
    const ctx = this.ctx;

    ctx.clearRect(0, 0, w, h);

    // 1. Draw baked terrain scaled up
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(this.terrainCache, 0, 0, w, h);

    const scaleX = w / worldMap.width;
    const scaleY = h / worldMap.height;

    // 2. Draw entity blips
    for (const e of entities) {
      if (e.vitality.needs.health <= 0) continue;

      const mx = e.position.x * scaleX;
      const my = e.position.y * scaleY;

      let color = '#4299e1'; // Human
      if (e.species === SpeciesType.ELF) color = '#48bb78';
      else if (e.species === SpeciesType.DWARF) color = '#ed8936';
      else if (e.species === SpeciesType.DEER) color = '#ecc94b';
      else if (e.species === SpeciesType.WOLF) color = '#e53e3e';
      else if (e.species === SpeciesType.WISP) color = '#b794f4';

      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(mx, my, 2.5, 0, Math.PI * 2);
      ctx.fill();
    }

    // 3. Draw Camera Viewport Bounding Box
    const bounds = camera.getVisibleTileBounds(worldMap.tileSize);
    const boxX = Math.max(0, bounds.minTx * scaleX);
    const boxY = Math.max(0, bounds.minTy * scaleY);
    const boxW = Math.min(w, (bounds.maxTx - bounds.minTx) * scaleX);
    const boxH = Math.min(h, (bounds.maxTy - bounds.minTy) * scaleY);

    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(boxX, boxY, boxW, boxH);

    ctx.fillStyle = 'rgba(255, 255, 255, 0.12)';
    ctx.fillRect(boxX, boxY, boxW, boxH);
  }

  private setupInteractions(): void {
    const handleMinimapClick = (e: MouseEvent) => {
      const rect = this.canvas.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const clickY = e.clientY - rect.top;

      const normX = Math.max(0, Math.min(1, clickX / rect.width));
      const normY = Math.max(0, Math.min(1, clickY / rect.height));

      this.eventBus.emit(Events.CAMERA_TELEPORT_TILE, { normX, normY });
    };

    this.canvas.addEventListener('mousedown', (e) => {
      this.isInteracting = true;
      handleMinimapClick(e);
    });

    window.addEventListener('mousemove', (e) => {
      if (this.isInteracting) {
        handleMinimapClick(e);
      }
    });

    window.addEventListener('mouseup', () => {
      this.isInteracting = false;
    });
  }
}
