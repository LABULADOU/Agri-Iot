import axios from 'axios';
import type { WeatherData, WeatherForecast } from '../types';

const HEWEATHER_KEY = 'ce4c45582c584880a5e17a6927e9a5ad';
const HEWEATHER_HOST = 'https://devapi.qweather.com/v7';

export const heweatherApi = {
  getNow: async (location: string = '101010100'): Promise<WeatherData> => {
    const res = await axios.get(`${HEWEATHER_HOST}/weather/now`, {
      params: { location, key: HEWEATHER_KEY },
    });
    const data = res.data;
    return {
      location: data.location?.name || location,
      temp: Number(data.now?.temp) || 0,
      humidity: Number(data.now?.humidity) || 0,
      text: data.now?.text || '未知',
      windSpeed: Number(data.now?.windSpeed) || 0,
      windDir: data.now?.windDir || '未知',
      updateTime: data.updateTime || new Date().toLocaleString('zh-CN'),
      forecast: [],
    };
  },

  getForecast: async (location: string = '101010100', days: number = 3): Promise<WeatherData> => {
    const res = await axios.get(`${HEWEATHER_HOST}/weather/3d`, {
      params: { location, key: HEWEATHER_KEY },
    });
    const data = res.data;
    const forecasts: WeatherForecast[] = (data.daily || []).slice(0, days).map((d: Record<string, string>) => ({
      date: d.fxDate,
      tempMax: Number(d.tempMax),
      tempMin: Number(d.tempMin),
      textDay: d.textDay,
      textNight: d.textNight,
      humidity: Number(d.humidity),
    }));
    return {
      location: data.location?.name || location,
      temp: 0,
      humidity: 0,
      text: '',
      windSpeed: 0,
      windDir: '',
      updateTime: new Date().toLocaleString('zh-CN'),
      forecast: forecasts,
    };
  },

  getWeather: async (location: string = '101010100'): Promise<WeatherData> => {
    const [now, forecast] = await Promise.all([
      heweatherApi.getNow(location),
      heweatherApi.getForecast(location, 3),
    ]);
    return { ...now, forecast: forecast.forecast };
  },
};

export default heweatherApi;