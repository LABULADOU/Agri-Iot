type MessageHandler = (data: Record<string, unknown>) => void;

class SSEService {
  private eventSource: EventSource | null = null;
  private handlers: Set<MessageHandler> = new Set();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private url: string = '';

  connect(url: string = '/api/v1/events') {
    if (this.eventSource) return;

    this.url = url;
    this.eventSource = new EventSource(url);

    this.eventSource.onopen = () => {
      console.log('SSE connected');
      if (this.reconnectTimer) {
        clearTimeout(this.reconnectTimer);
        this.reconnectTimer = null;
      }
    };

    this.eventSource.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as Record<string, unknown>;
        this.handlers.forEach(handler => handler(data));
      } catch {
        // ignore parse errors for keep-alive pings
      }
    };

    this.eventSource.onerror = () => {
      console.log('SSE disconnected, reconnecting...');
      this.eventSource?.close();
      this.eventSource = null;
      this.scheduleReconnect();
    };
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect(this.url);
    }, 5000);
  }

  subscribe(handler: MessageHandler) {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  disconnect() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.eventSource?.close();
    this.eventSource = null;
  }
}

export const sseService = new SSEService();
export default sseService;
