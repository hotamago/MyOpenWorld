// Comprehensive Entity and Tile Inspector Panel

import { BiomeType, Item, SpeciesType, TileData } from '../core/Types';
import { Entity } from '../entities/Entity';
import { WorldMap } from '../world/WorldMap';
import { BIOME_CONFIGS } from '../world/Biomes';
import { createItem } from '../entities/Items';
import { EventBus, Events } from '../core/EventBus';
import { InventoryModal } from './InventoryModal';

export class InspectorPanel {
  private container: HTMLElement;
  private currentEntity: Entity | null = null;
  private currentTile: TileData | null = null;
  private worldMap: WorldMap | null = null;
  private inventoryModal: InventoryModal;
  private eventBus = EventBus.getInstance();

  private onTrackEntityCallback?: (entity: Entity) => void;
  private onSpawnEntityCallback?: (species: SpeciesType, tx: number, ty: number) => void;

  constructor(
    container: HTMLElement,
    inventoryModal: InventoryModal,
    onTrackEntity?: (entity: Entity) => void,
    onSpawnEntity?: (species: SpeciesType, tx: number, ty: number) => void
  ) {
    this.container = container;
    this.inventoryModal = inventoryModal;
    this.onTrackEntityCallback = onTrackEntity;
    this.onSpawnEntityCallback = onSpawnEntity;

    this.renderEmpty();
  }

  public setWorldMap(worldMap: WorldMap): void {
    this.worldMap = worldMap;
  }

  public inspectEntity(entity: Entity): void {
    this.currentEntity = entity;
    this.currentTile = null;
    this.renderEntityView();
  }

  public inspectTile(tile: TileData): void {
    this.currentTile = tile;
    this.currentEntity = null;
    this.renderTileView();
  }

  public clearSelection(): void {
    this.currentEntity = null;
    this.currentTile = null;
    this.renderEmpty();
  }

  public updateLiveStats(): void {
    if (this.currentEntity) {
      this.updateEntityGauges(this.currentEntity);
    }
  }

  private renderEmpty(): void {
    this.container.innerHTML = `
      <div class="inspector-empty">
        <div class="empty-icon">🔭</div>
        <h3>Omniscient Observer</h3>
        <p>Click on any sentient being, creature, or terrain tile to inspect their live status and exercise True God capabilities.</p>
      </div>
    `;
  }

  private renderEntityView(): void {
    if (!this.currentEntity) return;
    const e = this.currentEntity;
    const v = e.vitality.needs;

    this.container.innerHTML = `
      <div class="inspector-header">
        <div class="avatar-box">
          <svg class="avatar-svg" viewBox="0 0 64 64" width="56" height="56">
            <!-- Background Halo -->
            <circle cx="32" cy="32" r="30" fill="${e.outfitColor}" opacity="0.25"/>
            <!-- Body -->
            <rect x="20" y="38" width="24" height="26" rx="6" fill="${e.outfitColor}"/>
            <!-- Head -->
            <circle cx="32" cy="24" r="14" fill="${e.skinColor}"/>
            <!-- Hair -->
            <circle cx="32" cy="18" r="14" fill="${e.hairColor}" clip-path="polygon(0 0, 64 0, 64 24, 0 24)"/>
            <!-- Eyes -->
            <circle cx="27" cy="24" r="2" fill="#1a202c"/>
            <circle cx="37" cy="24" r="2" fill="#1a202c"/>
          </svg>
          ${e.isBlessed ? '<span class="avatar-blessed-badge" title="Blessed by True God">✨</span>' : ''}
        </div>

        <div class="inspector-title-group">
          <div class="name-row">
            <h2 class="entity-name">${e.name}</h2>
            <button class="btn-close" id="btn-close-inspector" title="Close Panel">✕</button>
          </div>
          <div class="entity-tags">
            <span class="badge badge-species">${e.config.name}</span>
            <span class="badge badge-role">${e.role}</span>
            <span class="badge badge-trait">🏷️ ${e.trait}</span>
            <span class="badge badge-neutral">Age ${e.age}</span>
          </div>
        </div>
      </div>

      <!-- Current Action & Goal Card -->
      <div class="state-card">
        <div class="state-header">
          <span class="state-badge state-${e.state.toLowerCase()}">${this.formatStateBadge(e.state)}</span>
          <span class="state-pos">Pos: (${Math.floor(e.position.x)}, ${Math.floor(e.position.y)})</span>
        </div>
        <div class="state-goal">🎯 ${e.currentGoal}</div>
      </div>

      <!-- True God Divine Intervention Powers -->
      <div class="god-actions-card">
        <div class="card-title">⚡ Divine Interventions (True God)</div>
        <div class="god-btn-grid">
          <button class="btn-god" id="btn-god-bless" title="Restore all needs and bestow celestial blessing">
            🌟 Bless
          </button>
          <button class="btn-god" id="btn-god-feed" title="Instantly satisfy hunger with ambrosia">
            🍞 Feed
          </button>
          <button class="btn-god" id="btn-god-energize" title="Instantly refill energy">
            ⚡ Awaken
          </button>
          <button class="btn-god" id="btn-god-track" title="Lock camera to follow this entity">
            📍 Track
          </button>
        </div>
      </div>

      <!-- Live Homeostasis Gauges -->
      <div class="gauges-card">
        <div class="card-title">🧬 Homeostasis & Vitality</div>

        <!-- Health -->
        <div class="gauge-row">
          <div class="gauge-label">
            <span>❤️ Vital Health</span>
            <span id="txt-health">${Math.round(v.health)} / 100</span>
          </div>
          <div class="gauge-bar-bg">
            <div class="gauge-bar health" id="bar-health" style="width: ${(v.health / 100) * 100}%;"></div>
          </div>
        </div>

        <!-- Hunger -->
        <div class="gauge-row">
          <div class="gauge-label">
            <span>🍗 Hunger (0=Full, 100=Starving)</span>
            <span id="txt-hunger">${Math.round(v.hunger)}% ${v.hunger > 70 ? '⚠️' : ''}</span>
          </div>
          <div class="gauge-bar-bg">
            <div class="gauge-bar hunger" id="bar-hunger" style="width: ${(v.hunger / 100) * 100}%;"></div>
          </div>
        </div>

        <!-- Energy -->
        <div class="gauge-row">
          <div class="gauge-label">
            <span>⚡ Energy & Stamina</span>
            <span id="txt-energy">${Math.round(v.energy)}% ${v.energy < 30 ? '💤' : ''}</span>
          </div>
          <div class="gauge-bar-bg">
            <div class="gauge-bar energy" id="bar-energy" style="width: ${(v.energy / 100) * 100}%;"></div>
          </div>
        </div>

        <!-- Mood -->
        <div class="gauge-row">
          <div class="gauge-label">
            <span>😊 Mood & Morale</span>
            <span id="txt-mood">${Math.round(v.mood)}%</span>
          </div>
          <div class="gauge-bar-bg">
            <div class="gauge-bar mood" id="bar-mood" style="width: ${(v.mood / 100) * 100}%;"></div>
          </div>
        </div>

        <!-- Mana -->
        <div class="gauge-row">
          <div class="gauge-label">
            <span>✨ Mana Attunement</span>
            <span id="txt-mana">${Math.round(v.mana)} / 100</span>
          </div>
          <div class="gauge-bar-bg">
            <div class="gauge-bar mana" id="bar-mana" style="width: ${(v.mana / 100) * 100}%;"></div>
          </div>
        </div>
      </div>

      <!-- Inventory Section -->
      <div class="inventory-card">
        <div class="inventory-header">
          <div class="card-title">🎒 Inventory Pack</div>
          <span class="weight-badge">⚖️ ${e.inventory.getTotalWeight()} / ${e.inventory.maxWeight} kg</span>
        </div>

        <div class="inventory-grid">
          ${this.renderInventorySlots(e)}
        </div>
      </div>

      <!-- Personal Chronicle / Memories -->
      <div class="memories-card">
        <div class="card-title">📜 Living Chronicle & Thoughts</div>
        <div class="memories-stream">
          ${
            e.memories.length > 0
              ? e.memories
                  .map(
                    (m) => `
                <div class="memory-item ${m.type}">
                  <span class="memory-time">${m.timestamp}</span>
                  <span class="memory-text">${m.text}</span>
                </div>
              `
                  )
                  .join('')
              : '<div class="empty-text">No thoughts recorded yet.</div>'
          }
        </div>
      </div>
    `;

    this.bindEntityEvents(e);
  }

  private renderInventorySlots(entity: Entity): string {
    const slots: string[] = [];

    for (let i = 0; i < entity.inventory.maxSlots; i++) {
      const item = entity.inventory.getItemAt(i);
      if (item) {
        const rarityClass = `rarity-${item.rarity.toLowerCase()}`;
        slots.push(`
          <div class="item-slot filled ${rarityClass}" data-slot-index="${i}" title="${item.name} (Click for details)">
            <span class="slot-icon">${item.icon}</span>
            <span class="slot-qty">${item.quantity > 1 ? item.quantity : ''}</span>
          </div>
        `);
      } else {
        slots.push(`
          <div class="item-slot empty"></div>
        `);
      }
    }

    return slots.join('');
  }

  private renderTileView(): void {
    if (!this.currentTile) return;
    const t = this.currentTile;
    const config = BIOME_CONFIGS[t.biome];

    this.container.innerHTML = `
      <div class="inspector-header">
        <div class="avatar-box tile-avatar" style="background: ${t.baseColor};">
          <span style="font-size: 28px;">${config.icon}</span>
        </div>
        <div class="inspector-title-group">
          <div class="name-row">
            <h2 class="entity-name">${config.name}</h2>
            <button class="btn-close" id="btn-close-inspector">✕</button>
          </div>
          <div class="entity-tags">
            <span class="badge badge-neutral">Coordinates: (${t.x}, ${t.y})</span>
            <span class="badge ${t.walkable ? 'badge-walkable' : 'badge-blocked'}">${t.walkable ? 'Walkable' : 'Impassable'}</span>
          </div>
        </div>
      </div>

      <div class="state-card">
        <div class="state-goal">${config.description}</div>
      </div>

      <!-- Topography & Environmental Data -->
      <div class="gauges-card">
        <div class="card-title">🌍 Environmental Readings</div>
        <div class="tile-stats-grid">
          <div class="stat-card">
            <span class="stat-label">🌡️ Temperature</span>
            <span class="stat-value">${t.temperature}°C</span>
          </div>
          <div class="stat-card">
            <span class="stat-label">💧 Moisture</span>
            <span class="stat-value">${t.moisture}%</span>
          </div>
          <div class="stat-card">
            <span class="stat-label">🏔️ Elevation</span>
            <span class="stat-value">${Math.round(t.elevation * 1500 + 500)}m</span>
          </div>
          <div class="stat-card">
            <span class="stat-label">✨ Mana Flux</span>
            <span class="stat-value">${t.manaFlux} nJ/m³</span>
          </div>
        </div>
      </div>

      <!-- Resource Present on Tile -->
      ${
        t.resource
          ? `
        <div class="state-card">
          <div class="card-title">🌿 Resource Node: ${t.resource.type.replace('_', ' ').toUpperCase()}</div>
          <div class="state-goal">Available: <strong>${t.resource.amount} / ${t.resource.maxAmount}</strong> units</div>
        </div>
      `
          : ''
      }

      <!-- True God Creation & Spawn Powers -->
      <div class="god-actions-card">
        <div class="card-title">⚡ Genesis Powers at (${t.x}, ${t.y})</div>
        <div class="god-btn-grid">
          <button class="btn-god" id="btn-spawn-human">👤 Spawn Villager</button>
          <button class="btn-god" id="btn-spawn-elf">🧝 Spawn Elf</button>
          <button class="btn-god" id="btn-spawn-dwarf">🧔 Spawn Dwarf</button>
          <button class="btn-god" id="btn-spawn-deer">🦌 Spawn Deer</button>
          <button class="btn-god" id="btn-spawn-wisp">✨ Spawn Wisp</button>
          <button class="btn-god" id="btn-plant-food">🍓 Place Berries</button>
        </div>
      </div>
    `;

    this.bindTileEvents(t);
  }

  private updateEntityGauges(entity: Entity): void {
    const v = entity.vitality.needs;
    const hBar = document.getElementById('bar-health');
    const hTxt = document.getElementById('txt-health');
    const uBar = document.getElementById('bar-hunger');
    const uTxt = document.getElementById('txt-hunger');
    const eBar = document.getElementById('bar-energy');
    const eTxt = document.getElementById('txt-energy');
    const mBar = document.getElementById('bar-mood');
    const mTxt = document.getElementById('txt-mood');
    const mnBar = document.getElementById('bar-mana');
    const mnTxt = document.getElementById('txt-mana');

    if (hBar && hTxt) {
      hBar.style.width = `${(v.health / 100) * 100}%`;
      hTxt.innerText = `${Math.round(v.health)} / 100`;
    }
    if (uBar && uTxt) {
      uBar.style.width = `${(v.hunger / 100) * 100}%`;
      uTxt.innerText = `${Math.round(v.hunger)}% ${v.hunger > 70 ? '⚠️' : ''}`;
    }
    if (eBar && eTxt) {
      eBar.style.width = `${(v.energy / 100) * 100}%`;
      eTxt.innerText = `${Math.round(v.energy)}% ${v.energy < 30 ? '💤' : ''}`;
    }
    if (mBar && mTxt) {
      mBar.style.width = `${(v.mood / 100) * 100}%`;
      mTxt.innerText = `${Math.round(v.mood)}%`;
    }
    if (mnBar && mnTxt) {
      mnBar.style.width = `${(v.mana / 100) * 100}%`;
      mnTxt.innerText = `${Math.round(v.mana)} / 100`;
    }
  }

  private bindEntityEvents(entity: Entity): void {
    document.getElementById('btn-close-inspector')?.addEventListener('click', () => {
      this.clearSelection();
      this.eventBus.emit(Events.ENTITY_DESELECTED);
    });

    // Divine Bless
    document.getElementById('btn-god-bless')?.addEventListener('click', () => {
      entity.applyDivineBlessing();
      this.eventBus.emit(Events.CHRONICLE_LOG, {
        text: `True God bestowed Divine Blessing upon ${entity.name}!`,
        type: 'divine',
      });
      this.renderEntityView();
    });

    // Divine Feed
    document.getElementById('btn-god-feed')?.addEventListener('click', () => {
      entity.vitality.consumeFood(100, 20, 25, 10);
      entity.say('🍞 Divine ambrosia descended from the heavens!', 4.0);
      entity.addMemory('Feasted upon celestial ambrosia granted by True God.', 'divine');
      this.renderEntityView();
    });

    // Divine Energize
    document.getElementById('btn-god-energize')?.addEventListener('click', () => {
      entity.vitality.needs.energy = 100;
      entity.say('⚡ A bolt of pure energy surges through me!', 4.0);
      entity.addMemory('Energized with celestial vigor by True God.', 'divine');
      this.renderEntityView();
    });

    // Camera Track
    document.getElementById('btn-god-track')?.addEventListener('click', () => {
      if (this.onTrackEntityCallback) {
        this.onTrackEntityCallback(entity);
      }
    });

    // Inventory Slots Click
    document.querySelectorAll('.item-slot.filled').forEach((slot) => {
      slot.addEventListener('click', (e) => {
        const slotIdx = parseInt((e.currentTarget as HTMLElement).dataset.slotIndex || '0', 10);
        const item = entity.inventory.getItemAt(slotIdx);
        if (item) {
          this.inventoryModal.showItemCard(
            item,
            entity,
            (consumedItem) => {
              // Consume Item
              if (consumedItem.effects) {
                const ef = consumedItem.effects;
                entity.vitality.consumeFood(
                  Math.abs(ef.hunger || 0),
                  ef.energy || 0,
                  ef.mood || 0,
                  ef.health || 0
                );
                entity.inventory.removeItem(consumedItem.id, 1);
                entity.say(`Tastes delicious! (+${Math.abs(ef.hunger || 0)} satiety)`);
                entity.addMemory(`Consumed 1x ${consumedItem.name}.`);
                this.renderEntityView();
              }
            },
            (droppedItem) => {
              // Drop Item
              entity.inventory.removeItem(droppedItem.id, 1);
              entity.addMemory(`Dropped 1x ${droppedItem.name} on the ground.`);
              this.renderEntityView();
            }
          );
        }
      });
    });
  }

  private bindTileEvents(tile: TileData): void {
    document.getElementById('btn-close-inspector')?.addEventListener('click', () => {
      this.clearSelection();
    });

    const spawn = (species: SpeciesType) => {
      if (this.onSpawnEntityCallback) {
        this.onSpawnEntityCallback(species, tile.x, tile.y);
      }
    };

    document.getElementById('btn-spawn-human')?.addEventListener('click', () => spawn(SpeciesType.HUMAN));
    document.getElementById('btn-spawn-elf')?.addEventListener('click', () => spawn(SpeciesType.ELF));
    document.getElementById('btn-spawn-dwarf')?.addEventListener('click', () => spawn(SpeciesType.DWARF));
    document.getElementById('btn-spawn-deer')?.addEventListener('click', () => spawn(SpeciesType.DEER));
    document.getElementById('btn-spawn-wisp')?.addEventListener('click', () => spawn(SpeciesType.WISP));

    document.getElementById('btn-plant-food')?.addEventListener('click', () => {
      tile.resource = {
        type: 'berry_bush',
        amount: 3,
        maxAmount: 3,
        growthStage: 2,
        regrowTime: 40,
        currentRegrow: 0,
        isOccupied: false,
      };
      tile.decoration = 'berry_bush';
      this.eventBus.emit(Events.CHRONICLE_LOG, {
        text: `True God planted a wild berry bush at (${tile.x}, ${tile.y}).`,
        type: 'divine',
      });
      this.renderTileView();
    });
  }

  private formatStateBadge(state: string): string {
    switch (state) {
      case 'SEEK_FOOD': return '🍗 Seeking Food';
      case 'EATING': return '🍴 Eating Meal';
      case 'SEEK_REST': return '🏃 Seeking Rest';
      case 'SLEEPING': return '💤 Sleeping';
      case 'WORK_GATHER': return '🌾 Gathering';
      case 'SOCIALIZE': return '💬 Chatting';
      case 'MEDITATE': return '✨ Meditating';
      case 'FLEE': return '⚡ Fleeing';
      case 'IDLE_WANDER': default: return '🚶 Wandering';
    }
  }
}
