// Autonomous Utility AI Decision Making and State Machine

import { EntityState, ItemCategory, Position, SpeciesType, TileCoord } from '../core/Types';
import { WorldMap } from '../world/WorldMap';
import { Pathfinding } from '../world/Pathfinding';
import { Entity } from './Entity';
import { DialogueGenerator } from './Dialogue';
import { EventBus, Events } from '../core/EventBus';

export class AIController {
  private entity: Entity;
  private currentPath: TileCoord[] = [];
  private pathIndex: number = 0;
  private actionTimer: number = 0;
  private targetTile: TileCoord | null = null;
  private targetEntity: Entity | null = null;
  private homePosition: TileCoord;

  private eventBus = EventBus.getInstance();

  constructor(entity: Entity) {
    this.entity = entity;
    this.homePosition = {
      tx: Math.floor(entity.position.x),
      ty: Math.floor(entity.position.y),
    };
  }

  public update(deltaTimeSec: number, worldMap: WorldMap, allEntities: Entity[]): void {
    const vitality = this.entity.vitality.needs;

    // Check action timers (for eating, sleeping, gathering)
    if (this.actionTimer > 0) {
      this.actionTimer -= deltaTimeSec;
      if (this.actionTimer <= 0) {
        this.onActionTimerComplete(worldMap);
      }
      return;
    }

    // Follow current path if moving
    if (this.currentPath.length > 0 && this.pathIndex < this.currentPath.length) {
      this.followPath(deltaTimeSec, worldMap);
      return;
    }

    // Behavior Priority Decision Loop
    this.evaluateNextState(worldMap, allEntities);
  }

  private evaluateNextState(worldMap: WorldMap, allEntities: Entity[]): void {
    const vitality = this.entity.vitality.needs;
    const currentTx = Math.floor(this.entity.position.x);
    const currentTy = Math.floor(this.entity.position.y);

    // 1. STARVATION / HUNGER CHECK (> 60)
    if (vitality.hunger > 60) {
      // First check inventory for food
      const foodItem = this.entity.inventory.findFood();
      if (foodItem) {
        this.entity.setState(EntityState.EATING);
        this.actionTimer = 2.0; // 2 seconds eating animation
        this.entity.addMemory(`Consuming ${foodItem.name} from pack.`);
        return;
      }

      // Wildlife grazes on grass or hunts
      if (this.entity.species === SpeciesType.DEER) {
        this.entity.setState(EntityState.EATING);
        this.actionTimer = 3.0;
        this.entity.addMemory('Grazing on sweet meadow clover.');
        return;
      }

      // Seek nearest food source (Berry Bush, Farm Crops)
      const foodNode = worldMap.findNearestResource(
        currentTx,
        currentTy,
        ['berry_bush', 'wheat_crop'],
        true
      );

      if (foodNode) {
        this.targetTile = { tx: foodNode.x, ty: foodNode.y };
        this.entity.setState(EntityState.SEEK_FOOD);
        this.entity.setGoal(`Seeking food at (${foodNode.x}, ${foodNode.y})`);
        this.navigateToward(currentTx, currentTy, foodNode.x, foodNode.y, worldMap);
        return;
      }
    }

    // 2. EXHAUSTION / SLEEP CHECK (< 30)
    if (vitality.energy < 30) {
      if (this.entity.species === SpeciesType.WISP) {
        // Wisps meditate at mana shrines
        const shrine = worldMap.findNearestResource(currentTx, currentTy, ['mana_crystal']);
        if (shrine) {
          this.targetTile = { tx: shrine.x, ty: shrine.y };
          this.entity.setState(EntityState.MEDITATE);
          this.entity.setGoal(`Recharging at Mana Shrine (${shrine.x}, ${shrine.y})`);
          this.navigateToward(currentTx, currentTy, shrine.x, shrine.y, worldMap);
          return;
        }
      }

      // Humanoids seek beds or campfires
      const restSpot = worldMap.findNearestResource(
        currentTx,
        currentTy,
        ['bed', 'campfire'],
        false
      );

      if (restSpot) {
        this.targetTile = { tx: restSpot.x, ty: restSpot.y };
        this.entity.setState(EntityState.SEEK_REST);
        this.entity.setGoal(`Seeking rest at (${restSpot.x}, ${restSpot.y})`);
        this.navigateToward(currentTx, currentTy, restSpot.x, restSpot.y, worldMap);
        return;
      } else {
        // Wildlife or open-air resting
        this.entity.setState(EntityState.SLEEPING);
        this.actionTimer = 8.0;
        this.entity.setGoal('Resting on the soft meadow grass');
        this.entity.addMemory('Falling asleep under the open sky.');
        return;
      }
    }

    // 3. SOCIALIZING (Humanoids with low mood or friendly neighbors nearby)
    if (
      this.entity.species === SpeciesType.HUMAN ||
      this.entity.species === SpeciesType.ELF ||
      this.entity.species === SpeciesType.DWARF
    ) {
      if (Math.random() < 0.25 && vitality.mood < 75) {
        const neighbor = allEntities.find(
          (e) =>
            e.id !== this.entity.id &&
            e.vitality.needs.health > 0 &&
            (e.species === SpeciesType.HUMAN || e.species === SpeciesType.ELF || e.species === SpeciesType.DWARF) &&
            Math.hypot(e.position.x - currentTx, e.position.y - currentTy) < 6
        );

        if (neighbor) {
          this.targetEntity = neighbor;
          this.entity.setState(EntityState.SOCIALIZE);
          this.entity.setGoal(`Chatting with ${neighbor.name}`);
          this.actionTimer = 4.0;

          const dialogue = DialogueGenerator.generateSocialDialogue(
            this.entity.name,
            neighbor.name,
            this.entity.role
          );
          this.entity.say(dialogue);
          this.entity.vitality.needs.mood = Math.min(100, this.entity.vitality.needs.mood + 15);
          neighbor.vitality.needs.mood = Math.min(100, neighbor.vitality.needs.mood + 10);
          this.entity.addMemory(`Conversed with ${neighbor.name}: ${dialogue}`);
          return;
        }
      }
    }

    // 4. ROLE-BASED WORK & GATHERING
    if (Math.random() < 0.4) {
      let targetResources: string[] = [];

      if (this.entity.role.includes('Farmer')) targetResources = ['wheat_crop'];
      else if (this.entity.role.includes('Lumberjack')) targetResources = ['tree'];
      else if (this.entity.role.includes('Miner')) targetResources = ['iron_ore'];
      else if (this.entity.role.includes('Druid') || this.entity.species === SpeciesType.ELF) targetResources = ['berry_bush', 'mana_crystal'];
      else targetResources = ['berry_bush', 'water_well'];

      const workNode = worldMap.findNearestResource(currentTx, currentTy, targetResources, true);

      if (workNode) {
        this.targetTile = { tx: workNode.x, ty: workNode.y };
        this.entity.setState(EntityState.WORK_GATHER);
        this.entity.setGoal(`Gathering from ${workNode.resource.type} at (${workNode.x}, ${workNode.y})`);
        this.navigateToward(currentTx, currentTy, workNode.x, workNode.y, worldMap);
        return;
      }
    }

    // 5. IDLE WANDERING
    this.performIdleWander(currentTx, currentTy, worldMap);
  }

  private navigateToward(
    fromX: number,
    fromY: number,
    toX: number,
    toY: number,
    worldMap: WorldMap
  ): void {
    const path = Pathfinding.findPath(worldMap, fromX, fromY, toX, toY);
    if (path.length > 0) {
      this.currentPath = path;
      this.pathIndex = 0;
    } else {
      // Could not pathfind directly, wander nearby
      this.performIdleWander(fromX, fromY, worldMap);
    }
  }

  private performIdleWander(currentTx: number, currentTy: number, worldMap: WorldMap): void {
    this.entity.setState(EntityState.IDLE_WANDER);
    this.entity.setGoal('Wandering through the realm');

    // Pick a random walkable neighbor within 3 tiles
    for (let attempts = 0; attempts < 6; attempts++) {
      const rx = currentTx + Math.floor((Math.random() - 0.5) * 8);
      const ry = currentTy + Math.floor((Math.random() - 0.5) * 8);

      if (worldMap.isWalkable(rx, ry)) {
        const path = Pathfinding.findPath(worldMap, currentTx, currentTy, rx, ry, 150);
        if (path.length > 0) {
          this.currentPath = path;
          this.pathIndex = 0;
          this.targetTile = { tx: rx, ty: ry };
          return;
        }
      }
    }

    // If no path found, idle for 2 seconds
    this.actionTimer = 2.0;
  }

  private followPath(deltaTimeSec: number, worldMap: WorldMap): void {
    if (this.pathIndex >= this.currentPath.length) {
      this.currentPath = [];
      this.pathIndex = 0;
      return;
    }

    const nextTile = this.currentPath[this.pathIndex];
    const targetX = nextTile.tx + 0.5; // Center of tile
    const targetY = nextTile.ty + 0.5;

    const dx = targetX - this.entity.position.x;
    const dy = targetY - this.entity.position.y;
    const dist = Math.hypot(dx, dy);

    // Calculate dynamic speed based on species, tile speed modifier, and exhaustion
    const currentTile = worldMap.getTile(Math.floor(this.entity.position.x), Math.floor(this.entity.position.y));
    const tileMod = currentTile ? currentTile.walkSpeedMod : 1.0;
    const exhaustionMod = this.entity.vitality.needs.energy < 20 ? 0.6 : 1.0;
    const speed = this.entity.baseSpeed * tileMod * exhaustionMod;

    const moveDist = speed * deltaTimeSec;

    if (dist <= moveDist) {
      // Arrived at waypoint
      this.entity.position.x = targetX;
      this.entity.position.y = targetY;
      this.pathIndex++;

      // Check if finished entire path
      if (this.pathIndex >= this.currentPath.length) {
        this.currentPath = [];
        this.pathIndex = 0;
        this.onReachedDestination(worldMap);
      }
    } else {
      // Step towards waypoint
      this.entity.position.x += (dx / dist) * moveDist;
      this.entity.position.y += (dy / dist) * moveDist;
      this.entity.facingDirection = dx >= 0 ? 'right' : 'left';
    }
  }

  private onReachedDestination(worldMap: WorldMap): void {
    const currentTx = Math.floor(this.entity.position.x);
    const currentTy = Math.floor(this.entity.position.y);

    if (this.entity.state === EntityState.SEEK_FOOD) {
      // Arrived at food node -> harvest and eat
      const harvest = worldMap.harvestResource(currentTx, currentTy);
      if (harvest) {
        this.entity.setState(EntityState.EATING);
        this.actionTimer = 3.0;
        this.entity.addMemory(`Harvested and enjoying fresh ${harvest.type.replace('_', ' ')}.`);
      } else {
        // Graze or wander
        this.entity.setState(EntityState.IDLE_WANDER);
      }
    } else if (this.entity.state === EntityState.SEEK_REST) {
      this.entity.setState(EntityState.SLEEPING);
      this.actionTimer = 6.0; // Sleep duration
      this.entity.addMemory('Resting comfortably, recovering stamina.');
    } else if (this.entity.state === EntityState.WORK_GATHER) {
      const harvest = worldMap.harvestResource(currentTx, currentTy);
      this.actionTimer = 3.5;
      if (harvest) {
        this.entity.addMemory(`Worked at ${harvest.type.replace('_', ' ')} (obtained ${harvest.count}).`);
      }
    } else {
      this.entity.setState(EntityState.IDLE_WANDER);
      this.actionTimer = 1.0 + Math.random() * 2.0;
    }
  }

  private onActionTimerComplete(worldMap: WorldMap): void {
    if (this.entity.state === EntityState.EATING) {
      this.entity.vitality.consumeFood(45, 15, 10, 5);
      this.entity.setState(EntityState.IDLE_WANDER);
    } else if (this.entity.state === EntityState.SLEEPING) {
      this.entity.vitality.needs.energy = 95;
      this.entity.setState(EntityState.IDLE_WANDER);
      this.entity.addMemory('Woke up refreshed and full of vigor.');
    } else if (this.entity.state === EntityState.WORK_GATHER) {
      this.entity.vitality.needs.hunger += 10;
      this.entity.setState(EntityState.IDLE_WANDER);
    } else if (this.entity.state === EntityState.SOCIALIZE) {
      this.entity.setState(EntityState.IDLE_WANDER);
    } else if (this.entity.state === EntityState.MEDITATE) {
      this.entity.vitality.needs.mana = 100;
      this.entity.vitality.needs.mood = 90;
      this.entity.setState(EntityState.IDLE_WANDER);
      this.entity.addMemory('Meditation complete. Mana resonance peaking.');
    } else {
      this.entity.setState(EntityState.IDLE_WANDER);
    }
  }

  public getPath(): TileCoord[] {
    return this.currentPath.slice(this.pathIndex);
  }

  public getTargetTile(): TileCoord | null {
    return this.targetTile;
  }
}
