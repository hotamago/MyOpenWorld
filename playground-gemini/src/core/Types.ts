// Core Types and Interfaces for "My Open World" Simulation

export enum BiomeType {
  DEEP_OCEAN = 'DEEP_OCEAN',
  COASTAL_WATER = 'COASTAL_WATER',
  SAND_BEACH = 'SAND_BEACH',
  LUSH_MEADOW = 'LUSH_MEADOW',
  DENSE_FOREST = 'DENSE_FOREST',
  FARMLAND = 'FARMLAND',
  SETTLEMENT = 'SETTLEMENT',
  ROCKY_HILLS = 'ROCKY_HILLS',
  SNOW_PEAK = 'SNOW_PEAK',
  MYSTIC_GROVE = 'MYSTIC_GROVE',
  VOLCANIC_CRATER = 'VOLCANIC_CRATER',
  DESERT_DUNES = 'DESERT_DUNES',
}

export enum WeatherType {
  CLEAR = 'CLEAR',
  RAIN = 'RAIN',
  MIST = 'MIST',
  MANA_STORM = 'MANA_STORM',
}

export enum SeasonType {
  SPRING = 'SPRING',
  SUMMER = 'SUMMER',
  AUTUMN = 'AUTUMN',
  WINTER = 'WINTER',
}

export enum DayPhase {
  DAWN = 'DAWN',     // 05:00 - 07:59
  DAY = 'DAY',       // 08:00 - 16:59
  DUSK = 'DUSK',     // 17:00 - 19:59
  NIGHT = 'NIGHT',   // 20:00 - 04:59
}

export enum OverlayType {
  NONE = 'NONE',
  TEMPERATURE = 'TEMPERATURE',
  MOISTURE = 'MOISTURE',
  POPULATION_DENSITY = 'POPULATION_DENSITY',
  MANA_FLUX = 'MANA_FLUX',
  ELEVATION = 'ELEVATION',
}

export enum ColorblindMode {
  NORMAL = 'NORMAL',
  DEUTERANOPIA = 'DEUTERANOPIA', // Green weak
  PROTANOPIA = 'PROTANOPIA',     // Red weak
  TRITANOPIA = 'TRITANOPIA',     // Blue weak
  HIGH_CONTRAST = 'HIGH_CONTRAST',
}

export enum EntityState {
  IDLE_WANDER = 'IDLE_WANDER',
  SEEK_FOOD = 'SEEK_FOOD',
  EATING = 'EATING',
  SEEK_REST = 'SEEK_REST',
  SLEEPING = 'SLEEPING',
  WORK_GATHER = 'WORK_GATHER',
  SOCIALIZE = 'SOCIALIZE',
  MEDITATE = 'MEDITATE',
  FLEE = 'FLEE',
}

export enum SpeciesType {
  HUMAN = 'HUMAN',
  ELF = 'ELF',
  DWARF = 'DWARF',
  DEER = 'DEER',
  WOLF = 'WOLF',
  WISP = 'WISP',
}

export enum ItemCategory {
  FOOD = 'FOOD',
  RESOURCE = 'RESOURCE',
  TOOL = 'TOOL',
  ARTIFACT = 'ARTIFACT',
  POTION = 'POTION',
}

export enum ItemRarity {
  COMMON = 'COMMON',
  UNCOMMON = 'UNCOMMON',
  RARE = 'RARE',
  EPIC = 'EPIC',
  CELESTIAL = 'CELESTIAL',
}

export interface Position {
  x: number;
  y: number;
}

export interface TileCoord {
  tx: number;
  ty: number;
}

export interface ResourceNode {
  type: 'berry_bush' | 'wheat_crop' | 'tree' | 'iron_ore' | 'water_well' | 'mana_crystal' | 'campfire' | 'bed' | 'house';
  amount: number;
  maxAmount: number;
  growthStage?: number; // 0: seedling, 1: growing, 2: ripe
  regrowTime: number;   // ticks to regenerate
  currentRegrow: number;
  isOccupied: boolean;
}

export interface TileData {
  x: number;
  y: number;
  elevation: number;       // -1.0 to 1.0 (meters mapped 0 - 2500m)
  temperature: number;     // Celsius (-15°C to 45°C)
  moisture: number;        // Percentage (0% to 100%)
  manaFlux: number;        // nJ/m³ (0 to 1000 nJ/m³)
  biome: BiomeType;
  baseColor: string;
  walkable: boolean;
  walkSpeedMod: number;    // Multiplier (1.0 = normal, 0.5 = slow, 1.2 = road)
  resource?: ResourceNode;
  decoration?: string;     // e.g. 'flower', 'pebbles', 'mushroom', 'grass_tuft'
  variant: number;         // 0 - 3 for sprite variety
}

export interface Homeostasis {
  health: number;
  maxHealth: number;
  hunger: number;          // 0 = full, 100 = starving
  maxHunger: number;
  energy: number;          // 100 = full energy, 0 = exhausted
  maxEnergy: number;
  mood: number;            // 0 = depressed, 100 = ecstatic
  maxMood: number;
  mana: number;
  maxMana: number;
}

export interface ItemEffect {
  hunger?: number;
  energy?: number;
  health?: number;
  mood?: number;
  mana?: number;
  buffDuration?: number;
}

export interface Item {
  id: string;
  name: string;
  description: string;
  category: ItemCategory;
  rarity: ItemRarity;
  icon: string;
  weight: number;          // kg
  quantity: number;
  maxStack: number;
  effects?: ItemEffect;
  value: number;           // Gold value
}

export interface EntityMemory {
  id: string;
  timestamp: string;
  text: string;
  type: 'thought' | 'action' | 'interaction' | 'divine';
}

export interface GameTime {
  totalTicks: number;
  divineEra: number;
  year: number;
  season: SeasonType;
  day: number;
  hour: number;
  minute: number;
  second: number;
  dayPhase: DayPhase;
  timeSpeed: number;       // 0, 0.5, 1, 2, 4, 16
  isPaused: boolean;
}

export interface SimulationStats {
  fps: number;
  tps: number;
  entityCount: number;
  aliveCount: number;
  chunkCount: number;
  weather: WeatherType;
  activeOverlay: OverlayType;
}
