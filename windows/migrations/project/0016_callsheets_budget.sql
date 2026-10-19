-- Call sheets, budget, templates, samples and exports (masterplan area 15; doc 33 sections 5 and 6).
--
-- 1. Lightweight budget: every line gets an Actual amount next to its Planned amount. The existing
--    `amount` column keeps its name and now means Planned, so every amount entered before this
--    migration becomes the Planned amount. Actual starts empty (NULL = not spent or not known yet).
--    Totals and the difference are derived at read time and never stored.
ALTER TABLE budget_line ADD COLUMN actual INTEGER CHECK (actual IS NULL OR actual >= 0);

-- 2. Call sheet status words (owner decision 2026-10-07, ADR-0012 amendment): Draft, Needs refresh,
--    Final. "Needs refresh" is derived from the source fingerprint and the needs_refresh flag, so the
--    stored status is only Draft or Final from now on. Earlier words map onto the new ones:
--    Ready -> Draft; a stored Needs Refresh -> Draft with the flag set; Issued and Superseded -> Final
--    (a replaced revision keeps its superseded_by link, and the screen says which revision replaced it).
UPDATE call_sheet SET status = 'Draft', needs_refresh = 1 WHERE status = 'Needs Refresh';
UPDATE call_sheet SET status = 'Draft' WHERE status = 'Ready';
UPDATE call_sheet SET status = 'Final' WHERE status IN ('Issued', 'Superseded');
