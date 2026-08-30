// Master Entity Class

import {
  EntityMemory,
  EntityState,
  Position,
  SpeciesType,
  TileCoord,
} from '../core/Types';
import { SPECIES_CONFIGS, SpeciesConfig } from './Species';
import { VitalityManager } from './Homeostasis';
import { Inventory } from './Inventory';
import { AIController } from './AIController';
import { createItem } from './Items';
import { WorldMap } from '../world/WorldMap';
import { EventBus, Events } from '../core/EventBus';

export class Entity {
  public readonly id: string;
  public name: string;
  public species: SpeciesType;
  public config: SpeciesConfig;
  public role: string;
  public trait: string;
  public age: number;

  public position: Position;
  public baseSpeed: number;
  public facingDirection: 'left' | 'right' = 'right';
  public walkAnimCycle: number = 0;

  // Appearance
  public skinColor: string;
  public hairColor: string;
  public outfitColor: string;

  // Systems
  public vitality: VitalityManager;
  public inventory: Inventory;
  public ai: AIController;

  // State & Memories
  public state: EntityState = EntityState.IDLE_WANDER;
  public currentGoal: string = 'Wandering peacefully';
  public memories: EntityMemory[] = [];
  public currentSpeech: string | null = null;
  public speechTimer: number = 0;
  public isBlessed: boolean = false;
  public blessedTimer: number = 0;

  private eventBus = EventBus.getInstance();

  constructor(
    id: string,
    name: string,
    species: SpeciesType,
    startX: number,
    startY: number,
    role?: string
  ) {
    this.id = id;
    this.name = name;
    this.species = species;
    this.config = SPECIES_CONFIGS[species];
    this.role = role || this.config.roles[Math.floor(Math.random() * this.config.roles.length)];
    this.trait = this.config.traits[Math.floor(Math.random() * this.config.traits.length)];
    this.age = 18 + Math.floor(Math.random() * 35);

    this.position = { x: startX + 0.5, y: startY + 0.5 };
    this.baseSpeed = this.config.baseSpeed;

    // Procedural appearance
    this.skinColor = this.config.skinColors[Math.floor(Math.random() * this.config.skinColors.length)];
    this.hairColor = this.config.hairColors[Math.floor(Math.random() * this.config.hairColors.length)];
    this.outfitColor = this.config.spriteColor;

    this.vitality = new VitalityManager();
    this.inventory = new Inventory(species === SpeciesType.DWARF ? 35.0 : 25.0);
    this.ai = new AIController(this);

    // Initial Starter Items
    this.giveStarterInventory();

    this.addMemory(`Born in the realm of Gaia as a ${this.role}.`);
  }

  private giveStarterInventory(): void {
    if (this.species === SpeciesType.HUMAN || this.species === SpeciesType.ELF || this.species === SpeciesType.DWARF) {
      this.inventory.addItem(createItem('wild_berries', 3 + Math.floor(Math.random() * 4)));
      this.inventory.addItem(createItem('hearty_bread', 1));

      if (this.role.includes('Farmer')) {
        this.inventory.addItem(createItem('wheat_grain', 5));
      } else if (this.role.includes('Lumberjack')) {
        this.inventory.addItem(createItem('oak_wood', 3));
      } else if (this.role.includes('Miner')) {
        this.inventory.addItem(createItem('iron_ore', 2));
        this.inventory.addItem(createItem('forged_pickaxe', 1));
      } else if (this.species === SpeciesType.ELF) {
        this.inventory.addItem(createItem('mana_mushroom', 2));
      }
    }
  }

  public update(deltaTimeSec: number, worldMap: WorldMap, allEntities: Entity[]): void {
    const isSleeping = this.state === EntityState.SLEEPING;
    const isWorking = this.state === EntityState.WORK_GATHER;
    const isMoving = this.ai.getPath().length > 0;

    const currentTile = worldMap.getTile(Math.floor(this.position.x), Math.floor(this.position.y));
    const nearCampfire = !!(currentTile && currentTile.decoration === 'campfire');

    // Tick vitality
    this.vitality.tick(isSleeping, isWorking, isMoving, nearCampfire);

    // Walking animation cycle
    if (isMoving) {
      this.walkAnimCycle += deltaTimeSec * 8;
    } else {
      this.walkAnimCycle = 0;
    }

    // Speech bubble timer
    if (this.speechTimer > 0) {
      this.speechTimer -= deltaTimeSec;
      if (this.speechTimer <= 0) {
        this.currentSpeech = null;
      }
    }

    // Divine blessing timer
    if (this.isBlessed) {
      this.blessedTimer -= deltaTimeSec;
      if (this.blessedTimer <= 0) {
        this.isBlessed = false;
      }
    }

    // Update AI behavior
    this.ai.update(deltaTimeSec, worldMap, allEntities);
  }

  public setState(state: EntityState): void {
    if (this.state !== state) {
      this.state = state;
      this.eventBus.emit(Events.ENTITY_STATE_CHANGED, { entityId: this.id, state });
    }
  }

  public setGoal(goal: string): void {
    this.currentGoal = goal;
  }

  public say(text: string, durationSec: number = 4.0): void {
    this.currentSpeech = text;
    this.speechTimer = durationSec;
  }

  public addMemory(text: string, type: 'thought' | 'action' | 'interaction' | 'divine' = 'action'): void {
    const memory: EntityMemory = {
      id: Math.random().toString(36).substring(2, 9),
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
      text,
      type,
    };
    this.memories.unshift(memory);
    if (this.memories.length > 25) {
      this.memories.pop();
    }
  }

  public applyDivineBlessing(): void {
    this.vitality.applyDivineBlessing();
    this.isBlessed = true;
    this.blessedTimer = 60.0;
    this.say('✨ By the True God, I am filled with celestial vitality!', 5.0);
    this.addMemory('Received direct Divine Blessing from the True God!', 'divine');
  }
}
