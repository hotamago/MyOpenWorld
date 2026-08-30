// Map Overlays data calculations and legend definitions

import { OverlayType, TileData } from '../core/Types';
import { AccessibilityManager } from '../accessibility/AccessibilityManager';

export interface OverlayLegendInfo {
  type: OverlayType;
  title: string;
  unit: string;
  min: number;
  max: number;
  ticks: { value: number; label: string }[];
  description: string;
}

export const OVERLAY_LEGENDS: Record<OverlayType, OverlayLegendInfo | null> = {
  [OverlayType.NONE]: null,
  [OverlayType.TEMPERATURE]: {
    type: OverlayType.TEMPERATURE,
    title: 'Thermal Distribution',
    unit: '°C',
    min: -15,
    max: 45,
    ticks: [
      { value: -15, label: '-15°C (Glacial)' },
      { value: 0, label: '0°C (Freezing)' },
      { value: 15, label: '15°C (Temperate)' },
      { value: 30, label: '30°C (Warm)' },
      { value: 45, label: '45°C (Torrid)' },
    ],
    description: 'Ambient ground temperature mapped via colorblind-friendly thermal gradient.',
  },
  [OverlayType.MOISTURE]: {
    type: OverlayType.MOISTURE,
    title: 'Moisture & Humidity',
    unit: '%',
    min: 0,
    max: 100,
    ticks: [
      { value: 0, label: '0% (Arid)' },
      { value: 25, label: '25% (Dry)' },
      { value: 50, label: '50% (Moderate)' },
      { value: 75, label: '75% (Humid)' },
      { value: 100, label: '100% (Saturated)' },
    ],
    description: 'Atmospheric and soil water saturation influencing plant growth and rainfall.',
  },
  [OverlayType.POPULATION_DENSITY]: {
    type: OverlayType.POPULATION_DENSITY,
    title: 'Entity & Population Density',
    unit: 'entities/chunk',
    min: 0,
    max: 15,
    ticks: [
      { value: 0, label: '0 (Wilderness)' },
      { value: 3, label: '3 (Sparse)' },
      { value: 7, label: '7 (Settled)' },
      { value: 11, label: '11 (Dense)' },
      { value: 15, label: '15+ (Metropolis)' },
    ],
    description: 'Real-time spatial concentration of sentient and wild beings across the realm.',
  },
  [OverlayType.MANA_FLUX]: {
    type: OverlayType.MANA_FLUX,
    title: 'Metaphysical Mana Resonance',
    unit: 'nJ/m³',
    min: 0,
    max: 1000,
    ticks: [
      { value: 0, label: '0 nJ/m³ (Dormant)' },
      { value: 250, label: '250 (Faint)' },
      { value: 500, label: '500 (Attuned)' },
      { value: 750, label: '750 (Surging)' },
      { value: 1000, label: '1000 nJ/m³ (Nexus)' },
    ],
    description: 'Ambient arcane energy radiating from ley-lines, ancient shrines, and wisps.',
  },
  [OverlayType.ELEVATION]: {
    type: OverlayType.ELEVATION,
    title: 'Topographic Elevation',
    unit: 'm',
    min: -500,
    max: 2500,
    ticks: [
      { value: -500, label: '-500m (Abyss)' },
      { value: 0, label: '0m (Sea Level)' },
      { value: 600, label: '600m (Hills)' },
      { value: 1400, label: '1400m (Highlands)' },
      { value: 2500, label: '2500m (Summit)' },
    ],
    description: 'Physical relief and topography from ocean trenches to mountain peaks.',
  },
};

export class OverlayCalculator {
  private accessibilityManager = AccessibilityManager.getInstance();

  public getTileOverlayColor(
    tile: TileData,
    overlayType: OverlayType,
    densityValue: number = 0
  ): string | null {
    switch (overlayType) {
      case OverlayType.NONE:
        return null;

      case OverlayType.TEMPERATURE:
        return this.accessibilityManager.getTemperatureColor(tile.temperature);

      case OverlayType.MOISTURE:
        return this.accessibilityManager.getMoistureColor(tile.moisture);

      case OverlayType.POPULATION_DENSITY:
        return this.accessibilityManager.getPopulationDensityColor(densityValue, 12);

      case OverlayType.MANA_FLUX:
        return this.accessibilityManager.getManaFluxColor(tile.manaFlux, 1000);

      case OverlayType.ELEVATION: {
        // Map elevation (-1.0 to 1.0) -> meters (-500m to 2500m)
        const meters = tile.elevation * 1500 + 500;
        const norm = Math.max(0, Math.min(1, (meters + 500) / 3000));
        // Grayscale contour elevation with distinct shading
        const shade = Math.round(norm * 240 + 15);
        return `rgb(${shade}, ${shade}, ${shade})`;
      }

      default:
        return null;
    }
  }
}
