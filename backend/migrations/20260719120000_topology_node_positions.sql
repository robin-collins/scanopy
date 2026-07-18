-- Persist manual node placement separately from the topology graph. The graph
-- remains derived from live entity data; these rows are best-effort overrides
-- applied by the renderer when their node and parent still exist.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE topology_node_positions (
    topology_id UUID NOT NULL REFERENCES topologies(id) ON DELETE CASCADE,
    view TEXT NOT NULL,
    node_id UUID NOT NULL,
    parent_node_id UUID,
    x BIGINT NOT NULL,
    y BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (topology_id, view, node_id),
    CONSTRAINT topology_node_positions_view_check CHECK (
        view IN ('L2Physical', 'L3Logical', 'Workloads', 'Application')
    ),
    CONSTRAINT topology_node_positions_x_check CHECK (x BETWEEN -1000000 AND 1000000),
    CONSTRAINT topology_node_positions_y_check CHECK (y BETWEEN -1000000 AND 1000000)
);

CREATE INDEX topology_node_positions_topology_view_idx
    ON topology_node_positions (topology_id, view);
