import { useState, useEffect, useCallback, useRef } from 'react';
import RegionSelector, { getSavedLocation } from './RegionSelector';
import { IconCloud, IconSun, IconCloudRain, IconCloudSnow, IconCloudFog, IconDroplets, IconWind, IconEye, IconGauge, IconFlame } from './Icons';

const SAMPLE = [
  { fxDate: '今天', tempMin: 18, tempMax: 28, textDay: '晴', iconDay: '100', precip: '0' },
  { fxDate: '明天', tempMin: 20, tempMax: 30, textDay: '多云', iconDay: '101', precip: '10' },
  { fxDate: '后天', tempMin: 19, tempMax: 27, textDay: '阴', iconDay: '104', precip: '30' },
];

function initialLoc() {
  const saved = getSavedLocation();
  return saved
    ? { id: saved.id, name: saved.name, lat: saved.lat || '39.90', lon: saved.lon || '116.40' }
    : { id: '101010100', name: '北京', lat: '39.90', lon: '116.40' };
}

export default function WeatherCard() {
  const init = initialLoc();
  const [locId, setLocId] = useState(init.id);
  const [locName, setLocName] = useState(init.name);
  const [locLat, setLocLat] = useState(init.lat);
  const [locLon, setLocLon] = useState(init.lon);

  const [forecast, setForecast] = useState(null);
  const [now, setNow] = useState(null);
  const [indices, setIndices] = useState(null);
  const [minutely, setMinutely] = useState(null);
  const [loading, setLoading] = useState(true);
  const hasDataRef = useRef(false);
  const timerRef = useRef(null);

  const fetchAll = useCallback(async (id, lat, lon) => {
    if (!hasDataRef.current) setLoading(true);
    const t = (p, ms) => Promise.race([p, new Promise((_, r) => setTimeout(() => r(new Error('timeout')), ms))]);

    try {
      const [f3d, nw, idx, min] = await Promise.allSettled([
        t(fetch(`/api/v1/weather/3d?location=${id}`).then(r => r.json()), 5000),
        t(fetch(`/api/v1/weather/now?location=${id}`).then(r => r.json()), 5000),
        t(fetch(`/api/v1/weather/indices?location=${id}&type=1,2,3`).then(r => r.json()), 5000),
        t(fetch(`/api/v1/weather/minutely?location=${lon},${lat}`).then(r => r.json()), 5000),
      ]);

      if (f3d.status === 'fulfilled' && f3d.value?.code === '200') setForecast(f3d.value.daily);
      else setForecast(SAMPLE);

      if (nw.status === 'fulfilled' && nw.value?.code === '200') setNow(nw.value.now);
      else setNow(null);

      if (idx.status === 'fulfilled' && idx.value?.code === '200') setIndices(idx.value.daily);
      else setIndices(null);

      if (min.status === 'fulfilled' && min.value?.code === '200') setMinutely(min.value);
      else setMinutely(null);

      hasDataRef.current = true;

      let nextMs = 600000;
      const nwData = nw.status === 'fulfilled' ? nw.value?.now : null;
      const minData = min.status === 'fulfilled' ? min.value : null;
      const f3dData = f3d.status === 'fulfilled' ? f3d.value?.daily : null;

      if (minData?.minutely?.some(m => parseFloat(m.precip) > 0)) nextMs = 120000;
      if (nwData?.text && /雨|雪|雷|暴/.test(nwData.text)) nextMs = Math.min(nextMs, 120000);
      if (f3dData?.some(d => /雨|雪|雷|暴/.test(d.textDay || ''))) nextMs = Math.min(nextMs, 300000);

      const ws = parseInt(nwData?.windScale, 10);
      if (ws >= 6) nextMs = Math.min(nextMs, 120000);
      else if (ws >= 4) nextMs = Math.min(nextMs, 300000);

      clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => fetchAll(id, lat, lon), nextMs);
    } catch (_) {
      clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => fetchAll(id, lat, lon), 600000);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    hasDataRef.current = false;
    clearTimeout(timerRef.current);
    fetchAll(locId, locLat, locLon);
    return () => clearTimeout(timerRef.current);
  }, [locId, locLat, locLon, fetchAll]);

  const handleLocationSelect = (loc) => {
    setLocId(loc.id);
    setLocName(loc.name);
    if (loc.lat && loc.lon) { setLocLat(loc.lat); setLocLon(loc.lon); }
  };

  const getIcon = (code) => {
    const style = { verticalAlign: '-2px' };
    if (code === '100') return <IconSun size={14} style={style} />;
    if (code.startsWith('3')) return <IconCloudRain size={14} style={style} />;
    if (code.startsWith('4')) return <IconCloudSnow size={14} style={style} />;
    if (code.startsWith('5')) return <IconCloudFog size={14} style={style} />;
    return <IconSun size={14} style={style} />;
  };

  const topIndex = indices && indices.length > 0 ? indices[0] : null;
  const windScale = parseInt(now?.windScale, 10);
  const windWarning = windScale >= 6 ? `⚠️ 大风（${now.windDir} ${windScale}级）` : null;

  let rainSummary = '暂无数据';
  if (minutely?.summary) {
    rainSummary = minutely.summary;
  } else if (minutely?.minutely) {
    const u = minutely.minutely.filter(m => parseFloat(m.precip) > 0);
    rainSummary = u.length === 0 ? '未来两小时无降水' : `预计${u[0].fxTime?.slice(11, 16) || ''}起有降水`;
  }

  const days = (forecast || SAMPLE).slice(0, 3);

  return (
    <div className="weather-wrapper">
      <div className="weather-header">
        <span className="fw-600 text-sm"><IconCloud size={16} style={{verticalAlign:'-3px',marginRight:4}} />气象服务</span>
        <RegionSelector onSelect={handleLocationSelect} />
      </div>

      {loading ? (
        <div className="skeleton" style={{ height: 56 }} />
      ) : (
        <div className="weather-data">
          <div className="weather-row">
            <span className="wc-temp">{now ? now.temp : '--'}<span className="wc-temp-unit">℃</span></span>
            <span className="wc-feels">体感{now ? now.feelsLike : '--'}℃</span>

            <span className="wc-sep" />

            {days.map((d, i) => (
              <span key={i} className="wc-day">
                <span className="text-dim" style={{fontSize:10,marginRight:2}}>{d.fxDate?.length > 3 ? d.fxDate.slice(5) : d.fxDate}</span>
                <span className="wc-day-icon">{getIcon(d.iconDay)}</span>
                <span className="wc-day-temps">{d.tempMax}°/{d.tempMin}°</span>
                <span className="wc-day-wind"><IconWind size={10} style={{verticalAlign:'-1px',marginRight:1}} />{d.windScaleDay || '--'}级</span>
              </span>
            ))}

            <span className="wc-sep" />

            <span className="wc-metric"><IconDroplets size={12} style={{verticalAlign:'-2px'}} /> <strong>{now ? now.humidity : '--'}</strong>%</span>
            <span className="wc-metric"><IconWind size={12} style={{verticalAlign:'-2px'}} /> <strong>{now ? `${now.windDir} ${now.windScale}级${now.windSpeed ? ` (${now.windSpeed}km/h)` : ''}` : '--'}</strong></span>
            <span className="wc-metric"><IconEye size={12} style={{verticalAlign:'-2px'}} /> <strong>{now ? `${now.vis}km` : '--'}</strong></span>
            <span className="wc-metric"><IconGauge size={12} style={{verticalAlign:'-2px'}} /> <strong>{now ? `${now.pressure}hPa` : '--'}</strong></span>
          </div>

          <div className="weather-footer">
            <span><IconCloudRain size={12} style={{verticalAlign:'-2px'}} /> <strong>{rainSummary}</strong></span>
            {windWarning && <span style={{color:'var(--yellow)'}}><strong>{windWarning}</strong></span>}
            <span><IconFlame size={12} style={{verticalAlign:'-2px'}} /> <strong>{topIndex ? `${topIndex.name} ${topIndex.category || ''}` : '暂无数据'}</strong></span>
          </div>
        </div>
      )}
    </div>
  );
}
