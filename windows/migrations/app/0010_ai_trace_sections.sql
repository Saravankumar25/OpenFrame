-- What the section-scoped assistant read for an answer (spec ai-section-scoped-assistant, section
-- 12.3, ADR-0040). Codes and numbers only, like every column of the trace: never the question, the
-- facts or the names of anything in the project.
--
-- sections:    the leaf ids the person picked, comma separated (`screenplay/scenes,production/cast-crew`).
-- leaf_tokens: the dossier tokens each picked leaf took, `leaf=tokens` comma separated.
-- pointer:     the leaf the answer pointed to because the question belongs there, empty when none.
-- retrieval:   how text passages were found: `hybrid`, `fulltext`, or empty when none were wanted.
-- degraded:    1 when retrieval fell back to full text because the project index was not ready.
-- Rows written before these columns existed read as empty and zero.

ALTER TABLE sys_ai_trace ADD COLUMN sections TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN leaf_tokens TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN pointer TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN retrieval TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN degraded INTEGER NOT NULL DEFAULT 0;
