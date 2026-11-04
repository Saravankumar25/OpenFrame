-- The sections a person picked for a request (ADR-0041, ADR-0028 decision 8). A small JSON text of
-- leaf ids, counts, tokens per leaf and the search path: codes and numbers, never the facts or the message.
ALTER TABLE sys_ai_trace ADD COLUMN sections_json TEXT;
