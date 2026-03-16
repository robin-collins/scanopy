-- Add scan_settings column with default values
ALTER TABLE discovery ADD COLUMN scan_settings JSONB NOT NULL DEFAULT '{}';

-- Migrate probe_raw_socket_ports from discovery_type to scan_settings for Network discovery
UPDATE discovery
SET scan_settings = jsonb_build_object('probe_raw_socket_ports', (discovery_type->>'probe_raw_socket_ports')::boolean)
WHERE discovery_type->>'type' = 'Network'
  AND (discovery_type->>'probe_raw_socket_ports')::boolean = true;

-- Remove probe_raw_socket_ports from discovery_type JSONB
UPDATE discovery
SET discovery_type = discovery_type - 'probe_raw_socket_ports'
WHERE discovery_type ? 'probe_raw_socket_ports';
