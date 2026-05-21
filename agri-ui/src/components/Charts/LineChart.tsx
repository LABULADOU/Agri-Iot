import React from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import type { AggregatedReading } from '../../types';
import dayjs from 'dayjs';

const metricLabels: Record<string, string> = {
  air_temp: '空气温度',
  air_humidity: '空气湿度',
  soil_temp: '土壤温度',
  soil_moisture: '土壤湿度',
  ec_value: 'EC值',
};

const metricColors: Record<string, string> = {
  air_temp: '#22C55E',
  air_humidity: '#0EA5E9',
  soil_temp: '#F59E0B',
  soil_moisture: '#22C55E',
  ec_value: '#8B5CF6',
};

interface LineChartProps {
  data: AggregatedReading[];
  height?: number;
  showLegend?: boolean;
}

const LineChart: React.FC<LineChartProps> = ({ data, height = 400, showLegend = true }) => {
  const metrics = [...new Set(data.map(d => d.metric))];
  const timestamps = [...new Set(data.map(d => d.timestamp))].sort();

  const series = metrics.map(metric => {
    const metricData = data.filter(d => d.metric === metric);
    return {
      name: metricLabels[metric] || metric,
      type: 'line' as const,
      smooth: true,
      symbol: 'circle' as const,
      symbolSize: 6,
      itemStyle: { color: metricColors[metric] || '#22C55E' },
      data: timestamps.map(ts => {
        const item = metricData.find(d => d.timestamp === ts);
        return item ? item.avg : null;
      }),
    };
  });

  const option: EChartsOption = {
    tooltip: {
      trigger: 'axis' as const,
    },
    legend: showLegend ? {
      data: metrics.map(m => metricLabels[m] || m),
      bottom: 0,
      textStyle: { color: '#6B7280' },
    } : undefined,
    grid: {
      left: '3%',
      right: '4%',
      bottom: showLegend ? '15%' : '3%',
      top: '3%',
      containLabel: true,
    },
    xAxis: {
      type: 'category' as const,
      boundaryGap: false,
      data: timestamps.map(ts => dayjs(ts).format('MM-DD HH:mm')),
      axisLine: { lineStyle: { color: '#E5E7EB' } },
      axisLabel: {
        rotate: 45,
        fontSize: 10,
        color: '#9CA3AF',
      },
    },
    yAxis: {
      type: 'value' as const,
      splitLine: { lineStyle: { color: '#F3F4F6' } },
      axisLabel: { color: '#9CA3AF' },
    },
    series,
    dataZoom: [
      {
        type: 'inside' as const,
        start: 0,
        end: 100,
      },
    ],
  };

  return (
    <ReactECharts
      option={option}
      style={{ height }}
      opts={{ renderer: 'canvas' }}
    />
  );
};

export default LineChart;
