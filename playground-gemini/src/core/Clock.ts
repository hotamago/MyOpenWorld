// Game Simulation Clock and Time Management

import { DayPhase, GameTime, SeasonType, WeatherType } from './Types';
import { EventBus, Events } from './EventBus';

export class Clock {
  private totalTicks: number = 0;
  private divineEra: number = 1;
  private year: number = 1024;
  private season: SeasonType = SeasonType.SPRING;
  private day: number = 12;
  private hour: number = 8;
  private minute: number = 30;
  private second: number = 0;

  private timeSpeed: number = 1.0; // Multiplier: 0, 0.5, 1, 2, 4, 16
  private isPaused: boolean = false;
  private prevSpeed: number = 1.0;

  private currentWeather: WeatherType = WeatherType.CLEAR;
  private weatherDuration: number = 300; // ticks until possible weather change

  private tickAccumulator: number = 0;
  private readonly TICKS_PER_SECOND = 20; // 20 simulation ticks per real second at 1x
  private readonly SECONDS_PER_MINUTE = 60;
  private readonly MINUTES_PER_HOUR = 60;
  private readonly HOURS_PER_DAY = 24;
  private readonly DAYS_PER_SEASON = 30;

  private eventBus = EventBus.getInstance();

  constructor() {
    this.updateDayPhase(false);
  }

  public update(deltaTimeSec: number): void {
    if (this.isPaused || this.timeSpeed <= 0) return;

    this.tickAccumulator += deltaTimeSec * this.timeSpeed * this.TICKS_PER_SECOND;

    while (this.tickAccumulator >= 1.0) {
      this.stepTick();
      this.tickAccumulator -= 1.0;
    }
  }

  public stepTick(): void {
    this.totalTicks++;

    // Progress in-game time: 1 tick = 3 game seconds (so 20 ticks = 1 in-game minute)
    this.second += 3;
    if (this.second >= this.SECONDS_PER_MINUTE) {
      this.second = 0;
      this.minute++;

      if (this.minute >= this.MINUTES_PER_HOUR) {
        this.minute = 0;
        this.hour++;

        this.updateDayPhase(true);

        if (this.hour >= this.HOURS_PER_DAY) {
          this.hour = 0;
          this.day++;

          if (this.day > this.DAYS_PER_SEASON) {
            this.day = 1;
            this.advanceSeason();
          }
        }
      }
    }

    // Weather change logic
    this.weatherDuration--;
    if (this.weatherDuration <= 0) {
      this.evaluateWeatherChange();
    }

    this.eventBus.emit(Events.TIME_TICK, this.getTime());
  }

  private updateDayPhase(notify: boolean = true): void {
    const oldPhase = this.getDayPhase();
    let newPhase: DayPhase;

    if (this.hour >= 5 && this.hour < 8) {
      newPhase = DayPhase.DAWN;
    } else if (this.hour >= 8 && this.hour < 17) {
      newPhase = DayPhase.DAY;
    } else if (this.hour >= 17 && this.hour < 20) {
      newPhase = DayPhase.DUSK;
    } else {
      newPhase = DayPhase.NIGHT;
    }

    if (notify && oldPhase !== newPhase) {
      this.eventBus.emit(Events.DAY_PHASE_CHANGED, newPhase);
    }
  }

  private advanceSeason(): void {
    const seasons = [SeasonType.SPRING, SeasonType.SUMMER, SeasonType.AUTUMN, SeasonType.WINTER];
    const currentIndex = seasons.indexOf(this.season);
    const nextIndex = (currentIndex + 1) % seasons.length;
    this.season = seasons[nextIndex];

    if (this.season === SeasonType.SPRING) {
      this.year++;
    }

    this.eventBus.emit(Events.SEASON_CHANGED, this.season);
    this.eventBus.emit(Events.CHRONICLE_LOG, {
      text: `The wheel of time turns. Season of ${this.season} has arrived in Year ${this.year}.`,
      type: 'world',
    });
  }

  private evaluateWeatherChange(): void {
    // Determine possible weathers based on season
    const roll = Math.random();
    let newWeather = WeatherType.CLEAR;

    if (this.season === SeasonType.SPRING) {
      if (roll < 0.45) newWeather = WeatherType.CLEAR;
      else if (roll < 0.8) newWeather = WeatherType.RAIN;
      else if (roll < 0.95) newWeather = WeatherType.MIST;
      else newWeather = WeatherType.MANA_STORM;
    } else if (this.season === SeasonType.SUMMER) {
      if (roll < 0.6) newWeather = WeatherType.CLEAR;
      else if (roll < 0.8) newWeather = WeatherType.MANA_STORM;
      else if (roll < 0.95) newWeather = WeatherType.RAIN;
      else newWeather = WeatherType.MIST;
    } else if (this.season === SeasonType.AUTUMN) {
      if (roll < 0.4) newWeather = WeatherType.MIST;
      else if (roll < 0.75) newWeather = WeatherType.RAIN;
      else newWeather = WeatherType.CLEAR;
    } else {
      // Winter
      if (roll < 0.5) newWeather = WeatherType.MIST;
      else if (roll < 0.8) newWeather = WeatherType.CLEAR;
      else newWeather = WeatherType.MANA_STORM;
    }

    this.setWeather(newWeather, 200 + Math.floor(Math.random() * 400));
  }

  public setWeather(weather: WeatherType, duration: number = 300): void {
    if (this.currentWeather !== weather) {
      this.currentWeather = weather;
      this.weatherDuration = duration;
      this.eventBus.emit(Events.WEATHER_CHANGED, weather);
      this.eventBus.emit(Events.CHRONICLE_LOG, {
        text: `Atmospheric shift: Weather changed to ${weather}.`,
        type: 'world',
      });
    }
  }

  public togglePause(): boolean {
    this.isPaused = !this.isPaused;
    this.eventBus.emit(Events.TIME_SPEED_CHANGED, {
      speed: this.isPaused ? 0 : this.timeSpeed,
      isPaused: this.isPaused,
    });
    return this.isPaused;
  }

  public setPaused(paused: boolean): void {
    this.isPaused = paused;
    this.eventBus.emit(Events.TIME_SPEED_CHANGED, {
      speed: this.isPaused ? 0 : this.timeSpeed,
      isPaused: this.isPaused,
    });
  }

  public setSpeed(speed: number): void {
    if (speed === 0) {
      this.setPaused(true);
      return;
    }
    this.isPaused = false;
    this.timeSpeed = speed;
    this.eventBus.emit(Events.TIME_SPEED_CHANGED, {
      speed: this.timeSpeed,
      isPaused: false,
    });
  }

  public manualStep(): void {
    this.stepTick();
  }

  public getDayPhase(): DayPhase {
    if (this.hour >= 5 && this.hour < 8) return DayPhase.DAWN;
    if (this.hour >= 8 && this.hour < 17) return DayPhase.DAY;
    if (this.hour >= 17 && this.hour < 20) return DayPhase.DUSK;
    return DayPhase.NIGHT;
  }

  // Fractional time of day 0.0 to 1.0 (0 = midnight, 0.5 = noon)
  public getDayFraction(): number {
    return (this.hour * 3600 + this.minute * 60 + this.second) / (24 * 3600);
  }

  public getTime(): GameTime {
    return {
      totalTicks: this.totalTicks,
      divineEra: this.divineEra,
      year: this.year,
      season: this.season,
      day: this.day,
      hour: this.hour,
      minute: this.minute,
      second: this.second,
      dayPhase: this.getDayPhase(),
      timeSpeed: this.timeSpeed,
      isPaused: this.isPaused,
    };
  }

  public getWeather(): WeatherType {
    return this.currentWeather;
  }

  public getSeason(): SeasonType {
    return this.season;
  }

  public formatTimeString(): string {
    const hh = this.hour.toString().padStart(2, '0');
    const mm = this.minute.toString().padStart(2, '0');
    return `${hh}:${mm}`;
  }

  public formatDateString(): string {
    return `Era ${this.divineEra}, Yr ${this.year} • ${this.season} Day ${this.day}`;
  }
}
