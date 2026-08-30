// Master UI Orchestrator and Input Handler

import { ColorblindMode, OverlayType, SpeciesType, TileCoord } from '../core/Types';
import { Clock } from '../core/Clock';
import { WorldMap } from '../world/WorldMap';
import { Entity } from '../entities/Entity';
import { CanvasRenderer } from '../renderer/CanvasRenderer';
import { Minimap } from '../renderer/Minimap';
import { TopBar } from './TopBar';
import { InspectorPanel } from './InspectorPanel';
import { OverlayLegend } from './OverlayLegend';
import { InventoryModal } from './InventoryModal';
import { ChronicleLog } from './ChronicleLog';
import { HelpModal } from './HelpModal';
import { EventBus, Events } from '../core/EventBus';

export class UIManager {
  public topBar: TopBar;
  public inspector: InspectorPanel;
  public overlayLegend: OverlayLegend;
  public inventoryModal: InventoryModal;
  public chronicle: ChronicleLog;
  public helpModal: HelpModal;

  private canvas: HTMLCanvasElement;
  private renderer: CanvasRenderer;
  private minimap: Minimap;
  private worldMap: WorldMap;
  private clock: Clock;
  private entities: Entity[];
  private onSpawnEntityCallback?: (species: SpeciesType, tx: number, ty: number) => Entity;

  private isDragging: boolean = false;
  private dragStartX: number = 0;
  private dragStartY: number = 0;
  private hasDragged: boolean = false;

  private eventBus = EventBus.getInstance();

  constructor(
    canvas: HTMLCanvasElement,
    renderer: CanvasRenderer,
    minimap: Minimap,
    worldMap: WorldMap,
    clock: Clock,
    entities: Entity[],
    onSpawnEntity?: (species: SpeciesType, tx: number, ty: number) => Entity
  ) {
    this.canvas = canvas;
    this.renderer = renderer;
    this.minimap = minimap;
    this.worldMap = worldMap;
    this.clock = clock;
    this.entities = entities;
    this.onSpawnEntityCallback = onSpawnEntity;

    // Grab UI Containers
    const topBarContainer = document.getElementById('topbar-container')!;
    const inspectorContainer = document.getElementById('inspector-container')!;
    const legendContainer = document.getElementById('legend-container')!;
    const inventoryModalContainer = document.getElementById('inventory-modal-container')!;
    const chronicleContainer = document.getElementById('chronicle-container')!;
    const helpModalContainer = document.getElementById('help-modal-container')!;

    this.inventoryModal = new InventoryModal(inventoryModalContainer);
    this.helpModal = new HelpModal(helpModalContainer);
    this.overlayLegend = new OverlayLegend(legendContainer);
    this.chronicle = new ChronicleLog(chronicleContainer);

    this.inspector = new InspectorPanel(
      inspectorContainer,
      this.inventoryModal,
      (entity) => {
        this.renderer.camera.followEntity(entity);
        this.chronicle.addMessage(`Camera locked to follow ${entity.name}.`, 'divine');
      },
      (species, tx, ty) => {
        if (this.onSpawnEntityCallback) {
          const newEntity = this.onSpawnEntityCallback(species, tx, ty);
          this.inspector.inspectEntity(newEntity);
        }
      }
    );
    this.inspector.setWorldMap(worldMap);

    this.topBar = new TopBar(
      topBarContainer,
      clock,
      (overlayType) => {
        this.renderer.activeOverlay = overlayType;
        this.overlayLegend.update(overlayType);
      },
      () => {
        this.helpModal.show();
      }
    );

    this.setupInputHandlers();
    this.setupEventListeners();
  }

  private setupInputHandlers(): void {
    // 1. Mouse Drag & Click
    this.canvas.addEventListener('mousedown', (e) => {
      this.isDragging = true;
      this.hasDragged = false;
      this.dragStartX = e.clientX;
      this.dragStartY = e.clientY;
      this.renderer.camera.startDrag(e.clientX, e.clientY);
    });

    window.addEventListener('mousemove', (e) => {
      if (this.isDragging) {
        if (Math.hypot(e.clientX - this.dragStartX, e.clientY - this.dragStartY) > 5) {
          this.hasDragged = true;
        }
        this.renderer.camera.onDrag(e.clientX, e.clientY);
      }

      // Update hover tile
      const rect = this.canvas.getBoundingClientRect();
      const clientX = e.clientX - rect.left;
      const clientY = e.clientY - rect.top;
      this.renderer.hoveredTile = this.renderer.camera.screenToTile(clientX, clientY, this.worldMap.tileSize);
    });

    window.addEventListener('mouseup', (e) => {
      if (this.isDragging) {
        this.isDragging = false;
        this.renderer.camera.endDrag();

        // Handle Click (if not dragged significantly)
        if (!this.hasDragged && e.target === this.canvas) {
          const rect = this.canvas.getBoundingClientRect();
          const clickX = e.clientX - rect.left;
          const clickY = e.clientY - rect.top;
          this.handleCanvasClick(clickX, clickY);
        }
      }
    });

    // 2. Mouse Wheel Zoom
    this.canvas.addEventListener(
      'wheel',
      (e) => {
        e.preventDefault();
        const rect = this.canvas.getBoundingClientRect();
        const mouseX = e.clientX - rect.left;
        const mouseY = e.clientY - rect.top;
        const zoomDelta = e.deltaY < 0 ? 1 : -1;
        this.renderer.camera.zoomAt(mouseX, mouseY, zoomDelta);
      },
      { passive: false }
    );

    // 3. Keyboard Shortcuts
    window.addEventListener('keydown', (e) => {
      // Avoid shortcuts when typing in inputs
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;

      const camSpeed = 40 / this.renderer.camera.zoom;

      switch (e.code) {
        case 'KeyW':
        case 'ArrowUp':
          this.renderer.camera.y -= camSpeed;
          this.renderer.camera.targetEntity = null;
          break;
        case 'KeyS':
        case 'ArrowDown':
          this.renderer.camera.y += camSpeed;
          this.renderer.camera.targetEntity = null;
          break;
        case 'KeyA':
        case 'ArrowLeft':
          this.renderer.camera.x -= camSpeed;
          this.renderer.camera.targetEntity = null;
          break;
        case 'KeyD':
        case 'ArrowRight':
          this.renderer.camera.x += camSpeed;
          this.renderer.camera.targetEntity = null;
          break;

        case 'Space':
          e.preventDefault();
          this.clock.togglePause();
          break;
        case 'KeyT':
          this.clock.manualStep();
          break;
        case 'Digit1':
          this.clock.setSpeed(0.5);
          break;
        case 'Digit2':
          this.clock.setSpeed(1.0);
          break;
        case 'Digit3':
          this.clock.setSpeed(2.0);
          break;
        case 'Digit4':
          this.clock.setSpeed(4.0);
          break;
        case 'Digit5':
          this.clock.setSpeed(16.0);
          break;
        case 'Escape':
          this.inspector.clearSelection();
          this.renderer.selectedEntity = null;
          this.renderer.camera.targetEntity = null;
          break;
        case 'Slash':
        case 'KeyH':
          this.helpModal.show();
          break;
      }
    });

    // 4. Window Resize
    window.addEventListener('resize', () => {
      this.renderer.handleResize();
    });
  }

  private handleCanvasClick(screenX: number, screenY: number): void {
    const worldPos = this.renderer.camera.screenToWorld(screenX, screenY);
    const tileSize = this.worldMap.tileSize;

    // Check if clicked an entity (hitbox radius ~16px in world)
    let clickedEntity: Entity | null = null;
    let closestDist = 18; // In world pixels

    for (const e of this.entities) {
      if (e.vitality.needs.health <= 0) continue;
      const ex = e.position.x * tileSize;
      const ey = e.position.y * tileSize;
      const dist = Math.hypot(ex - worldPos.x, ey - worldPos.y);
      if (dist < closestDist) {
        closestDist = dist;
        clickedEntity = e;
      }
    }

    if (clickedEntity) {
      this.renderer.selectedEntity = clickedEntity;
      this.inspector.inspectEntity(clickedEntity);
      this.eventBus.emit(Events.ENTITY_SELECTED, clickedEntity);
    } else {
      // Clicked a tile
      const tx = Math.floor(worldPos.x / tileSize);
      const ty = Math.floor(worldPos.y / tileSize);
      const tile = this.worldMap.getTile(tx, ty);

      if (tile) {
        this.renderer.selectedEntity = null;
        this.inspector.inspectTile(tile);
        this.eventBus.emit(Events.TILE_SELECTED, tile);
      } else {
        this.inspector.clearSelection();
        this.renderer.selectedEntity = null;
      }
    }
  }

  private setupEventListeners(): void {
    // Time tick updates clock
    this.eventBus.on(Events.TIME_TICK, (time) => {
      this.topBar.updateTimeDisplay(time);
      this.inspector.updateLiveStats();
    });

    // Camera teleport from Minimap
    this.eventBus.on(Events.CAMERA_TELEPORT_TILE, (data: { normX: number; normY: number }) => {
      const tx = Math.floor(data.normX * this.worldMap.width);
      const ty = Math.floor(data.normY * this.worldMap.height);
      this.renderer.camera.centerOnTile(tx, ty, this.worldMap.tileSize);
    });
  }
}
