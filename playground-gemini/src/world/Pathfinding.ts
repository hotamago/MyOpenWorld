// Optimized Grid A* Pathfinding with 8-directional movement

import { TileCoord } from '../core/Types';
import { WorldMap } from './WorldMap';

interface PathNode {
  x: number;
  y: number;
  g: number; // Cost from start
  h: number; // Heuristic to end
  f: number; // Total cost
  parent: PathNode | null;
}

export class Pathfinding {
  // Octile distance heuristic for 8-directional movement
  private static heuristic(x1: number, y1: number, x2: number, y2: number): number {
    const dx = Math.abs(x1 - x2);
    const dy = Math.abs(y1 - y2);
    const D = 1.0;
    const D2 = 1.41421356; // sqrt(2)
    return D * (dx + dy) + (D2 - 2 * D) * Math.min(dx, dy);
  }

  public static findPath(
    worldMap: WorldMap,
    startX: number,
    startY: number,
    targetX: number,
    targetY: number,
    maxSearchSteps: number = 800
  ): TileCoord[] {
    // If target is out of bounds or impassable, check if an adjacent tile is walkable
    if (!worldMap.isWalkable(targetX, targetY)) {
      const neighbors = worldMap.getWalkableNeighbors(targetX, targetY);
      if (neighbors.length === 0) return [];
      // Pick closest walkable neighbor
      neighbors.sort(
        (a, b) =>
          this.heuristic(startX, startY, a.x, a.y) - this.heuristic(startX, startY, b.x, b.y)
      );
      targetX = neighbors[0].x;
      targetY = neighbors[0].y;
    }

    if (startX === targetX && startY === targetY) {
      return [{ tx: targetX, ty: targetY }];
    }

    const openList: PathNode[] = [];
    const closedSet = new Uint8Array(worldMap.width * worldMap.height);
    const nodeMap = new Map<number, PathNode>();

    const startKey = startY * worldMap.width + startX;
    const startNode: PathNode = {
      x: startX,
      y: startY,
      g: 0,
      h: this.heuristic(startX, startY, targetX, targetY),
      f: 0,
      parent: null,
    };
    startNode.f = startNode.g + startNode.h;

    openList.push(startNode);
    nodeMap.set(startKey, startNode);

    let steps = 0;

    while (openList.length > 0 && steps < maxSearchSteps) {
      steps++;

      // Find node with lowest f cost (simple min search)
      let lowestIdx = 0;
      for (let i = 1; i < openList.length; i++) {
        if (
          openList[i].f < openList[lowestIdx].f ||
          (openList[i].f === openList[lowestIdx].f && openList[i].h < openList[lowestIdx].h)
        ) {
          lowestIdx = i;
        }
      }

      const current = openList.splice(lowestIdx, 1)[0];
      const currentKey = current.y * worldMap.width + current.x;

      if (current.x === targetX && current.y === targetY) {
        // Reconstruct path
        const path: TileCoord[] = [];
        let curr: PathNode | null = current;
        while (curr) {
          path.unshift({ tx: curr.x, ty: curr.y });
          curr = curr.parent;
        }
        // Remove the starting node so entity moves to next step
        if (path.length > 1) {
          path.shift();
        }
        return path;
      }

      closedSet[currentKey] = 1;

      // 8 neighbor directions
      const dirs = [
        { dx: 0, dy: -1, cost: 1.0 },
        { dx: 1, dy: 0, cost: 1.0 },
        { dx: 0, dy: 1, cost: 1.0 },
        { dx: -1, dy: 0, cost: 1.0 },
        { dx: 1, dy: -1, cost: 1.414 },
        { dx: 1, dy: 1, cost: 1.414 },
        { dx: -1, dy: 1, cost: 1.414 },
        { dx: -1, dy: -1, cost: 1.414 },
      ];

      for (const dir of dirs) {
        const nx = current.x + dir.dx;
        const ny = current.y + dir.dy;

        if (nx < 0 || nx >= worldMap.width || ny < 0 || ny >= worldMap.height) continue;

        const neighborKey = ny * worldMap.width + nx;
        if (closedSet[neighborKey] === 1) continue;

        if (!worldMap.isWalkable(nx, ny)) continue;

        // Prevent corner cutting if both orthogonal neighbors are blocked
        if (dir.dx !== 0 && dir.dy !== 0) {
          if (!worldMap.isWalkable(current.x + dir.dx, current.y) && !worldMap.isWalkable(current.x, current.y + dir.dy)) {
            continue;
          }
        }

        const tile = worldMap.getTile(nx, ny);
        const speedMod = tile ? tile.walkSpeedMod : 1.0;
        const moveCost = (dir.cost / speedMod);

        const tentativeG = current.g + moveCost;

        let neighborNode = nodeMap.get(neighborKey);

        if (!neighborNode) {
          neighborNode = {
            x: nx,
            y: ny,
            g: tentativeG,
            h: this.heuristic(nx, ny, targetX, targetY),
            f: 0,
            parent: current,
          };
          neighborNode.f = neighborNode.g + neighborNode.h;
          nodeMap.set(neighborKey, neighborNode);
          openList.push(neighborNode);
        } else if (tentativeG < neighborNode.g) {
          neighborNode.g = tentativeG;
          neighborNode.f = neighborNode.g + neighborNode.h;
          neighborNode.parent = current;
        }
      }
    }

    // No direct path found within step budget
    return [];
  }
}
