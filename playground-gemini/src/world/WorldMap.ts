// World Map Grid, Spatial Index, and Tile Management

import { BiomeType, Position, ResourceNode, TileCoord, TileData } from '../core/Types';
import { BIOME_CONFIGS } from './Biomes';

export class WorldMap {
  public readonly width: number;
  public readonly height: number;
  public readonly tileSize: number = 32; // In pixels
  private tiles: TileData[];
  private densityGrid: Float32Array;

  constructor(width: number = 64, height: number = 64) {
    this.width = width;
    this.height = height;
    this.tiles = new Array(width * height);
    this.densityGrid = new Float32Array(width * height);
  }

  public getIndex(x: number, y: number): number {
    return y * this.width + x;
  }

  public isInBounds(x: number, y: number): boolean {
    return x >= 0 && x < this.width && y >= 0 && y < this.height;
  }

  public getTile(x: number, y: number): TileData | null {
    if (!this.isInBounds(x, y)) return null;
    return this.tiles[this.getIndex(x, y)] || null;
  }

  public setTile(x: number, y: number, tile: TileData): void {
    if (this.isInBounds(x, y)) {
      this.tiles[this.getIndex(x, y)] = tile;
    }
  }

  public isWalkable(x: number, y: number): boolean {
    const tile = this.getTile(x, y);
    if (!tile) return false;
    if (!tile.walkable) return false;
    if (tile.resource && tile.resource.type === 'house') return false; // House walls block walking
    return true;
  }

  public getWalkableNeighbors(x: number, y: number): { x: number; y: number }[] {
    const neighbors: { x: number; y: number }[] = [];
    const dirs = [
      { dx: 0, dy: -1 },
      { dx: 1, dy: 0 },
      { dx: 0, dy: 1 },
      { dx: -1, dy: 0 },
    ];

    for (const d of dirs) {
      const nx = x + d.dx;
      const ny = y + d.dy;
      if (this.isWalkable(nx, ny)) {
        neighbors.push({ x: nx, y: ny });
      }
    }
    return neighbors;
  }

  // Update dynamic resources (crops growing, berry bush regrowing, campfire burning)
  public tickResources(): void {
    for (let i = 0; i < this.tiles.length; i++) {
      const tile = this.tiles[i];
      if (!tile || !tile.resource) continue;

      const res = tile.resource;

      if (res.type === 'berry_bush' || res.type === 'wheat_crop') {
        if (res.amount < res.maxAmount) {
          res.currentRegrow++;
          if (res.currentRegrow >= res.regrowTime) {
            res.currentRegrow = 0;
            res.amount++;
            if (res.type === 'wheat_crop') {
              res.growthStage = Math.min(2, Math.floor((res.amount / res.maxAmount) * 2));
            }
          }
        }
      }
    }
  }

  // Harvest resource at tile
  public harvestResource(x: number, y: number): { type: string; count: number } | null {
    const tile = this.getTile(x, y);
    if (!tile || !tile.resource || tile.resource.amount <= 0) return null;

    const res = tile.resource;
    const harvestedCount = Math.min(res.amount, 2);
    res.amount -= harvestedCount;
    res.currentRegrow = 0;
    if (res.type === 'wheat_crop') {
      res.growthStage = 0;
    }

    return {
      type: res.type,
      count: harvestedCount,
    };
  }

  // Find nearest resource matching condition
  public findNearestResource(
    startX: number,
    startY: number,
    types: string[],
    requireAvailable: boolean = true
  ): { x: number; y: number; resource: ResourceNode } | null {
    let closestDist = Infinity;
    let closestTile: { x: number; y: number; resource: ResourceNode } | null = null;

    for (let y = 0; y < this.height; y++) {
      for (let x = 0; x < this.width; x++) {
        const tile = this.getTile(x, y);
        if (!tile || !tile.resource) continue;

        if (types.includes(tile.resource.type)) {
          if (requireAvailable && tile.resource.amount <= 0 && tile.resource.type !== 'bed' && tile.resource.type !== 'campfire') {
            continue;
          }

          const dist = Math.hypot(x - startX, y - startY);
          if (dist < closestDist) {
            closestDist = dist;
            closestTile = { x, y, resource: tile.resource };
          }
        }
      }
    }

    return closestTile;
  }

  // Update real-time entity density grid for heatmap overlays
  public updateDensityGrid(entityPositions: Position[]): void {
    this.densityGrid.fill(0);
    const radius = 4; // kernel radius in tiles

    for (const pos of entityPositions) {
      const tx = Math.floor(pos.x);
      const ty = Math.floor(pos.y);

      for (let dy = -radius; dy <= radius; dy++) {
        for (let dx = -radius; dx <= radius; dx++) {
          const nx = tx + dx;
          const ny = ty + dy;
          if (this.isInBounds(nx, ny)) {
            const dist = Math.hypot(dx, dy);
            if (dist <= radius) {
              const weight = 1.0 - dist / radius;
              this.densityGrid[this.getIndex(nx, ny)] += weight;
            }
          }
        }
      }
    }
  }

  public getDensityAt(x: number, y: number): number {
    if (!this.isInBounds(x, y)) return 0;
    return this.densityGrid[this.getIndex(x, y)];
  }

  public getAllTiles(): TileData[] {
    return this.tiles;
  }
}
