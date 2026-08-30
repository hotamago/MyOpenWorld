// Homeostasis Needs and Dynamic Vitality System

import { Homeostasis } from '../core/Types';

export class VitalityManager {
  public needs: Homeostasis;

  constructor(initial?: Partial<Homeostasis>) {
    this.needs = {
      health: initial?.health ?? 100,
      maxHealth: initial?.maxHealth ?? 100,
      hunger: initial?.hunger ?? 20,
      maxHunger: initial?.maxHunger ?? 100,
      energy: initial?.energy ?? 90,
      maxEnergy: initial?.maxEnergy ?? 100,
      mood: initial?.mood ?? 80,
      maxMood: initial?.maxMood ?? 100,
      mana: initial?.mana ?? 50,
      maxMana: initial?.maxMana ?? 100,
    };
  }

  public tick(isSleeping: boolean, isWorking: boolean, isMoving: boolean, nearCampfire: boolean): void {
    if (isSleeping) {
      // Sleep restores energy and slowly heals
      this.needs.energy = Math.min(this.needs.maxEnergy, this.needs.energy + 2.0);
      this.needs.health = Math.min(this.needs.maxHealth, this.needs.health + 0.3);
      this.needs.hunger = Math.min(this.needs.maxHunger, this.needs.hunger + 0.1); // Low hunger burn while resting
      this.needs.mood = Math.min(this.needs.maxMood, this.needs.mood + 0.2);
    } else {
      // Normal metabolic decay
      const activityMult = isWorking ? 1.8 : isMoving ? 1.3 : 1.0;

      this.needs.hunger = Math.min(this.needs.maxHunger, this.needs.hunger + 0.12 * activityMult);
      this.needs.energy = Math.max(0, this.needs.energy - 0.08 * activityMult);

      // Starvation damage
      if (this.needs.hunger >= 90) {
        this.needs.health = Math.max(10, this.needs.health - 0.4);
        this.needs.mood = Math.max(5, this.needs.mood - 0.6);
      }

      // Exhaustion penalty
      if (this.needs.energy <= 15) {
        this.needs.mood = Math.max(10, this.needs.mood - 0.3);
      }

      // Campfire warmth buff
      if (nearCampfire) {
        this.needs.mood = Math.min(this.needs.maxMood, this.needs.mood + 0.15);
      }

      // Well-fed bonus
      if (this.needs.hunger < 25 && this.needs.energy > 60) {
        this.needs.mood = Math.min(this.needs.maxMood, this.needs.mood + 0.1);
        this.needs.health = Math.min(this.needs.maxHealth, this.needs.health + 0.1);
      }
    }

    // Natural mana regeneration
    this.needs.mana = Math.min(this.needs.maxMana, this.needs.mana + 0.1);
  }

  public consumeFood(hungerRelief: number, energyBoost: number = 0, moodBoost: number = 0, healthBoost: number = 0): void {
    this.needs.hunger = Math.max(0, this.needs.hunger - hungerRelief);
    this.needs.energy = Math.min(this.needs.maxEnergy, this.needs.energy + energyBoost);
    this.needs.mood = Math.min(this.needs.maxMood, this.needs.mood + moodBoost);
    this.needs.health = Math.min(this.needs.maxHealth, this.needs.health + healthBoost);
  }

  public applyDivineBlessing(): void {
    this.needs.health = this.needs.maxHealth;
    this.needs.hunger = 0;
    this.needs.energy = this.needs.maxEnergy;
    this.needs.mood = this.needs.maxMood;
    this.needs.mana = this.needs.maxMana;
  }
}
