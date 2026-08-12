#!/usr/bin/env python3
"""
Obsidian Vault TF-IDF 搜索工具
输入自然语言查询，返回最相关的文本块。
"""

import json
import os
import pickle
import sys
from pathlib import Path

import jieba
from sklearn.metrics.pairwise import cosine_similarity

INDEX_DIR = Path(__file__).parent / "vault_index"
VAULT_PATH = os.environ.get(
    "OBSIDIAN_VAULT_PATH",
    "/home/xuhan/文档/agri_node",
)


def load_index():
    """加载索引"""
    matrix_path = INDEX_DIR / "tfidf_matrix.pkl"
    vocab_path = INDEX_DIR / "vocabulary.pkl"
    chunks_path = INDEX_DIR / "chunks.json"

    if not all(p.exists() for p in [matrix_path, vocab_path, chunks_path]):
        print("错误: 索引不存在，请先运行 vectorize_vault.py")
        sys.exit(1)

    with open(matrix_path, "rb") as f:
        tfidf_matrix = pickle.load(f)
    with open(vocab_path, "rb") as f:
        vectorizer = pickle.load(f)
    with open(chunks_path, "r", encoding="utf-8") as f:
        chunks = json.load(f)

    return tfidf_matrix, vectorizer, chunks


def search(query: str, top_k: int = 5) -> list[dict]:
    """搜索最相关的文本块"""
    tfidf_matrix, vectorizer, chunks = load_index()

    # 分词
    query_tokenized = " ".join(jieba.cut(query))
    query_vec = vectorizer.transform([query_tokenized])

    # 余弦相似度
    similarities = cosine_similarity(query_vec, tfidf_matrix).flatten()

    # 排序取 top-k
    top_indices = similarities.argsort()[::-1][:top_k]

    results = []
    for idx in top_indices:
        score = float(similarities[idx])
        if score < 0.01:  # 过滤无意义结果
            continue
        chunk = chunks[idx]
        results.append(
            {
                "file": chunk["file"],
                "chunk_index": chunk["chunk_index"],
                "total_chunks": chunk["total_chunks"],
                "score": round(score, 4),
                "text": chunk["text"],
                "frontmatter": chunk.get("frontmatter", {}),
            }
        )
    return results


def format_result(r: dict, idx: int) -> str:
    """格式化单条结果"""
    meta = r["frontmatter"]
    parts = [
        f"── 结果 {idx + 1} (相似度: {r['score']}) ──",
        f"📄 文件: {r['file']}  (块 {r['chunk_index'] + 1}/{r['total_chunks']})",
    ]
    if meta.get("date"):
        parts.append(f"📅 日期: {meta['date']}")
    if meta.get("type"):
        parts.append(f"🏷️  类型: {meta['type']}")
    if meta.get("category"):
        parts.append(f"📂 分类: {meta['category']}")
    parts.append(f"\n{r['text'][:600]}")
    if len(r["text"]) > 600:
        parts.append("...")
    return "\n".join(parts)


def main():
    if len(sys.argv) < 2:
        print("用法: python3 search_vault.py <查询内容> [top_k]")
        print("示例: python3 search_vault.py 'CS5383C SELC 配置'")
        print("示例: python3 search_vault.py 'RS485 TVS 选型' 3")
        sys.exit(1)

    query = sys.argv[1]
    top_k = int(sys.argv[2]) if len(sys.argv) > 2 else 5

    results = search(query, top_k)

    if not results:
        print(f"未找到与 '{query}' 相关的内容")
        return

    print(f"🔍 查询: {query}")
    print(f"📦 找到 {len(results)} 条相关结果\n")
    for i, r in enumerate(results):
        print(format_result(r, i))
        print()


if __name__ == "__main__":
    main()
