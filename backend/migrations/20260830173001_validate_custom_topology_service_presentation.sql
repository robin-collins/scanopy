-- Validate separately so the schema-expansion migration does not take a long
-- access-exclusive lock while checking existing rows.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    VALIDATE CONSTRAINT custom_topology_view_nodes_service_icon_position_check,
    VALIDATE CONSTRAINT custom_topology_view_nodes_service_icon_url_length_check,
    VALIDATE CONSTRAINT custom_topology_view_nodes_service_label_horizontal_align_check,
    VALIDATE CONSTRAINT custom_topology_view_nodes_service_label_vertical_align_check,
    VALIDATE CONSTRAINT custom_topology_view_nodes_service_label_offset_x_check,
    VALIDATE CONSTRAINT custom_topology_view_nodes_service_label_offset_y_check;
