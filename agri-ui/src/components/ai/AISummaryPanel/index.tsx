import React from 'react';
import { Typography } from 'antd';
import styles from './AISummaryPanel.module.css';

const { Text, Paragraph } = Typography;

interface AIRecommendation {
  id: string;
  content: string;
  targetArea: string;
  caseLink?: string;
}

interface AISummaryPanelProps {
  recommendations: AIRecommendation[];
}

const AISummaryPanel: React.FC<AISummaryPanelProps> = ({ recommendations }) => {
  if (!recommendations.length) {
    return (
      <div className={styles.empty}>
        <Text type="secondary">当前各区域状态正常</Text>
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      {recommendations.map(r => (
        <div key={r.id} className={styles.item}>
          <Text className={styles.area}>{r.targetArea}</Text>
          <Paragraph className={styles.content} ellipsis={{ rows: 2 }}>
            {r.content}
          </Paragraph>
          {r.caseLink && (
            <Text type="secondary" className={styles.link}>
              参考案例: {r.caseLink}
            </Text>
          )}
        </div>
      ))}
    </div>
  );
};

export default AISummaryPanel;
