import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Layout, Space, Typography, Badge, Spin, AutoComplete, Input } from 'antd';
import { WarningOutlined, SearchOutlined } from '@ant-design/icons';
import { pinyin } from 'pinyin-pro';
import { useRealtimeStore } from '../../stores/realtimeStore';
import { useWeatherStore } from '../../stores/weatherStore';
import { weatherApi } from '../../services/api';
import type { WeatherData, WeatherForecastDay, WeatherWarning, MinutelyForecast, GeoCity } from '../../types';
import styles from './TopBar.module.css';

const { Text } = Typography;
const { Header: AntHeader } = Layout;

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
    pressure: Number(raw.pressure) || 0,
    vis: Number(raw.vis) || 0,
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

function weatherIcon(icon: string): string {
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

function buildPinyinCandidates(q: string): string[] {
  // Open-Meteo 的 GeoNames 库对中国县级地名覆盖不全（如"富源县"搜不到），
  // 去掉行政区后缀字后生成拼音候选：整串 → 末词 → 首词
  const cleaned = q.replace(/[省市区县镇乡村盟旗州]/g, '');
  if (!cleaned) return [];
  const arr = pinyin(cleaned, { toneType: 'none', type: 'array', nonZh: 'consecutive' })
    .map(s => s.toLowerCase());
  const full = arr.join('');
  const words = arr.filter(w => /[a-z]/.test(w));
  const cand = [full];
  if (words.length > 1) {
    cand.push(words[words.length - 1]);
    cand.push(words[0]);
  }
  return [...new Set(cand)];
}

const TopBar: React.FC = () => {
  const { connected, lastUpdate } = useRealtimeStore();
  const { location, setLocation } = useWeatherStore();
  const locationId = location.id;
  const [now, setNow] = useState<WeatherData | null>(null);
  const [forecast, setForecast] = useState<WeatherForecastDay[]>([]);
  const [minutely, setMinutely] = useState<MinutelyForecast | null>(null);
  const [warnings, setWarnings] = useState<WeatherWarning[]>([]);
  const [loading, setLoading] = useState(true);
  const [weatherLastRefresh, setWeatherLastRefresh] = useState<string | null>(null);
  const [searchResults, setSearchResults] = useState<GeoCity[]>([]);
  const [searchValue, setSearchValue] = useState('');
  const [searching, setSearching] = useState(false);
  const [mobileExpanded, setMobileExpanded] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchAll = useCallback(async () => {
    setLoading(true);
    try {
      const [nowRaw, f3d, minRaw, warnRaw] = await Promise.all([
        weatherApi.getNow(locationId),
        weatherApi.getForecast3d(locationId),
        weatherApi.getMinutely(locationId),
        weatherApi.getWarning(locationId),
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
  }, [locationId]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      try {
        const [nowRaw, f3d, minRaw, warnRaw] = await Promise.all([
          weatherApi.getNow(locationId),
          weatherApi.getForecast3d(locationId),
          weatherApi.getMinutely(locationId),
          weatherApi.getWarning(locationId),
        ]);
        if (cancelled) return;
        if (nowRaw?.now) setNow(normalizeNow(nowRaw.now));
        if (f3d?.daily) setForecast(f3d.daily.map(normalizeDaily));
        if (minRaw) setMinutely({ summary: minRaw.summary, hourly: minRaw.hourly || [] });
        if (warnRaw?.warning) setWarnings(warnRaw.warning.map(normalizeWarning));
        else setWarnings([]);
        setWeatherLastRefresh(new Date().toLocaleTimeString('zh-CN'));
      } catch {
        // keep last state
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    const timer = setInterval(fetchAll, 300000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [fetchAll, locationId]);

  const handleSearch = useCallback((query: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!query || query.trim().length < 1) {
      setSearchResults([]);
      return;
    }
    debounceRef.current = setTimeout(async () => {
      setSearching(true);
      try {
        const res = await weatherApi.geoLookup(query.trim(), 10);
        let results = res.location || [];
        // Open-Meteo GeoNames 对中国县级地名覆盖不全（如"富源县"：中文仅命中广东/湖南的
        // "富源"镇，云南富源县需靠拼音 Fuyuan → 曲靖市中安镇命中，坐标即富源县）。
        // 只要输入含中文即尝试拼音候选（省名过滤 + 去重合并）
        if (/[\u4e00-\u9fa5]/.test(query)) {
          const provMatch = query.match(/([\u4e00-\u9fa5]+)省/);
          for (const cand of buildPinyinCandidates(query)) {
            const pyRes = await weatherApi.geoLookup(cand, 10).catch(() => null);
            if (!pyRes) continue;
            const pyCities = (pyRes.location || [])
              .filter(g => {
                if (g.country !== 'CN') return false;
                if (provMatch && g.adm1 && !g.adm1.includes(provMatch[1])) return false;
                return true;
              })
              .map(g => ({
                ...g,
                name: query.trim(),
                adm2: g.adm2 || g.adm1,
              }));
            if (pyCities.length > 0) {
              const seenIds = new Set(results.map(g => g.id));
              results = [...results, ...pyCities.filter(g => !seenIds.has(g.id))];
              break;
            }
          }
        }
        setSearchResults(results);
      } catch {
        setSearchResults([]);
      } finally {
        setSearching(false);
      }
    }, 300);
  }, []);

  const handleSelect = useCallback((value: string) => {
    const found = searchResults.find(g => g.id === value);
    if (found) {
      setLocation({ id: found.id, name: found.name, adm1: found.adm1, adm2: found.adm2 });
      setSearchValue(found.name);
    }
  }, [searchResults, setLocation]);

  const displayName = location.name;

  const searchOptions = searchResults.map(g => ({
    value: g.id,
    label: `${g.adm1} / ${g.adm2 || g.name} / ${g.name}`,
  }));

  const toggleMobileWeather = () => setMobileExpanded(v => !v);

  return (
    <AntHeader className={`${styles.topbar} ${mobileExpanded ? styles.topbarExpanded : ''}`}>
      <div className={styles.row1} onClick={toggleMobileWeather}>
        <Space size="small" className={styles.leftSpace}>
          <Text type="secondary" className={styles.cityLabel}>
            📍{displayName}
          </Text>
          {weatherLastRefresh && (
            <Text type="secondary" className={`${styles.updateTime} ${styles.desktopOnly}`}>
              天气 {weatherLastRefresh}
            </Text>
          )}
          {now && (
            <span className={styles.mobileWeatherInline}>
              <span className={styles.inlineIcon}>{weatherIcon(now.icon)}</span>
              <span className={styles.inlineTemp}>{now.temp}℃</span>
              <span className={styles.inlineMeta}>💧{now.humidity}%</span>
              <span className={styles.inlineMeta}>🌬️{now.windDir}{now.windScale}级</span>
              <span className={styles.inlineMeta}>🔽{now.pressure}hPa</span>
            </span>
          )}
        </Space>
        <Space size="small">
          <Badge status={connected ? 'success' : 'error'} />
          <Text type="secondary" className={styles.connText}>
            {connected ? '在线' : '离线'}
          </Text>
          {lastUpdate && (
            <Text type="secondary" className={`${styles.updateTime} ${styles.desktopOnly}`}>
              {new Date(lastUpdate).toLocaleTimeString('zh-CN')}
            </Text>
          )}
        </Space>
      </div>

      <div className={`${styles.row2} ${mobileExpanded ? styles.row2Visible : ''}`}>
        <div className={styles.cityBlock}>
          <AutoComplete
            value={searchValue}
            options={searchOptions}
            onSearch={handleSearch}
            onSelect={handleSelect}
            onChange={setSearchValue}
            style={{ width: 184 }}
            notFoundContent={searching ? <Spin size="small" /> : null}
          >
            <Input
              size="small"
              placeholder="搜索城市..."
              prefix={<SearchOutlined />}
              allowClear
              className={styles.cityInput}
            />
          </AutoComplete>
        </div>

        <div className={styles.vertDivider} />

        {loading ? (
          <Spin size="small" style={{ margin: '0 auto' }} />
        ) : (
          <>
            {/* Current conditions */}
            <div className={styles.currentBlock}>
              <Text className={styles.currentIcon}>
                {now ? weatherIcon(now.icon) : '🌤️'}
              </Text>
              <div className={styles.currentData}>
                <Text strong className={styles.currentTemp}>
                  {now ? `${now.temp}℃` : '--℃'}
                </Text>
                <Text className={styles.currentText}>{now?.text || '--'}</Text>
              </div>
              <div className={styles.currentMeta}>
                <Text type="secondary">💧 {now?.humidity ?? '--'}%</Text>
                <Text type="secondary" className={styles.metaSep}>|</Text>
                <Text type="secondary">🌬️ {now?.windDir ?? '--'} {now?.windScale ?? '--'}级</Text>
                <Text type="secondary" className={styles.metaSep}>|</Text>
                <Text type="secondary">🔽 {now?.pressure ?? '--'}hPa</Text>
                <Text type="secondary" className={styles.metaSep}>|</Text>
                <Text type="secondary">👁️ {now?.vis ?? '--'}km</Text>
              </div>
            </div>

            <div className={styles.vertDivider} />

            {/* 3-day forecast */}
            <div className={styles.forecastBlock}>
              {forecast.slice(0, 3).map(day => (
                <div key={day.date} className={styles.forecastDay}>
                  <Text className={styles.forecastDate}>{formatDate(day.date)}</Text>
                  <Text className={styles.forecastIcon}>{weatherIcon(day.iconDay)}</Text>
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

            {/* Hourly precipitation */}
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
                          <Text className={styles.hourlyIcon}>{weatherIcon('999')}</Text>
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

            {/* Warnings */}
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