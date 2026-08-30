// Help, Guide, and Keyboard Shortcut Modal

export class HelpModal {
  private container: HTMLElement;

  constructor(container: HTMLElement) {
    this.container = container;
  }

  public show(): void {
    this.container.innerHTML = `
      <div class="modal-backdrop" id="help-modal-backdrop">
        <div class="modal-card animate-scale-up">
          <div class="modal-header">
            <div class="modal-title-group">
              <span class="modal-icon">🌌</span>
              <h2 class="modal-title">My Open World — True God Guide</h2>
            </div>
            <button class="btn-close" id="btn-close-help">✕</button>
          </div>

          <div class="modal-body">
            <div class="help-section">
              <h3>🎮 Controls & Navigation</h3>
              <div class="shortcut-grid">
                <div class="shortcut-item"><kbd>W A S D</kbd> / <kbd>Arrows</kbd> <span>Pan camera</span></div>
                <div class="shortcut-item"><kbd>Left Drag</kbd> <span>Pan camera</span></div>
                <div class="shortcut-item"><kbd>Mouse Wheel</kbd> <span>Smooth zoom (in/out)</span></div>
                <div class="shortcut-item"><kbd>Space</kbd> <span>Pause / Resume simulation</span></div>
                <div class="shortcut-item"><kbd>T</kbd> <span>Step 1 simulation tick</span></div>
                <div class="shortcut-item"><kbd>1</kbd> <kbd>2</kbd> <kbd>3</kbd> <kbd>4</kbd> <kbd>5</kbd> <span>Speed (0.5x, 1x, 2x, 4x, 16x)</span></div>
                <div class="shortcut-item"><kbd>Click Entity</kbd> <span>Inspect live status & inventory</span></div>
                <div class="shortcut-item"><kbd>Click Tile</kbd> <span>Inspect biome, elevation, resources</span></div>
                <div class="shortcut-item"><kbd>Minimap Click</kbd> <span>Jump camera instantly</span></div>
              </div>
            </div>

            <div class="help-section">
              <h3>🧬 Simulation & Homeostasis Systems</h3>
              <p>Autonomous entities possess living vitality needs that evolve continuously:</p>
              <ul>
                <li><strong>Hunger:</strong> Rises with movement and labor. When hungry (&gt;60), entities find berry bushes or farms to forage.</li>
                <li><strong>Energy:</strong> Depletes over time. When exhausted (&lt;30), entities seek cottages, campfires, or soft grass to sleep.</li>
                <li><strong>Mood:</strong> Influenced by satiety, rest, social conversations, and cozy campfire warmth.</li>
                <li><strong>Mana:</strong> Ambient metaphysical energy flowing from shrines and channeled by magical beings.</li>
              </ul>
            </div>

            <div class="help-section">
              <h3>🌐 Overlays & Colorblind Accessibility</h3>
              <p>Toggle real-time geospatial overlays on the top toolbar: <strong>Thermal Gradient (°C)</strong>, <strong>Moisture (%)</strong>, <strong>Population Density</strong>, and <strong>Mana Resonance (nJ/m³)</strong>. High-contrast scientific palettes (Viridis, Cividis, Plasma) ensure accessibility for all users.</p>
            </div>
          </div>

          <div class="modal-footer">
            <button class="btn btn-primary" id="btn-ack-help">Understood, True God</button>
          </div>
        </div>
      </div>
    `;

    this.container.style.display = 'block';

    document.getElementById('btn-close-help')?.addEventListener('click', () => this.hide());
    document.getElementById('btn-ack-help')?.addEventListener('click', () => this.hide());
    document.getElementById('help-modal-backdrop')?.addEventListener('click', (e) => {
      if (e.target === document.getElementById('help-modal-backdrop')) {
        this.hide();
      }
    });
  }

  public hide(): void {
    this.container.style.display = 'none';
    this.container.innerHTML = '';
  }
}
