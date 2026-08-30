// Accessibility and Colorblind Mode Manager

import { ColorblindMode } from '../core/Types';
import { EventBus, Events } from '../core/EventBus';
import {
  applyColorblindSimulation,
  COOLWARM_STOPS,
  CIVIDIS_STOPS,
  PLASMA_STOPS,
  VIRIDIS_STOPS,
  MANA_STOPS,
  sampleMultiStopGradient,
  rgbToHex,
  RGB,
} from './ColorblindPalettes';

export class AccessibilityManager {
  private static instance: AccessibilityManager;
  private currentMode: ColorblindMode = ColorblindMode.NORMAL;
  private highContrastText: boolean = true;
  private eventBus = EventBus.getInstance();

  private constructor() {}

  public static getInstance(): AccessibilityManager {
    if (!AccessibilityManager.instance) {
      AccessibilityManager.instance = new AccessibilityManager();
    }
    return AccessibilityManager.instance;
  }

  public setMode(mode: ColorblindMode): void {
    if (this.currentMode !== mode) {
      this.currentMode = mode;
      this.applyDomFilter();
      this.eventBus.emit(Events.COLORBLIND_MODE_CHANGED, mode);
    }
  }

  public getMode(): ColorblindMode {
    return this.currentMode;
  }

  private applyDomFilter(): void {
    const root = document.documentElement;
    root.classList.remove('cb-deuteranopia', 'cb-protanopia', 'cb-tritanopia', 'cb-high-contrast');

    if (this.currentMode === ColorblindMode.DEUTERANOPIA) {
      root.classList.add('cb-deuteranopia');
    } else if (this.currentMode === ColorblindMode.PROTANOPIA) {
      root.classList.add('cb-protanopia');
    } else if (this.currentMode === ColorblindMode.TRITANOPIA) {
      root.classList.add('cb-tritanopia');
    } else if (this.currentMode === ColorblindMode.HIGH_CONTRAST) {
      root.classList.add('cb-high-contrast');
    }
  }

  // Get color for Temperature (-15°C to 45°C) -> Normalized [0, 1]
  public getTemperatureColor(celsius: number): string {
    const minT = -15;
    const maxT = 45;
    const norm = (celsius - minT) / (maxT - minT);
    const stops = this.currentMode === ColorblindMode.NORMAL ? COOLWARM_STOPS : VIRIDIS_STOPS;
    const rgb = sampleMultiStopGradient(stops, norm);
    return rgbToHex(applyColorblindSimulation(rgb, this.currentMode));
  }

  // Get color for Moisture (0% to 100%) -> Normalized [0, 1]
  public getMoistureColor(percent: number): string {
    const norm = percent / 100;
    const rgb = sampleMultiStopGradient(CIVIDIS_STOPS, norm);
    return rgbToHex(applyColorblindSimulation(rgb, this.currentMode));
  }

  // Get color for Population Density (0 to 20+ entities/chunk) -> Normalized [0, 1]
  public getPopulationDensityColor(density: number, maxDensity: number = 15): string {
    const norm = Math.min(1.0, density / maxDensity);
    const rgb = sampleMultiStopGradient(PLASMA_STOPS, norm);
    return rgbToHex(applyColorblindSimulation(rgb, this.currentMode));
  }

  // Get color for Mana Flux (0 to 1000 nJ/m³) -> Normalized [0, 1]
  public getManaFluxColor(mana: number, maxMana: number = 1000): string {
    const norm = Math.min(1.0, mana / maxMana);
    const rgb = sampleMultiStopGradient(MANA_STOPS, norm);
    return rgbToHex(applyColorblindSimulation(rgb, this.currentMode));
  }

  // Color transforms for tile rendering
  public transformRgb(rgb: RGB): RGB {
    return applyColorblindSimulation(rgb, this.currentMode);
  }
}
