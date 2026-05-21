import React, { useState } from 'react';
import { Typography, Collapse } from 'antd';
import styles from './AIAnalysisReport.module.css';

const { Text, Paragraph } = Typography;

interface SimilarCase {
  id: string;
  title: string;
  summary: string;
  date: string;
}

interface Assessment {
  score: number;
  status: string;
  summary: string;
  details?: string[];
}

interface AIAnalysisReportProps {
  assessment: Assessment | null;
  similarCases?: SimilarCase[];
}

const AIAnalysisReport: React.FC<AIAnalysisReportProps> = ({ assessment, similarCases = [] }) => {
  const [expanded, setExpanded] = useState(false);

  if (!assessment || assessment.score >= 80) {
    return null;
  }

  return (
    <div className={styles.container}>
      <div className={styles.header} onClick={() => setExpanded(!expanded)}>
        <Text strong>AI 分析报告</Text>
        <Text type="secondary" className={styles.toggle}>{expanded ? '收起' : '展开'}</Text>
      </div>

      {expanded && (
        <div className={styles.body}>
          <div className={styles.summary}>
            <Text>{assessment.summary}</Text>
          </div>

          {assessment.details && assessment.details.length > 0 && (
            <div className={styles.details}>
              {assessment.details.map((d, i) => (
                <Text key={i} type="secondary" className={styles.detailItem}>• {d}</Text>
              ))}
            </div>
          )}

          {similarCases.length > 0 && (
            <div className={styles.cases}>
              <Text strong className={styles.casesTitle}>参考案例</Text>
              {similarCases.map(c => (
                <div key={c.id} className={styles.caseItem}>
                  <Text>{c.title}</Text>
                  <Paragraph className={styles.caseSummary} ellipsis={{ rows: 1 }}>
                    {c.summary}
                  </Paragraph>
                  <Text type="secondary" className={styles.caseDate}>{c.date}</Text>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default AIAnalysisReport;
