-- 0004: Idea Vault research flags in the Global Idea Vault (FSD section 107.5).
-- Mirrors the vault_item part of migrations/project/0012_story_card_and_vault_research.sql so the
-- same module code serves both stores. NULL means no flag; OpenFrame never sets it on its own.

ALTER TABLE vault_item ADD COLUMN research_flag TEXT
    CHECK (research_flag IS NULL OR research_flag IN ('unchecked','checked','needs_expert_check'));
CREATE INDEX idx_vault_item_research ON vault_item(research_flag) WHERE research_flag IS NOT NULL;
