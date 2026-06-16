import React, { useState, useRef, useEffect, useCallback } from 'react';
import { Input, Button, Typography, Spin, Empty, Alert } from 'antd';
import { SendOutlined, RobotOutlined, UserOutlined, ClearOutlined } from '@ant-design/icons';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { aiApi } from '../../services/api';
import type { ChatMessage, AgentResponse } from '../../types';
import styles from './AgentChat.module.css';

const { Text, Title } = Typography;
const { TextArea } = Input;

const STORAGE_KEY = 'agent-chat-messages';

const WELCOME_MESSAGE: ChatMessage = {
  id: 'welcome',
  role: 'agent',
  content: `👋 你好！我是温室 AI 助手，可以帮你：

- 🌡️ **环境分析** — 查询温室当前状态、温度、湿度等
- 🌱 **作物建议** — 针对特定作物（如番茄、黄瓜）给出栽培建议
- ⚠️ **风险预警** — 恶劣天气应对、病虫害防治
- 🎯 **调控建议** — 通风、灌溉、保温等操作建议

有什么想问的吗？`,
  timestamp: Date.now(),
};

function loadMessages(): ChatMessage[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed: ChatMessage[] = JSON.parse(raw);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    }
  } catch { /* ignore */ }
  return [WELCOME_MESSAGE];
}

function saveMessages(msgs: ChatMessage[]) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(msgs));
  } catch { /* ignore */ }
}

const MAX_HISTORY = 20;

const AgentChat: React.FC = () => {
  const [messages, setMessages] = useState<ChatMessage[]>(loadMessages);
  const messagesRef = useRef(messages);
  messagesRef.current = messages;
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = useCallback(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  const handleSend = useCallback(async (text?: string) => {
    const query = (text || input).trim();
    if (!query || loading) return;

    const userMsg: ChatMessage = {
      id: `user-${Date.now()}`,
      role: 'user',
      content: query,
      timestamp: Date.now(),
    };

    setMessages(prev => {
      const updated = [...prev, userMsg];
      saveMessages(updated);
      return updated;
    });
    setInput('');
    setLoading(true);

    try {
      const history = messagesRef.current
        .filter(m => m.id !== 'welcome')
        .slice(-MAX_HISTORY)
        .map(m => ({ role: m.role, content: m.content }));
      const resp: AgentResponse = await aiApi.agentQuery(query, history);
      const agentMsg: ChatMessage = {
        id: `agent-${Date.now()}`,
        role: 'agent',
        content: resp.answer,
        timestamp: Date.now(),
        data_sources: resp.data_sources,
        follow_up_questions: resp.follow_up_questions,
      };
      setMessages(prev => {
        const updated = [...prev, agentMsg];
        saveMessages(updated);
        return updated;
      });
    } catch (err) {
      const errorMsg: ChatMessage = {
        id: `error-${Date.now()}`,
        role: 'agent',
        content: `❌ 请求失败：${err instanceof Error ? err.message : '未知错误'}`,
        timestamp: Date.now(),
      };
      setMessages(prev => {
        const updated = [...prev, errorMsg];
        saveMessages(updated);
        return updated;
      });
    } finally {
      setLoading(false);
    }
  }, [input, loading]);

  const handleFollowUp = useCallback((question: string) => {
    setInput(question);
    handleSend(question);
  }, [handleSend]);

  const handleClear = useCallback(() => {
    setMessages([WELCOME_MESSAGE]);
    localStorage.removeItem(STORAGE_KEY);
  }, []);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }, [handleSend]);

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <Title level={4} style={{ margin: 0 }}>
          <RobotOutlined style={{ marginRight: 8 }} />
          AI 助手对话
        </Title>
        <Button
          icon={<ClearOutlined />}
          size="small"
          onClick={handleClear}
          disabled={messages.length <= 1}
        >
          清空对话
        </Button>
      </div>

      <div className={styles.messageArea}>
        {messages.map(msg => (
          <div
            key={msg.id}
            className={`${styles.messageRow} ${msg.role === 'user' ? styles.userRow : styles.agentRow}`}
          >
            <div className={`${styles.avatar} ${msg.role === 'user' ? styles.userAvatar : styles.agentAvatar}`}>
              {msg.role === 'user' ? <UserOutlined /> : <RobotOutlined />}
            </div>
            <div className={`${styles.bubble} ${msg.role === 'user' ? styles.userBubble : styles.agentBubble}`}>
              {msg.role === 'agent' ? (
                <ReactMarkdown remarkPlugins={[remarkGfm]}>
                  {msg.content}
                </ReactMarkdown>
              ) : (
                <Text>{msg.content}</Text>
              )}
              {msg.data_sources && msg.data_sources.length > 0 && (
                <div className={styles.sources}>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    数据来源：{msg.data_sources.join('、')}
                  </Text>
                </div>
              )}
              {msg.follow_up_questions && msg.follow_up_questions.length > 0 && (
                <div className={styles.followUps}>
                  {msg.follow_up_questions.map((q, i) => (
                    <Button
                      key={i}
                      type="dashed"
                      size="small"
                      className={styles.followUpBtn}
                      onClick={() => handleFollowUp(q)}
                    >
                      {q}
                    </Button>
                  ))}
                </div>
              )}
            </div>
          </div>
        ))}
        {loading && (
          <div className={`${styles.messageRow} ${styles.agentRow}`}>
            <div className={`${styles.avatar} ${styles.agentAvatar}`}>
              <RobotOutlined />
            </div>
            <div className={`${styles.bubble} ${styles.agentBubble}`}>
              <Spin size="small" style={{ marginRight: 8 }} />
              <Text type="secondary">思考中...</Text>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className={styles.inputArea}>
        <TextArea
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="输入你的问题，按 Enter 发送..."
          autoSize={{ minRows: 1, maxRows: 4 }}
          disabled={loading}
          className={styles.textInput}
        />
        <Button
          type="primary"
          icon={<SendOutlined />}
          onClick={() => handleSend()}
          loading={loading}
          disabled={!input.trim()}
          className={styles.sendBtn}
        >
          发送
        </Button>
      </div>
    </div>
  );
};

export default AgentChat;
