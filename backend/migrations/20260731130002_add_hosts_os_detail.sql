-- Free-text OS detail for display (e.g. "Ubuntu 22.04.3 LTS"), paired with
-- the os_group family enum the same way manufacturer/model pair with
-- category: os_group is the coarse, daemon-branchable value; os_detail is
-- display-only and never read by scan-planning logic.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE hosts ADD COLUMN os_detail TEXT;

COMMENT ON COLUMN hosts.os_detail IS 'Free-text OS detail for display, paired with the os_group family enum';
