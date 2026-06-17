import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { Input, Tree, Typography, Tag, Empty, Spin, Alert, Button, Tooltip, Drawer } from 'antd';
import { BookOutlined, FolderOutlined, FileOutlined, SearchOutlined, VerticalAlignTopOutlined, CloseOutlined, RightOutlined, MenuOutlined, UnorderedListOutlined } from '@ant-design/icons';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import type { TreeProps } from 'antd';
import { aiApi } from '../../services/api';
import type { KnowledgeNoteMeta, KnowledgeNote } from '../../types';
import styles from './KnowledgeBase.module.css';

const { Text, Title } = Typography;
const { Search } = Input;

const TYPE_COLORS: Record<string, string> = {
  '通用知识': 'green',
  '单一作物': 'blue',
  '品种差异': 'purple',
};

const TOC_MIN_HEADINGS = 3;

interface TreeNode {
  key: string;
  title: string;
  isLeaf: boolean;
  icon: React.ReactNode;
  children?: TreeNode[];
  count?: number;
}

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

function countNotes(nodes: TreeNode[]): number {
  let count = 0;
  for (const node of nodes) {
    if (node.isLeaf) count++;
    if (node.children) count += countNotes(node.children);
  }
  return count;
}

function buildTree(notes: KnowledgeNoteMeta[]): TreeNode[] {
  const root: TreeNode[] = [];

  for (const note of notes) {
    const parts = note.path.split('/');
    let currentChildren = root;

    for (let i = 0; i < parts.length; i++) {
      const isFile = i === parts.length - 1;
      const key = parts.slice(0, i + 1).join('/');

      if (isFile) {
        currentChildren.push({
          key,
          title: note.title || parts[i].replace('.md', ''),
          isLeaf: true,
          icon: <FileOutlined />,
        });
      } else {
        let existing = currentChildren.find(n => n.key === key);
        if (!existing) {
          existing = {
            key,
            title: parts[i],
            isLeaf: false,
            icon: <FolderOutlined />,
            children: [],
          };
          currentChildren.push(existing);
        }
        currentChildren = existing.children!;
      }
    }
  }

  for (const node of root) {
    if (!node.isLeaf && node.children) {
      node.count = countNotes(node.children);
    }
  }

  return root;
}

function getDefaultExpandedKeys(tree: TreeNode[]): string[] {
  return tree.filter(n => !n.isLeaf).map(n => n.key);
}

function getBreadcrumb(path: string): string[] {
  return path.split('/');
}

const SidebarContent: React.FC<{
  notes: KnowledgeNoteMeta[];
  treeData: TreeNode[];
  expandedKeys: React.Key[];
  loading: boolean;
  error: string | null;
  searchMode: boolean;
  searchQuery: string;
  searchResults: KnowledgeNoteMeta[];
  selectedKey: string | null;
  onSearch: (value: string) => void;
  onSearchResultClick: (meta: KnowledgeNoteMeta) => void;
  onTreeSelect: TreeProps['onSelect'];
  onLoadNote: (path: string) => void;
  onExpand: (keys: React.Key[]) => void;
}> = ({
  notes, treeData, expandedKeys, loading, error,
  searchMode, searchQuery, searchResults, selectedKey,
  onSearch, onSearchResultClick, onTreeSelect, onLoadNote,
  onExpand,
}) => (
  <>
    <div className={styles.sidebarHeader}>
      <Title level={5} className={styles.title}>
        <BookOutlined /> 知识库
      </Title>
      <Search
        placeholder="搜索笔记..."
        allowClear
        value={searchQuery}
        onChange={e => onSearch(e.target.value)}
        prefix={<SearchOutlined />}
        className={styles.searchInput}
      />
    </div>

    <div className={styles.treeArea}>
      {loading ? (
        <div className={styles.loadingCenter}><Spin /></div>
      ) : error ? (
        <Alert message={error} type="error" showIcon />
      ) : searchMode ? (
        searchResults.length > 0 ? (
          <div className={styles.searchResultList}>
            <Text type="secondary" className={styles.searchResultCount}>
              找到 {searchResults.length} 篇笔记
            </Text>
            {searchResults.map(n => (
              <div
                key={n.path}
                className={styles.searchResultItem}
                onClick={() => onSearchResultClick(n)}
              >
                <Text strong className={styles.searchResultTitle}>{n.title}</Text>
                <Text type="secondary" ellipsis className={styles.searchResultPath}>{n.path}</Text>
                <div className={styles.searchResultTags}>
                  {n.knowledge_type && <Tag color={TYPE_COLORS[n.knowledge_type] || 'default'} style={{ fontSize: 11, lineHeight: '18px', height: 20 }}>{n.knowledge_type}</Tag>}
                </div>
              </div>
            ))}
          </div>
        ) : (
          <Empty description="无匹配结果" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        )
      ) : treeData.length === 0 ? (
        <Empty description="无笔记" image={Empty.PRESENTED_IMAGE_SIMPLE} />
      ) : (
        <Tree
          treeData={treeData}
          onSelect={onTreeSelect}
          selectedKeys={selectedKey ? [selectedKey] : []}
          expandedKeys={expandedKeys}
          onExpand={onExpand}
          showIcon
          className={styles.tree}
          titleRender={(node: any) => (
            <span
              className={styles.treeNodeLabel}
              onClick={node.isLeaf ? undefined : (e: React.MouseEvent) => {
                e.stopPropagation();
                const isExpanded = expandedKeys.includes(node.key);
                if (isExpanded) {
                  onExpand(expandedKeys.filter(k => k !== node.key));
                } else {
                  onExpand([...expandedKeys, node.key]);
                }
              }}
            >
              <span className={styles.treeNodeTitle}>{node.title}</span>
              {!node.isLeaf && node.count != null && (
                <span className={styles.treeNodeCount}>{node.count}</span>
              )}
            </span>
          )}
        />
      )}
    </div>
  </>
);

const KnowledgeBase: React.FC = () => {
  const [notes, setNotes] = useState<KnowledgeNoteMeta[]>([]);
  const [treeData, setTreeData] = useState<TreeNode[]>([]);
  const [defaultExpandedKeys, setDefaultExpandedKeys] = useState<React.Key[]>([]);
  const [expandedKeys, setExpandedKeys] = useState<React.Key[]>([]);
  const [selectedNote, setSelectedNote] = useState<KnowledgeNote | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [noteLoading, setNoteLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchMode, setSearchMode] = useState(false);
  const [searchResults, setSearchResults] = useState<KnowledgeNoteMeta[]>([]);
  const [tocHeadings, setTocHeadings] = useState<HeadingItem[]>([]);
  const [activeHeadingId, setActiveHeadingId] = useState('');
  const [tocVisible, setTocVisible] = useState(true);
  const [backToTopVisible, setBackToTopVisible] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [tocDrawerOpen, setTocDrawerOpen] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    setLoading(true);
    aiApi.listKnowledgeBase()
      .then(data => {
        setNotes(data.notes);
        const tree = buildTree(data.notes);
        setTreeData(tree);
        const keys = getDefaultExpandedKeys(tree);
        setDefaultExpandedKeys(keys);
        setExpandedKeys(keys);
      })
      .catch(() => setError('加载知识库失败'))
      .finally(() => setLoading(false));
  }, []);

  const loadNote = useCallback(async (path: string) => {
    setNoteLoading(true);
    setSidebarOpen(false);
    try {
      const data = await aiApi.readNote(path);
      const meta = notes.find(n => n.path === path);
      const note: KnowledgeNote = {
        ...meta,
        path: data.path,
        content: data.content || '',
        title: meta?.title || path.split('/').pop()?.replace('.md', '') || path,
      };
      setSelectedNote(note);
      setSelectedKey(path);
      const headings = extractHeadings(note.content || '');
      setTocHeadings(headings);
      setActiveHeadingId('');

      if (contentRef.current) {
        contentRef.current.scrollTop = 0;
      }
    } catch {
      setSelectedNote(null);
    } finally {
      setNoteLoading(false);
    }
  }, [notes]);

  useEffect(() => {
    if (!tocHeadings.length) return;
    const ids = tocHeadings.map(h => h.id);
    observerRef.current = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          setActiveHeadingId(entry.target.id);
        }
      }
    }, { root: contentRef.current, rootMargin: '-80px 0px -60% 0px' });

    for (const id of ids) {
      const el = contentRef.current?.querySelector(`#${CSS.escape(id)}`);
      if (el) observerRef.current.observe(el);
    }

    return () => observerRef.current?.disconnect();
  }, [tocHeadings, selectedNote]);

  useEffect(() => {
    const el = contentRef.current;
    if (!el) return;
    const onScroll = () => {
      setBackToTopVisible(el.scrollTop > 400);
    };
    el.addEventListener('scroll', onScroll);
    return () => el.removeEventListener('scroll', onScroll);
  }, []);

  const handleTreeSelect: TreeProps['onSelect'] = useCallback((selectedKeys) => {
    if (selectedKeys.length > 0) {
      const key = selectedKeys[0] as string;
      const meta = notes.find(n => n.path === key);
      if (meta) {
        setSearchMode(false);
        loadNote(key);
      }
    }
  }, [notes, loadNote]);

  const handleSearch = useCallback((value: string) => {
    setSearchQuery(value);
    if (!value.trim()) {
      setSearchMode(false);
      setSearchResults([]);
      return;
    }
    setSearchMode(true);
    setSelectedNote(null);
    setSelectedKey(null);
    const lower = value.toLowerCase();
    const filtered = notes.filter(n =>
      n.path.toLowerCase().includes(lower) ||
      n.title.toLowerCase().includes(lower)
    );
    setSearchResults(filtered);
  }, [notes]);

  const handleSearchResultClick = useCallback((meta: KnowledgeNoteMeta) => {
    setSearchQuery('');
    setSearchMode(false);
    setSearchResults([]);
    loadNote(meta.path);
  }, [loadNote]);

  const handleBackToTop = useCallback(() => {
    if (contentRef.current) {
      contentRef.current.scrollTo({ top: 0, behavior: 'smooth' });
    }
  }, []);

  const handleTocClick = useCallback((id: string) => {
    setTocDrawerOpen(false);
    const el = contentRef.current?.querySelector(`#${CSS.escape(id)}`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
  }, []);

  const handleExpand = useCallback((keys: React.Key[]) => {
    setExpandedKeys(keys);
  }, []);

  const breadcrumb = useMemo(() => {
    if (!selectedNote) return [];
    return getBreadcrumb(selectedNote.path);
  }, [selectedNote]);

  const HeadingRenderer = useCallback(({ level, children }: { level: number; children: React.ReactNode }) => {
    const text = React.Children.toArray(children).join('');
    const id = slugify(text);
    switch (level) {
      case 1: return <h1 id={id} className={styles.markdownHeading}>{children}</h1>;
      case 2: return <h2 id={id} className={styles.markdownHeading}>{children}</h2>;
      case 3: return <h3 id={id} className={styles.markdownHeading}>{children}</h3>;
      case 4: return <h4 id={id} className={styles.markdownHeading}>{children}</h4>;
      default: return <h5 id={id} className={styles.markdownHeading}>{children}</h5>;
    }
  }, []);

  const sidebarProps = {
    notes, treeData, expandedKeys, loading, error,
    searchMode, searchQuery, searchResults, selectedKey,
    onSearch: handleSearch,
    onSearchResultClick: handleSearchResultClick,
    onTreeSelect: handleTreeSelect,
    onLoadNote: loadNote,
    onExpand: handleExpand,
  };

  return (
    <div className={styles.container}>
      <div className={styles.sidebar}>
        <SidebarContent {...sidebarProps} />
      </div>

      <div className={styles.mobileHeader}>
        <Button
          type="text"
          icon={<MenuOutlined />}
          onClick={() => setSidebarOpen(true)}
          className={styles.menuBtn}
        />
        <Text strong className={styles.mobileTitle}>
          {selectedNote ? selectedNote.title : '知识库'}
        </Text>
        <Button
          type="text"
          icon={<UnorderedListOutlined />}
          onClick={() => setTocDrawerOpen(true)}
          className={`${styles.tocBtn} ${tocHeadings.length < TOC_MIN_HEADINGS ? styles.tocBtnHidden : ''}`}
        />
      </div>

      <Drawer
        title={<span><BookOutlined style={{ marginRight: 8 }} />知识库</span>}
        placement="left"
        open={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
        width={320}
        className={styles.mobileDrawer}
        styles={{ body: { padding: 0 } }}
      >
        <SidebarContent {...sidebarProps} />
      </Drawer>

      <div className={styles.content} ref={contentRef}>
        {noteLoading ? (
          <div className={styles.loadingCenter}><Spin size="large" /></div>
        ) : selectedNote ? (
          <div className={styles.noteView}>
            {breadcrumb.length > 0 && (
              <div className={styles.breadcrumb}>
                <BookOutlined className={styles.breadcrumbIcon} />
                {breadcrumb.map((part, i) => (
                  <React.Fragment key={i}>
                    {i > 0 && <span className={styles.breadcrumbSep}>/</span>}
                    <span className={i < breadcrumb.length - 1 ? styles.breadcrumbLink : styles.breadcrumbCurrent}>
                      {part.replace('.md', '')}
                    </span>
                  </React.Fragment>
                ))}
              </div>
            )}

            <Title level={4} className={styles.noteTitle}>{selectedNote.title}</Title>
            {(() => {
              const meta = selectedNote;
              const knowType = meta.knowledge_type;
              const crop = meta['适用作物'];
              const field = meta['知识领域'];
              return (
                <div className={styles.tags}>
                  {knowType && <Tag color={TYPE_COLORS[knowType] || 'default'}>{knowType}</Tag>}
                  {crop && <Tag>{crop}</Tag>}
                  {field && <Tag color="geekblue">{field}</Tag>}
                  {meta['置信度'] && <Tag color="orange">{meta['置信度']}</Tag>}
                </div>
              );
            })()}

            <div className={styles.markdownBody}>
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
            </div>
          </div>
        ) : (
          <div className={styles.emptyState}>
            <BookOutlined className={styles.emptyIcon} />
            <Text type="secondary">从左侧选择一篇笔记开始阅读</Text>
          </div>
        )}
      </div>

      {selectedNote && tocHeadings.length >= TOC_MIN_HEADINGS && tocVisible && (
        <div className={styles.tocSidebar}>
          <div className={styles.tocHeader}>
            <Text strong className={styles.tocTitle}>目录</Text>
            <Button type="text" size="small" icon={<CloseOutlined />} onClick={() => setTocVisible(false)} />
          </div>
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
        </div>
      )}

      {!tocVisible && tocHeadings.length >= TOC_MIN_HEADINGS && (
        <Tooltip title="显示目录" placement="left">
          <Button
            className={styles.tocToggle}
            icon={<RightOutlined />}
            onClick={() => setTocVisible(true)}
          />
        </Tooltip>
      )}

      {backToTopVisible && (
        <Tooltip title="回到顶部" placement="left">
          <Button
            className={styles.backToTop}
            icon={<VerticalAlignTopOutlined />}
            onClick={handleBackToTop}
            shape="circle"
          />
        </Tooltip>
      )}

      <Drawer
        title="目录"
        placement="bottom"
        open={tocDrawerOpen}
        onClose={() => setTocDrawerOpen(false)}
        height="50%"
        className={styles.tocDrawer}
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

export default KnowledgeBase;
