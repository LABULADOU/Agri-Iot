import { useEffect, useRef, useState, useMemo, useCallback } from 'react';
import dayjs from 'dayjs';
import { nodeApi, apiLong } from '../services/api';
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
  const metricsRef = useRef(metrics);
  const dateRangeRef = useRef(dateRange);
  metricsRef.current = metrics;
  dateRangeRef.current = dateRange;

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
        // 1. Fetch aggregated hourly data for the full date range (covers 24h+)
        //    Use apiLong (120s timeout) for aggregate queries which can be slow
        const currentMetrics = metricsRef.current;
        const startTs = Math.floor(dateRangeRef.current[0].valueOf() / 1000);
        const endTs = Math.floor(Date.now() / 1000);
        const aggPromises = currentMetrics.map(m =>
          apiLong.get<AggregatedReading[]>('/readings/aggregate', {
            params: {
              device_id: deviceId,
              metric: m,
              period: 'hour',
              start: startTs,
              end: endTs,
            },
          }).then(res => res.data)
          .catch(() => [] as AggregatedReading[])
        );
        const aggResults = await Promise.all(aggPromises);
        if (cancelled) return;

        const aggReadings: SensorReading[] = [];
        const aggKeySet = new Set<string>();
        for (const results of aggResults) {
          for (const a of results) {
            const ts = dayjs(a.timestamp).valueOf();
            const key = `agg:${a.metric}:${Math.floor(ts / 3600000)}`;
            if (aggKeySet.has(key)) continue;
            aggKeySet.add(key);
            aggReadings.push({
              id: Date.now() + Math.floor(Math.random() * 100000),
              device_id: deviceId,
              metric: a.metric,
              value: a.avg,
              unit: '',
              timestamp: ts,
            });
          }
        }

        // 2. Fetch raw readings for granular recent data
        const raw = await nodeApi.getReadings(deviceId, { limit: 5000 });
        if (cancelled) return;

        const rawReadings = raw.map(r => ({
          ...r,
          device_id: deviceId,
          timestamp: typeof r.timestamp === 'number'
            ? (r.timestamp as number) * 1000
            : r.timestamp,
        }));

        // 3. Merge: aggregate data first (coarse, covers full range),
        //    then raw data (granular, overwrites aggregates at same time)
        const merged = [...aggReadings, ...rawReadings.reverse()];
        bufferRef.current = merged;
        merged.forEach(r => seenKeysRef.current.add(`${r.metric}:${r.id}`));
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
