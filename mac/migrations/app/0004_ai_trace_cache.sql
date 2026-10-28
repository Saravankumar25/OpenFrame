-- Prompt tokens the engine served from its stored work (llama-server `cache_n`), summed over the
-- request's model calls. A number only, like every column of the trace. Rows written before this
-- column existed read as zero.

ALTER TABLE sys_ai_trace ADD COLUMN cached_tokens INTEGER NOT NULL DEFAULT 0;
