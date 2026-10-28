-- What the assistant fetched for the answer (spec ai-conversational-harness, slice S5). Numbers only,
-- like every column of the trace: never the facts or the message.
--
-- How many facts the dossier held, its size in tokens (four characters to the token), how many
-- statements it made about what the project does not hold, how many values a correct answer must
-- state, and the time the fetch took.
-- Rows written before these columns existed read as zero.

ALTER TABLE sys_ai_trace ADD COLUMN dossier_facts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN dossier_tokens INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN dossier_missing INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN dossier_required INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN dossier_ms INTEGER NOT NULL DEFAULT 0;
