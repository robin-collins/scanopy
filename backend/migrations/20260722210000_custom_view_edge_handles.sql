-- Records which side of each node a manually drawn edge was actually
-- dragged from/to (e.g. "handle-Top", "handle-Right") so re-rendering the
-- edge doesn't fall back to xyflow's default handle for nodes with more
-- than one handle of the same type.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_edges
    ADD COLUMN source_handle TEXT,
    ADD COLUMN target_handle TEXT;
