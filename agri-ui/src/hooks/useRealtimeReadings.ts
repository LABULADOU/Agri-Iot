import { useEffect, useRef, useState, useMemo, useCallback } from 'react';
import dayjs from 'dayjs';
import { nodeApi } from '../services/api';
import { wsService } from '../services/ws';
import type { SensorReading, AggregatedReading } from '../types';

interface UseRealtimeReadingsOptions {
  enabled: boolean;
  deviceId: string | null;
  nodeId: string | null;
  metrics: string[];
  dateRange: [dayjs.Dayjs, dayjs.Dayjs];
  maxBuffer?: number;
}

interface UseRealtimeReadingsResult {
  readings: AggregatedReading[];
  filteredReadings: SensorReading[];
  loading: boolean;
  lastUpdate: number;
  rawCount: number;
}

export function useRealtimeReadings({
  enabled,
  deviceId,
  nodeId,
  metrics,
  dateRange,
  maxBuffer = 30000,
}: UseRealtimeReadingsOptions): UseRealtimeReadingsResult {
  const [loading, setLoading] = useState(false);
  const [lastUpdate, setLastUpdate] = useState(0);
  const [tick, setTick] = useState(0);

  const bufferRef = useRef<SensorReading[]>([]);
  const seenKeysRef = useRef<Set<string>>(new Set());

  const bump = useCallback(() => setTick(t => t + 1), []);

  useEffect(() => {
    if (!enabled) {
      bufferRef.current = [];
      seenKeysRef.current.clear();
      return;
    }

    bufferRef.current = [];
    seenKeysRef.current.clear();
    bump();

    let cancelled = false;

    const fetchInitial = async () => {
      if (!deviceId) return;
      setLoading(true);
      try {
        const raw = await nodeApi.getReadings(deviceId, { limit: 5000 });
        if (cancelled) return;
        const mapped = raw.map(r => ({
          ...r,
          device_id: r.device_id,
          timestamp: typeof r.timestamp === 'number'
            ? (r.timestamp as number) * 1000
            : r.timestamp,
        }));
        bufferRef.current = mapped.reverse();
        mapped.forEach(r => seenKeysRef.current.add(`${r.metric}:${r.id}`));
        setLastUpdate(Date.now());
        bump();
      } catch (e) {
        if (!cancelled) {
          console.error('[useRealtimeReadings] fetch error:', e);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    fetchInitial();

    const wsNodes = nodeId ? [nodeId] : [];
    const unsub = wsService.subscribe('telemetry', wsNodes, (msg) => {
      const msgNodeId = (msg.node_id as string) || '';
      const readings = (msg.readings as Array<Record<string, string>> || [])
        .filter(r => r.metric && r.value != null)
        .map(r => ({
          id: Date.now() + Math.floor(Math.random() * 10000),
          device_id: msgNodeId || r.device_id || '',
          metric: r.metric as string,
          value: Number(r.value) || 0,
          unit: (r.unit as string) || '',
          timestamp: r.timestamp as string || new Date().toISOString(),
        }));

      if (!readings.length) return;

      const prev = bufferRef.current;
      const next = [
        ...prev,
        ...readings.filter(r => {
          const key = `${r.metric}:${r.id}`;
          if (seenKeysRef.current.has(key)) return false;
          seenKeysRef.current.add(key);
          return true;
        }),
      ];

      if (next.length > maxBuffer) {
        const removed = next.slice(0, next.length - maxBuffer);
        removed.forEach(r => seenKeysRef.current.delete(`${r.metric}:${r.id}`));
        bufferRef.current = next.slice(-maxBuffer);
      } else {
        bufferRef.current = next;
      }
      setLastUpdate(Date.now());
      bump();
    });

    return () => {
      cancelled = true;
      unsub();
    };
  }, [enabled, deviceId, nodeId, maxBuffer, bump]);

  const filteredReadings = useMemo(() => {
    if (!enabled) return [];
    const start = dateRange[0].valueOf();
    const end = dayjs().valueOf();
    return bufferRef.current
      .filter(r => {
        const ts = dayjs(r.timestamp).valueOf();
        return metrics.includes(r.metric) && ts >= start && ts <= end;
      })
      .sort((a, b) => dayjs(a.timestamp).valueOf() - dayjs(b.timestamp).valueOf());
  }, [enabled, tick, metrics, dateRange]);

  const readings = useMemo(() => {
    return filteredReadings.map(r => ({
      timestamp: r.timestamp,
      metric: r.metric,
      max: r.value,
      min: r.value,
      avg: r.value,
      count: 1,
    }));
  }, [filteredReadings]);

  return {
    readings,
    filteredReadings,
    loading,
    lastUpdate,
    rawCount: bufferRef.current.length,
  };
}
