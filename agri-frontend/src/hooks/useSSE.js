import { useEffect, useRef, useState } from 'react';

export function useSSE(url, onMessage) {
  const cb = useRef(onMessage);
  cb.current = onMessage;
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    let es;
    let reconnectTimer;

    function connect() {
      es = new EventSource(url);

      es.onopen = () => setConnected(true);

      es.onmessage = (ev) => {
        try {
          const data = JSON.parse(ev.data);
          cb.current(data);
        } catch {}
      };

      es.onerror = () => {
        setConnected(false);
        es.close();
        reconnectTimer = setTimeout(connect, 3000);
      };
    }

    connect();

    return () => {
      clearTimeout(reconnectTimer);
      if (es) es.close();
    };
  }, [url]);

  return connected;
}
