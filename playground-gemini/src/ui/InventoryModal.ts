// Inventory Item Card and Detail Modal

import { Item, ItemCategory, ItemRarity } from '../core/Types';
import { Entity } from '../entities/Entity';
import { EventBus, Events } from '../core/EventBus';

export class InventoryModal {
  private modalContainer: HTMLElement;
  private currentEntity: Entity | null = null;
  private currentItem: Item | null = null;
  private onConsumeCallback?: (item: Item) => void;
  private onDropCallback?: (item: Item) => void;

  constructor(modalContainer: HTMLElement) {
    this.modalContainer = modalContainer;
  }

  public showItemCard(
    item: Item,
    entity: Entity,
    onConsume?: (item: Item) => void,
    onDrop?: (item: Item) => void
  ): void {
    this.currentItem = item;
    this.currentEntity = entity;
    this.onConsumeCallback = onConsume;
    this.onDropCallback = onDrop;

    const rarityClass = `rarity-${item.rarity.toLowerCase()}`;

    let effectsHtml = '';
    if (item.effects) {
      const ef = item.effects;
      const list: string[] = [];
      if (ef.hunger) list.push(`<span class="effect-tag hunger">🍗 Hunger: ${ef.hunger > 0 ? '+' : ''}${ef.hunger}</span>`);
      if (ef.energy) list.push(`<span class="effect-tag energy">⚡ Energy: +${ef.energy}</span>`);
      if (ef.health) list.push(`<span class="effect-tag health">❤️ Health: +${ef.health}</span>`);
      if (ef.mood) list.push(`<span class="effect-tag mood">😊 Mood: +${ef.mood}</span>`);
      if (ef.mana) list.push(`<span class="effect-tag mana">✨ Mana: +${ef.mana}</span>`);
      effectsHtml = `<div class="item-effects">${list.join(' ')}</div>`;
    }

    this.modalContainer.innerHTML = `
      <div class="item-card-backdrop" id="item-card-backdrop">
        <div class="item-card ${rarityClass} animate-scale-up">
          <div class="item-card-header">
            <div class="item-card-icon">${item.icon}</div>
            <div class="item-card-title-group">
              <h3 class="item-card-name">${item.name}</h3>
              <div class="item-card-tags">
                <span class="badge ${rarityClass}">${item.rarity}</span>
                <span class="badge badge-neutral">${item.category}</span>
                <span class="badge badge-weight">⚖️ ${item.weight} kg</span>
                <span class="badge badge-gold">🪙 ${item.value}g</span>
              </div>
            </div>
            <button class="btn-close" id="btn-close-item-card">✕</button>
          </div>

          <div class="item-card-body">
            <p class="item-card-desc">${item.description}</p>
            ${effectsHtml}
            <div class="item-card-meta">
              <span>Stack: <strong>${item.quantity} / ${item.maxStack}</strong></span>
              <span>Total Weight: <strong>${(item.weight * item.quantity).toFixed(1)} kg</strong></span>
            </div>
          </div>

          <div class="item-card-actions">
            ${
              item.category === ItemCategory.FOOD || item.effects
                ? `<button class="btn btn-primary" id="btn-consume-item">🍴 Consume Item</button>`
                : ''
            }
            <button class="btn btn-secondary" id="btn-drop-item">📦 Drop 1x</button>
          </div>
        </div>
      </div>
    `;

    this.modalContainer.style.display = 'block';

    // Bind events
    document.getElementById('btn-close-item-card')?.addEventListener('click', () => this.hide());
    document.getElementById('item-card-backdrop')?.addEventListener('click', (e) => {
      if (e.target === document.getElementById('item-card-backdrop')) {
        this.hide();
      }
    });

    document.getElementById('btn-consume-item')?.addEventListener('click', () => {
      if (this.currentItem && this.onConsumeCallback) {
        this.onConsumeCallback(this.currentItem);
      }
      this.hide();
    });

    document.getElementById('btn-drop-item')?.addEventListener('click', () => {
      if (this.currentItem && this.onDropCallback) {
        this.onDropCallback(this.currentItem);
      }
      this.hide();
    });
  }

  public hide(): void {
    this.modalContainer.style.display = 'none';
    this.modalContainer.innerHTML = '';
    this.currentItem = null;
    this.currentEntity = null;
  }
}
