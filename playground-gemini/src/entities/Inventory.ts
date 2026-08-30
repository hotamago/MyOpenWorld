// Inventory System with weight capacity and stack management

import { Item, ItemCategory } from '../core/Types';

export class Inventory {
  public items: Item[] = [];
  public maxWeight: number = 25.0; // In kg
  public maxSlots: number = 8;

  constructor(maxWeight: number = 25.0, maxSlots: number = 8) {
    this.maxWeight = maxWeight;
    this.maxSlots = maxSlots;
  }

  public getTotalWeight(): number {
    return Math.round(
      this.items.reduce((total, item) => total + item.weight * item.quantity, 0) * 10
    ) / 10;
  }

  public isFull(): boolean {
    return this.items.length >= this.maxSlots || this.getTotalWeight() >= this.maxWeight;
  }

  public addItem(item: Item): boolean {
    // Check if item can stack with existing
    const existing = this.items.find((i) => i.id === item.id && i.quantity < i.maxStack);

    if (existing) {
      const space = existing.maxStack - existing.quantity;
      const addCount = Math.min(space, item.quantity);
      existing.quantity += addCount;
      item.quantity -= addCount;

      if (item.quantity <= 0) return true;
    }

    // Add new slot if space and weight permits
    if (this.items.length < this.maxSlots && this.getTotalWeight() + item.weight * item.quantity <= this.maxWeight) {
      this.items.push({ ...item });
      return true;
    }

    return false;
  }

  public removeItem(itemId: string, quantity: number = 1): Item | null {
    const idx = this.items.findIndex((i) => i.id === itemId);
    if (idx === -1) return null;

    const item = this.items[idx];
    const removeCount = Math.min(item.quantity, quantity);
    item.quantity -= removeCount;

    const removedItem: Item = { ...item, quantity: removeCount };

    if (item.quantity <= 0) {
      this.items.splice(idx, 1);
    }

    return removedItem;
  }

  public findFood(): Item | null {
    return this.items.find((i) => i.category === ItemCategory.FOOD && i.quantity > 0) || null;
  }

  public getItemAt(index: number): Item | null {
    return this.items[index] || null;
  }

  public clear(): void {
    this.items = [];
  }
}
