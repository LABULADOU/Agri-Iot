function SvgBase({ children, size = 24, className, style }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size} height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      style={style}
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

export function IconSprout(p) {
  return <SvgBase {...p}><path d="M12 22v-8" /><path d="M12 14c-3.3 0-6-2.7-6-6V2h2c3.3 0 6 2.7 6 6" /><path d="M12 14c3.3 0 6-2.7 6-6V2h-2c-3.3 0-6 2.7-6 6" /></SvgBase>;
}

export function IconLayoutDashboard(p) {
  return <SvgBase {...p}><rect x="3" y="3" width="7" height="9" /><rect x="14" y="3" width="7" height="5" /><rect x="14" y="12" width="7" height="9" /><rect x="3" y="16" width="7" height="5" /></SvgBase>;
}

export function IconMap(p) {
  return <SvgBase {...p}><path d="M3 7v14l6-3 6 3 6-3V3l-6 3-6-3z" /><path d="M9 4v14" /><path d="M15 7v14" /></SvgBase>;
}

export function IconBrainCircuit(p) {
  return <SvgBase {...p}><path d="M12 5a3 3 0 1 0-3 3 3 3 0 0 0 3-3z" /><path d="M12 12v4" /><path d="M10 20a2 2 0 1 0 4 0" /><circle cx="12" cy="5" r="3" /><circle cx="12" cy="20" r="2" /><path d="M17 9a3 3 0 1 0-3 3" /><path d="M7 9a3 3 0 0 0 3 3" /></SvgBase>;
}

export function IconBell(p) {
  return <SvgBase {...p}><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" /><path d="M13.73 21a2 2 0 0 1-3.46 0" /></SvgBase>;
}

export function IconSettings(p) {
  return <SvgBase {...p}><circle cx="12" cy="12" r="3" /><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" /></SvgBase>;
}

export function IconMapPin(p) {
  return <SvgBase {...p}><path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z" /><circle cx="12" cy="10" r="3" /></SvgBase>;
}

export function IconThermometer(p) {
  return <SvgBase {...p}><path d="M14 14.76V3.5a2.5 2.5 0 0 0-5 0v11.26a4.5 4.5 0 1 0 5 0z" /></SvgBase>;
}

export function IconDroplets(p) {
  return <SvgBase {...p}><path d="M12 22a7 7 0 0 0 7-7c0-4-7-14-7-14S5 11 5 15a7 7 0 0 0 7 7z" /></SvgBase>;
}

export function IconLeaf(p) {
  return <SvgBase {...p}><path d="M11 20A7 7 0 0 1 9.8 6.9C15.5 4.9 17 3.5 19 2c1 2 2 4.5 2 8 0 5.5-4.78 10-10 10Z" /><path d="M2 21c0-3 1.85-5.36 5.08-6C9.5 14.52 12 13 13 12" /></SvgBase>;
}

export function IconZap(p) {
  return <SvgBase {...p}><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" /></SvgBase>;
}

export function IconSun(p) {
  return <SvgBase {...p}><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41" /></SvgBase>;
}

export function IconLightbulb(p) {
  return <SvgBase {...p}><path d="M9 18h6" /><path d="M10 22h4" /><path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.57 4.57 0 0 1 8.91 14" /></SvgBase>;
}

export function IconBarChart3(p) {
  return <SvgBase {...p}><path d="M3 20V10" /><path d="M9 20V4" /><path d="M15 20v-8" /><path d="M21 20v-6" /></SvgBase>;
}

export function IconAlertTriangle(p) {
  return <SvgBase {...p}><path d="M12 9v4" /><path d="M12 17h0" /><path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" /></SvgBase>;
}

export function IconAlertOctagon(p) {
  return <SvgBase {...p}><polygon points="7.86 2 16.14 2 22 7.86 22 16.14 16.14 22 7.86 22 2 16.14 2 7.86 7.86 2" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></SvgBase>;
}

export function IconInfo(p) {
  return <SvgBase {...p}><circle cx="12" cy="12" r="10" /><line x1="12" y1="16" x2="12" y2="12" /><line x1="12" y1="8" x2="12.01" y2="8" /></SvgBase>;
}

export function IconMaximize2(p) {
  return <SvgBase {...p}><polyline points="15 3 21 3 21 9" /><polyline points="9 21 3 21 3 15" /><line x1="21" y1="3" x2="14" y2="10" /><line x1="3" y1="21" x2="10" y2="14" /></SvgBase>;
}

export function IconRefreshCw(p) {
  return <SvgBase {...p}><path d="M23 4v6h-6" /><path d="M1 20v-6h6" /><path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" /></SvgBase>;
}

export function IconChevronDown(p) {
  return <SvgBase {...p}><polyline points="6 9 12 15 18 9" /></SvgBase>;
}

export function IconChevronUp(p) {
  return <SvgBase {...p}><polyline points="18 15 12 9 6 15" /></SvgBase>;
}

export function IconMenu(p) {
  return <SvgBase {...p}><line x1="4" x2="20" y1="12" y2="12" /><line x1="4" x2="20" y1="6" y2="6" /><line x1="4" x2="20" y1="18" y2="18" /></SvgBase>;
}

export function IconX(p) {
  return <SvgBase {...p}><path d="M18 6 6 18" /><path d="m6 6 12 12" /></SvgBase>;
}

export function IconArrowLeft(p) {
  return <SvgBase {...p}><path d="m12 19-7-7 7-7" /><path d="M19 12H5" /></SvgBase>;
}

export function IconCheck(p) {
  return <SvgBase {...p}><polyline points="20 6 9 17 4 12" /></SvgBase>;
}

export function IconPlus(p) {
  return <SvgBase {...p}><path d="M12 5v14" /><path d="M5 12h14" /></SvgBase>;
}

export function IconCircle(p) {
  const { fill, ...rest } = p;
  return <SvgBase {...rest}><circle cx="12" cy="12" r="10" fill={fill || 'none'} /></SvgBase>;
}

export function IconCloud(p) {
  return <SvgBase {...p}><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z" /></SvgBase>;
}

export function IconCloudRain(p) {
  return <SvgBase {...p}><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z" /><path d="M11 14v6" /><path d="M15 14v6" /></SvgBase>;
}

export function IconCloudSnow(p) {
  return <SvgBase {...p}><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z" /><circle cx="9" cy="16" r="1" /><circle cx="13" cy="18" r="1" /><circle cx="11" cy="20" r="1" /></SvgBase>;
}

export function IconCloudFog(p) {
  return <SvgBase {...p}><path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 0 1 0 9Z" /><path d="M6 14h8" /><path d="M4 17h10" /></SvgBase>;
}

export function IconEye(p) {
  return <SvgBase {...p}><path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z" /><circle cx="12" cy="12" r="3" /></SvgBase>;
}

export function IconWind(p) {
  return <SvgBase {...p}><path d="M9.5 4.5A2.5 2.5 0 0 1 12 2a2.5 2.5 0 0 1 2.5 2.5c0 1.2-.7 2.2-1.7 2.6" /><path d="M9.5 12A2.5 2.5 0 0 1 12 9.5a2.5 2.5 0 0 1 2.5 2.5c0 1.2-.7 2.2-1.7 2.6" /><path d="M9.5 19.5A2.5 2.5 0 0 1 12 17a2.5 2.5 0 0 1 2.5 2.5c0 1.2-.7 2.2-1.7 2.6" /><path d="M4 7h14" /><path d="M4 14h10" /><path d="M4 21h8" /></SvgBase>;
}

export function IconGauge(p) {
  return <SvgBase {...p}><path d="m12 14 4-4" /><path d="M3.34 19a10 10 0 1 1 17.32 0" /><circle cx="12" cy="12" r="10" /></SvgBase>;
}

export function IconSearch(p) {
  return <SvgBase {...p}><circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" /></SvgBase>;
}

export function IconFlame(p) {
  return <SvgBase {...p}><path d="M12 2c0 4 2 6 2 8a2 2 0 1 1-4 0c0-2 2-4 2-8Z" /><path d="M8 15c0 2.2 1.8 4 4 4s4-1.8 4-4c0-2-2-4-4-7-2 3-4 5-4 7Z" /></SvgBase>;
}

export function IconActivity(p) {
  return <SvgBase {...p}><polyline points="22 12 18 12 15 21 9 3 6 12 2 12" /></SvgBase>;
}

export function IconPower(p) {
  return <SvgBase {...p}><path d="M12 2v10" /><path d="M18.36 6.64a9 9 0 1 1-12.73 0" /></SvgBase>;
}

export function IconToggleLeft(p) {
  return <SvgBase {...p}><rect x="1" y="5" width="22" height="14" rx="7" ry="7" /><circle cx="8" cy="12" r="3.5" /></SvgBase>;
}

export function IconToggleRight(p) {
  return <SvgBase {...p}><rect x="1" y="5" width="22" height="14" rx="7" ry="7" /><circle cx="16" cy="12" r="3.5" /></SvgBase>;
}
