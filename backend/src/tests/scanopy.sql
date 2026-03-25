--
-- PostgreSQL database dump
--

\restrict nbHSsyqjKAB9awCdCI6soNmAZ8ViG1xFFpXM8JzZolPOTUULxlzQLvxhUeUL4jg

-- Dumped from database version 17.9
-- Dumped by pg_dump version 17.9

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_topology_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_created_by_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_subnet_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_neighbor_if_entry_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_neighbor_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_host_id_fkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_credential_id_fkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_fkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_binding_id_fkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_tag_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_daemon_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_user_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_api_key_id_fkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_organization_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_service_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_port_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_network_id_fkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_interface_id_fkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_network_id_fkey;
DROP TRIGGER IF EXISTS reassign_daemons_before_user_delete ON public.users;
DROP INDEX IF EXISTS public.idx_users_password_reset_token;
DROP INDEX IF EXISTS public.idx_users_organization;
DROP INDEX IF EXISTS public.idx_users_oidc_provider_subject;
DROP INDEX IF EXISTS public.idx_users_email_verification_token;
DROP INDEX IF EXISTS public.idx_users_email_lower;
DROP INDEX IF EXISTS public.idx_user_network_access_user;
DROP INDEX IF EXISTS public.idx_user_network_access_network;
DROP INDEX IF EXISTS public.idx_user_api_keys_user;
DROP INDEX IF EXISTS public.idx_user_api_keys_org;
DROP INDEX IF EXISTS public.idx_user_api_keys_key;
DROP INDEX IF EXISTS public.idx_user_api_key_network_access_network;
DROP INDEX IF EXISTS public.idx_user_api_key_network_access_key;
DROP INDEX IF EXISTS public.idx_topologies_network;
DROP INDEX IF EXISTS public.idx_tags_organization;
DROP INDEX IF EXISTS public.idx_tags_org_name;
DROP INDEX IF EXISTS public.idx_subnets_network;
DROP INDEX IF EXISTS public.idx_shares_topology;
DROP INDEX IF EXISTS public.idx_shares_network;
DROP INDEX IF EXISTS public.idx_shares_enabled;
DROP INDEX IF EXISTS public.idx_services_network;
DROP INDEX IF EXISTS public.idx_services_host_position;
DROP INDEX IF EXISTS public.idx_services_host_id;
DROP INDEX IF EXISTS public.idx_ports_number;
DROP INDEX IF EXISTS public.idx_ports_network;
DROP INDEX IF EXISTS public.idx_ports_host;
DROP INDEX IF EXISTS public.idx_organizations_stripe_customer;
DROP INDEX IF EXISTS public.idx_networks_owner_organization;
DROP INDEX IF EXISTS public.idx_invites_organization;
DROP INDEX IF EXISTS public.idx_invites_expires_at;
DROP INDEX IF EXISTS public.idx_interfaces_subnet;
DROP INDEX IF EXISTS public.idx_interfaces_network;
DROP INDEX IF EXISTS public.idx_interfaces_host_mac;
DROP INDEX IF EXISTS public.idx_interfaces_host;
DROP INDEX IF EXISTS public.idx_if_entries_network;
DROP INDEX IF EXISTS public.idx_if_entries_neighbor_if_entry;
DROP INDEX IF EXISTS public.idx_if_entries_neighbor_host;
DROP INDEX IF EXISTS public.idx_if_entries_mac_address;
DROP INDEX IF EXISTS public.idx_if_entries_interface;
DROP INDEX IF EXISTS public.idx_if_entries_host;
DROP INDEX IF EXISTS public.idx_hosts_network;
DROP INDEX IF EXISTS public.idx_hosts_chassis_id;
DROP INDEX IF EXISTS public.idx_groups_network;
DROP INDEX IF EXISTS public.idx_group_bindings_group;
DROP INDEX IF EXISTS public.idx_group_bindings_binding;
DROP INDEX IF EXISTS public.idx_entity_tags_tag_id;
DROP INDEX IF EXISTS public.idx_entity_tags_entity;
DROP INDEX IF EXISTS public.idx_discovery_network;
DROP INDEX IF EXISTS public.idx_discovery_daemon;
DROP INDEX IF EXISTS public.idx_daemons_network;
DROP INDEX IF EXISTS public.idx_daemons_api_key;
DROP INDEX IF EXISTS public.idx_daemon_host_id;
DROP INDEX IF EXISTS public.idx_credentials_type;
DROP INDEX IF EXISTS public.idx_credentials_org;
DROP INDEX IF EXISTS public.idx_bindings_service;
DROP INDEX IF EXISTS public.idx_bindings_port;
DROP INDEX IF EXISTS public.idx_bindings_network;
DROP INDEX IF EXISTS public.idx_bindings_interface;
DROP INDEX IF EXISTS public.idx_api_keys_network;
DROP INDEX IF EXISTS public.idx_api_keys_key;
ALTER TABLE IF EXISTS ONLY tower_sessions.session DROP CONSTRAINT IF EXISTS session_pkey;
ALTER TABLE IF EXISTS ONLY public.users DROP CONSTRAINT IF EXISTS users_pkey;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_user_id_network_id_key;
ALTER TABLE IF EXISTS ONLY public.user_network_access DROP CONSTRAINT IF EXISTS user_network_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_keys DROP CONSTRAINT IF EXISTS user_api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_pkey;
ALTER TABLE IF EXISTS ONLY public.user_api_key_network_access DROP CONSTRAINT IF EXISTS user_api_key_network_access_api_key_id_network_id_key;
ALTER TABLE IF EXISTS ONLY public.topologies DROP CONSTRAINT IF EXISTS topologies_pkey;
ALTER TABLE IF EXISTS ONLY public.tags DROP CONSTRAINT IF EXISTS tags_pkey;
ALTER TABLE IF EXISTS ONLY public.subnets DROP CONSTRAINT IF EXISTS subnets_pkey;
ALTER TABLE IF EXISTS ONLY public.shares DROP CONSTRAINT IF EXISTS shares_pkey;
ALTER TABLE IF EXISTS ONLY public.services DROP CONSTRAINT IF EXISTS services_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_pkey;
ALTER TABLE IF EXISTS ONLY public.ports DROP CONSTRAINT IF EXISTS ports_host_id_port_number_protocol_key;
ALTER TABLE IF EXISTS ONLY public.organizations DROP CONSTRAINT IF EXISTS organizations_pkey;
ALTER TABLE IF EXISTS ONLY public.networks DROP CONSTRAINT IF EXISTS networks_pkey;
ALTER TABLE IF EXISTS ONLY public.network_credentials DROP CONSTRAINT IF EXISTS network_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.invites DROP CONSTRAINT IF EXISTS invites_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_pkey;
ALTER TABLE IF EXISTS ONLY public.interfaces DROP CONSTRAINT IF EXISTS interfaces_host_id_subnet_id_ip_address_key;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_pkey;
ALTER TABLE IF EXISTS ONLY public.if_entries DROP CONSTRAINT IF EXISTS if_entries_host_id_if_index_key;
ALTER TABLE IF EXISTS ONLY public.hosts DROP CONSTRAINT IF EXISTS hosts_pkey;
ALTER TABLE IF EXISTS ONLY public.host_credentials DROP CONSTRAINT IF EXISTS host_credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.groups DROP CONSTRAINT IF EXISTS groups_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.group_bindings DROP CONSTRAINT IF EXISTS group_bindings_group_id_binding_id_key;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_pkey;
ALTER TABLE IF EXISTS ONLY public.entity_tags DROP CONSTRAINT IF EXISTS entity_tags_entity_id_entity_type_tag_id_key;
ALTER TABLE IF EXISTS ONLY public.discovery DROP CONSTRAINT IF EXISTS discovery_pkey;
ALTER TABLE IF EXISTS ONLY public.daemons DROP CONSTRAINT IF EXISTS daemons_pkey;
ALTER TABLE IF EXISTS ONLY public.credentials DROP CONSTRAINT IF EXISTS credentials_pkey;
ALTER TABLE IF EXISTS ONLY public.bindings DROP CONSTRAINT IF EXISTS bindings_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_pkey;
ALTER TABLE IF EXISTS ONLY public.api_keys DROP CONSTRAINT IF EXISTS api_keys_key_key;
ALTER TABLE IF EXISTS ONLY public._sqlx_migrations DROP CONSTRAINT IF EXISTS _sqlx_migrations_pkey;
DROP TABLE IF EXISTS tower_sessions.session;
DROP TABLE IF EXISTS public.users;
DROP TABLE IF EXISTS public.user_network_access;
DROP TABLE IF EXISTS public.user_api_keys;
DROP TABLE IF EXISTS public.user_api_key_network_access;
DROP TABLE IF EXISTS public.topologies;
DROP TABLE IF EXISTS public.tags;
DROP TABLE IF EXISTS public.subnets;
DROP TABLE IF EXISTS public.shares;
DROP TABLE IF EXISTS public.services;
DROP TABLE IF EXISTS public.ports;
DROP TABLE IF EXISTS public.organizations;
DROP TABLE IF EXISTS public.networks;
DROP TABLE IF EXISTS public.network_credentials;
DROP TABLE IF EXISTS public.invites;
DROP TABLE IF EXISTS public.interfaces;
DROP TABLE IF EXISTS public.if_entries;
DROP TABLE IF EXISTS public.hosts;
DROP TABLE IF EXISTS public.host_credentials;
DROP TABLE IF EXISTS public.groups;
DROP TABLE IF EXISTS public.group_bindings;
DROP TABLE IF EXISTS public.entity_tags;
DROP TABLE IF EXISTS public.discovery;
DROP TABLE IF EXISTS public.daemons;
DROP TABLE IF EXISTS public.credentials;
DROP TABLE IF EXISTS public.bindings;
DROP TABLE IF EXISTS public.api_keys;
DROP TABLE IF EXISTS public._sqlx_migrations;
DROP FUNCTION IF EXISTS public.reassign_daemons_on_user_delete();
DROP EXTENSION IF EXISTS pgcrypto;
DROP SCHEMA IF EXISTS tower_sessions;
--
-- Name: tower_sessions; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA tower_sessions;


ALTER SCHEMA tower_sessions OWNER TO postgres;

--
-- Name: pgcrypto; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS pgcrypto WITH SCHEMA public;


--
-- Name: EXTENSION pgcrypto; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION pgcrypto IS 'cryptographic functions';


--
-- Name: reassign_daemons_on_user_delete(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.reassign_daemons_on_user_delete() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE
    new_owner_id UUID;
BEGIN
    SELECT id INTO new_owner_id
    FROM users
    WHERE organization_id = OLD.organization_id
      AND permissions = 'Owner'
      AND id != OLD.id
    ORDER BY created_at ASC
    LIMIT 1;

    IF new_owner_id IS NOT NULL THEN
        UPDATE daemons
        SET user_id = new_owner_id
        WHERE user_id = OLD.id;
    END IF;

    RETURN OLD;
END;
$$;


ALTER FUNCTION public.reassign_daemons_on_user_delete() OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO postgres;

--
-- Name: api_keys; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.api_keys (
    id uuid NOT NULL,
    key text NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL,
    plaintext text
);


ALTER TABLE public.api_keys OWNER TO postgres;

--
-- Name: bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.bindings (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    service_id uuid NOT NULL,
    binding_type text NOT NULL,
    interface_id uuid,
    port_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT bindings_binding_type_check CHECK ((binding_type = ANY (ARRAY['Interface'::text, 'Port'::text]))),
    CONSTRAINT valid_binding CHECK ((((binding_type = 'Interface'::text) AND (interface_id IS NOT NULL) AND (port_id IS NULL)) OR ((binding_type = 'Port'::text) AND (port_id IS NOT NULL))))
);


ALTER TABLE public.bindings OWNER TO postgres;

--
-- Name: credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.credentials (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    credential_type jsonb NOT NULL,
    target_ips inet[],
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.credentials OWNER TO postgres;

--
-- Name: daemons; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.daemons (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    last_seen timestamp with time zone,
    capabilities jsonb DEFAULT '{}'::jsonb,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    mode text DEFAULT '"Push"'::text,
    url text NOT NULL,
    name text,
    version text,
    user_id uuid NOT NULL,
    api_key_id uuid,
    is_unreachable boolean DEFAULT false NOT NULL,
    standby boolean DEFAULT false NOT NULL
);


ALTER TABLE public.daemons OWNER TO postgres;

--
-- Name: discovery; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.discovery (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    daemon_id uuid NOT NULL,
    run_type jsonb NOT NULL,
    discovery_type jsonb NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    scan_count integer DEFAULT 0 NOT NULL,
    force_full_scan boolean DEFAULT false NOT NULL,
    pending_credential_ids uuid[] DEFAULT '{}'::uuid[] NOT NULL
);


ALTER TABLE public.discovery OWNER TO postgres;

--
-- Name: entity_tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.entity_tags (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    entity_id uuid NOT NULL,
    entity_type character varying(50) NOT NULL,
    tag_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.entity_tags OWNER TO postgres;

--
-- Name: group_bindings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.group_bindings (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    group_id uuid NOT NULL,
    binding_id uuid NOT NULL,
    "position" integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.group_bindings OWNER TO postgres;

--
-- Name: groups; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.groups (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    source jsonb NOT NULL,
    color text NOT NULL,
    edge_style text DEFAULT '"SmoothStep"'::text,
    group_type text NOT NULL
);


ALTER TABLE public.groups OWNER TO postgres;

--
-- Name: host_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.host_credentials (
    host_id uuid NOT NULL,
    credential_id uuid NOT NULL,
    interface_ids uuid[]
);


ALTER TABLE public.host_credentials OWNER TO postgres;

--
-- Name: hosts; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.hosts (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    hostname text,
    description text,
    source jsonb NOT NULL,
    virtualization jsonb,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    hidden boolean DEFAULT false,
    sys_descr text,
    sys_object_id text,
    sys_location text,
    sys_contact text,
    management_url text,
    chassis_id text,
    manufacturer text,
    model text,
    serial_number text,
    sys_name text
);


ALTER TABLE public.hosts OWNER TO postgres;

--
-- Name: COLUMN hosts.sys_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_descr IS 'SNMP sysDescr.0 - full system description';


--
-- Name: COLUMN hosts.sys_object_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_object_id IS 'SNMP sysObjectID.0 - vendor OID for device identification';


--
-- Name: COLUMN hosts.sys_location; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_location IS 'SNMP sysLocation.0 - physical location';


--
-- Name: COLUMN hosts.sys_contact; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.sys_contact IS 'SNMP sysContact.0 - admin contact info';


--
-- Name: COLUMN hosts.management_url; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.management_url IS 'URL for device management interface (manual or discovered)';


--
-- Name: COLUMN hosts.chassis_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.hosts.chassis_id IS 'LLDP lldpLocChassisId - globally unique device identifier for deduplication';


--
-- Name: if_entries; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.if_entries (
    id uuid NOT NULL,
    host_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    if_index integer NOT NULL,
    if_descr text NOT NULL,
    if_alias text,
    if_type integer NOT NULL,
    speed_bps bigint,
    admin_status integer NOT NULL,
    oper_status integer NOT NULL,
    mac_address macaddr,
    interface_id uuid,
    neighbor_if_entry_id uuid,
    neighbor_host_id uuid,
    lldp_chassis_id jsonb,
    lldp_port_id jsonb,
    lldp_sys_name text,
    lldp_port_desc text,
    lldp_mgmt_addr inet,
    lldp_sys_desc text,
    cdp_device_id text,
    cdp_port_id text,
    cdp_platform text,
    cdp_address inet,
    if_name text,
    fdb_macs jsonb,
    CONSTRAINT chk_neighbor_exclusive CHECK (((neighbor_if_entry_id IS NULL) OR (neighbor_host_id IS NULL)))
);


ALTER TABLE public.if_entries OWNER TO postgres;

--
-- Name: TABLE if_entries; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.if_entries IS 'SNMP ifTable entries - physical/logical interfaces on network devices';


--
-- Name: COLUMN if_entries.if_index; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_index IS 'SNMP ifIndex - stable identifier within device';


--
-- Name: COLUMN if_entries.if_descr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_descr IS 'SNMP ifDescr - interface description (e.g., GigabitEthernet0/1)';


--
-- Name: COLUMN if_entries.if_alias; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_alias IS 'SNMP ifAlias - user-configured description';


--
-- Name: COLUMN if_entries.if_type; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.if_type IS 'SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)';


--
-- Name: COLUMN if_entries.speed_bps; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.speed_bps IS 'Interface speed from ifSpeed/ifHighSpeed in bits per second';


--
-- Name: COLUMN if_entries.admin_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.admin_status IS 'SNMP ifAdminStatus: 1=up, 2=down, 3=testing';


--
-- Name: COLUMN if_entries.oper_status; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.oper_status IS 'SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown';


--
-- Name: COLUMN if_entries.interface_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.interface_id IS 'FK to Interface entity when this ifEntry has an IP address (must be on same host)';


--
-- Name: COLUMN if_entries.neighbor_if_entry_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.neighbor_if_entry_id IS 'Full neighbor resolution: FK to remote IfEntry discovered via LLDP/CDP';


--
-- Name: COLUMN if_entries.neighbor_host_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.neighbor_host_id IS 'Partial neighbor resolution: FK to remote Host when specific port is unknown';


--
-- Name: COLUMN if_entries.lldp_mgmt_addr; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.lldp_mgmt_addr IS 'LLDP remote management address (lldpRemManAddr)';


--
-- Name: COLUMN if_entries.lldp_sys_desc; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.lldp_sys_desc IS 'LLDP remote system description (lldpRemSysDesc)';


--
-- Name: COLUMN if_entries.cdp_device_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_device_id IS 'CDP cache remote device ID (typically hostname)';


--
-- Name: COLUMN if_entries.cdp_port_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_port_id IS 'CDP cache remote port ID string';


--
-- Name: COLUMN if_entries.cdp_platform; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_platform IS 'CDP cache remote device platform (e.g., Cisco IOS)';


--
-- Name: COLUMN if_entries.cdp_address; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.if_entries.cdp_address IS 'CDP cache remote device management IP address';


--
-- Name: interfaces; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.interfaces (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    subnet_id uuid NOT NULL,
    ip_address inet NOT NULL,
    mac_address macaddr,
    name text,
    "position" integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.interfaces OWNER TO postgres;

--
-- Name: invites; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.invites (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text NOT NULL,
    network_ids uuid[] NOT NULL,
    url text NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    send_to text
);


ALTER TABLE public.invites OWNER TO postgres;

--
-- Name: network_credentials; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.network_credentials (
    network_id uuid NOT NULL,
    credential_id uuid NOT NULL
);


ALTER TABLE public.network_credentials OWNER TO postgres;

--
-- Name: networks; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.networks (
    id uuid NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    organization_id uuid NOT NULL
);


ALTER TABLE public.networks OWNER TO postgres;

--
-- Name: COLUMN networks.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.networks.organization_id IS 'The organization that owns and pays for this network';


--
-- Name: organizations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.organizations (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    name text NOT NULL,
    stripe_customer_id text,
    plan jsonb NOT NULL,
    plan_status text,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    onboarding jsonb DEFAULT '[]'::jsonb,
    brevo_company_id text,
    has_payment_method boolean DEFAULT false NOT NULL,
    trial_end_date timestamp with time zone,
    plan_limit_notifications jsonb DEFAULT '{}'::jsonb NOT NULL,
    use_case text
);


ALTER TABLE public.organizations OWNER TO postgres;

--
-- Name: TABLE organizations; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON TABLE public.organizations IS 'Organizations that own networks and have Stripe subscriptions';


--
-- Name: COLUMN organizations.plan; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.organizations.plan IS 'The current billing plan for the organization (e.g., Community, Pro)';


--
-- Name: ports; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.ports (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    host_id uuid NOT NULL,
    port_number integer NOT NULL,
    protocol text NOT NULL,
    port_type text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT ports_port_number_check CHECK (((port_number >= 0) AND (port_number <= 65535))),
    CONSTRAINT ports_protocol_check CHECK ((protocol = ANY (ARRAY['Tcp'::text, 'Udp'::text])))
);


ALTER TABLE public.ports OWNER TO postgres;

--
-- Name: services; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.services (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    name text NOT NULL,
    host_id uuid NOT NULL,
    service_definition text NOT NULL,
    virtualization jsonb,
    source jsonb NOT NULL,
    "position" integer DEFAULT 0 NOT NULL
);


ALTER TABLE public.services OWNER TO postgres;

--
-- Name: shares; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.shares (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    topology_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_by uuid NOT NULL,
    name text NOT NULL,
    is_enabled boolean DEFAULT true NOT NULL,
    expires_at timestamp with time zone,
    password_hash text,
    allowed_domains text[],
    options jsonb NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.shares OWNER TO postgres;

--
-- Name: subnets; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.subnets (
    id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    cidr text NOT NULL,
    name text NOT NULL,
    description text,
    subnet_type text NOT NULL,
    source jsonb NOT NULL
);


ALTER TABLE public.subnets OWNER TO postgres;

--
-- Name: tags; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tags (
    id uuid NOT NULL,
    organization_id uuid NOT NULL,
    name text NOT NULL,
    description text,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    color text NOT NULL
);


ALTER TABLE public.tags OWNER TO postgres;

--
-- Name: topologies; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.topologies (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    network_id uuid NOT NULL,
    name text NOT NULL,
    edges jsonb NOT NULL,
    nodes jsonb NOT NULL,
    options jsonb NOT NULL,
    hosts jsonb NOT NULL,
    subnets jsonb NOT NULL,
    services jsonb NOT NULL,
    groups jsonb NOT NULL,
    is_stale boolean,
    last_refreshed timestamp with time zone DEFAULT now() NOT NULL,
    is_locked boolean,
    locked_at timestamp with time zone,
    locked_by uuid,
    removed_hosts uuid[],
    removed_services uuid[],
    removed_subnets uuid[],
    removed_groups uuid[],
    parent_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    interfaces jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_interfaces uuid[] DEFAULT '{}'::uuid[],
    ports jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_ports uuid[] DEFAULT '{}'::uuid[],
    bindings jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_bindings uuid[] DEFAULT '{}'::uuid[],
    if_entries jsonb DEFAULT '[]'::jsonb NOT NULL,
    removed_if_entries uuid[] DEFAULT '{}'::uuid[],
    entity_tags jsonb DEFAULT '[]'::jsonb NOT NULL
);


ALTER TABLE public.topologies OWNER TO postgres;

--
-- Name: user_api_key_network_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_key_network_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    api_key_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_api_key_network_access OWNER TO postgres;

--
-- Name: user_api_keys; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_api_keys (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    key text NOT NULL,
    user_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    permissions text DEFAULT 'Viewer'::text NOT NULL,
    name text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used timestamp with time zone,
    expires_at timestamp with time zone,
    is_enabled boolean DEFAULT true NOT NULL
);


ALTER TABLE public.user_api_keys OWNER TO postgres;

--
-- Name: user_network_access; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.user_network_access (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    user_id uuid NOT NULL,
    network_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.user_network_access OWNER TO postgres;

--
-- Name: users; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.users (
    id uuid NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    password_hash text,
    oidc_provider text,
    oidc_subject text,
    oidc_linked_at timestamp with time zone,
    email text NOT NULL,
    organization_id uuid NOT NULL,
    permissions text DEFAULT 'Member'::text NOT NULL,
    tags uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    terms_accepted_at timestamp with time zone,
    email_verified boolean DEFAULT false NOT NULL,
    email_verification_token text,
    email_verification_expires timestamp with time zone,
    password_reset_token text,
    password_reset_expires timestamp with time zone,
    pending_email text
);


ALTER TABLE public.users OWNER TO postgres;

--
-- Name: COLUMN users.organization_id; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.users.organization_id IS 'The single organization this user belongs to';


--
-- Name: COLUMN users.permissions; Type: COMMENT; Schema: public; Owner: postgres
--

COMMENT ON COLUMN public.users.permissions IS 'User role within their organization: Owner, Member, Viewer';


--
-- Name: session; Type: TABLE; Schema: tower_sessions; Owner: postgres
--

CREATE TABLE tower_sessions.session (
    id text NOT NULL,
    data bytea NOT NULL,
    expiry_date timestamp with time zone NOT NULL
);


ALTER TABLE tower_sessions.session OWNER TO postgres;

--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
20251006215000	users	2026-03-25 22:23:11.855075+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	4004382
20251006215100	networks	2026-03-25 22:23:11.860447+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	5734396
20251006215151	create hosts	2026-03-25 22:23:11.866555+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	4148573
20251006215155	create subnets	2026-03-25 22:23:11.87106+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	4034738
20251006215201	create groups	2026-03-25 22:23:11.875519+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	4206542
20251006215204	create daemons	2026-03-25 22:23:11.880188+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	4618157
20251006215212	create services	2026-03-25 22:23:11.885366+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	5792395
20251029193448	user-auth	2026-03-25 22:23:11.891664+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	9141928
20251030044828	daemon api	2026-03-25 22:23:11.901144+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	1891258
20251030170438	host-hide	2026-03-25 22:23:11.903337+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	1191642
20251102224919	create discovery	2026-03-25 22:23:11.904867+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	14402720
20251106235621	normalize-daemon-cols	2026-03-25 22:23:11.919876+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	2739963
20251107034459	api keys	2026-03-25 22:23:11.923096+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	10090889
20251107222650	oidc-auth	2026-03-25 22:23:11.933646+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	35614917
20251110181948	orgs-billing	2026-03-25 22:23:11.969771+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	13292782
20251113223656	group-enhancements	2026-03-25 22:23:11.983411+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	1181201
20251117032720	daemon-mode	2026-03-25 22:23:11.984977+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	1359958
20251118143058	set-default-plan	2026-03-25 22:23:11.98668+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	1333799
20251118225043	save-topology	2026-03-25 22:23:11.98841+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	11238699
20251123232748	network-permissions	2026-03-25 22:23:12.00013+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	3724986
20251125001342	billing-updates	2026-03-25 22:23:12.00419+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	3404163
20251128035448	org-onboarding-status	2026-03-25 22:23:12.008042+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	1751043
20251129180942	nfs-consolidate	2026-03-25 22:23:12.010251+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	1440680
20251206052641	discovery-progress	2026-03-25 22:23:12.012143+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	2392951
20251206202200	plan-fix	2026-03-25 22:23:12.014993+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	1069903
20251207061341	daemon-url	2026-03-25 22:23:12.016363+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	3006976
20251210045929	tags	2026-03-25 22:23:12.01978+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	10631787
20251210175035	terms	2026-03-25 22:23:12.030742+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	996234
20251213025048	hash-keys	2026-03-25 22:23:12.032076+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	14590803
20251214050638	scanopy	2026-03-25 22:23:12.047308+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	1647970
20251215215724	topo-scanopy-fix	2026-03-25 22:23:12.049348+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	917035
20251217153736	category rename	2026-03-25 22:23:12.050563+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	1920753
20251218053111	invite-persistence	2026-03-25 22:23:12.05277+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	6394267
20251219211216	create shares	2026-03-25 22:23:12.059732+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	8667262
20251220170928	permissions-cleanup	2026-03-25 22:23:12.068769+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	1892390
20251220180000	commercial-to-community	2026-03-25 22:23:12.071056+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	955687
20251221010000	cleanup subnet type	2026-03-25 22:23:12.07234+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	1071295
20251221020000	remove host target	2026-03-25 22:23:12.073836+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	1333639
20251221030000	user network access	2026-03-25 22:23:12.075644+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	8568336
20251221040000	interfaces table	2026-03-25 22:23:12.084603+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	11526470
20251221050000	ports table	2026-03-25 22:23:12.096593+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	10913477
20251221060000	bindings table	2026-03-25 22:23:12.10801+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	12818610
20251221070000	group bindings	2026-03-25 22:23:12.121199+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	7607218
20251222020000	tag cascade delete	2026-03-25 22:23:12.129224+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	1409962
20251223232524	network remove default	2026-03-25 22:23:12.130977+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	1140485
20251225100000	color enum	2026-03-25 22:23:12.132509+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	1511964
20251227010000	topology snapshot migration	2026-03-25 22:23:12.134523+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	4905507
20251228010000	user api keys	2026-03-25 22:23:12.139848+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	14370469
20251230160000	daemon version and maintainer	2026-03-25 22:23:12.154641+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	3100682
20260103000000	service position	2026-03-25 22:23:12.158145+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	2281071
20260106000000	interface mac index	2026-03-25 22:23:12.16083+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	2116871
20260106204402	entity tags junction	2026-03-25 22:23:12.163371+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	15240302
20260108033856	fix entity tags json format	2026-03-25 22:23:12.179158+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	1679168
20260110000000	email verification	2026-03-25 22:23:12.181191+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	3908943
20260114145808	daemon user fk set null	2026-03-25 22:23:12.185424+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	1216778
20260116010000	snmp credentials	2026-03-25 22:23:12.186979+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	8230220
20260116020000	host snmp fields	2026-03-25 22:23:12.195621+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	4904488
20260116030000	if entries	2026-03-25 22:23:12.200875+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	16032495
20260116100000	daemon api key link	2026-03-25 22:23:12.217355+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	3909483
20260131190000	add hubspot company id	2026-03-25 22:23:12.221658+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	1212701
20260201021238	fix service acronym capitalization	2026-03-25 22:23:12.22322+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	1707512
20260204004436	add entity tags to topology	2026-03-25 22:23:12.225267+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	1218592
20260205120000	billing overhaul	2026-03-25 22:23:12.226819+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	2394744
20260205183207	rename hubspot to brevo	2026-03-25 22:23:12.229619+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	1283193
20260221120000	add plan limit notifications	2026-03-25 22:23:12.231239+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	1356632
20260222120000	add pending email	2026-03-25 22:23:12.232951+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	1057318
20260301120000	add if name to if entries	2026-03-25 22:23:12.234323+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	1037722
20260306002816	cleanup standby	2026-03-25 22:23:12.235708+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	1091342
20260309120000	add org use case	2026-03-25 22:23:12.237159+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	984843
20260313120000	snmp extended discovery	2026-03-25 22:23:12.238441+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	2288985
20260315120000	universal credentials	2026-03-25 22:23:12.241091+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	23006933
20260315120001	discovery scan settings	2026-03-25 22:23:12.264595+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	1315404
20260315120002	backfill org created milestone	2026-03-25 22:23:12.266401+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	999109
20260316120000	fix jsonb null if entries	2026-03-25 22:23:12.267701+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	2120047
20260319120000	rename snmp to snmpv2c	2026-03-25 22:23:12.270228+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	1233600
20260321120000	add discovery scan count	2026-03-25 22:23:12.271844+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	2168568
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext) FROM stdin;
489048a1-8886-4dab-a214-d940435d4f7f	d66155a8ba0962cb28a7df4b14f2acccbcb01d7031b73ba7091e4365a1b021a0	2e6706ca-040f-4e9c-bc34-182f684f5a32	scanopy-daemon-serverpoll API Key	2026-03-25 22:25:29.888651+00	2026-03-25 22:25:29.888651+00	2026-03-25 22:30:33.672592+00	\N	t	scp_d_AXUKPZahrWR2vrCI521osNKGkIBUEquq
413eae43-3d50-4a8b-9df1-1319c454c425	32a7d9f48fc642c681df902a09aa1dde8a957d9cb0adc20ab09294f24291ef5a	2e6706ca-040f-4e9c-bc34-182f684f5a32	Compat Test API Key	2026-03-25 22:30:13.448612+00	2026-03-25 22:30:13.448612+00	2026-03-25 22:30:26.412513+00	\N	t	\N
81da47ed-d251-4ede-a93f-63579a15e8ea	4a6f1bf850cd7c70cfa97f63596cb3994570454a87fa3cdb2e433c1a07c49a45	2e6706ca-040f-4e9c-bc34-182f684f5a32	Integrated Daemon API Key	2026-03-25 22:23:14.37247+00	2026-03-25 22:23:14.37247+00	2026-03-25 22:30:25.132851+00	\N	t	\N
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, interface_id, port_id, created_at, updated_at) FROM stdin;
963a87f1-b6a8-4d9f-aab5-089ae500303f	2e6706ca-040f-4e9c-bc34-182f684f5a32	abae1495-5c8e-4f13-8717-a64f5e857e0e	Port	e5612590-afaf-4ff7-973a-a22456210fd2	318b9ff2-ab97-490b-8ed5-19a195ca767d	2026-01-25 23:12:10.20886+00	2026-01-25 23:12:10.20886+00
\.


--
-- Data for Name: credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.credentials (id, organization_id, name, credential_type, target_ips, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: daemons; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.daemons (id, network_id, host_id, created_at, last_seen, capabilities, updated_at, mode, url, name, version, user_id, api_key_id, is_unreachable, standby) FROM stdin;
5655d066-08fa-4996-b08a-71dfd734ec6a	2e6706ca-040f-4e9c-bc34-182f684f5a32	9eae46ea-5edb-41bd-9955-7134bd96e594	2026-03-25 22:25:29.892594+00	2026-03-25 22:30:12.331145+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["19ab527e-ace0-46fc-9ede-64d7f94916a6", "eaae23dd-ccb7-42a1-a5a0-757d707a04c2"]}	2026-03-25 22:25:29.892594+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.15.2	76ed9952-2433-4146-9fd1-7bdb5747641c	489048a1-8886-4dab-a214-d940435d4f7f	f	f
a8653f7d-4091-46e9-9afe-1cb99e450ea2	2e6706ca-040f-4e9c-bc34-182f684f5a32	78271680-321c-493e-a2ec-308629a7e853	2026-03-25 22:23:14.446462+00	2026-03-25 22:30:26.361049+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["d721ac54-f1d3-4fd5-9009-10797ed60714"]}	2026-03-25 22:23:14.446462+00	"daemon_poll"		scanopy-daemon	0.13.6	76ed9952-2433-4146-9fd1-7bdb5747641c	\N	f	f
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, scan_count, force_full_scan, pending_credential_ids) FROM stdin;
00b3591d-50cd-462a-a324-571e5913ab13	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Scheduled", "enabled": true, "last_run": "2026-03-25T22:23:14.458608203Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "78271680-321c-493e-a2ec-308629a7e853", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-25 22:23:14.456177+00	2026-03-25 22:25:29.459352+00	1	f	{}
c150aa23-9ec7-497c-800d-c2bdedc1d6f6	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "6d7e63b5-e109-4889-a6ce-c16eb21dbaea", "started_at": "2026-03-25T22:23:25.254951305Z", "finished_at": "2026-03-25T22:25:29.446467699Z", "discovery_type": {"type": "Unified", "host_id": "78271680-321c-493e-a2ec-308629a7e853", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "78271680-321c-493e-a2ec-308629a7e853", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-25 22:23:25.254951+00	2026-03-25 22:25:29.457516+00	0	f	{}
7307e367-fc31-47a5-8581-d84a4ef3ca6e	2e6706ca-040f-4e9c-bc34-182f684f5a32	5655d066-08fa-4996-b08a-71dfd734ec6a	{"type": "Scheduled", "enabled": true, "last_run": "2026-03-25T22:25:42.342658151Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-25 22:25:42.340234+00	2026-03-25 22:25:42.342659+00	0	f	{}
5f6a4c40-30ab-4d39-aac5-5987c7d0d123	2e6706ca-040f-4e9c-bc34-182f684f5a32	5655d066-08fa-4996-b08a-71dfd734ec6a	{"type": "AdHoc", "last_run": "2026-03-25T22:25:30.282533032Z"}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	ServerPoll Integration Test Discovery	2026-03-25 22:25:30.273153+00	2026-03-25 22:30:12.488089+00	1	f	{}
9da36e5b-66fb-4973-943e-5efe913961a4	2e6706ca-040f-4e9c-bc34-182f684f5a32	5655d066-08fa-4996-b08a-71dfd734ec6a	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "ef1a4d64-afd3-4762-b6ae-2ff80e8dbc2f", "started_at": "2026-03-25T22:26:12.417430962Z", "finished_at": "2026-03-25T22:30:12.475012669Z", "discovery_type": {"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-25 22:26:12.41743+00	2026-03-25 22:30:12.487191+00	0	f	{}
495e6713-db3c-4794-b3b0-ab21c5b1e381	2e6706ca-040f-4e9c-bc34-182f684f5a32	5655d066-08fa-4996-b08a-71dfd734ec6a	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-03-25T22:30:27.138121501Z", "finished_at": "2026-03-25T22:30:27.145215422Z", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-03-25 22:30:27.138121+00	2026-03-25 22:30:27.151479+00	0	f	{}
2814662e-6c50-4617-ae74-79d2c6c1d206	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-03-25T22:30:27.408772328Z", "finished_at": "2026-03-25T22:30:27.418095754Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:27.408772+00	2026-03-25 22:30:27.425927+00	0	f	{}
60a46cae-a8a4-4f0f-bda8-aa1db3be5c77	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-03-25T22:30:27.682568810Z", "finished_at": "2026-03-25T22:30:27.689690380Z", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Network	2026-03-25 22:30:27.682568+00	2026-03-25 22:30:27.696458+00	0	f	{}
f4508dab-fd1e-4b1a-9546-936cbedabf6c	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-03-25T22:30:27.952050763Z", "finished_at": "2026-03-25T22:30:27.959279715Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:27.95205+00	2026-03-25 22:30:27.966184+00	0	f	{}
e1e4970b-ffc1-41c5-a54e-433e35d47b75	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-03-25T22:30:28.332702601Z", "finished_at": "2026-03-25T22:30:28.339309616Z", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-25 22:30:28.332702+00	2026-03-25 22:30:28.344989+00	0	f	{}
1f2c2cfa-edab-4584-8be8-25a2154009bc	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-03-25T22:30:28.768842702Z", "finished_at": "2026-03-25T22:30:28.776127022Z", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-03-25 22:30:28.768842+00	2026-03-25 22:30:28.783598+00	0	f	{}
794e8728-3c0c-46e6-8d50-6f1a4c1edbbc	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-03-25T22:30:30.133344381Z", "finished_at": "2026-03-25T22:30:30.140679917Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:30.133344+00	2026-03-25 22:30:30.146733+00	0	f	{}
72d71640-c28f-4873-8c9e-90f83050c311	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-03-25T22:30:30.783713800Z", "finished_at": "2026-03-25T22:30:30.790561532Z", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-25 22:30:30.783713+00	2026-03-25 22:30:30.796024+00	0	f	{}
7adc64ef-6a34-492a-8929-1e6431b581f8	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-03-25T22:30:31.502537430Z", "finished_at": "2026-03-25T22:30:31.509955912Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:31.502537+00	2026-03-25 22:30:31.516524+00	0	f	{}
d249a427-d876-45b0-9d20-129dad2ecca0	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-03-25T22:30:28.501536086Z", "finished_at": "2026-03-25T22:30:28.508995538Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:28.501536+00	2026-03-25 22:30:28.515668+00	0	f	{}
2f5b30f7-4225-4c7f-9102-97e36eb23fd6	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-03-25T22:30:29.310325107Z", "finished_at": "2026-03-25T22:30:29.318071553Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:29.310325+00	2026-03-25 22:30:29.324373+00	0	f	{}
58ae1228-e5f6-40ac-b957-10dce8131a70	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-03-25T22:30:30.953379751Z", "finished_at": "2026-03-25T22:30:30.960789059Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:30.953379+00	2026-03-25 22:30:30.967268+00	0	f	{}
4de93ec8-a29f-4ff5-9bb0-2ce2b78af585	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-03-25T22:30:31.776693586Z", "finished_at": "2026-03-25T22:30:31.784048949Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:31.776693+00	2026-03-25 22:30:31.790143+00	0	f	{}
d4117aae-d728-485f-8724-66001d55f7eb	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-03-25T22:30:32.600245832Z", "finished_at": "2026-03-25T22:30:32.608871711Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:32.600245+00	2026-03-25 22:30:32.614824+00	0	f	{}
bb761ef0-2b4e-495a-8225-725365ac1b33	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-03-25T22:30:29.040364094Z", "finished_at": "2026-03-25T22:30:29.047764720Z", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Network	2026-03-25 22:30:29.040364+00	2026-03-25 22:30:29.053757+00	0	f	{}
d5763c76-42a2-4e79-a79b-e984eb060045	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-03-25T22:30:29.583901014Z", "finished_at": "2026-03-25T22:30:29.591910843Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:29.583901+00	2026-03-25 22:30:29.598766+00	0	f	{}
4176c56d-a49f-47cd-a425-eaaf26613aac	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-03-25T22:30:29.859335330Z", "finished_at": "2026-03-25T22:30:29.866829492Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:29.859335+00	2026-03-25 22:30:29.874066+00	0	f	{}
b2f007af-5c6e-46b6-9f32-eae87793bf1e	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-03-25T22:30:30.407040391Z", "finished_at": "2026-03-25T22:30:30.414884451Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:30.40704+00	2026-03-25 22:30:30.421467+00	0	f	{}
84088f5b-7317-4145-860c-ef060d3507e0	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-03-25T22:30:31.228564542Z", "finished_at": "2026-03-25T22:30:31.236226306Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:31.228564+00	2026-03-25 22:30:31.242598+00	0	f	{}
828c605a-1a27-41cd-8580-20da78699625	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-03-25T22:30:32.051865574Z", "finished_at": "2026-03-25T22:30:32.060027726Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:32.051865+00	2026-03-25 22:30:32.066186+00	0	f	{}
369b4ce3-fd2d-430f-bbb2-10acfc0d54af	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-03-25T22:30:33.135143057Z", "finished_at": "2026-03-25T22:30:33.144218042Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:33.135143+00	2026-03-25 22:30:33.150342+00	0	f	{}
42bbf1ef-67a9-4391-b2bf-7bf86f4c2ac5	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-03-25T22:30:32.325631923Z", "finished_at": "2026-03-25T22:30:32.333796733Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-25 22:30:32.325631+00	2026-03-25 22:30:32.340142+00	0	f	{}
23b3165f-a418-441f-806a-6847d03d9ceb	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-03-25T22:30:32.864285719Z", "finished_at": "2026-03-25T22:30:32.872367330Z", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Network	2026-03-25 22:30:32.864285+00	2026-03-25 22:30:32.878363+00	0	f	{}
3307dc04-57f1-4b4e-9f27-7ab7e9ce8b32	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-03-25T22:30:33.662464326Z", "finished_at": "2026-03-25T22:30:33.671009530Z", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Network	2026-03-25 22:30:33.662464+00	2026-03-25 22:30:33.677141+00	0	f	{}
e8c6dab8-8f3d-48ac-b2a1-2078359f1252	2e6706ca-040f-4e9c-bc34-182f684f5a32	a8653f7d-4091-46e9-9afe-1cb99e450ea2	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-03-25T22:30:33.400988169Z", "finished_at": "2026-03-25T22:30:33.408606838Z", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Network	2026-03-25 22:30:33.400988+00	2026-03-25 22:30:33.414483+00	0	f	{}
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at) FROM stdin;
6872820a-2f94-4cf5-826c-28e311d6b9ef	c8d209fa-6113-45fe-87c7-b87514c4ea3e	"Service"	3850f6c5-a9a5-4085-9fe7-46c31aeae7ad	2026-03-25 22:30:12.518799+00
\.


--
-- Data for Name: group_bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.group_bindings (id, group_id, binding_id, "position", created_at) FROM stdin;
\.


--
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.groups (id, network_id, name, description, created_at, updated_at, source, color, edge_style, group_type) FROM stdin;
da86d4fe-9099-404e-ae9b-2f5df6cab50c	2e6706ca-040f-4e9c-bc34-182f684f5a32		\N	2026-03-25 22:30:12.524272+00	2026-03-25 22:30:12.524272+00	{"type": "Manual"}	Yellow	"SmoothStep"	RequestPath
\.


--
-- Data for Name: host_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.host_credentials (host_id, credential_id, interface_ids) FROM stdin;
\.


--
-- Data for Name: hosts; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.hosts (id, network_id, name, hostname, description, source, virtualization, created_at, updated_at, hidden, sys_descr, sys_object_id, sys_location, sys_contact, management_url, chassis_id, manufacturer, model, serial_number, sys_name) FROM stdin;
7c51d243-60ef-4994-b7db-fa41b23b3644	2e6706ca-040f-4e9c-bc34-182f684f5a32	843ded61a959	843ded61a959	Scanopy daemon	{"type": "Discovery", "metadata": [{"date": "2026-01-25T23:12:10.208844254Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2"}]}	null	2026-01-25 23:12:10.208845+00	2026-01-25 23:12:10.208845+00	f	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
\.


--
-- Data for Name: if_entries; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.if_entries (id, host_id, network_id, created_at, updated_at, if_index, if_descr, if_alias, if_type, speed_bps, admin_status, oper_status, mac_address, interface_id, neighbor_if_entry_id, neighbor_host_id, lldp_chassis_id, lldp_port_id, lldp_sys_name, lldp_port_desc, lldp_mgmt_addr, lldp_sys_desc, cdp_device_id, cdp_port_id, cdp_platform, cdp_address, if_name, fdb_macs) FROM stdin;
\.


--
-- Data for Name: interfaces; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.interfaces (id, network_id, host_id, subnet_id, ip_address, mac_address, name, "position", created_at, updated_at) FROM stdin;
e5612590-afaf-4ff7-973a-a22456210fd2	2e6706ca-040f-4e9c-bc34-182f684f5a32	7c51d243-60ef-4994-b7db-fa41b23b3644	d721ac54-f1d3-4fd5-9009-10797ed60714	172.25.0.4	96:d5:6f:59:0f:cc	eth0	0	2026-01-25 23:12:10.202095+00	2026-01-25 23:12:10.202095+00
\.


--
-- Data for Name: invites; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.invites (id, organization_id, permissions, network_ids, url, created_by, created_at, updated_at, expires_at, send_to) FROM stdin;
\.


--
-- Data for Name: network_credentials; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.network_credentials (network_id, credential_id) FROM stdin;
\.


--
-- Data for Name: networks; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.networks (id, name, created_at, updated_at, organization_id) FROM stdin;
2e6706ca-040f-4e9c-bc34-182f684f5a32	My Network	2026-03-25 22:23:14.343394+00	2026-03-25 22:23:14.343394+00	398ff10b-bd33-420b-98ca-a633a8976497
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, brevo_company_id, has_payment_method, trial_end_date, plan_limit_notifications, use_case) FROM stdin;
398ff10b-bd33-420b-98ca-a633a8976497	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "trial_days": 0, "network_cents": null, "included_hosts": null, "included_seats": null, "included_networks": null}	active	2026-03-25 22:23:14.330637+00	2026-03-25 22:23:14.330637+00	["OnboardingModalCompleted", "OrgCreated", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstGroupCreated", "FirstUserApiKeyCreated", "SecondNetworkCreated"]	\N	f	\N	{"hosts": "None", "seats": "None", "networks": "None"}	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at) FROM stdin;
318b9ff2-ab97-490b-8ed5-19a195ca767d	2e6706ca-040f-4e9c-bc34-182f684f5a32	7c51d243-60ef-4994-b7db-fa41b23b3644	60073	Tcp	Custom	2026-01-25 23:12:10.207381+00	2026-01-25 23:12:10.207381+00
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, "position") FROM stdin;
abae1495-5c8e-4f13-8717-a64f5e857e0e	2e6706ca-040f-4e9c-bc34-182f684f5a32	2026-01-25 23:12:10.212728+00	2026-01-25 23:12:10.212728+00	Scanopy Daemon	7c51d243-60ef-4994-b7db-fa41b23b3644	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-01-25T23:12:10.208864212Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2"}]}	0
\.


--
-- Data for Name: shares; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.shares (id, topology_id, network_id, created_by, name, is_enabled, expires_at, password_hash, allowed_domains, options, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: subnets; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.subnets (id, network_id, created_at, updated_at, cidr, name, description, subnet_type, source) FROM stdin;
d721ac54-f1d3-4fd5-9009-10797ed60714	2e6706ca-040f-4e9c-bc34-182f684f5a32	2026-01-25 23:12:10.160483+00	2026-01-25 23:12:10.160483+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery", "metadata": [{"date": "2026-01-25T23:12:10.160444754Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2"}]}
89cf8184-0082-43aa-b040-3f83a62336ce	2e6706ca-040f-4e9c-bc34-182f684f5a32	2026-03-25 22:30:35.425284+00	2026-03-25 22:30:35.425284+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "System"}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
3850f6c5-a9a5-4085-9fe7-46c31aeae7ad	398ff10b-bd33-420b-98ca-a633a8976497	Integration Test Tag	\N	2026-03-25 22:30:12.499725+00	2026-03-25 22:30:12.499725+00	Yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags, interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings, if_entries, removed_if_entries, entity_tags) FROM stdin;
ed627403-2069-4046-9a31-4ce833157ce3	2e6706ca-040f-4e9c-bc34-182f684f5a32	My Topology	[]	[]	{"local": {"tag_filter": {"hidden_host_tag_ids": [], "hidden_subnet_tag_ids": [], "hidden_service_tag_ids": []}, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": ["HostVirtualization"], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": ["OpenPorts"], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[]	[{"id": "d721ac54-f1d3-4fd5-9009-10797ed60714", "cidr": "172.25.0.0/28", "name": "172.25.0.0/28", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2026-01-25T23:12:10.160444754Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "a8653f7d-4091-46e9-9afe-1cb99e450ea2"}]}, "created_at": "2026-01-25T23:12:10.160483Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-01-25T23:12:10.160483Z", "description": null, "subnet_type": "Lan"}, {"id": "89cf8184-0082-43aa-b040-3f83a62336ce", "cidr": "10.1.0.0/24", "name": "Blocked Subnet", "tags": [], "source": {"type": "System"}, "created_at": "2026-03-25T22:30:35.425284Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:30:35.425284Z", "description": null, "subnet_type": "Lan"}]	[{"id": "24ba4749-8a96-4729-8dd3-1d3617982180", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-03-25T22:26:12.425367627Z", "type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}]}, "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "bindings": [{"id": "defcf938-7fd7-47a8-8b99-8e19df92a0f5", "type": "Port", "port_id": "f91815ff-772d-4df0-b2f3-e62f2907022b", "created_at": "2026-03-25T22:26:12.425362Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "24ba4749-8a96-4729-8dd3-1d3617982180", "updated_at": "2026-03-25T22:26:12.425362Z", "interface_id": "bceb06bc-a294-45cf-800a-d0a84d8bd12f"}, {"id": "2b9c6cd4-4a4a-440a-95e5-636d8a01ee7a", "type": "Port", "port_id": "f91815ff-772d-4df0-b2f3-e62f2907022b", "created_at": "2026-03-25T22:26:12.425364Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "24ba4749-8a96-4729-8dd3-1d3617982180", "updated_at": "2026-03-25T22:26:12.425364Z", "interface_id": "c839503e-8bfa-45ea-9164-f408a3acef3a"}], "position": 0, "created_at": "2026-03-25T22:26:12.425368Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:26:12.425368Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "b8b824ed-a5be-4149-988e-b36b1c344c7e", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.4:60073/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-25T22:27:58.183264253Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "35f6eb3b-151c-4b89-9e13-273a520c12a0", "bindings": [{"id": "2e323adb-b6e0-4407-873a-383a709dbca6", "type": "Port", "port_id": "f4cd8b2d-fb72-4d5c-9c2e-a28a0050dd58", "created_at": "2026-03-25T22:27:58.183284Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "b8b824ed-a5be-4149-988e-b36b1c344c7e", "updated_at": "2026-03-25T22:27:58.183284Z", "interface_id": "3d8e6df5-0104-4629-be66-60e980c10c39"}], "position": 0, "created_at": "2026-03-25T22:27:58.183288Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:27:58.183288Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "8230a8c3-3b62-4a48-b7f0-91bcbc881e7b", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-25T22:28:23.260263803Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "c298e422-e52f-446b-b3ae-59575c172972", "bindings": [{"id": "6b765e0b-410e-45f0-a06b-1684ed865668", "type": "Port", "port_id": "bb50407d-0d13-4902-8264-0af4dd844b91", "created_at": "2026-03-25T22:28:23.260282Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "8230a8c3-3b62-4a48-b7f0-91bcbc881e7b", "updated_at": "2026-03-25T22:28:23.260282Z", "interface_id": "a64ff390-eb8c-4755-8acf-40d12bcf88aa"}], "position": 0, "created_at": "2026-03-25T22:28:23.260285Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:28:23.260285Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "bafa4056-bc45-41e0-9a61-db481491d117", "name": "PostgreSQL", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-03-25T22:28:41.561754427Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "2b2d90be-ff12-4f4e-9aa4-d44b9b183d92", "bindings": [{"id": "8eeebdaf-542c-4f3b-9c3a-679d53930000", "type": "Port", "port_id": "88a7a089-f2a9-4c17-9b40-cab5722725ae", "created_at": "2026-03-25T22:28:41.561770Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "bafa4056-bc45-41e0-9a61-db481491d117", "updated_at": "2026-03-25T22:28:41.561770Z", "interface_id": "a8ddde69-769d-4151-9252-be1e0b75d6c7"}], "position": 0, "created_at": "2026-03-25T22:28:41.561775Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:28:41.561775Z", "virtualization": null, "service_definition": "PostgreSQL"}, {"id": "c8d209fa-6113-45fe-87c7-b87514c4ea3e", "name": "Home Assistant", "tags": ["3850f6c5-a9a5-4085-9fe7-46c31aeae7ad"], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.5:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-25T22:28:54.062635217Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "96209138-558a-4c2f-9ec8-65270b2d61a8", "bindings": [{"id": "a239f786-7562-4fca-8060-53a5a5cffb54", "type": "Port", "port_id": "2aef219b-cadc-4294-bc4e-13efa6145ac6", "created_at": "2026-03-25T22:28:54.062652Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "c8d209fa-6113-45fe-87c7-b87514c4ea3e", "updated_at": "2026-03-25T22:28:54.062652Z", "interface_id": "bfc169a8-3604-4841-9789-eef731006875"}], "position": 0, "created_at": "2026-03-25T22:28:54.062656Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:28:54.062656Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "787dfaa0-1d6e-4329-be88-9a81b5b09ddb", "name": "Home Assistant", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-25T22:29:12.336343924Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "019996d3-48ec-4a31-9c47-07733c99e3ef", "bindings": [{"id": "ef732612-01b5-4b33-adda-9faa052c1833", "type": "Port", "port_id": "49c05a91-2b97-4590-ab1a-fd3532e8941e", "created_at": "2026-03-25T22:29:12.336359Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "787dfaa0-1d6e-4329-be88-9a81b5b09ddb", "updated_at": "2026-03-25T22:29:12.336359Z", "interface_id": "17b84b2a-0b6b-42f8-ac0e-015d5eca8eaf"}], "position": 0, "created_at": "2026-03-25T22:29:12.336363Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:29:12.336363Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "953ecd1c-d07e-4c32-9ad3-f23051fb91bd", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-25T22:29:13.811200193Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "019996d3-48ec-4a31-9c47-07733c99e3ef", "bindings": [{"id": "23abf81e-9e73-4313-8ed7-fe7415165758", "type": "Port", "port_id": "9f603cac-04e6-4489-aa16-c6aa87ce7c6a", "created_at": "2026-03-25T22:29:13.811216Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "953ecd1c-d07e-4c32-9ad3-f23051fb91bd", "updated_at": "2026-03-25T22:29:13.811216Z", "interface_id": "17b84b2a-0b6b-42f8-ac0e-015d5eca8eaf"}], "position": 1, "created_at": "2026-03-25T22:29:13.811220Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:29:13.811220Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "d0b2e0aa-014f-4a00-9785-fdb36dd4ab85", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-03-25T22:29:16.020988292Z", "type": "Network", "daemon_id": "5655d066-08fa-4996-b08a-71dfd734ec6a", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "019996d3-48ec-4a31-9c47-07733c99e3ef", "bindings": [{"id": "4c9cd1c9-5d5c-4a06-b5ad-0935d892bb0a", "type": "Port", "port_id": "bf1f97e3-eaa0-4c13-b506-3ed6fdb64083", "created_at": "2026-03-25T22:29:16.021004Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "service_id": "d0b2e0aa-014f-4a00-9785-fdb36dd4ab85", "updated_at": "2026-03-25T22:29:16.021004Z", "interface_id": "17b84b2a-0b6b-42f8-ac0e-015d5eca8eaf"}], "position": 2, "created_at": "2026-03-25T22:29:16.021007Z", "network_id": "2e6706ca-040f-4e9c-bc34-182f684f5a32", "updated_at": "2026-03-25T22:29:16.021007Z", "virtualization": null, "service_definition": "SSH"}]	[]	t	2026-03-25 22:23:14.366659+00	f	\N	\N	{9d5c071d-ba25-4549-9f4e-ee26fecc02c0,0ffe0513-c498-4994-b0cb-11e6220cb5bd,dd6e476b-6d24-44dd-9d70-14580c5a3e80}	{b49c6942-474b-4e4f-8bf7-a6469610fd2e}	{8697aa01-4d31-48aa-812a-2119b66c7867}	{dd2bcfbc-2f29-4df4-bf22-d3c6770a09fa}	\N	2026-03-25 22:23:14.352252+00	2026-03-25 22:23:14.352252+00	{}	[]	{}	[]	{}	[]	{}	[]	{}	[]
\.


--
-- Data for Name: user_api_key_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_key_network_access (id, api_key_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: user_api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_api_keys (id, key, user_id, organization_id, permissions, name, created_at, updated_at, last_used, expires_at, is_enabled) FROM stdin;
\.


--
-- Data for Name: user_network_access; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.user_network_access (id, user_id, network_id, created_at) FROM stdin;
\.


--
-- Data for Name: users; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.users (id, created_at, updated_at, password_hash, oidc_provider, oidc_subject, oidc_linked_at, email, organization_id, permissions, tags, terms_accepted_at, email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires, pending_email) FROM stdin;
76ed9952-2433-4146-9fd1-7bdb5747641c	2026-03-25 22:23:14.333256+00	2026-03-25 22:23:14.333256+00	$argon2id$v=19$m=19456,t=2,p=1$KIofO9RzUqsqNdUumTqvuw$jxFgEDzxDQRrQnT7eyi/yn4WELtrIFF3GHWkKABaf8I	\N	\N	\N	user@gmail.com	398ff10b-bd33-420b-98ca-a633a8976497	Owner	{}	\N	t	\N	\N	\N	\N	\N
d993693f-4ebd-4ba5-a6eb-ed25a76234a1	2026-03-25 22:30:35.069065+00	2026-03-25 22:30:35.069065+00	\N	\N	\N	\N	user@example.com	398ff10b-bd33-420b-98ca-a633a8976497	Owner	{}	\N	f	\N	\N	\N	\N	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
vYRIWw1wJX_4WlnBPA7ttA	\\x93c410b4ed0e3cc1595af87f25700d5b4884bd81a7757365725f6964d92437366564393935322d323433332d343134362d396664312d37626462353734373634316399cd07ea5b16170ece1baac4e4000000	2026-04-01 22:23:14.464176+00
YIPmtahhPCzYqGIcRwQ1yQ	\\x93c410c93504471c62a8d82c3c61a8b5e6836082a7757365725f6964d92437366564393935322d323433332d343134362d396664312d376264623537343736343163ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92436663661346633382d636533652d343530632d613837352d346361343663656630623339ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea5b161e0dce1a94e7d7000000	2026-04-01 22:30:13.445966+00
9qOenrChicuwDJQD4jK8Lg	\\x93c4102ebc32e203940cb0cb89a1b09e9ea3f682ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92439626137393361302d336664622d346338352d623336632d326535306336373637346364ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92437366564393935322d323433332d343134362d396664312d37626462353734373634316399cd07ea5b161e1ace380148cc000000	2026-04-01 22:30:26.939608+00
b5fwbQjnb17sqoMgfRK3Ug	\\x93c41052b7127d2083aaec5e6fe7086df0976f82ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92434333234356439312d646164372d343431382d616432652d663737636435646465373035ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92437366564393935322d323433332d343134362d396664312d37626462353734373634316399cd07ea5b161e22ce0de63b63000000	2026-04-01 22:30:34.233192+00
\.


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: api_keys api_keys_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_key_key UNIQUE (key);


--
-- Name: api_keys api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_pkey PRIMARY KEY (id);


--
-- Name: bindings bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_pkey PRIMARY KEY (id);


--
-- Name: credentials credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_pkey PRIMARY KEY (id);


--
-- Name: daemons daemons_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_pkey PRIMARY KEY (id);


--
-- Name: discovery discovery_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_pkey PRIMARY KEY (id);


--
-- Name: entity_tags entity_tags_entity_id_entity_type_tag_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_entity_id_entity_type_tag_id_key UNIQUE (entity_id, entity_type, tag_id);


--
-- Name: entity_tags entity_tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_pkey PRIMARY KEY (id);


--
-- Name: group_bindings group_bindings_group_id_binding_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_group_id_binding_id_key UNIQUE (group_id, binding_id);


--
-- Name: group_bindings group_bindings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_pkey PRIMARY KEY (id);


--
-- Name: groups groups_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
    ADD CONSTRAINT groups_pkey PRIMARY KEY (id);


--
-- Name: host_credentials host_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_pkey PRIMARY KEY (host_id, credential_id);


--
-- Name: hosts hosts_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_pkey PRIMARY KEY (id);


--
-- Name: if_entries if_entries_host_id_if_index_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_host_id_if_index_key UNIQUE (host_id, if_index);


--
-- Name: if_entries if_entries_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_pkey PRIMARY KEY (id);


--
-- Name: interfaces interfaces_host_id_subnet_id_ip_address_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_subnet_id_ip_address_key UNIQUE (host_id, subnet_id, ip_address);


--
-- Name: interfaces interfaces_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_pkey PRIMARY KEY (id);


--
-- Name: invites invites_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_pkey PRIMARY KEY (id);


--
-- Name: network_credentials network_credentials_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_pkey PRIMARY KEY (network_id, credential_id);


--
-- Name: networks networks_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT networks_pkey PRIMARY KEY (id);


--
-- Name: organizations organizations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.organizations
    ADD CONSTRAINT organizations_pkey PRIMARY KEY (id);


--
-- Name: ports ports_host_id_port_number_protocol_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_port_number_protocol_key UNIQUE (host_id, port_number, protocol);


--
-- Name: ports ports_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_pkey PRIMARY KEY (id);


--
-- Name: services services_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_pkey PRIMARY KEY (id);


--
-- Name: shares shares_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_pkey PRIMARY KEY (id);


--
-- Name: subnets subnets_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_pkey PRIMARY KEY (id);


--
-- Name: tags tags_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_pkey PRIMARY KEY (id);


--
-- Name: topologies topologies_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_pkey PRIMARY KEY (id);


--
-- Name: user_api_key_network_access user_api_key_network_access_api_key_id_network_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_api_key_id_network_id_key UNIQUE (api_key_id, network_id);


--
-- Name: user_api_key_network_access user_api_key_network_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_pkey PRIMARY KEY (id);


--
-- Name: user_api_keys user_api_keys_key_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_key_key UNIQUE (key);


--
-- Name: user_api_keys user_api_keys_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_pkey PRIMARY KEY (id);


--
-- Name: user_network_access user_network_access_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_pkey PRIMARY KEY (id);


--
-- Name: user_network_access user_network_access_user_id_network_id_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_user_id_network_id_key UNIQUE (user_id, network_id);


--
-- Name: users users_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_pkey PRIMARY KEY (id);


--
-- Name: session session_pkey; Type: CONSTRAINT; Schema: tower_sessions; Owner: postgres
--

ALTER TABLE ONLY tower_sessions.session
    ADD CONSTRAINT session_pkey PRIMARY KEY (id);


--
-- Name: idx_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_key ON public.api_keys USING btree (key);


--
-- Name: idx_api_keys_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_api_keys_network ON public.api_keys USING btree (network_id);


--
-- Name: idx_bindings_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_interface ON public.bindings USING btree (interface_id);


--
-- Name: idx_bindings_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_network ON public.bindings USING btree (network_id);


--
-- Name: idx_bindings_port; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_port ON public.bindings USING btree (port_id);


--
-- Name: idx_bindings_service; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_bindings_service ON public.bindings USING btree (service_id);


--
-- Name: idx_credentials_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_org ON public.credentials USING btree (organization_id);


--
-- Name: idx_credentials_type; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_credentials_type ON public.credentials USING btree (((credential_type ->> 'type'::text)));


--
-- Name: idx_daemon_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemon_host_id ON public.daemons USING btree (host_id);


--
-- Name: idx_daemons_api_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_api_key ON public.daemons USING btree (api_key_id) WHERE (api_key_id IS NOT NULL);


--
-- Name: idx_daemons_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_daemons_network ON public.daemons USING btree (network_id);


--
-- Name: idx_discovery_daemon; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_daemon ON public.discovery USING btree (daemon_id);


--
-- Name: idx_discovery_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_discovery_network ON public.discovery USING btree (network_id);


--
-- Name: idx_entity_tags_entity; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_entity ON public.entity_tags USING btree (entity_id, entity_type);


--
-- Name: idx_entity_tags_tag_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_entity_tags_tag_id ON public.entity_tags USING btree (tag_id);


--
-- Name: idx_group_bindings_binding; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_group_bindings_binding ON public.group_bindings USING btree (binding_id);


--
-- Name: idx_group_bindings_group; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_group_bindings_group ON public.group_bindings USING btree (group_id);


--
-- Name: idx_groups_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_groups_network ON public.groups USING btree (network_id);


--
-- Name: idx_hosts_chassis_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_chassis_id ON public.hosts USING btree (chassis_id);


--
-- Name: idx_hosts_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_hosts_network ON public.hosts USING btree (network_id);


--
-- Name: idx_if_entries_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_host ON public.if_entries USING btree (host_id);


--
-- Name: idx_if_entries_interface; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_interface ON public.if_entries USING btree (interface_id);


--
-- Name: idx_if_entries_mac_address; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_mac_address ON public.if_entries USING btree (mac_address);


--
-- Name: idx_if_entries_neighbor_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_neighbor_host ON public.if_entries USING btree (neighbor_host_id);


--
-- Name: idx_if_entries_neighbor_if_entry; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_neighbor_if_entry ON public.if_entries USING btree (neighbor_if_entry_id);


--
-- Name: idx_if_entries_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_if_entries_network ON public.if_entries USING btree (network_id);


--
-- Name: idx_interfaces_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host ON public.interfaces USING btree (host_id);


--
-- Name: idx_interfaces_host_mac; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_host_mac ON public.interfaces USING btree (host_id, mac_address) WHERE (mac_address IS NOT NULL);


--
-- Name: idx_interfaces_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_network ON public.interfaces USING btree (network_id);


--
-- Name: idx_interfaces_subnet; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_interfaces_subnet ON public.interfaces USING btree (subnet_id);


--
-- Name: idx_invites_expires_at; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_expires_at ON public.invites USING btree (expires_at);


--
-- Name: idx_invites_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_invites_organization ON public.invites USING btree (organization_id);


--
-- Name: idx_networks_owner_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_networks_owner_organization ON public.networks USING btree (organization_id);


--
-- Name: idx_organizations_stripe_customer; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_organizations_stripe_customer ON public.organizations USING btree (stripe_customer_id);


--
-- Name: idx_ports_host; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_host ON public.ports USING btree (host_id);


--
-- Name: idx_ports_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_network ON public.ports USING btree (network_id);


--
-- Name: idx_ports_number; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_ports_number ON public.ports USING btree (port_number);


--
-- Name: idx_services_host_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_id ON public.services USING btree (host_id);


--
-- Name: idx_services_host_position; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_host_position ON public.services USING btree (host_id, "position");


--
-- Name: idx_services_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_services_network ON public.services USING btree (network_id);


--
-- Name: idx_shares_enabled; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_enabled ON public.shares USING btree (is_enabled) WHERE (is_enabled = true);


--
-- Name: idx_shares_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_network ON public.shares USING btree (network_id);


--
-- Name: idx_shares_topology; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_shares_topology ON public.shares USING btree (topology_id);


--
-- Name: idx_subnets_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_subnets_network ON public.subnets USING btree (network_id);


--
-- Name: idx_tags_org_name; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_tags_org_name ON public.tags USING btree (organization_id, name);


--
-- Name: idx_tags_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_tags_organization ON public.tags USING btree (organization_id);


--
-- Name: idx_topologies_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_topologies_network ON public.topologies USING btree (network_id);


--
-- Name: idx_user_api_key_network_access_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_network_access_key ON public.user_api_key_network_access USING btree (api_key_id);


--
-- Name: idx_user_api_key_network_access_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_key_network_access_network ON public.user_api_key_network_access USING btree (network_id);


--
-- Name: idx_user_api_keys_key; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_key ON public.user_api_keys USING btree (key);


--
-- Name: idx_user_api_keys_org; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_org ON public.user_api_keys USING btree (organization_id);


--
-- Name: idx_user_api_keys_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_api_keys_user ON public.user_api_keys USING btree (user_id);


--
-- Name: idx_user_network_access_network; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_network_access_network ON public.user_network_access USING btree (network_id);


--
-- Name: idx_user_network_access_user; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_user_network_access_user ON public.user_network_access USING btree (user_id);


--
-- Name: idx_users_email_lower; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_email_lower ON public.users USING btree (lower(email));


--
-- Name: idx_users_email_verification_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_email_verification_token ON public.users USING btree (email_verification_token) WHERE (email_verification_token IS NOT NULL);


--
-- Name: idx_users_oidc_provider_subject; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX idx_users_oidc_provider_subject ON public.users USING btree (oidc_provider, oidc_subject) WHERE ((oidc_provider IS NOT NULL) AND (oidc_subject IS NOT NULL));


--
-- Name: idx_users_organization; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_organization ON public.users USING btree (organization_id);


--
-- Name: idx_users_password_reset_token; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX idx_users_password_reset_token ON public.users USING btree (password_reset_token) WHERE (password_reset_token IS NOT NULL);


--
-- Name: users reassign_daemons_before_user_delete; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER reassign_daemons_before_user_delete BEFORE DELETE ON public.users FOR EACH ROW EXECUTE FUNCTION public.reassign_daemons_on_user_delete();


--
-- Name: api_keys api_keys_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_port_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_port_id_fkey FOREIGN KEY (port_id) REFERENCES public.ports(id) ON DELETE CASCADE;


--
-- Name: bindings bindings_service_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.bindings
    ADD CONSTRAINT bindings_service_id_fkey FOREIGN KEY (service_id) REFERENCES public.services(id) ON DELETE CASCADE;


--
-- Name: credentials credentials_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.credentials
    ADD CONSTRAINT credentials_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.api_keys(id) ON DELETE SET NULL;


--
-- Name: daemons daemons_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: daemons daemons_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.daemons
    ADD CONSTRAINT daemons_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id);


--
-- Name: discovery discovery_daemon_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_daemon_id_fkey FOREIGN KEY (daemon_id) REFERENCES public.daemons(id) ON DELETE CASCADE;


--
-- Name: discovery discovery_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.discovery
    ADD CONSTRAINT discovery_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: entity_tags entity_tags_tag_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.entity_tags
    ADD CONSTRAINT entity_tags_tag_id_fkey FOREIGN KEY (tag_id) REFERENCES public.tags(id) ON DELETE CASCADE;


--
-- Name: group_bindings group_bindings_binding_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_binding_id_fkey FOREIGN KEY (binding_id) REFERENCES public.bindings(id) ON DELETE CASCADE;


--
-- Name: group_bindings group_bindings_group_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.group_bindings
    ADD CONSTRAINT group_bindings_group_id_fkey FOREIGN KEY (group_id) REFERENCES public.groups(id) ON DELETE CASCADE;


--
-- Name: groups groups_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.groups
    ADD CONSTRAINT groups_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: host_credentials host_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: host_credentials host_credentials_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.host_credentials
    ADD CONSTRAINT host_credentials_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: hosts hosts_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.hosts
    ADD CONSTRAINT hosts_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: if_entries if_entries_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: if_entries if_entries_interface_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_interface_id_fkey FOREIGN KEY (interface_id) REFERENCES public.interfaces(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_neighbor_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_neighbor_host_id_fkey FOREIGN KEY (neighbor_host_id) REFERENCES public.hosts(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_neighbor_if_entry_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_neighbor_if_entry_id_fkey FOREIGN KEY (neighbor_if_entry_id) REFERENCES public.if_entries(id) ON DELETE SET NULL;


--
-- Name: if_entries if_entries_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.if_entries
    ADD CONSTRAINT if_entries_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: interfaces interfaces_subnet_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.interfaces
    ADD CONSTRAINT interfaces_subnet_id_fkey FOREIGN KEY (subnet_id) REFERENCES public.subnets(id) ON DELETE CASCADE;


--
-- Name: invites invites_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: invites invites_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.invites
    ADD CONSTRAINT invites_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: network_credentials network_credentials_credential_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_credential_id_fkey FOREIGN KEY (credential_id) REFERENCES public.credentials(id) ON DELETE CASCADE;


--
-- Name: network_credentials network_credentials_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.network_credentials
    ADD CONSTRAINT network_credentials_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: networks organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.networks
    ADD CONSTRAINT organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: ports ports_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: ports ports_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.ports
    ADD CONSTRAINT ports_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: services services_host_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_host_id_fkey FOREIGN KEY (host_id) REFERENCES public.hosts(id) ON DELETE CASCADE;


--
-- Name: services services_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.services
    ADD CONSTRAINT services_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: shares shares_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: shares shares_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: shares shares_topology_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.shares
    ADD CONSTRAINT shares_topology_id_fkey FOREIGN KEY (topology_id) REFERENCES public.topologies(id) ON DELETE CASCADE;


--
-- Name: subnets subnets_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.subnets
    ADD CONSTRAINT subnets_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: tags tags_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: topologies topologies_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.topologies
    ADD CONSTRAINT topologies_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_api_key_network_access user_api_key_network_access_api_key_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_api_key_id_fkey FOREIGN KEY (api_key_id) REFERENCES public.user_api_keys(id) ON DELETE CASCADE;


--
-- Name: user_api_key_network_access user_api_key_network_access_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_key_network_access
    ADD CONSTRAINT user_api_key_network_access_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_api_keys user_api_keys_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- Name: user_api_keys user_api_keys_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_api_keys
    ADD CONSTRAINT user_api_keys_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: user_network_access user_network_access_network_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_network_id_fkey FOREIGN KEY (network_id) REFERENCES public.networks(id) ON DELETE CASCADE;


--
-- Name: user_network_access user_network_access_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.user_network_access
    ADD CONSTRAINT user_network_access_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.users(id) ON DELETE CASCADE;


--
-- Name: users users_organization_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_organization_id_fkey FOREIGN KEY (organization_id) REFERENCES public.organizations(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict nbHSsyqjKAB9awCdCI6soNmAZ8ViG1xFFpXM8JzZolPOTUULxlzQLvxhUeUL4jg

