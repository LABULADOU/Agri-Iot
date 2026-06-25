import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Input, Typography, Tag, Spin, Button, Tooltip, Drawer, Card, Empty, Space, Collapse } from 'antd';
import {
  BookOutlined, SearchOutlined, VerticalAlignTopOutlined,
  UnorderedListOutlined, CloseOutlined,
  RightOutlined, LeftOutlined, FileTextOutlined,
  FolderOutlined, ArrowLeftOutlined,
} from '@ant-design/icons';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { aiApi } from '../../services/api';
import type { KnowledgeNoteMeta, KnowledgeNote } from '../../types';
import VarietyTable from './VarietyTable';
import styles from './KnowledgeBase.module.css';

const { Text, Title } = Typography;
const { Search } = Input;
const { Panel } = Collapse;

const VARIETY_TABLE_PATH = '切花菊/01-品种选择与特性解析.md';

const TYPE_COLORS: Record<string, string> = {
  '通用知识': 'green',
  '单一作物': 'blue',
  '品种差异': 'purple',
  '病虫害防治': 'red',
  '施肥管理': 'orange',
  '环境控制': 'cyan',
};

const CROP_ICONS: Record<string, React.ReactNode> = {
  '切花菊': <BookOutlined />,
  '洋桔梗': <FolderOutlined />,
};

interface HeadingItem {
  id: string;
  text: string;
  level: number;
}

function slugify(text: string): string {
  return text.toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/[^\w\u4e00-\u9fff-]/g, '')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

function extractHeadings(content: string): HeadingItem[] {
  const headingRegex = /^(#{1,6})\s+(.+)$/gm;
  const headings: HeadingItem[] = [];
  let match;
  while ((match = headingRegex.exec(content)) !== null) {
    const text = match[2].trim();
    const id = slugify(text);
    headings.push({ id, text, level: match[1].length });
  }
  return headings;
}

function groupByCrop(notes: KnowledgeNoteMeta[]): Record<string, KnowledgeNoteMeta[]> {
  const groups: Record<string, KnowledgeNoteMeta[]> = {};
  for (const note of notes) {
    const crop = note.path.split('/')[0];
    if (!groups[crop]) groups[crop] = [];
    groups[crop].push(note);
  }
  return groups;
}

const KnowledgeBase: React.FC = () => {
  const [notes, setNotes] = useState<KnowledgeNoteMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [view, setView] = useState<'home' | 'list' | 'read'>('home');
  const [selectedCrop, setSelectedCrop] = useState<string>('');
  const [selectedNote, setSelectedNote] = useState<KnowledgeNote | null>(null);

  const [noteLoading, setNoteLoading] = useState(false);
  const [tocHeadings, setTocHeadings] = useState<HeadingItem[]>([]);
  const [activeHeadingId, setActiveHeadingId] = useState('');
  const [tocVisible, setTocVisible] = useState(true);
  const [backToTopVisible, setBackToTopVisible] = useState(false);
  const [tocDrawerOpen, setTocDrawerOpen] = useState(false);

  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<KnowledgeNoteMeta[]>([]);

  const contentRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  const cropGroups = useMemo(() => groupByCrop(notes), [notes]);
  const crops = useMemo(() => Object.keys(cropGroups), [cropGroups]);

  // Load notes
  useEffect(() => {
    setLoading(true);
    aiApi.listKnowledgeBase()
      .then(data => setNotes(data.notes))
      .catch(() => setError('加载知识库失败'))
      .finally(() => setLoading(false));
  }, []);

  // Search
  const handleSearch = useCallback((value: string) => {
    setSearchQuery(value);
    if (!value.trim()) {
      setSearchResults([]);
      return;
    }
    const lower = value.toLowerCase();
    setSearchResults(notes.filter(n =>
      n.path.toLowerCase().includes(lower) || n.title.toLowerCase().includes(lower)
    ));
  }, [notes]);

  // Load note
  const loadNote = useCallback(async (path: string) => {
    setNoteLoading(true);
    setSearchQuery('');
    setSearchResults([]);
    try {
      const data = await aiApi.readNote(path);
      const meta = notes.find(n => n.path === path);
      setSelectedNote({
        ...meta!,
        path: data.path,
        content: data.content || '',
        title: meta?.title || path.split('/').pop()?.replace('.md', '') || path,
      });
      setTocHeadings(extractHeadings(data.content || ''));
      setActiveHeadingId('');
      setView('read');
      if (contentRef.current) contentRef.current.scrollTop = 0;
    } catch {
      setSelectedNote(null);
    } finally {
      setNoteLoading(false);
    }
  }, [notes]);

  // TOC intersection observer
  useEffect(() => {
    if (!tocHeadings.length || view !== 'read') return;
    observerRef.current = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) setActiveHeadingId(entry.target.id);
      }
    }, { root: contentRef.current, rootMargin: '-80px 0px -60% 0px' });

    for (const id of tocHeadings.map(h => h.id)) {
      const el = contentRef.current?.querySelector(`#${CSS.escape(id)}`);
      if (el) observerRef.current.observe(el);
    }
    return () => observerRef.current?.disconnect();
  }, [tocHeadings, view]);

  // Scroll listener
  useEffect(() => {
    const el = contentRef.current;
    if (!el) return;
    const onScroll = () => setBackToTopVisible(el.scrollTop > 400);
    el.addEventListener('scroll', onScroll);
    return () => el.removeEventListener('scroll', onScroll);
  }, []);

  const handleBackToTop = useCallback(() => {
    if (contentRef.current) contentRef.current.scrollTo({ top: 0, behavior: 'smooth' });
  }, []);

  const handleTocClick = useCallback((id: string) => {
    setTocDrawerOpen(false);
    const el = contentRef.current?.querySelector(`#${CSS.escape(id)}`);
    if (el) el.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }, []);

  const goHome = useCallback(() => {
    setView('home'); setSelectedNote(null); setSelectedCrop('');
    setSearchQuery(''); setSearchResults([]);
  }, []);

  const goToCrop = useCallback((crop: string) => {
    setSelectedCrop(crop); setView('list');
    setSearchQuery(''); setSearchResults([]);
  }, []);

  const HeadingRenderer = useCallback(({ level, children }: { level: number; children: React.ReactNode }) => {
    const text = React.Children.toArray(children).join('');
    const id = slugify(text);
    const TagName = `h${level}` as keyof React.JSX.IntrinsicElements;
    return <TagName id={id} className={styles.markdownHeading}>{children}</TagName>;
  }, []);

  // ===== HOME VIEW =====
  const renderHome = () => (
    <div className={styles.homeView}>
      <div className={styles.searchBarWrapper}>
        <Search
          placeholder="搜索知识库...（标题、路径、描述）"
          size="large"
          value={searchQuery}
          onChange={e => handleSearch(e.target.value)}
          onFocus={() => { if (searchQuery) handleSearch(searchQuery); }}
          onSearch={handleSearch}
          prefix={<SearchOutlined />}
          allowClear
          autoFocus
          className={styles.searchBar}
        />
        {searchResults.length > 0 && (
          <div className={styles.searchDropdown}>
            <Text type="secondary" className={styles.searchGroupTitle}>
              找到 {searchResults.length} 篇笔记
            </Text>
            {searchResults.slice(0, 12).map(n => (
              <div key={n.path} className={styles.searchResultRow} onClick={() => loadNote(n.path)}>
                <FileTextOutlined className={styles.searchResultIcon} />
                <div className={styles.searchResultInfo}>
                  <Text strong className={styles.searchResultName}>{n.title}</Text>
                  <Text type="secondary" className={styles.searchResultDesc}>{n.path}</Text>
                </div>
                {n.knowledge_type && (
                  <Tag color={TYPE_COLORS[n.knowledge_type] || 'default'} style={{ fontSize: 11 }}>
                    {n.knowledge_type}
                  </Tag>
                )}
              </div>
            ))}
          </div>
        )}
      </div>

      <div className={styles.cropsSection}>
        <div className={styles.cropsGrid}>
          {crops.map(crop => {
            const cropNotes = cropGroups[crop] || [];
            return (
              <Card key={crop} hoverable className={styles.cropCard} onClick={() => goToCrop(crop)}>
                <div className={styles.cropCardInner}>
                  <div className={styles.cropCardIcon}>
                    {CROP_ICONS[crop] || <FolderOutlined />}
                  </div>
                  <div className={styles.cropCardContent}>
                    <Title level={5} className={styles.cropCardTitle}>{crop}</Title>
                    <Text type="secondary" className={styles.cropCardDesc}>
                      {cropNotes.length} 篇种植知识
                    </Text>
                  </div>
                  <RightOutlined className={styles.cropCardArrow} />
                </div>
              </Card>
            );
          })}
        </div>
      </div>
    </div>
  );

  // ===== LIST VIEW =====
  const renderList = () => {
    const cropNotes = cropGroups[selectedCrop] || [];
    const topics = cropNotes.reduce<Record<string, KnowledgeNoteMeta[]>>((acc, note) => {
      const parts = note.path.split('/');
      const topic = parts.length > 1 ? parts[0] : '其他';
      if (!acc[topic]) acc[topic] = [];
      acc[topic].push(note);
      return acc;
    }, {});

    return (
      <div className={styles.listView}>
        <div className={styles.listHeader}>
          <Button type="text" icon={<ArrowLeftOutlined />} onClick={goHome} className={styles.backBtn}>
            返回
          </Button>
          <div className={styles.listTitleArea}>
            <Title level={4} className={styles.listTitle}>{selectedCrop}</Title>
            <Text type="secondary">{cropNotes.length} 篇种植知识</Text>
          </div>
        </div>

        {Object.entries(topics).map(([topic, topicNotes]) => (
          <div key={topic} className={styles.topicGroup}>
            <Title level={5} className={styles.topicGroupTitle}>
              <FolderOutlined /> {topic}
            </Title>
            <div className={styles.articleCards}>
              {topicNotes.sort((a, b) => (a.title || '').localeCompare(b.title || 'zzz')).map(note => (
                <Card key={note.path} hoverable className={styles.articleCard} onClick={() => loadNote(note.path)}>
                  <div className={styles.articleCardHeader}>
                    <FileTextOutlined className={styles.articleCardIcon} />
                    <Text strong className={styles.articleCardTitle}>
                      {note.title || note.path.split('/').pop()?.replace('.md', '')}
                    </Text>
                  </div>
                  <div className={styles.articleCardFooter}>
                    {note.knowledge_type && (
                      <Tag color={TYPE_COLORS[note.knowledge_type] || 'default'} style={{ fontSize: 11 }}>
                        {note.knowledge_type}
                      </Tag>
                    )}
                  </div>
                </Card>
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  // ===== READ VIEW =====
  const renderRead = () => {
    if (!selectedNote) return null;
    const breadcrumb = selectedNote.path.split('/');

    return (
      <div className={styles.readView}>
        <div className={styles.readHeader}>
          <Button type="text" icon={<ArrowLeftOutlined />} onClick={() => setView('list')} className={styles.backBtn}>
            返回目录
          </Button>
          <Space>
            <Button
              type="text" size="small"
              icon={tocVisible ? <LeftOutlined /> : <RightOutlined />}
              onClick={() => setTocVisible(!tocVisible)}
              className={styles.tocToggle}
            >
              目录
            </Button>
            <Button
              type="text" size="small"
              icon={<UnorderedListOutlined />}
              onClick={() => setTocDrawerOpen(true)}
              className={styles.tocDrawerBtn}
            />
          </Space>
        </div>

        <div className={styles.readMain}>
          <div className={styles.readContent} ref={contentRef}>
            {noteLoading ? (
              <div className={styles.loadingCenter}><Spin size="large" /></div>
            ) : (
              <div className={styles.noteView}>
                <div className={styles.breadcrumb}>
                  <Button type="link" size="small" icon={<ArrowLeftOutlined />} onClick={() => setView('list')}>
                    {breadcrumb[0]}
                  </Button>
                  {breadcrumb.slice(1).map((part: string, i: number) => (
                    <React.Fragment key={i}>
                      <span className={styles.breadcrumbSep}>/</span>
                      <Text type="secondary" style={{ fontSize: 12 }}>{part.replace('.md', '')}</Text>
                    </React.Fragment>
                  ))}
                </div>

                {selectedNote.path !== VARIETY_TABLE_PATH && (
                  <Title level={2} className={styles.noteTitle}>{selectedNote.title}</Title>
                )}

                <div className={styles.tags}>
                  {selectedNote.knowledge_type && (
                    <Tag color={TYPE_COLORS[selectedNote.knowledge_type] || 'default'}>{selectedNote.knowledge_type}</Tag>
                  )}
                  {(selectedNote as any)['适用作物'] && (
                    <Tag>{(selectedNote as any)['适用作物']}</Tag>
                  )}
                  {(selectedNote as any)['知识领域'] && (
                    <Tag color="geekblue">{(selectedNote as any)['知识领域']}</Tag>
                  )}
                </div>

                <div className={styles.markdownBody}>
                  {selectedNote.path === VARIETY_TABLE_PATH ? (
                    <VarietyTable />
                  ) : (
                    <ReactMarkdown
                      remarkPlugins={[remarkGfm]}
                      components={{
                        h1: ({ children, ...props }) => <HeadingRenderer level={1} {...props}>{children}</HeadingRenderer>,
                        h2: ({ children, ...props }) => <HeadingRenderer level={2} {...props}>{children}</HeadingRenderer>,
                        h3: ({ children, ...props }) => <HeadingRenderer level={3} {...props}>{children}</HeadingRenderer>,
                        h4: ({ children, ...props }) => <HeadingRenderer level={4} {...props}>{children}</HeadingRenderer>,
                        h5: ({ children, ...props }) => <HeadingRenderer level={5} {...props}>{children}</HeadingRenderer>,
                        h6: ({ children, ...props }) => <HeadingRenderer level={6} {...props}>{children}</HeadingRenderer>,
                      }}
                    >
                      {selectedNote.content || ''}
                    </ReactMarkdown>
                  )}
                </div>
              </div>
            )}
          </div>

          {tocVisible && tocHeadings.length >= 3 && (
            <div className={styles.tocSidebar}>
              <div className={styles.tocHeader}>
                <Text strong className={styles.tocTitle}>目录</Text>
                <Button type="text" size="small" icon={<CloseOutlined />} onClick={() => setTocVisible(false)} />
              </div>
              <Collapse
                size="small" ghost className={styles.tocCollapse}
                items={tocHeadings.filter(h => h.level <= 3).map(h => ({
                  key: h.id,
                  label: <span className={styles.tocItemLabel}>{h.text}</span>,
                  children: h.level > 1 ? (
                    <div className={styles.tocSubList}>
                      {tocHeadings
                        .filter(sub => sub.level === h.level + 1)
                        .map(sub => (
                          <div
                            key={sub.id}
                            className={`${styles.tocItem} ${styles.tocLevel3} ${activeHeadingId === sub.id ? styles.tocItemActive : ''}`}
                            onClick={() => handleTocClick(sub.id)}
                          >
                            {sub.text}
                          </div>
                        ))}
                    </div>
                  ) : null,
                }))}
              />
            </div>
          )}
        </div>

        {backToTopVisible && (
          <Tooltip title="回到顶部" placement="left">
            <Button className={styles.backToTop} icon={<VerticalAlignTopOutlined />} onClick={handleBackToTop} shape="circle" />
          </Tooltip>
        )}

        <Drawer
          title="目录" placement="bottom" open={tocDrawerOpen}
          onClose={() => setTocDrawerOpen(false)} height="50%" className={styles.tocDrawer}
        >
          <div className={styles.tocList}>
            {tocHeadings.map(h => (
              <div
                key={h.id}
                className={`${styles.tocItem} ${styles[`tocLevel${h.level}`]} ${activeHeadingId === h.id ? styles.tocItemActive : ''}`}
                onClick={() => handleTocClick(h.id)}
              >
                {h.text}
              </div>
            ))}
          </div>
        </Drawer>
      </div>
    );
  };

  // ===== MAIN RENDER =====
  return (
    <div className={styles.container}>
      {loading ? (
        <div className={styles.loadingCenter}><Spin size="large" /></div>
      ) : error ? (
        <div className={styles.errorView}>
          <Empty description={error} />
          <Button type="primary" onClick={() => window.location.reload()}>重试</Button>
        </div>
      ) : (
        <>
          {view === 'home' && renderHome()}
          {view === 'list' && renderList()}
          {view === 'read' && renderRead()}
        </>
      )}
    </div>
  );
};

export default KnowledgeBase;
