import React from 'react';
import ReactECharts from 'echarts-for-react';
import type { EChartsOption } from 'echarts';
import type { AggregatedReading } from '../../types';
import { metricLabels, metricColors, chartGrid, CHART_COLORS } from '../../theme/echartsTheme';
import dayjs from 'dayjs';

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
      itemStyle: { color: metricColors[metric] || CHART_COLORS.primary },
      data: timestamps.map(ts => {
        const item = metricData.find(d => d.timestamp === ts);
        return item ? item.avg : null;
      }),
    };
  });

  const option: EChartsOption = {
    color: Object.values(metricColors),
    tooltip: {
      trigger: 'axis' as const,
    },
    legend: showLegend ? {
      data: metrics.map(m => metricLabels[m] || m),
      bottom: 0,
      textStyle: { color: CHART_COLORS.gray500 },
    } : undefined,
    grid: {
      ...chartGrid,
      bottom: showLegend ? '15%' : '3%',
    },
    xAxis: {
      type: 'category' as const,
      boundaryGap: false,
      data: timestamps.map(ts => dayjs(ts).format('MM-DD HH:mm')),
      axisLine: { lineStyle: { color: CHART_COLORS.gray200 } },
      axisLabel: {
        rotate: 45,
        fontSize: 10,
        color: CHART_COLORS.gray400,
      },
    },
    yAxis: {
      type: 'value' as const,
      splitLine: { lineStyle: { color: CHART_COLORS.gray100 } },
      axisLabel: { color: CHART_COLORS.gray400 },
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
      opts={{ renderer: 'canvas' }} notMerge={true}
    />
  );
};

export default LineChart;
