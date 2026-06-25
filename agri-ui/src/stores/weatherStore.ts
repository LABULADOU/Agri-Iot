import { create } from 'zustand';
import type { GeoCity } from '../types';

const STORAGE_KEY = 'agri_weather_location';

interface WeatherStore {
  location: GeoCity;
  setLocation: (loc: GeoCity) => void;
}

function loadLocation(): GeoCity {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return { id: '39.92,116.41', name: '北京', adm1: '北京市', adm2: '' };
}

export const useWeatherStore = create<WeatherStore>((set) => ({
  location: loadLocation(),
  setLocation: (loc: GeoCity) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(loc));
    set({ location: loc });
  },
}));
