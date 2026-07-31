-- Router/Switch move from being HostOsGroup variants to being Categories
-- (device-role concepts don't belong in an OS-family enum). Point any host
-- currently tagged os_group='Router'/'Switch' at the matching built-in
-- category, then clear the old os_group value. Safe to run even if a row is
-- missed: HostOsGroup::FromStr degrades any unrecognized string to Unknown
-- rather than erroring.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

UPDATE hosts h SET category_id = c.id
FROM categories c
WHERE c.organization_id IS NULL
  AND c.name = h.os_group
  AND h.os_group IN ('Router', 'Switch')
  AND h.category_id IS NULL;

UPDATE hosts SET os_group = NULL WHERE os_group IN ('Router', 'Switch');
