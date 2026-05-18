import { useState, useEffect, useRef, useCallback } from 'react';
import { PROVINCES, simplifyProvince } from '../data/regions';
import { IconMapPin, IconChevronDown, IconSearch, IconLightbulb, IconCheck, IconX } from './Icons';

const LS_KEY = 'weatherLoc';

function loadSaved() {
  try {
    const raw = localStorage.getItem(LS_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch { return null; }
}

export default function RegionSelector({ onSelect }) {
  const [saved] = useState(loadSaved);
  const [open, setOpen] = useState(false);
  const [province, setProvince] = useState('');
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);
  const [searching, setSearching] = useState(false);
  const [errMsg, setErrMsg] = useState('');
  const [selectedId, setSelectedId] = useState(saved?.id || '101010100');
  const [selectedName, setSelectedName] = useState(saved?.name || '北京');
  const ref = useRef(null);
  const timerRef = useRef(null);

  useEffect(() => {
    const h = (e) => { if (ref.current && !ref.current.contains(e.target)) setOpen(false); };
    document.addEventListener('mousedown', h);
    return () => document.removeEventListener('mousedown', h);
  }, []);

  useEffect(() => {
    if (!open) { setQuery(''); setResults([]); setErrMsg(''); }
  }, [open]);

  const doSearch = useCallback(async (q, prov) => {
    if (!q || q.length < 2) { setResults([]); setErrMsg(''); return; }
    setSearching(true);
    setErrMsg('');
    try {
      const res = await fetch(`/api/v1/weather/geo?location=${encodeURIComponent(q)}&number=20`);
      if (!res.ok) { setErrMsg(`请求失败 (${res.status})`); setResults([]); return; }
      const data = await res.json();
      if (data.code === '200') {
        let list = data.location || [];
        if (prov) {
          const short = simplifyProvince(prov);
          list = list.filter(r => (r.adm1 || '').includes(short));
        }
        list.sort((a, b) => {
          const rank = { 'city': 0, 'district': 1, 'town': 2, 'village': 3 };
          return (rank[a.type] ?? 9) - (rank[b.type] ?? 9);
        });
        setResults(list);
        setErrMsg(list.length === 0 ? '未找到匹配位置，请尝试其他关键词' : '');
      } else {
        setErrMsg(`查询失败 (code: ${data.code})`);
        setResults([]);
      }
    } catch (e) {
      setErrMsg('网络错误，请检查连接');
      setResults([]);
    }
    setSearching(false);
  }, []);

  const onQueryChange = (val) => {
    setQuery(val);
    clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => doSearch(val, province), 300);
  };

  const onProvinceChange = (val) => {
    setProvince(val);
    if (query.length >= 2) doSearch(query, val);
  };

  const select = (r) => {
    const path = [r.adm1, r.adm2].filter(Boolean).join(' > ');
    const label = path ? `${r.name}（${path}）` : r.name;
    const loc = { id: r.id, name: label, lat: r.lat, lon: r.lon };
    setSelectedId(r.id);
    setSelectedName(label);
    try { localStorage.setItem(LS_KEY, JSON.stringify(loc)); } catch {}
    onSelect(loc);
    setOpen(false);
  };

  const handleKeyDown = (e) => {
    if (e.key === 'Escape') setOpen(false);
  };

  const displayName = selectedName.length > 20 ? selectedName.slice(0, 18) + '…' : selectedName;

  return (
    <div className="rs-wrap" ref={ref} onKeyDown={handleKeyDown}>
      <button className="btn btn-sm" onClick={() => setOpen(!open)}>
        <IconMapPin size={14} /> {displayName} <IconChevronDown size={12} />
      </button>

      {open && (
        <div className="rs-dropdown">
          <select className="rs-province" value={province} onChange={e => onProvinceChange(e.target.value)} aria-label="选择省份">
            <option value="">全国（不限省份）</option>
            {PROVINCES.map(p => <option key={p} value={p}>{p}</option>)}
          </select>

          <input
            className="rs-input"
            placeholder={province ? `搜索${simplifyProvince(province)}下属区县/乡镇...` : '输入城市/区县/乡镇名称...'}
            value={query}
            onChange={e => onQueryChange(e.target.value)}
            autoFocus
            aria-label="搜索位置"
          />

          {province && query.length < 2 && (
            <div className="rs-hint">选择了 {province}，请输入区县/乡镇名搜索</div>
          )}

          <div className="rs-results" role="listbox" aria-label="搜索结果">
            {searching ? (
              <div className="rs-hint"><IconSearch size={14} style={{verticalAlign:'-2px',marginRight:4}} />搜索中...</div>
            ) : errMsg ? (
              <div className="rs-hint">{errMsg}</div>
            ) : query.length < 2 && !province ? (
              <div className="rs-hint"><IconLightbulb size={14} style={{verticalAlign:'-2px',marginRight:4}} />先选省份缩小范围，或直接输入城市名称</div>
            ) : (
              results.map((r, i) => {
                const isSelected = r.id === selectedId;
                return (
                  <div
                    key={r.id || i}
                    className={`rs-item ${isSelected ? 'rs-item-active' : ''}`}
                    onClick={() => select(r)}
                    onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); select(r); } }}
                    role="option"
                    tabIndex={0}
                    aria-selected={isSelected}
                  >
                    <div className="rs-item-main">
                      <span className="rs-item-name">{r.name}</span>
                      <span className="rs-item-path">{r.adm1}{r.adm2 && r.adm2 !== r.adm1 ? ` > ${r.adm2}` : ''}</span>
                    </div>
                    <div className="rs-item-meta">
                      <span className="rs-item-type">{r.type || ''}</span>
                      {isSelected && <IconCheck size={14} className="rs-item-check" />}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export function getSavedLocation() {
  return loadSaved();
}
