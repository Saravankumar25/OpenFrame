-- The assistant reads the sections the person picks (ADR-0041): the scope kind and the routing topics
-- of a request are gone. The picks stay in sections_json.
ALTER TABLE sys_ai_trace DROP COLUMN scope_kind;
ALTER TABLE sys_ai_trace DROP COLUMN turn_domains;
