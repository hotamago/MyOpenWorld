// Emergent Thought and Dialogue Generator

import { EntityState, SpeciesType, WeatherType } from '../core/Types';

export class DialogueGenerator {
  public static generateThought(
    species: SpeciesType,
    state: EntityState,
    hunger: number,
    energy: number,
    mood: number,
    weather: WeatherType
  ): string {
    if (hunger > 75) {
      const hungerThoughts = [
        'My stomach is growling loudly... I need food now.',
        'Must find ripe berries or freshly baked bread.',
        'Can barely concentrate on anything except a meal.',
        'Is that the aroma of roasted oats in the air?',
      ];
      return hungerThoughts[Math.floor(Math.random() * hungerThoughts.length)];
    }

    if (energy < 25) {
      const tiredThoughts = [
        'Yawn... my eyelids feel so heavy.',
        'Need to find a cozy bed or a warm campfire.',
        'Just a short nap under a shady oak would do wonders.',
        'Running on pure willpower at this point.',
      ];
      return tiredThoughts[Math.floor(Math.random() * tiredThoughts.length)];
    }

    if (weather === WeatherType.RAIN) {
      return Math.random() < 0.5
        ? 'The rain is refreshing, though my boots are soaked.'
        : 'The crops will thrive with this downpour.';
    }

    if (weather === WeatherType.MANA_STORM) {
      return 'The air crackles with pure arcane resonance!';
    }

    switch (state) {
      case EntityState.EATING:
        return 'Mmm, delicious! Vitality is returning.';
      case EntityState.SLEEPING:
        return 'Zzz... dreaming of endless sunlit meadows...';
      case EntityState.WORK_GATHER:
        return 'Honest work yields a bountiful harvest.';
      case EntityState.SOCIALIZE:
        return 'It is wonderful sharing tales with fellow folk.';
      case EntityState.MEDITATE:
        return 'Communing with the ancient ley-lines of Gaia.';
      case EntityState.FLEE:
        return 'Danger! Must get to safety!';
      case EntityState.IDLE_WANDER:
      default:
        const wanderThoughts = [
          'What a peaceful day in the realm.',
          'Admiring the gentle sway of the grass in the breeze.',
          'Wonder what lies beyond the northern peaks.',
          'The True God watches over us quietly.',
        ];
        return wanderThoughts[Math.floor(Math.random() * wanderThoughts.length)];
    }
  }

  public static generateSocialDialogue(speakerName: string, listenerName: string, role: string): string {
    const dialogues = [
      `"Good tidings, ${listenerName}! The morning air is crisp today."`,
      `"Have you seen the harvest yields on the eastern plots, ${listenerName}?"`,
      `"I was near the crystal shrine yesterday, ${listenerName}. The aura was breathtaking."`,
      `"Let us rest by the town campfire once dusk falls, ${listenerName}."`,
      `"${listenerName}, may the blessings of Gaia accompany your journey."`,
      `"The well water is particularly sweet and pure this season."`,
    ];
    return dialogues[Math.floor(Math.random() * dialogues.length)];
  }
}
