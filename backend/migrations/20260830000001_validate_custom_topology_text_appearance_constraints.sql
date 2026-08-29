-- Validate the text-alignment constraint separately so adding the nullable
-- column does not scan the table while holding its schema-change lock.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_views
    VALIDATE CONSTRAINT custom_topology_views_default_text_align_check;
