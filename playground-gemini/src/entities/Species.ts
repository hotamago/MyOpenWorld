// Species Archetypes and Traits

import { SpeciesType } from '../core/Types';

export interface SpeciesConfig {
  type: SpeciesType;
  name: string;
  category: 'humanoid' | 'wildlife' | 'magical';
  baseSpeed: number; // Tiles per second
  spriteColor: string;
  accentColor: string;
  hairColors: string[];
  skinColors: string[];
  roles: string[];
  traits: string[];
  description: string;
}

export const SPECIES_CONFIGS: Record<SpeciesType, SpeciesConfig> = {
  [SpeciesType.HUMAN]: {
    type: SpeciesType.HUMAN,
    name: 'Human',
    category: 'humanoid',
    baseSpeed: 1.8,
    spriteColor: '#4299e1',
    accentColor: '#2b6cb0',
    skinColors: ['#fbd38d', '#f6ad55', '#dd6b20', '#c05621'],
    hairColors: ['#744210', '#1a202c', '#d69e2e', '#e2e8f0'],
    roles: ['Farmer', 'Lumberjack', 'Villager', 'Town Baker', 'Blacksmith'],
    traits: ['Adaptable', 'Industrious', 'Social', 'Curious'],
    description: 'Resourceful dwellers of Gaia building thriving agrarian communities.',
  },
  [SpeciesType.ELF]: {
    type: SpeciesType.ELF,
    name: 'Elf',
    category: 'humanoid',
    baseSpeed: 2.2,
    spriteColor: '#48bb78',
    accentColor: '#2f855a',
    skinColors: ['#feebc8', '#fefcbf', '#fbd38d'],
    hairColors: ['#faf089', '#9ae6b4', '#feb2b2', '#e9d8fd'],
    roles: ['Ranger', 'Forest Druid', 'Herbalist', 'Wind Scout'],
    traits: ['Nature-Bonded', 'Nimble', 'Mana-Attuned', 'Vegetarian'],
    description: 'Graceful beings deeply in tune with the rhythms of woodland and wind.',
  },
  [SpeciesType.DWARF]: {
    type: SpeciesType.DWARF,
    name: 'Dwarf',
    category: 'humanoid',
    baseSpeed: 1.5,
    spriteColor: '#ed8936',
    accentColor: '#c05621',
    skinColors: ['#fed7d7', '#fbd38d', '#e2e8f0'],
    hairColors: ['#9b2c2c', '#744210', '#a0aec0', '#1a202c'],
    roles: ['Miner', 'Deep Smith', 'Stonemason', 'Rune Scholar'],
    traits: ['Stout', 'Tough', 'Ore-Seeker', 'Ale-Lover'],
    description: 'Hardy mountain folk renowned for masterful metallurgy and stonework.',
  },
  [SpeciesType.DEER]: {
    type: SpeciesType.DEER,
    name: 'Forest Deer',
    category: 'wildlife',
    baseSpeed: 2.4,
    spriteColor: '#d69e2e',
    accentColor: '#975a16',
    skinColors: ['#c05621', '#d69e2e'],
    hairColors: ['#fff'],
    roles: ['Grazer', 'Stag'],
    traits: ['Timid', 'Gentle', 'Fleet-Footed'],
    description: 'Peaceful herbivore roaming the emerald glades in search of fresh shoots.',
  },
  [SpeciesType.WOLF]: {
    type: SpeciesType.WOLF,
    name: 'Wild Wolf',
    category: 'wildlife',
    baseSpeed: 2.5,
    spriteColor: '#718096',
    accentColor: '#4a5568',
    skinColors: ['#4a5568', '#2d3748', '#e2e8f0'],
    hairColors: ['#1a202c'],
    roles: ['Pack Hunter', 'Lone Roamer'],
    traits: ['Predatory', 'Alert', 'Pack-Oriented'],
    description: 'Cunning apex hunter maintaining ecological balance in the forests.',
  },
  [SpeciesType.WISP]: {
    type: SpeciesType.WISP,
    name: 'Luminous Wisp',
    category: 'magical',
    baseSpeed: 1.9,
    spriteColor: '#9f7aea',
    accentColor: '#00f5ff',
    skinColors: ['#e9d8fd', '#bee3f8'],
    hairColors: ['#fff'],
    roles: ['Ley-Sprout', 'Shrine Guardian'],
    traits: ['Ethereal', 'Mana-Radiant', 'Weightless'],
    description: 'Semi-corporeal spirit coalesced from raw concentrated ley-mana.',
  },
};
