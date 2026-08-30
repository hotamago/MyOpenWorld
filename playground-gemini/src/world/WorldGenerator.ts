// Procedural World Generation with Geological, Hydrological, and Settlement Systems

import { BiomeType, ResourceNode, TileData } from '../core/Types';
import { BIOME_CONFIGS } from './Biomes';
import { SimplexNoise } from '../core/Noise';
import { WorldMap } from './WorldMap';

export class WorldGenerator {
  public static generateWorld(width: number = 72, height: number = 72, seed: number = 2026): WorldMap {
    const worldMap = new WorldMap(width, height);
    const noise = new SimplexNoise(seed);
    const tempNoise = new SimplexNoise(seed + 101);
    const moistureNoise = new SimplexNoise(seed + 202);
    const manaNoise = new SimplexNoise(seed + 303);

    const centerX = width / 2;
    const centerY = height / 2;
    const maxDist = Math.hypot(centerX, centerY);

    // 1. First Pass: Base Heights, Temperature, Moisture, Mana
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        // Radial falloff so edges are oceans/bays
        const distFromCenter = Math.hypot(x - centerX, y - centerY) / maxDist;
        const radialFalloff = Math.pow(distFromCenter, 1.8) * 1.3;

        // Multi-octave elevation
        const nx = x / 24.0;
        const ny = y / 24.0;
        let elevation = noise.fbm(nx, ny, 4, 2.0, 0.5) - radialFalloff + 0.25;

        // Northern mountain elevation bias
        const northBias = (1.0 - y / height) * 0.45;
        elevation += northBias;

        // Temperature (warmer south, colder north peaks)
        const latTemp = (y / height) * 20 + 8; // 8°C north to 28°C south
        const elevationCooling = Math.max(0, elevation) * 25; // High altitude is freezing
        const localTempNoise = tempNoise.fbm(x / 18.0, y / 18.0, 3) * 8;
        const temperature = Math.round((latTemp - elevationCooling + localTempNoise) * 10) / 10;

        // Moisture / Humidity
        const localMoistNoise = moistureNoise.fbm(x / 20.0, y / 20.0, 3);
        let moisture = Math.round(Math.max(5, Math.min(100, (localMoistNoise + 0.5) * 60 + 20)));

        // Mana flux
        const rawMana = manaNoise.warpedFbm(x / 15.0, y / 15.0, 3);
        let manaFlux = Math.max(0, Math.min(1000, Math.round((rawMana + 0.6) * 450)));

        // East Mana Grove hotspot
        const eastDist = Math.hypot(x - (width * 0.78), y - (height * 0.45));
        if (eastDist < 14) {
          manaFlux = Math.min(1000, manaFlux + Math.round((14 - eastDist) * 45));
        }

        // Determine Biome
        let biome: BiomeType;

        if (elevation < -0.3) {
          biome = BiomeType.DEEP_OCEAN;
          moisture = 95;
        } else if (elevation < 0.0) {
          biome = BiomeType.COASTAL_WATER;
          moisture = 85;
        } else if (elevation < 0.08) {
          biome = BiomeType.SAND_BEACH;
        } else if (elevation > 0.68) {
          biome = BiomeType.SNOW_PEAK;
        } else if (elevation > 0.42) {
          biome = BiomeType.ROCKY_HILLS;
        } else if (eastDist < 10 && manaFlux > 600) {
          biome = BiomeType.MYSTIC_GROVE;
        } else if (temperature > 32 && moisture < 25) {
          biome = BiomeType.DESERT_DUNES;
        } else if (moisture > 65) {
          biome = BiomeType.DENSE_FOREST;
        } else {
          biome = BiomeType.LUSH_MEADOW;
        }

        const config = BIOME_CONFIGS[biome];
        const tile: TileData = {
          x,
          y,
          elevation: Math.round(elevation * 100) / 100,
          temperature,
          moisture,
          manaFlux,
          biome,
          baseColor: config.baseColor,
          walkable: config.walkable,
          walkSpeedMod: config.walkSpeedMod,
          variant: Math.floor(Math.random() * 4),
        };

        worldMap.setTile(x, y, tile);
      }
    }

    // 2. Hydrology: Carve a Meandering River from North Mountain to South Ocean
    this.carveRiver(worldMap, width, height);

    // 3. Construct Central Settlement, Roads, Cottages, Farmland, and POIs
    this.buildSettlementAndPOIs(worldMap, width, height);

    // 4. Populate Natural Resource Nodes (Berry Bushes, Trees, Ores, Shrines)
    this.populateResources(worldMap, width, height);

    return worldMap;
  }

  private static carveRiver(worldMap: WorldMap, width: number, height: number): void {
    let rx = Math.floor(width * 0.42);
    let ry = Math.floor(height * 0.12);

    while (ry < height - 4 && rx > 2 && rx < width - 2) {
      // Carve 2-tile wide river
      for (let ox = -1; ox <= 0; ox++) {
        const tile = worldMap.getTile(rx + ox, ry);
        if (tile && tile.elevation > -0.1) {
          tile.biome = BiomeType.COASTAL_WATER;
          tile.baseColor = BIOME_CONFIGS[BiomeType.COASTAL_WATER].baseColor;
          tile.walkable = false;
          tile.elevation = -0.15;
          tile.moisture = 90;
        }
      }

      // Meander south with slight sinuous curve
      ry++;
      if (Math.random() < 0.4) {
        rx += Math.sin(ry * 0.3) > 0 ? 1 : -1;
      }
    }
  }

  private static buildSettlementAndPOIs(worldMap: WorldMap, width: number, height: number): void {
    const villageCenterX = Math.floor(width * 0.48);
    const villageCenterY = Math.floor(height * 0.52);

    // 1. Build Cobblestone Crossroads
    for (let x = villageCenterX - 8; x <= villageCenterX + 8; x++) {
      this.makeRoadTile(worldMap, x, villageCenterY);
    }
    for (let y = villageCenterY - 7; y <= villageCenterY + 7; y++) {
      this.makeRoadTile(worldMap, villageCenterX, y);
    }

    // River Bridge Crossing
    const bridgeX = Math.floor(width * 0.42);
    const bridgeY = villageCenterY;
    for (let ox = -1; ox <= 1; ox++) {
      const tile = worldMap.getTile(bridgeX + ox, bridgeY);
      if (tile) {
        tile.biome = BiomeType.SETTLEMENT;
        tile.baseColor = '#8c7853'; // Wooden planks bridge
        tile.walkable = true;
        tile.walkSpeedMod = 1.1;
        tile.decoration = 'wooden_bridge';
      }
    }

    // 2. Town Square Centerpiece (Well & Campfire)
    const centerTile = worldMap.getTile(villageCenterX, villageCenterY);
    if (centerTile) {
      centerTile.resource = {
        type: 'water_well',
        amount: 100,
        maxAmount: 100,
        regrowTime: 10,
        currentRegrow: 0,
        isOccupied: false,
      };
      centerTile.decoration = 'water_well';
    }

    // Village Gathering Campfire
    const fireTile = worldMap.getTile(villageCenterX + 2, villageCenterY + 1);
    if (fireTile) {
      fireTile.resource = {
        type: 'campfire',
        amount: 50,
        maxAmount: 50,
        regrowTime: 0,
        currentRegrow: 0,
        isOccupied: false,
      };
      fireTile.decoration = 'campfire';
    }

    // 3. Cozy Cottages with Beds
    const houseCoords = [
      { x: villageCenterX - 4, y: villageCenterY - 3, name: 'Weaver Cottage' },
      { x: villageCenterX + 4, y: villageCenterY - 3, name: 'Farmer Home' },
      { x: villageCenterX - 4, y: villageCenterY + 3, name: 'Lumberjack Lodge' },
      { x: villageCenterX + 4, y: villageCenterY + 4, name: 'Scholar Rest' },
    ];

    for (const h of houseCoords) {
      // 2x2 Cottage structure
      for (let dy = 0; dy < 2; dy++) {
        for (let dx = 0; dx < 2; dx++) {
          const tile = worldMap.getTile(h.x + dx, h.y + dy);
          if (tile) {
            tile.biome = BiomeType.SETTLEMENT;
            tile.walkable = (dx === 0 && dy === 0); // Doorway is walkable, walls block
            tile.baseColor = '#4a5568';
            if (dx === 0 && dy === 0) {
              tile.resource = {
                type: 'bed',
                amount: 1,
                maxAmount: 1,
                regrowTime: 0,
                currentRegrow: 0,
                isOccupied: false,
              };
              tile.decoration = 'cottage_door';
            } else {
              tile.resource = {
                type: 'house',
                amount: 1,
                maxAmount: 1,
                regrowTime: 0,
                currentRegrow: 0,
                isOccupied: false,
              };
              tile.decoration = 'cottage_roof';
            }
          }
        }
      }
    }

    // 4. Farmland Fields with Wheat & Veggies (North-East of Town)
    const farmStartX = villageCenterX + 4;
    const farmStartY = villageCenterY - 7;
    for (let fy = 0; fy < 4; fy++) {
      for (let fx = 0; fx < 5; fx++) {
        const tile = worldMap.getTile(farmStartX + fx, farmStartY + fy);
        if (tile && tile.walkable) {
          tile.biome = BiomeType.FARMLAND;
          tile.baseColor = BIOME_CONFIGS[BiomeType.FARMLAND].baseColor;
          tile.resource = {
            type: 'wheat_crop',
            amount: 3,
            maxAmount: 3,
            growthStage: 2,
            regrowTime: 40,
            currentRegrow: 0,
            isOccupied: false,
          };
          tile.decoration = 'wheat_stalk';
        }
      }
    }

    // 5. Eastern Ancient Mana Crystal Shrine
    const shrineX = Math.floor(width * 0.78);
    const shrineY = Math.floor(height * 0.45);
    const shrineTile = worldMap.getTile(shrineX, shrineY);
    if (shrineTile) {
      shrineTile.biome = BiomeType.MYSTIC_GROVE;
      shrineTile.resource = {
        type: 'mana_crystal',
        amount: 10,
        maxAmount: 10,
        regrowTime: 30,
        currentRegrow: 0,
        isOccupied: false,
      };
      shrineTile.decoration = 'mana_shrine';
      shrineTile.manaFlux = 950;
    }
  }

  private static makeRoadTile(worldMap: WorldMap, x: number, y: number): void {
    const tile = worldMap.getTile(x, y);
    if (tile && tile.biome !== BiomeType.COASTAL_WATER && tile.biome !== BiomeType.DEEP_OCEAN) {
      tile.biome = BiomeType.SETTLEMENT;
      tile.baseColor = '#718096';
      tile.walkable = true;
      tile.walkSpeedMod = 1.3;
      tile.decoration = 'cobble_stones';
    }
  }

  private static populateResources(worldMap: WorldMap, width: number, height: number): void {
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const tile = worldMap.getTile(x, y);
        if (!tile || tile.resource || !tile.walkable) continue;

        const roll = Math.random();

        if (tile.biome === BiomeType.LUSH_MEADOW) {
          if (roll < 0.08) {
            // Wild Berry Bush
            tile.resource = {
              type: 'berry_bush',
              amount: 3,
              maxAmount: 3,
              growthStage: 2,
              regrowTime: 50,
              currentRegrow: 0,
              isOccupied: false,
            };
            tile.decoration = 'berry_bush';
          } else if (roll < 0.14) {
            tile.decoration = 'flower_' + (['red', 'blue', 'yellow'][Math.floor(Math.random() * 3)]);
          }
        } else if (tile.biome === BiomeType.DENSE_FOREST) {
          if (roll < 0.25) {
            tile.resource = {
              type: 'tree',
              amount: 5,
              maxAmount: 5,
              regrowTime: 80,
              currentRegrow: 0,
              isOccupied: false,
            };
            tile.decoration = Math.random() < 0.5 ? 'tree_oak' : 'tree_pine';
          } else if (roll < 0.35) {
            // Forest berries
            tile.resource = {
              type: 'berry_bush',
              amount: 2,
              maxAmount: 2,
              growthStage: 2,
              regrowTime: 45,
              currentRegrow: 0,
              isOccupied: false,
            };
            tile.decoration = 'berry_bush';
          }
        } else if (tile.biome === BiomeType.ROCKY_HILLS) {
          if (roll < 0.12) {
            tile.resource = {
              type: 'iron_ore',
              amount: 4,
              maxAmount: 4,
              regrowTime: 120,
              currentRegrow: 0,
              isOccupied: false,
            };
            tile.decoration = 'ore_vein';
          }
        } else if (tile.biome === BiomeType.MYSTIC_GROVE) {
          if (roll < 0.15) {
            tile.resource = {
              type: 'mana_crystal',
              amount: 3,
              maxAmount: 3,
              regrowTime: 60,
              currentRegrow: 0,
              isOccupied: false,
            };
            tile.decoration = 'mana_crystal';
          }
        }
      }
    }
  }
}
