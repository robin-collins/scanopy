-- Contract half of the expand-and-contract pattern started in
-- 20260801120000_custom_topology_baseline_styles.sql: validate the NOT VALID
-- opacity check now that it's safe to take the (trivial, since `opacity` is
-- still NULL on every pre-existing row) validation scan.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    VALIDATE CONSTRAINT custom_topology_view_nodes_opacity_check;
