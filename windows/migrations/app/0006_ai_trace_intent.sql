-- Short codes and numbers only, like every column of the trace. Rows written before these columns
-- existed read as empty or zero.
--
-- The intent the question was closest to (the name of one of the app's built-in intents), how close
-- (cosine, 0 to 1), how far ahead of the next different intent, and whether the router used it.
-- Requests answered before the intent step ran read as an empty intent.
--
-- Whether the person pressed Stop and the request ended early ("Stopped. Nothing was changed.").

ALTER TABLE sys_ai_trace ADD COLUMN intent TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN intent_score REAL NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN intent_margin REAL NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN intent_used INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sys_ai_trace ADD COLUMN cancelled INTEGER NOT NULL DEFAULT 0;
