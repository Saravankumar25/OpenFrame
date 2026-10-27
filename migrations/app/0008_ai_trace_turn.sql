-- What the assistant read from the message before it answered (spec ai-conversational-harness,
-- slice S4). Short codes and numbers only, like every column of the trace: never the message.
--
-- The kind of answer wanted (lookup, list, count, explain, relation, compare, brainstorm, navigate,
-- change, help, chat), its group (facts, narrative, brainstorm, navigate, change, help, chat), the
-- domains the facts live in (budget,schedule), the length class (one_sentence, short, paragraph,
-- brief), what decided it (rule, repair, neighbour, model, fallback), the share of the nearest
-- example messages that agreed (0 to 1), and the time the read took.
-- Rows written before these columns existed read as empty or zero.

ALTER TABLE sys_ai_trace ADD COLUMN answer_type TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN answer_group TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN turn_domains TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN length_class TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN turn_how TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN turn_vote REAL NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN turn_ms INTEGER NOT NULL DEFAULT 0;
