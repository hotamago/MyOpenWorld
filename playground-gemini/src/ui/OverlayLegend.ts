// Floating Dynamic Overlay Legend with Units and Color Gradients

import { OverlayType } from '../core/Types';
import { OVERLAY_LEGENDS } from '../world/Overlays';
import { AccessibilityManager } from '../accessibility/AccessibilityManager';

export class OverlayLegend {
  private container: HTMLElement;
  private accessibilityManager = AccessibilityManager.getInstance();

  constructor(container: HTMLElement) {
    this.container = container;
  }

  public update(overlayType: OverlayType): void {
    const info = OVERLAY_LEGENDS[overlayType];

    if (!info || overlayType === OverlayType.NONE) {
      this.container.style.display = 'none';
      return;
    }

    this.container.style.display = 'flex';

    // Generate continuous gradient CSS based on active overlay
    let gradientCss = '';
    if (overlayType === OverlayType.TEMPERATURE) {
      const colors = [-15, 0, 15, 30, 45].map((t) => this.accessibilityManager.getTemperatureColor(t));
      gradientCss = `linear-gradient(to right, ${colors.join(', ')})`;
    } else if (overlayType === OverlayType.MOISTURE) {
      const colors = [0, 25, 50, 75, 100].map((m) => this.accessibilityManager.getMoistureColor(m));
      gradientCss = `linear-gradient(to right, ${colors.join(', ')})`;
    } else if (overlayType === OverlayType.POPULATION_DENSITY) {
      const colors = [0, 3, 7, 11, 15].map((d) => this.accessibilityManager.getPopulationDensityColor(d, 15));
      gradientCss = `linear-gradient(to right, ${colors.join(', ')})`;
    } else if (overlayType === OverlayType.MANA_FLUX) {
      const colors = [0, 250, 500, 750, 1000].map((m) => this.accessibilityManager.getManaFluxColor(m, 1000));
      gradientCss = `linear-gradient(to right, ${colors.join(', ')})`;
    } else if (overlayType === OverlayType.ELEVATION) {
      gradientCss = 'linear-gradient(to right, #1a365d, #2b6cb0, #ecc94b, #48bb78, #a0aec0, #edf2f7)';
    }

    this.container.innerHTML = `
      <div class="legend-header">
        <span class="legend-title">${info.title}</span>
        <span class="legend-unit badge">${info.unit}</span>
      </div>
      <div class="legend-gradient-bar" style="background: ${gradientCss};"></div>
      <div class="legend-ticks">
        ${info.ticks
          .map(
            (t) => `
          <div class="legend-tick">
            <span class="tick-label">${t.label}</span>
          </div>
        `
          )
          .join('')}
      </div>
      <div class="legend-desc">${info.description}</div>
    `;
  }
}
