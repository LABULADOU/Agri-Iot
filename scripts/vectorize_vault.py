#!/usr/bin/env python3
"""
Obsidian Vault TF-IDF 向量化索引构建器
扫描 vault 中所有 markdown 文件，分词、向量化、保存索引。
"""

import json
import os
import re
import sys
from pathlib import Path

import jieba
import numpy as np
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.metrics.pairwise import cosine_similarity

# === 配置 ===
VAULT_PATH = os.environ.get(
    "OBSIDIAN_VAULT_PATH",
    "/home/xuhan/文档/agri_node",
)
INDEX_DIR = Path(__file__).parent / "vault_index"
CHUNK_SIZE = 500  # 每个文本块的目标字符数
CHUNK_OVERLAP = 100  # 块间重叠字符数


def parse_frontmatter(text: str) -> dict:
    """解析 YAML frontmatter（轻量级，不依赖 pyyaml）"""
    meta = {}
    m = re.match(r"^---\s*\n(.*?)\n---\s*\n", text, re.DOTALL)
    if not m:
        return meta, text
    raw = m.group(1)
    body = text[m.end():]
    for line in raw.strip().splitlines():
        if ":" in line:
            k, v = line.split(":", 1)
            meta[k.strip()] = v.strip().strip('"').strip("'")
    return meta, body


def chunk_text(text: str, size: int = CHUNK_SIZE, overlap: int = CHUNK_OVERLAP) -> list[str]:
    """按段落 + 字符数切分文本块"""
    paragraphs = re.split(r"\n{2,}", text)
    chunks = []
    buf = ""
    for para in paragraphs:
        para = para.strip()
        if not para:
            continue
        if len(buf) + len(para) + 1 <= size:
            buf = f"{buf}\n{para}" if buf else para
        else:
            if buf:
                chunks.append(buf)
            # 如果单段超长，滑动窗口切分
            if len(para) > size:
                for i in range(0, len(para), size - overlap):
                    chunks.append(para[i : i + size])
                buf = ""
            else:
                buf = para
    if buf:
        chunks.append(buf)
    return chunks


def tokenize_chinese(text: str) -> str:
    """jieba 分词，空格连接"""
    words = jieba.cut(text)
    return " ".join(w.strip() for w in words if w.strip())


def scan_vault(vault_path: str) -> list[dict]:
    """扫描 vault，返回所有文本块"""
    vault = Path(vault_path)
    if not vault.exists():
        print(f"错误: Vault 路径不存在 {vault_path}")
        sys.exit(1)

    all_chunks = []
    md_files = sorted(vault.rglob("*.md"))
    # 跳过 .obsidian 目录
    md_files = [f for f in md_files if ".obsidian" not in f.parts]

    print(f"扫描到 {len(md_files)} 个 markdown 文件")

    for fpath in md_files:
        rel = fpath.relative_to(vault)
        try:
            raw = fpath.read_text(encoding="utf-8")
        except Exception as e:
            print(f"  跳过 {rel}: {e}")
            continue

        meta, body = parse_frontmatter(raw)
        if not body.strip():
            continue

        chunks = chunk_text(body)
        for i, chunk in enumerate(chunks):
            all_chunks.append(
                {
                    "file": str(rel),
                    "chunk_index": i,
                    "total_chunks": len(chunks),
                    "frontmatter": meta,
                    "text": chunk,
                    "text_tokenized": tokenize_chinese(chunk),
                }
            )

    print(f"共生成 {len(all_chunks)} 个文本块")
    return all_chunks


def build_index(chunks: list[dict]) -> None:
    """构建 TF-IDF 索引并保存"""
    INDEX_DIR.mkdir(parents=True, exist_ok=True)

    corpus = [c["text_tokenized"] for c in chunks]
    if not corpus:
        print("错误: 无文本块可索引")
        sys.exit(1)

    print("构建 TF-IDF 矩阵...")
    vectorizer = TfidfVectorizer(
        max_features=10000,
        min_df=1,
        max_df=0.95,
    )
    tfidf_matrix = vectorizer.fit_transform(corpus)

    # 保存
    import pickle

    with open(INDEX_DIR / "tfidf_matrix.pkl", "wb") as f:
        pickle.dump(tfidf_matrix, f)
    with open(INDEX_DIR / "vocabulary.pkl", "wb") as f:
        pickle.dump(vectorizer, f)

    # 保存原始块（不含 tokenized，节省空间）
    serializable = []
    for c in chunks:
        serializable.append(
            {
                "file": c["file"],
                "chunk_index": c["chunk_index"],
                "total_chunks": c["total_chunks"],
                "frontmatter": c["frontmatter"],
                "text": c["text"],
            }
        )
    with open(INDEX_DIR / "chunks.json", "w", encoding="utf-8") as f:
        json.dump(serializable, f, ensure_ascii=False, indent=2)

    print(f"索引已保存到 {INDEX_DIR}/")
    print(f"  矩阵形状: {tfidf_matrix.shape}")
    print(f"  词汇表大小: {len(vectorizer.vocabulary_)}")


def main():
    vault_path = VAULT_PATH
    if len(sys.argv) > 1:
        vault_path = sys.argv[1]

    print(f"Vault 路径: {vault_path}")
    chunks = scan_vault(vault_path)
    build_index(chunks)
    print("完成!")


if __name__ == "__main__":
    main()
