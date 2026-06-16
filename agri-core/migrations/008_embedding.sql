-- Embedding 向量存储表（配合 rig + OpenAI/Claude embedding API）
CREATE TABLE IF NOT EXISTS vec_embeddings (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    source_id TEXT,
    content TEXT NOT NULL,
    embedding TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_vec_embeddings_source_type ON vec_embeddings(source_type);
CREATE INDEX IF NOT EXISTS idx_vec_embeddings_source_id ON vec_embeddings(source_id);
