// Smooth Pan/Zoom Camera and Coordinate Transforms

import { Position, TileCoord } from '../core/Types';
import { Entity } from '../entities/Entity';

export class Camera {
  public x: number = 0; // World coordinate in pixels
  public y: number = 0;
  public zoom: number = 1.0; // 0.4 to 2.5
  public minZoom: number = 0.35;
  public maxZoom: number = 2.5;

  public viewportWidth: number = 800;
  public viewportHeight: number = 600;

  public targetEntity: Entity | null = null;
  public isDragging: boolean = false;
  private dragStartX: number = 0;
  private dragStartY: number = 0;
  private camStartX: number = 0;
  private camStartY: number = 0;

  constructor(viewportWidth: number, viewportHeight: number) {
    this.viewportWidth = viewportWidth;
    this.viewportHeight = viewportHeight;
  }

  public resize(width: number, height: number): void {
    this.viewportWidth = width;
    this.viewportHeight = height;
  }

  public centerOnTile(tx: number, ty: number, tileSize: number = 32): void {
    this.x = tx * tileSize + tileSize / 2;
    this.y = ty * tileSize + tileSize / 2;
    this.targetEntity = null;
  }

  public followEntity(entity: Entity | null): void {
    this.targetEntity = entity;
  }

  public update(deltaTimeSec: number, tileSize: number = 32): void {
    if (this.targetEntity && !this.isDragging) {
      const targetWorldX = this.targetEntity.position.x * tileSize;
      const targetWorldY = this.targetEntity.position.y * tileSize;

      // Smooth camera lerp tracking
      const lerpFactor = Math.min(1.0, deltaTimeSec * 5.0);
      this.x += (targetWorldX - this.x) * lerpFactor;
      this.y += (targetWorldY - this.y) * lerpFactor;
    }
  }

  public startDrag(screenX: number, screenY: number): void {
    this.isDragging = true;
    this.dragStartX = screenX;
    this.dragStartY = screenY;
    this.camStartX = this.x;
    this.camStartY = this.y;
    this.targetEntity = null; // Break tracking on manual drag
  }

  public onDrag(screenX: number, screenY: number): void {
    if (!this.isDragging) return;
    const dx = (screenX - this.dragStartX) / this.zoom;
    const dy = (screenY - this.dragStartY) / this.zoom;
    this.x = this.camStartX - dx;
    this.y = this.camStartY - dy;
  }

  public endDrag(): void {
    this.isDragging = false;
  }

  public zoomAt(screenX: number, screenY: number, zoomDelta: number): void {
    const oldZoom = this.zoom;
    const factor = zoomDelta > 0 ? 1.15 : 0.85;
    let newZoom = this.zoom * factor;
    newZoom = Math.max(this.minZoom, Math.min(this.maxZoom, newZoom));

    if (newZoom !== oldZoom) {
      // Zoom toward cursor position
      const worldMouseX = this.x + (screenX - this.viewportWidth / 2) / oldZoom;
      const worldMouseY = this.y + (screenY - this.viewportHeight / 2) / oldZoom;

      this.zoom = newZoom;

      this.x = worldMouseX - (screenX - this.viewportWidth / 2) / this.zoom;
      this.y = worldMouseY - (screenY - this.viewportHeight / 2) / this.zoom;
    }
  }

  public screenToWorld(screenX: number, screenY: number): Position {
    return {
      x: this.x + (screenX - this.viewportWidth / 2) / this.zoom,
      y: this.y + (screenY - this.viewportHeight / 2) / this.zoom,
    };
  }

  public worldToScreen(worldX: number, worldY: number): Position {
    return {
      x: (worldX - this.x) * this.zoom + this.viewportWidth / 2,
      y: (worldY - this.y) * this.zoom + this.viewportHeight / 2,
    };
  }

  public screenToTile(screenX: number, screenY: number, tileSize: number = 32): TileCoord {
    const worldPos = this.screenToWorld(screenX, screenY);
    return {
      tx: Math.floor(worldPos.x / tileSize),
      ty: Math.floor(worldPos.y / tileSize),
    };
  }

  public getVisibleTileBounds(tileSize: number = 32): { minTx: number; maxTx: number; minTy: number; maxTy: number } {
    const topLeft = this.screenToWorld(0, 0);
    const bottomRight = this.screenToWorld(this.viewportWidth, this.viewportHeight);

    return {
      minTx: Math.floor(topLeft.x / tileSize) - 1,
      maxTx: Math.ceil(bottomRight.x / tileSize) + 1,
      minTy: Math.floor(topLeft.y / tileSize) - 1,
      maxTy: Math.ceil(bottomRight.y / tileSize) + 1,
    };
  }
}
