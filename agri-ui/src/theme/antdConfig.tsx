import type { ThemeConfig } from 'antd';

export const antdTheme: ThemeConfig = {
  token: {
    colorPrimary: '#22C55E',
    colorSuccess: '#22C55E',
    colorWarning: '#F59E0B',
    colorError: '#EF4444',
    colorInfo: '#0EA5E9',
    fontFamily: '"DM Sans", "Inter", system-ui, -apple-system, sans-serif',
    fontSize: 14,
    fontSizeHeading1: 28,
    fontSizeHeading2: 22,
    fontSizeHeading3: 18,
    fontSizeHeading4: 16,
    borderRadius: 8,
    borderRadiusLG: 12,
    borderRadiusSM: 4,
    padding: 12,
    paddingLG: 16,
    paddingSM: 8,
    paddingXS: 4,
    boxShadow: 'none',
    boxShadowSecondary: '0 1px 3px rgba(0,0,0,0.06)',
  },
  components: {
    Card: {
      borderRadiusLG: 12,
      paddingLG: 16,
    },
    Button: {
      borderRadius: 8,
      controlHeight: 36,
    },
    Table: {
      borderRadius: 8,
      headerBg: '#fafafa',
      rowHoverBg: '#f5f5f5',
    },
    Modal: {
      borderRadiusLG: 16,
    },
  },
};
