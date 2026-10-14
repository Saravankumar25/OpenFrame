-- 0012: Story Board long scene description and Idea Vault research flags
-- (Story Board and Idea Vault area; docs/engineering/33-production-depth-contract.md section 1;
-- FSD section 107.5). Additive only: no existing column changes.
--
-- story_scene_card.long_description: canonical scene-development text, separate from the short
-- description shown on the minimized card. Searchable and readable by the assistant. NULL means
-- the user has not written one.
--
-- vault_item.research_flag: the user's own verification flag for a research source. NULL means no
-- flag. OpenFrame never sets it on its own (FSD 107.6). The same column is added to the Global Idea
-- Vault by migrations/global/0004_vault_research.sql so one module serves both stores.
--
-- Card Display Settings are view settings, not content: they live in sys_view_state and need no table.

ALTER TABLE story_scene_card ADD COLUMN long_description TEXT;

ALTER TABLE vault_item ADD COLUMN research_flag TEXT
    CHECK (research_flag IS NULL OR research_flag IN ('unchecked','checked','needs_expert_check'));
CREATE INDEX idx_vault_item_research ON vault_item(research_flag) WHERE research_flag IS NOT NULL;
