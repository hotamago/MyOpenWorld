// Strongly-typed Event Bus for Decoupled Communication

type EventCallback<T = any> = (data: T) => void;

export class EventBus {
  private static instance: EventBus;
  private listeners: Map<string, Set<EventCallback>> = new Map();

  private constructor() {}

  public static getInstance(): EventBus {
    if (!EventBus.instance) {
      EventBus.instance = new EventBus();
    }
    return EventBus.instance;
  }

  public on<T = any>(event: string, callback: EventCallback<T>): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(callback);

    // Return unsubscribe function
    return () => {
      this.off(event, callback);
    };
  }

  public off<T = any>(event: string, callback: EventCallback<T>): void {
    const set = this.listeners.get(event);
    if (set) {
      set.delete(callback);
      if (set.size === 0) {
        this.listeners.delete(event);
      }
    }
  }

  public emit<T = any>(event: string, data?: T): void {
    const set = this.listeners.get(event);
    if (set) {
      set.forEach((cb) => {
        try {
          cb(data);
        } catch (e) {
          console.error(`Error in event listener for '${event}':`, e);
        }
      });
    }
  }

  public clear(): void {
    this.listeners.clear();
  }
}

export const Events = {
  // Time & Environment
  TIME_TICK: 'time:tick',
  TIME_SPEED_CHANGED: 'time:speed_changed',
  DAY_PHASE_CHANGED: 'day_phase:changed',
  WEATHER_CHANGED: 'weather:changed',
  SEASON_CHANGED: 'season:changed',

  // Selection
  ENTITY_SELECTED: 'entity:selected',
  ENTITY_DESELECTED: 'entity:deselected',
  TILE_SELECTED: 'tile:selected',

  // Overlays
  OVERLAY_CHANGED: 'overlay:changed',
  COLORBLIND_MODE_CHANGED: 'colorblind:changed',

  // Entities
  ENTITY_SPAWNED: 'entity:spawned',
  ENTITY_DIED: 'entity:died',
  ENTITY_STATE_CHANGED: 'entity:state_changed',
  ENTITY_INVENTORY_CHANGED: 'entity:inventory_changed',

  // Narrative / Chronicle
  CHRONICLE_LOG: 'chronicle:log',

  // Divine Intervention
  GOD_ACTION: 'god:action',

  // Camera
  CAMERA_FOCUS_ENTITY: 'camera:focus_entity',
  CAMERA_TELEPORT_TILE: 'camera:teleport_tile',
};
