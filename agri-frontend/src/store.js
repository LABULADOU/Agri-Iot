import { create } from 'zustand';

export const useStore = create((set) => ({
  devices: [],
  rules: [],
  alerts: [],
  areas: [],
  summary: null,
  areaReadings: [],
  systemInfo: null,
  selectedZone: null,

  setDevices: (devices) => set({ devices }),
  setRules: (rules) => set({ rules }),
  setAlerts: (alerts) => set({ alerts }),
  setAreas: (areas) => set({ areas }),
  setSummary: (summary) => set({ summary }),
  setAreaReadings: (areaReadings) => set({ areaReadings }),
  setSystemInfo: (info) => set({ systemInfo: info }),
  setSelectedZone: (zone) => set({ selectedZone: zone }),
}));
