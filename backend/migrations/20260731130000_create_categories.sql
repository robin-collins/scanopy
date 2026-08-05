-- Device categories for hosts (Router, Switch, WiFi AP, Printer, ...).
-- Same shared-catalog model as library_objects: `organization_id IS NULL`
-- rows are the seeded built-in catalog (protected from update/delete at the
-- service layer); non-null rows are an organization's own additions.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

CREATE TABLE categories (
    id UUID PRIMARY KEY,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT NOT NULL,
    icon TEXT NOT NULL,
    -- Scan-planning hints the daemon reads when a host is assigned this
    -- category (see HostScanHints) — sensible defaults on built-ins, opt-in
    -- for a user's own categories.
    skip_full_port_scan BOOLEAN NOT NULL DEFAULT false,
    preferred_ports JSONB,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_categories_organization ON categories(organization_id);
-- Two partial indexes rather than one plain UNIQUE(organization_id, name):
-- Postgres treats NULL <> NULL, so a plain unique index would let unlimited
-- built-ins share a name. Split into "unique per org" and "unique among
-- built-ins" instead.
CREATE UNIQUE INDEX idx_categories_org_name ON categories(organization_id, name) WHERE organization_id IS NOT NULL;
CREATE UNIQUE INDEX idx_categories_global_name ON categories(name) WHERE organization_id IS NULL;

INSERT INTO categories (id, organization_id, name, description, color, icon, skip_full_port_scan, preferred_ports, created_at, updated_at) VALUES
    (gen_random_uuid(), NULL, 'Router', 'Router or firewall appliance (Cisco, HP, Fortinet, Ubiquiti, etc.).', 'Purple', 'router', true, '[22,23,80,443]', now(), now()),
    (gen_random_uuid(), NULL, 'Switch', 'Managed or unmanaged network switch (Cisco, HP, TP-Link, UniFi, etc.).', 'Indigo', 'network', true, '[22,23,80,443]', now(), now()),
    (gen_random_uuid(), NULL, 'WiFi AP', 'Wireless access point.', 'Cyan', 'wifi', true, '[22,80,443]', now(), now()),
    (gen_random_uuid(), NULL, 'Printer', 'Network printer or multi-function device.', 'Gray', 'printer', true, '[9100,515,631]', now(), now()),
    (gen_random_uuid(), NULL, 'Camera', 'IP camera or video surveillance device.', 'Gray', 'cctv', true, '[80,443,554]', now(), now()),
    (gen_random_uuid(), NULL, 'Server', 'General-purpose server.', 'Amber', 'server', false, NULL, now(), now()),
    (gen_random_uuid(), NULL, 'Workstation', 'Desktop or laptop workstation.', 'Teal', 'monitor', false, NULL, now(), now()),
    (gen_random_uuid(), NULL, 'NAS', 'Network-attached storage device.', 'Violet', 'hard-drive', false, NULL, now(), now()),
    (gen_random_uuid(), NULL, 'Firewall', 'Dedicated firewall or security appliance.', 'Red', 'shield', true, '[22,443]', now(), now()),
    (gen_random_uuid(), NULL, 'IoT Device', 'Smart-home or other IoT device.', 'Lime', 'circuit-board', true, NULL, now(), now());
