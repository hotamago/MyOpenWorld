// Chronicle and World Event Notification Feed

import { EventBus, Events } from '../core/EventBus';

export interface ChronicleMessage {
  id: string;
  text: string;
  type: 'world' | 'entity' | 'divine';
  time: string;
}

export class ChronicleLog {
  private container: HTMLElement;
  private messages: ChronicleMessage[] = [];
  private eventBus = EventBus.getInstance();

  constructor(container: HTMLElement) {
    this.container = container;

    this.eventBus.on(Events.CHRONICLE_LOG, (data: { text: string; type?: 'world' | 'entity' | 'divine' }) => {
      this.addMessage(data.text, data.type || 'world');
    });
  }

  public addMessage(text: string, type: 'world' | 'entity' | 'divine' = 'world'): void {
    const time = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    const msg: ChronicleMessage = {
      id: Math.random().toString(36).substring(2, 9),
      text,
      type,
      time,
    };

    this.messages.unshift(msg);
    if (this.messages.length > 6) {
      this.messages.pop();
    }

    this.render();
  }

  private render(): void {
    this.container.innerHTML = `
      <div class="chronicle-feed">
        ${this.messages
          .map((m) => {
            const icon = m.type === 'divine' ? '✨' : m.type === 'entity' ? '👤' : '📜';
            const badgeClass = m.type === 'divine' ? 'badge-divine' : m.type === 'entity' ? 'badge-entity' : 'badge-world';
            return `
            <div class="chronicle-entry animate-fade-in ${badgeClass}">
              <span class="chronicle-icon">${icon}</span>
              <span class="chronicle-time">${m.time}</span>
              <span class="chronicle-text">${m.text}</span>
            </div>
          `;
          })
          .join('')}
      </div>
    `;
  }
}
