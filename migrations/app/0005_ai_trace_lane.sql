-- The lane a request took (Direct, Chat, Agent, Recipe or Action), the next best lane and how sure
-- the rule was (0 to 1; 1 for a hard rule, 0.5 when the question matched no clear kind). Short codes
-- and a number only, like every column of the trace. Rows written before this column existed read
-- as an empty lane.

ALTER TABLE sys_ai_trace ADD COLUMN lane TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN runner_up_lane TEXT NOT NULL DEFAULT '';
ALTER TABLE sys_ai_trace ADD COLUMN lane_margin REAL NOT NULL DEFAULT 0;
