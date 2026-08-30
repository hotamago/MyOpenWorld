// Top Header Bar with Clock, Time Speed Controls, Overlays, and Accessibility

import { ColorblindMode, GameTime, OverlayType, SeasonType, WeatherType } from '../core/Types';
import { Clock } from '../core/Clock';
import { EventBus, Events } from '../core/EventBus';
import { AccessibilityManager } from '../accessibility/AccessibilityManager';

export class TopBar {
  private container: HTMLElement;
  private clock: Clock;
  private accessibilityManager = AccessibilityManager.getInstance();
  private eventBus = EventBus.getInstance();

  private activeOverlay: OverlayType = OverlayType.NONE;
  private onOverlayChange?: (type: OverlayType) => void;
  private onHelpClick?: () => void;

  constructor(
    container: HTMLElement,
    clock: Clock,
    onOverlayChange?: (type: OverlayType) => void,
    onHelpClick?: () => void
  ) {
    this.container = container;
    this.clock = clock;
    this.onOverlayChange = onOverlayChange;
    this.onHelpClick = onHelpClick;

    this.render();
    this.bindEvents();
  }

  public render(): void {
    const time = this.clock.getTime();
    const weather = this.clock.getWeather();

    this.container.innerHTML = `
      <div class="topbar-inner">
        <!-- Left: Realm & Clock Info -->
        <div class="topbar-left">
          <div class="brand-group">
            <span class="realm-icon">🌐</span>
            <div>
              <span class="realm-title">Gaia • My Open World</span>
              <span class="seed-badge">Seed: #2026-GAIA</span>
            </div>
          </div>

          <div class="divider"></div>

          <!-- Live Clock Widget -->
          <div class="clock-widget" id="clock-widget">
            <div class="clock-row">
              <span class="season-badge" id="season-badge">${this.getSeasonIcon(time.season)} ${time.season}</span>
              <span class="date-text" id="date-text">${this.clock.formatDateString()}</span>
            </div>
            <div class="clock-row time-row">
              <span class="time-text" id="time-text">${this.clock.formatTimeString()}</span>
              <span class="phase-pill" id="phase-pill">${this.getPhaseEmoji(time.dayPhase)} ${time.dayPhase}</span>
            </div>
          </div>

          <!-- Dynamic Weather Selector -->
          <div class="weather-widget">
            <select class="select-dropdown" id="weather-select">
              <option value="${WeatherType.CLEAR}" ${weather === WeatherType.CLEAR ? 'selected' : ''}>☀️ Clear Skies</option>
              <option value="${WeatherType.RAIN}" ${weather === WeatherType.RAIN ? 'selected' : ''}>🌧️ Gentle Rain</option>
              <option value="${WeatherType.MIST}" ${weather === WeatherType.MIST ? 'selected' : ''}>🌫️ Morning Mist</option>
              <option value="${WeatherType.MANA_STORM}" ${weather === WeatherType.MANA_STORM ? 'selected' : ''}>⚡ Mana Storm</option>
            </select>
          </div>
        </div>

        <!-- Center: Time Controls -->
        <div class="topbar-center">
          <div class="time-controls-group">
            <button class="btn-time ${time.isPaused ? 'active' : ''}" id="btn-pause" title="Toggle Pause (Space)">
              ${time.isPaused ? '▶ Play' : '⏸ Pause'}
            </button>
            <button class="btn-time" id="btn-step" title="Step 1 Tick (T)">
              ⏯ Step
            </button>
            <div class="speed-buttons">
              <button class="btn-speed ${!time.isPaused && time.timeSpeed === 0.5 ? 'active' : ''}" data-speed="0.5">0.5x</button>
              <button class="btn-speed ${!time.isPaused && time.timeSpeed === 1.0 ? 'active' : ''}" data-speed="1.0">1x</button>
              <button class="btn-speed ${!time.isPaused && time.timeSpeed === 2.0 ? 'active' : ''}" data-speed="2.0">2x</button>
              <button class="btn-speed ${!time.isPaused && time.timeSpeed === 4.0 ? 'active' : ''}" data-speed="4.0">4x</button>
              <button class="btn-speed ${!time.isPaused && time.timeSpeed === 16.0 ? 'active' : ''}" data-speed="16.0">16x</button>
            </div>
          </div>
        </div>

        <!-- Right: Overlays, Accessibility & Help -->
        <div class="topbar-right">
          <!-- Overlays Selector -->
          <div class="overlay-selector-group">
            <span class="control-label">Overlays:</span>
            <div class="btn-group-segmented">
              <button class="btn-overlay ${this.activeOverlay === OverlayType.NONE ? 'active' : ''}" data-overlay="${OverlayType.NONE}">Normal</button>
              <button class="btn-overlay ${this.activeOverlay === OverlayType.TEMPERATURE ? 'active' : ''}" data-overlay="${OverlayType.TEMPERATURE}" title="Temperature (°C)">🌡️ Temp</button>
              <button class="btn-overlay ${this.activeOverlay === OverlayType.MOISTURE ? 'active' : ''}" data-overlay="${OverlayType.MOISTURE}" title="Moisture (%)">💧 Moisture</button>
              <button class="btn-overlay ${this.activeOverlay === OverlayType.POPULATION_DENSITY ? 'active' : ''}" data-overlay="${OverlayType.POPULATION_DENSITY}" title="Population Density">👥 Density</button>
              <button class="btn-overlay ${this.activeOverlay === OverlayType.MANA_FLUX ? 'active' : ''}" data-overlay="${OverlayType.MANA_FLUX}" title="Mana Resonance">✨ Mana</button>
              <button class="btn-overlay ${this.activeOverlay === OverlayType.ELEVATION ? 'active' : ''}" data-overlay="${OverlayType.ELEVATION}" title="Topography Elevation">🏔️ Relief</button>
            </div>
          </div>

          <div class="divider"></div>

          <!-- Colorblind Mode -->
          <div class="accessibility-group">
            <select class="select-dropdown" id="colorblind-select" title="Colorblind Accessibility Filter">
              <option value="${ColorblindMode.NORMAL}">👁️ Normal Vision</option>
              <option value="${ColorblindMode.DEUTERANOPIA}">Deuteranopia (Green)</option>
              <option value="${ColorblindMode.PROTANOPIA}">Protanopia (Red)</option>
              <option value="${ColorblindMode.TRITANOPIA}">Tritanopia (Blue)</option>
              <option value="${ColorblindMode.HIGH_CONTRAST}">High Contrast Mono</option>
            </select>
          </div>

          <!-- Entities / FPS Monitor -->
          <div class="stats-pill" id="stats-pill">
            <span id="entity-count-text">Entities: 16</span>
            <span id="fps-text">60 FPS</span>
          </div>

          <!-- Help Button -->
          <button class="btn-icon" id="btn-help-guide" title="Guide & Shortcuts (?)">❓</button>
        </div>
      </div>
    `;
  }

  public updateTimeDisplay(time: GameTime): void {
    const dateEl = document.getElementById('date-text');
    const timeEl = document.getElementById('time-text');
    const seasonEl = document.getElementById('season-badge');
    const phaseEl = document.getElementById('phase-pill');
    const pauseBtn = document.getElementById('btn-pause');

    if (dateEl) dateEl.innerText = this.clock.formatDateString();
    if (timeEl) timeEl.innerText = this.clock.formatTimeString();
    if (seasonEl) seasonEl.innerHTML = `${this.getSeasonIcon(time.season)} ${time.season}`;
    if (phaseEl) phaseEl.innerHTML = `${this.getPhaseEmoji(time.dayPhase)} ${time.dayPhase}`;

    if (pauseBtn) {
      pauseBtn.innerHTML = time.isPaused ? '▶ Play' : '⏸ Pause';
      if (time.isPaused) pauseBtn.classList.add('active');
      else pauseBtn.classList.remove('active');
    }
  }

  public updateStats(entityCount: number, fps: number, tps: number): void {
    const countEl = document.getElementById('entity-count-text');
    const fpsEl = document.getElementById('fps-text');
    if (countEl) countEl.innerText = `Entities: ${entityCount}`;
    if (fpsEl) fpsEl.innerText = `${fps} FPS • ${tps} TPS`;
  }

  private bindEvents(): void {
    // Pause / Resume
    document.getElementById('btn-pause')?.addEventListener('click', () => {
      this.clock.togglePause();
    });

    // Step 1 Tick
    document.getElementById('btn-step')?.addEventListener('click', () => {
      this.clock.manualStep();
    });

    // Speeds
    document.querySelectorAll('.btn-speed').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        const speed = parseFloat((e.currentTarget as HTMLElement).dataset.speed || '1.0');
        this.clock.setSpeed(speed);
        document.querySelectorAll('.btn-speed').forEach((b) => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
      });
    });

    // Weather Dropdown
    document.getElementById('weather-select')?.addEventListener('change', (e) => {
      const val = (e.target as HTMLSelectElement).value as WeatherType;
      this.clock.setWeather(val, 600);
    });

    // Overlays
    document.querySelectorAll('.btn-overlay').forEach((btn) => {
      btn.addEventListener('click', (e) => {
        const overlay = (e.currentTarget as HTMLElement).dataset.overlay as OverlayType;
        this.activeOverlay = overlay;
        document.querySelectorAll('.btn-overlay').forEach((b) => b.classList.remove('active'));
        (e.currentTarget as HTMLElement).classList.add('active');
        if (this.onOverlayChange) this.onOverlayChange(overlay);
      });
    });

    // Colorblind Select
    document.getElementById('colorblind-select')?.addEventListener('change', (e) => {
      const mode = (e.target as HTMLSelectElement).value as ColorblindMode;
      this.accessibilityManager.setMode(mode);
    });

    // Help Button
    document.getElementById('btn-help-guide')?.addEventListener('click', () => {
      if (this.onHelpClick) this.onHelpClick();
    });
  }

  private getSeasonIcon(season: SeasonType): string {
    switch (season) {
      case SeasonType.SPRING: return '🌸';
      case SeasonType.SUMMER: return '☀️';
      case SeasonType.AUTUMN: return '🍂';
      case SeasonType.WINTER: return '❄️';
    }
  }

  private getPhaseEmoji(phase: string): string {
    switch (phase) {
      case 'DAWN': return '🌅';
      case 'DAY': return '☀️';
      case 'DUSK': return '🌇';
      case 'NIGHT': return '🌙';
      default: return '☀️';
    }
  }
}
