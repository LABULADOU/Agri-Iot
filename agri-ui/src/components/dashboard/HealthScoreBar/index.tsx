import React from 'react';
import { Typography } from 'antd';
import styles from './HealthScoreBar.module.css';

const { Text } = Typography;

interface HealthScoreBarProps {
  score: number;
  trend: number;
  targetScore?: number;
}

const getBarColor = (score: number): string => {
  if (score >= 80) return '#22C55E';
  if (score >= 60) return '#F59E0B';
  return '#EF4444';
};

const HealthScoreBar: React.FC<HealthScoreBarProps> = ({ score, trend, targetScore = 80 }) => {
  const pct = Math.min(Math.max(score, 0), 100);
  const barColor = getBarColor(score);

  return (
    <div className={styles.container}>
      <div className={styles.track}>
        <div
          className={styles.fill}
          style={{ width: `${pct}%`, background: barColor }}
        />
        <div
          className={styles.targetLine}
          style={{ left: `${targetScore}%` }}
        />
      </div>
      <div className={styles.info}>
        <Text className={styles.score} style={{ color: barColor }}>{score}</Text>
        <span className={styles.trend} style={{ color: trend >= 0 ? '#22C55E' : '#EF4444' }}>
          {trend >= 0 ? '↑' : '↓'}
          {Math.abs(trend)}
        </span>
      </div>
    </div>
  );
};

export default HealthScoreBar;
