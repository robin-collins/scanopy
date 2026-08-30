-- Service nodes on custom canvases reuse the inventory service-definition icon
-- and can independently place that icon and their label.  Nullable enum/offset
-- columns preserve today's presentation through frontend fallbacks for existing
-- rows; the boolean default makes the detected icon visible by default.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

ALTER TABLE custom_topology_view_nodes
    ADD COLUMN show_service_icon BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN service_icon_position TEXT,
    ADD COLUMN service_icon_url TEXT,
    ADD COLUMN service_label_horizontal_align TEXT,
    ADD COLUMN service_label_vertical_align TEXT,
    ADD COLUMN service_label_offset_x BIGINT,
    ADD COLUMN service_label_offset_y BIGINT,
    ADD CONSTRAINT custom_topology_view_nodes_service_icon_position_check
        CHECK (service_icon_position IS NULL OR service_icon_position IN ('BeforeName', 'AfterName', 'Center')) NOT VALID,
    ADD CONSTRAINT custom_topology_view_nodes_service_icon_url_length_check
        CHECK (service_icon_url IS NULL OR char_length(service_icon_url) <= 2048) NOT VALID,
    ADD CONSTRAINT custom_topology_view_nodes_service_label_horizontal_align_check
        CHECK (service_label_horizontal_align IS NULL OR service_label_horizontal_align IN ('Left', 'Center', 'Right')) NOT VALID,
    ADD CONSTRAINT custom_topology_view_nodes_service_label_vertical_align_check
        CHECK (service_label_vertical_align IS NULL OR service_label_vertical_align IN ('Top', 'Middle', 'Bottom')) NOT VALID,
    ADD CONSTRAINT custom_topology_view_nodes_service_label_offset_x_check
        CHECK (service_label_offset_x IS NULL OR service_label_offset_x BETWEEN -1000 AND 1000) NOT VALID,
    ADD CONSTRAINT custom_topology_view_nodes_service_label_offset_y_check
        CHECK (service_label_offset_y IS NULL OR service_label_offset_y BETWEEN -1000 AND 1000) NOT VALID;
