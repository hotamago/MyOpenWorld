// Main Application Bootstrap and Game Simulation Loop

import './styles/main.css';
import { SpeciesType } from './core/Types';
import { Clock } from './core/Clock';
import { WorldGenerator } from './world/WorldGenerator';
import { Entity } from './entities/Entity';
import { CanvasRenderer } from './renderer/CanvasRenderer';
import { Minimap } from './renderer/Minimap';
import { UIManager } from './ui/UIManager';
import { EventBus, Events } from './core/EventBus';

class Application {
  private clock: Clock;
  private renderer: CanvasRenderer;
  private minimap: Minimap;
  private uiManager: UIManager;
  private worldMap = WorldGenerator.generateWorld(72, 72, 2026);
  private entities: Entity[] = [];

  private lastTime: number = 0;
  private frameCount: number = 0;
  private fpsTimer: number = 0;
  private currentFps: number = 60;
  private currentTps: number = 20;

  private entityIdCounter: number = 1;

  constructor() {
    const worldCanvas = document.getElementById('world-canvas') as HTMLCanvasElement;
    const minimapCanvas = document.getElementById('minimap-canvas') as HTMLCanvasElement;

    this.clock = new Clock();
    this.renderer = new CanvasRenderer(worldCanvas);
    this.minimap = new Minimap(minimapCanvas);

    // Initial Population Setup
    this.populateInitialEntities();

    // Minimap terrain bake
    this.minimap.bakeTerrain(this.worldMap);

    // UI Manager
    this.uiManager = new UIManager(
      worldCanvas,
      this.renderer,
      this.minimap,
      this.worldMap,
      this.clock,
      this.entities,
      (species, tx, ty) => this.spawnEntity(species, tx, ty)
    );

    // Center camera on Village Center (approx tile 35, 37)
    this.renderer.camera.centerOnTile(35, 37, this.worldMap.tileSize);
    this.renderer.camera.zoom = 1.25;

    // Welcome chronicle log
    EventBus.getInstance().emit(Events.CHRONICLE_LOG, {
      text: 'True God initialized Gaia realm (Seed #2026-GAIA). Simulation active.',
      type: 'divine',
    });

    // Start Game Loop
    this.lastTime = performance.now();
    requestAnimationFrame((t) => this.gameLoop(t));
  }

  private spawnEntity(species: SpeciesType, tx: number, ty: number, role?: string, customName?: string): Entity {
    const names = {
      [SpeciesType.HUMAN]: ['Aria', 'Kael', 'Elena', 'Rowan', 'Cassian', 'Lyra', 'Gareth', 'Sylvia'],
      [SpeciesType.ELF]: ['Eldrin', 'Faeriel', 'Theron', 'Lunaria', 'Aeris', 'Valen'],
      [SpeciesType.DWARF]: ['Thorin', 'Borek', 'Grimli', 'Dagna', 'Brom', 'Helga'],
      [SpeciesType.DEER]: ['Fawn', 'Stag', 'Roebuck', 'Glade Strider'],
      [SpeciesType.WOLF]: ['Shadowfang', 'Silverclaw', 'Greyback', 'Ash'],
      [SpeciesType.WISP]: ['Aura', 'Glimmer', 'Lumina', 'Spark', 'Zephyr'],
    };

    const namePool = names[species];
    const name = customName || `${namePool[Math.floor(Math.random() * namePool.length)]} #${this.entityIdCounter++}`;

    const entity = new Entity(
      `entity-${this.entityIdCounter}`,
      name,
      species,
      tx,
      ty,
      role
    );

    this.entities.push(entity);

    EventBus.getInstance().emit(Events.CHRONICLE_LOG, {
      text: `${entity.name} (${entity.config.name}) appeared in Gaia.`,
      type: 'entity',
    });

    return entity;
  }

  private populateInitialEntities(): void {
    // 1. Human Village Residents
    this.spawnEntity(SpeciesType.HUMAN, 34, 37, 'Town Baker', 'Aria');
    this.spawnEntity(SpeciesType.HUMAN, 35, 36, 'Village Elder', 'Gareth');
    this.spawnEntity(SpeciesType.HUMAN, 38, 32, 'Farmer', 'Rowan');
    this.spawnEntity(SpeciesType.HUMAN, 40, 33, 'Farmer', 'Lyra');
    this.spawnEntity(SpeciesType.HUMAN, 31, 39, 'Lumberjack', 'Cassian');
    this.spawnEntity(SpeciesType.HUMAN, 36, 40, 'Weaver', 'Sylvia');

    // 2. Elven Rangers & Druids (Forest & East Grove)
    this.spawnEntity(SpeciesType.ELF, 22, 28, 'Forest Druid', 'Eldrin');
    this.spawnEntity(SpeciesType.ELF, 25, 32, 'Wind Ranger', 'Lunaria');
    this.spawnEntity(SpeciesType.ELF, 54, 32, 'Herbalist', 'Aeris');

    // 3. Dwarven Miners (Northern Highlands)
    this.spawnEntity(SpeciesType.DWARF, 32, 20, 'Deep Miner', 'Thorin');
    this.spawnEntity(SpeciesType.DWARF, 36, 18, 'Stonemason', 'Borek');

    // 4. Wildlife
    this.spawnEntity(SpeciesType.DEER, 18, 42, 'Glade Strider', 'Fawn');
    this.spawnEntity(SpeciesType.DEER, 20, 45, 'Glade Strider', 'Stag');
    this.spawnEntity(SpeciesType.WOLF, 16, 24, 'Pack Hunter', 'Shadowfang');

    // 5. Ethereal Wisps (Eastern Mana Shrine)
    this.spawnEntity(SpeciesType.WISP, 56, 33, 'Shrine Guardian', 'Lumina');
    this.spawnEntity(SpeciesType.WISP, 57, 31, 'Ley-Sprout', 'Glimmer');
  }

  private gameLoop(currentTime: number): void {
    const deltaTimeMs = Math.min(100, currentTime - this.lastTime);
    const deltaTimeSec = deltaTimeMs / 1000.0;
    this.lastTime = currentTime;

    // 1. Update Simulation Clock
    this.clock.update(deltaTimeSec);

    // 2. Update Dynamic World Resources
    if (!this.clock.getTime().isPaused && this.clock.getTime().timeSpeed > 0) {
      this.worldMap.tickResources();

      // 3. Update Living Entities
      const effectiveDelta = deltaTimeSec * this.clock.getTime().timeSpeed;
      for (const entity of this.entities) {
        if (entity.vitality.needs.health > 0) {
          entity.update(effectiveDelta, this.worldMap, this.entities);
        }
      }

      // 4. Update Spatial Density Grid for Overlays
      const alivePositions = this.entities
        .filter((e) => e.vitality.needs.health > 0)
        .map((e) => e.position);
      this.worldMap.updateDensityGrid(alivePositions);
    }

    // 5. Render Viewport Canvas
    const dayFraction = this.clock.getDayFraction();
    const weather = this.clock.getWeather();

    this.renderer.render(
      this.worldMap,
      this.entities,
      deltaTimeSec,
      dayFraction,
      weather
    );

    // 6. Render Minimap
    this.minimap.render(this.worldMap, this.entities, this.renderer.camera);

    // 7. FPS & Performance Monitoring
    this.frameCount++;
    this.fpsTimer += deltaTimeSec;
    if (this.fpsTimer >= 0.5) {
      this.currentFps = Math.round(this.frameCount / this.fpsTimer);
      this.currentTps = this.clock.getTime().isPaused ? 0 : Math.round(20 * this.clock.getTime().timeSpeed);
      this.frameCount = 0;
      this.fpsTimer = 0;

      const aliveCount = this.entities.filter((e) => e.vitality.needs.health > 0).length;
      this.uiManager.topBar.updateStats(aliveCount, this.currentFps, this.currentTps);
    }

    requestAnimationFrame((t) => this.gameLoop(t));
  }
}

// Bootstrap when DOM ready
window.addEventListener('DOMContentLoaded', () => {
  new Application();
});
