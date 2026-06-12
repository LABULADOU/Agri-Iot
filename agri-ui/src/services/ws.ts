type WsMessageHandler = (data: Record<string, unknown>) => void;

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (reason: Error) => void;
  timer: ReturnType<typeof setTimeout>;
}

interface Subscription {
  type: string;
  nodes: string[];
  handlers: Set<WsMessageHandler>;
}

class WsService {
  private ws: WebSocket | null = null;
  private url: string = '';
  private nextId = 0;
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private shouldReconnect = false;

  private subs: Map<string, Subscription> = new Map();
  private pending: Map<string, PendingRequest> = new Map();
  private pendingMessages: string[] = [];

  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  connect(url = '/api/v1/ws') {
    if (this.ws?.readyState === WebSocket.OPEN) return;
    if (this.ws?.readyState === WebSocket.CONNECTING) return;

    this.url = url;
    this.shouldReconnect = true;
    this.ws = new WebSocket(url);

    this.ws.onopen = () => this.onOpen();
    this.ws.onmessage = (event) => this.onMessage(event);
    this.ws.onclose = () => this.onClose();
    this.ws.onerror = () => { /* onclose will fire next */ };
  }

  disconnect() {
    this.shouldReconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
  }

  private onOpen() {
    this.reconnectAttempts = 0;

    for (const msg of this.pendingMessages) {
      this.ws?.send(msg);
    }
    this.pendingMessages = [];

    for (const [id, sub] of this.subs) {
      for (const _handler of sub.handlers) {
        this.ws?.send(JSON.stringify({ id, cmd: 'subscribe', type: sub.type, nodes: sub.nodes }));
        break;
      }
    }
  }

  private onMessage(event: MessageEvent) {
    let data: Record<string, unknown>;
    try {
      data = JSON.parse(event.data as string) as Record<string, unknown>;
    } catch {
      return;
    }

    const id = data.id as string | undefined;
    const cmd = data.cmd as string | undefined;

    if (id && this.pending.has(id)) {
      const p = this.pending.get(id)!;
      clearTimeout(p.timer);
      this.pending.delete(id);
      if (cmd === 'data') {
        p.resolve(data.data);
      } else if (cmd === 'error') {
        p.reject(new Error(data.error as string || 'query error'));
      }
      return;
    }

    if (cmd === 'push' && id) {
      const sub = this.subs.get(id);
      if (sub) {
        const payload = data.data as Record<string, unknown>;
        for (const handler of sub.handlers) {
          try { handler(payload); } catch { /* handler error */ }
        }
      }
    }
  }

  private onClose() {
    if (this.shouldReconnect) {
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect() {
    const delay = Math.min(5000 * Math.pow(2, this.reconnectAttempts), 30000);
    this.reconnectAttempts++;
      this.reconnectTimer = setTimeout(() => {
        this.reconnectTimer = null;
        this.connect(this.url || '/api/v1/ws');
      }, delay);
  }

  private send(msg: string) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(msg);
    } else {
      this.pendingMessages.push(msg);
    }
  }

  subscribe(type: string, nodes: string[], handler: WsMessageHandler): () => void {
    const id = 'sub_' + (this.nextId++);

    if (!this.subs.has(id)) {
      this.subs.set(id, { type, nodes, handlers: new Set() });
    }
    this.subs.get(id)!.handlers.add(handler);

    if (!this.connected) {
      if (!this.ws || this.ws.readyState === WebSocket.CLOSED) {
        this.connect(this.url || '/api/v1/ws');
      }
      return () => {
        const sub = this.subs.get(id);
        if (sub) {
          sub.handlers.delete(handler);
          if (sub.handlers.size === 0) {
            this.subs.delete(id);
            this.send(JSON.stringify({ id, cmd: 'unsubscribe' }));
          }
        }
      };
    }

    this.send(JSON.stringify({ id, cmd: 'subscribe', type, nodes }));

    return () => {
      const sub = this.subs.get(id);
      if (sub) {
        sub.handlers.delete(handler);
        if (sub.handlers.size === 0) {
          this.subs.delete(id);
          this.send(JSON.stringify({ id, cmd: 'unsubscribe' }));
        }
      }
    };
  }

  query(type: string, params: Record<string, unknown>): Promise<unknown> {
    const id = 'q_' + (this.nextId++);

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error('query timeout'));
      }, 15000);

      this.pending.set(id, { resolve, reject, timer });

      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({ id, cmd: 'query', type, ...params }));
      } else {
        this.pendingMessages.push(JSON.stringify({ id, cmd: 'query', type, ...params }));
        if (!this.ws || this.ws.readyState === WebSocket.CLOSED) {
          this.connect(this.url || '/api/v1/ws');
        }
      }
    });
  }
}

export const wsService = new WsService();
export default wsService;
