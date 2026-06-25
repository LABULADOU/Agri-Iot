use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::io;

/// 知识库错误
#[derive(Debug)]
pub enum KnowledgeError {
    ReadFailed(String),
    WriteFailed(String),
    NotFound(String),
    IoError(String),
}

impl fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KnowledgeError::ReadFailed(s) => write!(f, "读取知识库失败: {}", s),
            KnowledgeError::WriteFailed(s) => write!(f, "写入知识库失败: {}", s),
            KnowledgeError::NotFound(s) => write!(f, "知识库条目未找到: {}", s),
            KnowledgeError::IoError(s) => write!(f, "文件系统错误: {}", s),
        }
    }
}

impl std::error::Error for KnowledgeError {}

impl From<io::Error> for KnowledgeError {
    fn from(e: io::Error) -> Self {
        KnowledgeError::IoError(e.to_string())
    }
}

/// 搜索结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub title: String,
    pub snippet: String,
    pub note_type: String,
}

/// Obsidian 知识库引擎
pub struct ObsidianKnowledge {
    vault_path: PathBuf,
}

impl ObsidianKnowledge {
    /// 初始化知识库连接
    pub fn new(vault_path: &str) -> Self {
        Self {
            vault_path: PathBuf::from(vault_path),
        }
    }

    /// 获取 vault 根目录
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// 列出 vault 下所有 Markdown 文件的相对路径
    pub fn list_markdown_files(&self) -> Result<Vec<String>, KnowledgeError> {
        let mut files = Vec::new();
        self.collect_md(&self.vault_path, &mut files)?;
        Ok(files)
    }

    fn collect_md(&self, dir: &Path, files: &mut Vec<String>) -> Result<(), KnowledgeError> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.collect_md(&path, files)?;
            } else if path.extension().map_or(false, |e| e == "md") {
                if let Ok(relative) = path.strip_prefix(&self.vault_path) {
                    files.push(relative.display().to_string());
                }
            }
        }
        Ok(())
    }

    /// 安全地解析 vault 内路径，防止路径穿越
    fn safe_path(&self, user_path: &str) -> Result<PathBuf, KnowledgeError> {
        let user_path = user_path.trim_start_matches('/');
        if user_path.starts_with("..") || user_path.contains("../") || user_path.contains("..\\") {
            return Err(KnowledgeError::ReadFailed("路径包含非法字符 (..)".into()));
        }
        let joined = self.vault_path.join(user_path);
        if self.vault_path.exists() {
            let canonical_vault = self.vault_path.canonicalize()
                .map_err(|e| KnowledgeError::ReadFailed(format!("vault 路径解析失败: {}", e)))?;
            if joined.exists() {
                let canonical_joined = joined.canonicalize()
                    .map_err(|e| KnowledgeError::ReadFailed(e.to_string()))?;
                if !canonical_joined.starts_with(&canonical_vault) {
                    return Err(KnowledgeError::ReadFailed("路径超出 vault 范围".into()));
                }
            }
        }
        Ok(joined)
    }

    /// 读取笔记内容
    pub fn read_note(&self, note_path: &str) -> Result<String, KnowledgeError> {
        let full_path = self.safe_path(note_path)?;
        if !full_path.exists() {
            return Err(KnowledgeError::NotFound(note_path.to_string()));
        }
        fs::read_to_string(&full_path).map_err(|e| KnowledgeError::ReadFailed(e.to_string()))
    }

    /// 搜索知识库（遍历目录匹配内容）
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, KnowledgeError> {
        let mut results = Vec::new();
        self.search_dir(&self.vault_path, query, &mut results)?;
        Ok(results)
    }

    fn search_dir(&self, dir: &Path, query: &str, results: &mut Vec<SearchResult>) -> Result<(), KnowledgeError> {
        if !dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.search_dir(&path, query, results)?;
            } else if path.extension().map_or(false, |e| e == "md") {
                self.search_file(&path, query, results)?;
            }
        }
        Ok(())
    }

    fn search_file(&self, path: &Path, query: &str, results: &mut Vec<SearchResult>) -> Result<(), KnowledgeError> {
        let content = fs::read_to_string(path)?;
        if content.to_lowercase().contains(&query.to_lowercase()) {
            let relative = path.strip_prefix(&self.vault_path)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            let title = extract_title(&content).unwrap_or_else(|| {
                path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
            });
            let note_type = path.parent()
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let snippet = extract_snippet(&content, query, 100);
            results.push(SearchResult {
                file_path: relative,
                title,
                snippet,
                note_type,
            });
        }
        Ok(())
    }

    /// 追加调控案例笔记
    pub fn append_case(&self, area_id: &str, id: &str, situation: &str, outcome: &str) -> Result<String, KnowledgeError> {
        let safe_area = area_id.replace('/', "_").replace("..", "_");
        let case_dir = self.vault_path.join("02-Cases").join(&safe_area);
        fs::create_dir_all(&case_dir)?;
        let file_path = case_dir.join(format!("{}.md", id));
        let now = chrono::Utc::now();
        let content = format!(
            r#"---
type: control-case
id: {}
area_id: {}
date: {}
outcome: {}
tags:
  - 调控案例
  - {}
---

# {} - 调控案例

## 初始环境

{situation}

## 执行结果

- 结果: {}

## 相关知识

<!-- AI 和人工补充 -->
"#,
            id, area_id, now.format("%Y-%m-%d"), outcome, area_id,
            now.format("%Y-%m-%d"), outcome
        );
        fs::write(&file_path, content).map_err(|e| KnowledgeError::WriteFailed(e.to_string()))?;
        Ok(file_path.display().to_string())
    }

    /// 生成日常评估笔记
    pub fn write_daily_assessment(&self, area_id: &str, scores: &serde_json::Value) -> Result<String, KnowledgeError> {
        let daily_dir = self.vault_path.join("05-Daily");
        fs::create_dir_all(&daily_dir)?;
        let now = chrono::Utc::now();
        let filename = now.format("%Y-%m-%d").to_string();
        let file_path = daily_dir.join(format!("{}.md", filename));
        let content = format!(
            r#"---
type: daily-assessment
date: {}
area: {}
---

# {} 评估报告

## 区域 {} 评分

```json
{}
```

## 紧急情况

- 无

## 备注

<!-- AI 生成每日评估 -->
"#,
            filename, area_id, filename, area_id, scores
        );
        fs::write(&file_path, content).map_err(|e| KnowledgeError::WriteFailed(e.to_string()))?;
        Ok(file_path.display().to_string())
    }
}

/// 笔记元数据
#[derive(Debug, Clone, serde::Serialize)]
pub struct NoteMetadata {
    pub path: String,
    pub title: String,
    #[serde(flatten)]
    pub frontmatter: HashMap<String, String>,
}

impl ObsidianKnowledge {
    /// 列出 vault 内所有笔记及其元数据
    pub fn list_notes_metadata(&self) -> Result<Vec<NoteMetadata>, KnowledgeError> {
        let mut files = self.list_markdown_files()?;
        files.sort();
        let mut notes = Vec::new();
        for f in &files {
            match self.read_note(f) {
                Ok(content) => {
                    let fm = parse_frontmatter(&content);
                    let title = extract_title(&content)
                        .or_else(|| fm.get("title").cloned())
                        .unwrap_or_default();
                    notes.push(NoteMetadata {
                        path: f.clone(),
                        title,
                        frontmatter: fm,
                    });
                }
                Err(_) => continue,
            }
        }
        Ok(notes)
    }
}

/// 提取 YAML frontmatter（介于 `---` 之间的键值对）
fn parse_frontmatter(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 3 { return map; }
    if lines[0].trim() != "---" { return map; }
    let mut i = 1;
    while i < lines.len() && lines[i].trim() != "---" {
        let line = lines[i].trim();
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_string();
            let value = line[pos+1..].trim().to_string();
            if !key.is_empty() {
                map.insert(key, value);
            }
        }
        i += 1;
    }
    map
}

fn extract_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed.trim_start_matches("# ").to_string());
        }
    }
    None
}

fn extract_snippet(content: &str, query: &str, max_len: usize) -> String {
    let lower = content.to_lowercase();
    let q_lower = query.to_lowercase();
    if let Some(pos) = lower.find(&q_lower) {
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + q_lower.len() + max_len / 2).min(content.len());
        let snippet = &content[start..end];
        let lines: Vec<&str> = snippet.lines().collect();
        let snippet = lines.into_iter().take(3).collect::<Vec<_>>().join("\n");
        if start > 0 { format!("...{}...", snippet) } else { snippet }
    } else {
        let lines: Vec<&str> = content.lines().take(5).collect();
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_vault() -> (tempfile::TempDir, ObsidianKnowledge) {
        let dir = tempfile::tempdir().unwrap();
        let vault = ObsidianKnowledge::new(dir.path().to_str().unwrap());

        // 创建 vault 示例笔记
        let crops_dir = dir.path().join("00-Crops");
        fs::create_dir_all(&crops_dir).unwrap();
        fs::write(crops_dir.join("番茄.md"),
            "# 番茄\n\n## 环境参数\n土壤温度最适: 22°C\n").unwrap();

        let pests_dir = dir.path().join("01-Pests");
        fs::create_dir_all(&pests_dir).unwrap();
        fs::write(pests_dir.join("灰霉病.md"),
            "# 灰霉病\n\n## 触发条件\n湿度 > 80%\n温度 15-25°C").unwrap();

        (dir, vault)
    }

    #[test]
    fn test_read_note() {
        let (_tmp, vault) = setup_test_vault();
        let content = vault.read_note("00-Crops/番茄.md").unwrap();
        assert!(content.contains("番茄"));
    }

    #[test]
    fn test_read_note_not_found() {
        let (_tmp, vault) = setup_test_vault();
        let result = vault.read_note("nonexistent.md");
        assert!(matches!(result, Err(KnowledgeError::NotFound(_))));
    }

    #[test]
    fn test_search_found() {
        let (_tmp, vault) = setup_test_vault();
        let results = vault.search("灰霉病").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.title.contains("灰霉病")));
    }

    #[test]
    fn test_search_empty() {
        let (_tmp, vault) = setup_test_vault();
        let results = vault.search("不存在的关键词").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_append_case() {
        let (_tmp, vault) = setup_test_vault();
        let path = vault.append_case("zone-1", "case-001", "高温天气", "success").unwrap();
        assert!(path.contains("case-001"));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("success"));
    }

    #[test]
    fn test_extract_title() {
        assert_eq!(extract_title("# Hello"), Some("Hello".to_string()));
        assert_eq!(extract_title("## Sub\n# Main"), Some("Main".to_string()));
        assert_eq!(extract_title("No heading"), None);
    }

    #[test]
    fn test_extract_snippet() {
        let content = "# 测试文档\n\n这是一段包含关键词的文本";
        let snippet = extract_snippet(content, "关键词", 100);
        assert!(snippet.contains("关键词"));
    }

    #[test]
    fn test_write_daily_assessment() {
        let (_tmp, vault) = setup_test_vault();
        let scores = serde_json::json!({"overall": 85, "soil_temp": 90});
        let path = vault.write_daily_assessment("zone-1", &scores).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("85"));
    }
}
