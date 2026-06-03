import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Layout, Space, Typography, Badge, Spin, Select } from 'antd';
import { WarningOutlined, CloudOutlined } from '@ant-design/icons';
import { useRealtimeStore } from '../../stores/realtimeStore';
import { weatherApi } from '../../services/api';
import type { WeatherData, WeatherForecastDay, WeatherWarning, MinutelyForecast, CityLocation } from '../../types';
import styles from './TopBar.module.css';

const { Text } = Typography;
const { Header: AntHeader } = Layout;

const STORAGE_KEY = 'weather_location_id';
const DEFAULT_LOCATION = '101010100';

function getSavedLocation(): string {
  try { return localStorage.getItem(STORAGE_KEY) || DEFAULT_LOCATION; }
  catch { return DEFAULT_LOCATION; }
}

function saveLocation(id: string) {
  try { localStorage.setItem(STORAGE_KEY, id); } catch { /* noop */ }
}

function normalizeNow(raw: Record<string, string>): WeatherData {
  return {
    temp: Number(raw.temp) || 0,
    feelsLike: Number(raw.feelsLike) || 0,
    text: raw.text || '--',
    icon: raw.icon || '999',
    humidity: Number(raw.humidity) || 0,
    windDir: raw.windDir || '--',
    windScale: raw.windScale || '0',
    windSpeed: Number(raw.windSpeed) || 0,
    precip: Number(raw.precip) || 0,
    updateTime: raw.obsTime || '',
  };
}

function normalizeDaily(raw: Record<string, string>): WeatherForecastDay {
  return {
    date: raw.fxDate || '',
    tempMax: Number(raw.tempMax) || 0,
    tempMin: Number(raw.tempMin) || 0,
    textDay: raw.textDay || '--',
    iconDay: raw.iconDay || '999',
    windDirDay: raw.windDirDay || '--',
    windScaleDay: raw.windScaleDay || '0',
  };
}

function normalizeWarning(raw: Record<string, string>): WeatherWarning {
  return {
    title: raw.title || '',
    level: raw.level || '',
    type: raw.type || '',
    pubTime: raw.pubTime || '',
  };
}

const iconMap: Record<string, string> = {
  '100': '☀️', '101': '🌤️', '102': '⛅', '103': '🌥️',
  '104': '☁️', '150': '🌙', '151': '🌤️', '152': '🌥️', '153': '☁️',
  '300': '🌦️', '301': '🌧️', '302': '🌧️', '303': '🌧️',
  '305': '🌧️', '306': '🌧️', '307': '🌧️', '308': '🌧️', '309': '🌧️',
  '310': '🌧️', '311': '🌧️', '312': '🌧️', '313': '🌧️', '314': '🌧️', '315': '🌧️', '316': '🌧️', '317': '🌧️', '318': '🌧️',
  '399': '🌧️',
  '400': '❄️', '401': '❄️', '402': '❄️', '403': '❄️', '404': '❄️', '405': '❄️', '406': '❄️', '407': '❄️', '408': '❄️', '409': '❄️', '410': '❄️',
  '499': '❄️',
  '500': '🌫️', '501': '🌫️', '502': '🌫️', '503': '🌫️', '504': '🌫️', '507': '🌫️', '508': '🌫️',
  '509': '🌫️', '510': '🌫️', '511': '🌫️', '512': '🌫️', '513': '🌫️', '514': '🌫️', '515': '🌫️',
  '999': '🌤️',
};

function weatherIcon(icon: string, text: string): string {
  return iconMap[icon] || iconMap['999'];
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr);
  const week = ['日', '一', '二', '三', '四', '五', '六'];
  return `${d.getMonth() + 1}/${d.getDate()}(${week[d.getDay()]})`;
}

const levelColors: Record<string, string> = {
  '白色': '#999', '蓝色': '#3b82f6', '黄色': '#eab308', '橙色': '#f97316', '红色': '#ef4444',
};

const TopBar: React.FC = () => {
  const { connected, lastUpdate } = useRealtimeStore();
  const [location, setLocation] = useState(getSavedLocation);
  const [now, setNow] = useState<WeatherData | null>(null);
  const [forecast, setForecast] = useState<WeatherForecastDay[]>([]);
  const [minutely, setMinutely] = useState<MinutelyForecast | null>(null);
  const [warnings, setWarnings] = useState<WeatherWarning[]>([]);
  const [loading, setLoading] = useState(true);
  const [weatherLastRefresh, setWeatherLastRefresh] = useState<string | null>(null);

  const [searchResults, setSearchResults] = useState<CityLocation[]>([]);
  const [searching, setSearching] = useState(false);
  const searchTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const [cityLabel, setCityLabel] = useState('');

  const fetchWeather = useCallback(async (loc: string) => {
    setLoading(true);
    try {
      const [nowRaw, f3d, minRaw, warnRaw] = await Promise.all([
        weatherApi.getNow(loc),
        weatherApi.getForecast3d(loc),
        weatherApi.getMinutely(loc),
        weatherApi.getWarning(loc),
      ]);
      if (nowRaw?.now) setNow(normalizeNow(nowRaw.now));
      if (f3d?.daily) setForecast(f3d.daily.map(normalizeDaily));
      if (minRaw) setMinutely({ summary: minRaw.summary, hourly: minRaw.hourly || [] });
      if (warnRaw?.warning) setWarnings(warnRaw.warning.map(normalizeWarning));
      else setWarnings([]);
      setWeatherLastRefresh(new Date().toLocaleTimeString('zh-CN'));
    } catch {
      // keep last state
    } finally {
      setLoading(false);
    }
  }, []);

  const lookupCity = useCallback(async (loc: string) => {
    try {
      const res = await weatherApi.geoLookup(loc);
      const list = res?.location;
      if (list && list.length > 0) {
        setCityLabel(`${list[0].name}, ${list[0].adm1}`);
      }
    } catch { /* noop */ }
  }, []);

  useEffect(() => {
    fetchWeather(location);
    lookupCity(location);
    const timer = setInterval(() => fetchWeather(location), 300000);
    return () => clearInterval(timer);
  }, [location, fetchWeather, lookupCity]);

  const handleSearch = useCallback((keyword: string) => {
    if (!keyword || keyword.trim().length < 2) {
      setSearchResults([]);
      return;
    }
    if (searchTimer.current) clearTimeout(searchTimer.current);
    searchTimer.current = setTimeout(async () => {
      setSearching(true);
      try {
        const res = await weatherApi.geoLookup(keyword.trim());
        setSearchResults(res?.location || []);
      } catch { setSearchResults([]); }
      finally { setSearching(false); }
    }, 400);
  }, []);

  const handleLocationChange = useCallback((value: string) => {
    saveLocation(value);
    setLocation(value);
  }, []);

  const nowH = new Date().getHours().toString().padStart(2, '0');
  const nowM = new Date().getMinutes().toString().padStart(2, '0');

  return (
    <AntHeader className={styles.topbar}>
      <div className={styles.row1}>
        <Space size="small">
          <CloudOutlined style={{ fontSize: 14, color: '#3b82f6' }} />
          <Select
            showSearch
            allowClear={false}
            value={location}
            placeholder="搜索城市..."
            notFoundContent={searching ? <Spin size="small" /> : null}
            filterOption={false}
            onSearch={handleSearch}
            onChange={handleLocationChange}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && searchResults.length > 0) {
                handleLocationChange(searchResults[0].id);
                (document.activeElement as HTMLElement)?.blur();
              }
            }}
            style={{ width: 160 }}
            size="small"
            options={searchResults.map(c => ({
              label: `${c.name}, ${c.adm1}${c.country !== '中国' ? `, ${c.country}` : ''}`,
              value: c.id,
            }))}
          />
          {weatherLastRefresh && (
            <Text type="secondary" className={styles.updateTime}>
              更新 {weatherLastRefresh}
            </Text>
          )}
        </Space>
        <Space size="small">
          <Text type="secondary" className={styles.time}>{nowH}:{nowM}</Text>
          <Badge status={connected ? 'success' : 'error'} />
          <Text type="secondary" className={styles.connText}>
            {connected ? '在线' : '离线'}
          </Text>
          {lastUpdate && (
            <Text type="secondary" className={styles.updateTime}>
              {new Date(lastUpdate).toLocaleTimeString('zh-CN')}
            </Text>
          )}
        </Space>
      </div>

      <div className={styles.row2}>
        {loading ? (
          <Spin size="small" style={{ margin: '0 auto' }} />
        ) : (
          <>
            <div className={styles.currentBlock}>
              <Text className={styles.currentIcon}>
                {now ? weatherIcon(now.icon, now.text) : '🌤️'}
              </Text>
              <div className={styles.currentInfo}>
                <div className={styles.currentData}>
                  <Text strong className={styles.currentTemp}>
                    {now ? `${now.temp}℃` : '--℃'}
                  </Text>
                  <Text className={styles.currentText}>{now?.text || '--'}</Text>
                </div>
                <div className={styles.currentMeta}>
                  <Text type="secondary">湿度 {now?.humidity ?? '--'}%</Text>
                  <Text type="secondary" className={styles.metaSep}>|</Text>
                  <Text type="secondary">{now?.windDir ?? '--'} {now?.windScale ?? '--'}级</Text>
                </div>
                <div className={styles.locationLine}>
                  <Text type="secondary">{cityLabel || '--'}</Text>
                </div>
              </div>
            </div>

            <div className={styles.vertDivider} />

            <div className={styles.forecastBlock}>
              {forecast.slice(0, 3).map(day => (
                <div key={day.date} className={styles.forecastDay}>
                  <Text className={styles.forecastDate}>{formatDate(day.date)}</Text>
                  <Text className={styles.forecastIcon}>{weatherIcon(day.iconDay, day.textDay)}</Text>
                  <Text className={styles.forecastText}>{day.textDay}</Text>
                  <Text className={styles.forecastTemp}>
                    <span className={styles.tempHigh}>{day.tempMax}°</span>
                    <span className={styles.tempSep}>/</span>
                    <span className={styles.tempLow}>{day.tempMin}°</span>
                  </Text>
                </div>
              ))}
            </div>

            <div className={styles.vertDivider} />

            <div className={styles.minutelyBlock}>
              <Text className={styles.minutelyIcon}>🌧️</Text>
              <div className={styles.hourlyPrecipCol}>
                <Text className={styles.minutelyText}>{minutely?.summary || '获取中...'}</Text>
                {minutely?.hourly && minutely.hourly.length > 0 && (
                  <div className={styles.hourlyList}>
                    {minutely.hourly.slice(0, 4).map(h => {
                      const hh = new Date(h.time).getHours().toString().padStart(2, '0');
                      const pop = Number(h.pop);
                      return (
                        <div key={h.time} className={styles.hourlyItem}>
                          <Text className={styles.hourlyTime}>{hh}:00</Text>
                          <Text className={styles.hourlyIcon}>{weatherIcon('999', h.text)}</Text>
                          <Text className={styles.hourlyPop} data-rain={pop > 30 ? 'true' : 'false'}>
                            {pop}%
                          </Text>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>

            {warnings.length > 0 && (
              <>
                <div className={styles.vertDivider} />
                <div className={styles.warningsBlock}>
                  {warnings.slice(0, 2).map((w, i) => (
                    <div key={i} className={styles.warningItem}
                      title={w.title}
                      style={{ borderColor: levelColors[w.level] || '#eab308' }}>
                      <WarningOutlined style={{ color: levelColors[w.level] || '#eab308', fontSize: 13 }} />
                      <Text className={styles.warningText}
                        style={{ color: levelColors[w.level] || '#eab308' }}>
                        {w.type}{w.level}预警
                      </Text>
                    </div>
                  ))}
                </div>
              </>
            )}
          </>
        )}
      </div>
    </AntHeader>
  );
};

export default TopBar;
