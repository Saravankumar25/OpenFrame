-- The assistant reads the sections the person picks (ADR-0041): a request has no scope kind any more.
-- The picks, the draft and the scene it read stay in scope_json, the label in scope_label.
ALTER TABLE ai_request DROP COLUMN scope_kind;
