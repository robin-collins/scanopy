--
-- PostgreSQL database dump
--

\restrict VmoNqUwCvOYuWVbj0w3AA5Zdd9yfsC291ve6mz8fUKXQkpycFH35v90tkHfCffa

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
20251006215000	users	2026-03-26 01:31:33.227512+00	t	\\x4f13ce14ff67ef0b7145987c7b22b588745bf9fbb7b673450c26a0f2f9a36ef8ca980e456c8d77cfb1b2d7a4577a64d7	4849104
20251006215100	networks	2026-03-26 01:31:33.233518+00	t	\\xeaa5a07a262709f64f0c59f31e25519580c79e2d1a523ce72736848946a34b17dd9adc7498eaf90551af6b7ec6d4e0e3	6083416
20251006215151	create hosts	2026-03-26 01:31:33.239939+00	t	\\x6ec7487074c0724932d21df4cf1ed66645313cf62c159a7179e39cbc261bcb81a24f7933a0e3cf58504f2a90fc5c1962	5295294
20251006215155	create subnets	2026-03-26 01:31:33.245647+00	t	\\xefb5b25742bd5f4489b67351d9f2494a95f307428c911fd8c5f475bfb03926347bdc269bbd048d2ddb06336945b27926	5069659
20251006215201	create groups	2026-03-26 01:31:33.251103+00	t	\\x0a7032bf4d33a0baf020e905da865cde240e2a09dda2f62aa535b2c5d4b26b20be30a3286f1b5192bd94cd4a5dbb5bcd	5303097
20251006215204	create daemons	2026-03-26 01:31:33.256803+00	t	\\xcfea93403b1f9cf9aac374711d4ac72d8a223e3c38a1d2a06d9edb5f94e8a557debac3668271f8176368eadc5105349f	5768423
20251006215212	create services	2026-03-26 01:31:33.262957+00	t	\\xd5b07f82fc7c9da2782a364d46078d7d16b5c08df70cfbf02edcfe9b1b24ab6024ad159292aeea455f15cfd1f4740c1d	6675147
20251029193448	user-auth	2026-03-26 01:31:33.269974+00	t	\\xfde8161a8db89d51eeade7517d90a41d560f19645620f2298f78f116219a09728b18e91251ae31e46a47f6942d5a9032	7581536
20251030044828	daemon api	2026-03-26 01:31:33.277872+00	t	\\x181eb3541f51ef5b038b2064660370775d1b364547a214a20dde9c9d4bb95a1c273cd4525ef29e61fa65a3eb4fee0400	2099220
20251030170438	host-hide	2026-03-26 01:31:33.280289+00	t	\\x87c6fda7f8456bf610a78e8e98803158caa0e12857c5bab466a5bb0004d41b449004a68e728ca13f17e051f662a15454	1412688
20251102224919	create discovery	2026-03-26 01:31:33.282024+00	t	\\xb32a04abb891aba48f92a059fae7341442355ca8e4af5d109e28e2a4f79ee8e11b2a8f40453b7f6725c2dd6487f26573	14957883
20251106235621	normalize-daemon-cols	2026-03-26 01:31:33.297322+00	t	\\x5b137118d506e2708097c432358bf909265b3cf3bacd662b02e2c81ba589a9e0100631c7801cffd9c57bb10a6674fb3b	2628756
20251107034459	api keys	2026-03-26 01:31:33.300265+00	t	\\x3133ec043c0c6e25b6e55f7da84cae52b2a72488116938a2c669c8512c2efe72a74029912bcba1f2a2a0a8b59ef01dde	11348512
20251107222650	oidc-auth	2026-03-26 01:31:33.311994+00	t	\\xd349750e0298718cbcd98eaff6e152b3fb45c3d9d62d06eedeb26c75452e9ce1af65c3e52c9f2de4bd532939c2f31096	34661825
20251110181948	orgs-billing	2026-03-26 01:31:33.347043+00	t	\\x5bbea7a2dfc9d00213bd66b473289ddd66694eff8a4f3eaab937c985b64c5f8c3ad2d64e960afbb03f335ac6766687aa	15520110
20251113223656	group-enhancements	2026-03-26 01:31:33.362975+00	t	\\xbe0699486d85df2bd3edc1f0bf4f1f096d5b6c5070361702c4d203ec2bb640811be88bb1979cfe51b40805ad84d1de65	1525991
20251117032720	daemon-mode	2026-03-26 01:31:33.36485+00	t	\\xdd0d899c24b73d70e9970e54b2c748d6b6b55c856ca0f8590fe990da49cc46c700b1ce13f57ff65abd6711f4bd8a6481	1416975
20251118143058	set-default-plan	2026-03-26 01:31:33.36661+00	t	\\xd19142607aef84aac7cfb97d60d29bda764d26f513f2c72306734c03cec2651d23eee3ce6cacfd36ca52dbddc462f917	1682686
20251118225043	save-topology	2026-03-26 01:31:33.368614+00	t	\\x011a594740c69d8d0f8b0149d49d1b53cfbf948b7866ebd84403394139cb66a44277803462846b06e762577adc3e61a3	12311553
20251123232748	network-permissions	2026-03-26 01:31:33.381307+00	t	\\x161be7ae5721c06523d6488606f1a7b1f096193efa1183ecdd1c2c9a4a9f4cad4884e939018917314aaf261d9a3f97ae	3920106
20251125001342	billing-updates	2026-03-26 01:31:33.385665+00	t	\\xa235d153d95aeb676e3310a52ccb69dfbd7ca36bba975d5bbca165ceeec7196da12119f23597ea5276c364f90f23db1e	1164551
20251128035448	org-onboarding-status	2026-03-26 01:31:33.387096+00	t	\\x1d7a7e9bf23b5078250f31934d1bc47bbaf463ace887e7746af30946e843de41badfc2b213ed64912a18e07b297663d8	1803313
20251129180942	nfs-consolidate	2026-03-26 01:31:33.389213+00	t	\\xb38f41d30699a475c2b967f8e43156f3b49bb10341bddbde01d9fb5ba805f6724685e27e53f7e49b6c8b59e29c74f98e	1675212
20251206052641	discovery-progress	2026-03-26 01:31:33.391202+00	t	\\x9d433b7b8c58d0d5437a104497e5e214febb2d1441a3ad7c28512e7497ed14fb9458e0d4ff786962a59954cb30da1447	1986397
20251206202200	plan-fix	2026-03-26 01:31:33.393526+00	t	\\x242f6699dbf485cf59a8d1b8cd9d7c43aeef635a9316be815a47e15238c5e4af88efaa0daf885be03572948dc0c9edac	1104308
20251207061341	daemon-url	2026-03-26 01:31:33.394938+00	t	\\x01172455c4f2d0d57371d18ef66d2ab3b7a8525067ef8a86945c616982e6ce06f5ea1e1560a8f20dadcd5be2223e6df1	3210601
20251210045929	tags	2026-03-26 01:31:33.398497+00	t	\\xe3dde83d39f8552b5afcdc1493cddfeffe077751bf55472032bc8b35fc8fc2a2caa3b55b4c2354ace7de03c3977982db	12041476
20251210175035	terms	2026-03-26 01:31:33.410945+00	t	\\xe47f0cf7aba1bffa10798bede953da69fd4bfaebf9c75c76226507c558a3595c6bfc6ac8920d11398dbdf3b762769992	1206198
20251213025048	hash-keys	2026-03-26 01:31:33.412554+00	t	\\xfc7cbb8ce61f0c225322297f7459dcbe362242b9001c06cb874b7f739cea7ae888d8f0cfaed6623bcbcb9ec54c8cd18b	13754635
20251214050638	scanopy	2026-03-26 01:31:33.426742+00	t	\\x0108bb39832305f024126211710689adc48d973ff66e5e59ff49468389b75c1ff95d1fbbb7bdb50e33ec1333a1f29ea6	2315246
20251215215724	topo-scanopy-fix	2026-03-26 01:31:33.429386+00	t	\\xed88a4b71b3c9b61d46322b5053362e5a25a9293cd3c420c9df9fcaeb3441254122b8a18f58c297f535c842b8a8b0a38	1241045
20251217153736	category rename	2026-03-26 01:31:33.430961+00	t	\\x03af7ec905e11a77e25038a3c272645da96014da7c50c585a25cea3f9a7579faba3ff45114a5e589d144c9550ba42421	1978830
20251218053111	invite-persistence	2026-03-26 01:31:33.433246+00	t	\\x21d12f48b964acfd600f88e70ceb14abd9cf2a8a10db2eae2a6d8f44cf7d20749f93293631e6123e92b7c3c1793877c2	6463781
20251219211216	create shares	2026-03-26 01:31:33.440093+00	t	\\x036485debd3536f9e58ead728f461b925585911acf565970bf3b2ab295b12a2865606d6a56d334c5641dcd42adeb3d68	8532934
20251220170928	permissions-cleanup	2026-03-26 01:31:33.449019+00	t	\\x632f7b6702b494301e0d36fd3b900686b1a7f9936aef8c084b5880f1152b8256a125566e2b5ac40216eaadd3c4c64a03	1728833
20251220180000	commercial-to-community	2026-03-26 01:31:33.45109+00	t	\\x26fc298486c225f2f01271d611418377c403183ae51daf32fef104ec07c027f2017d138910c4fbfb5f49819a5f4194d6	915873
20251221010000	cleanup subnet type	2026-03-26 01:31:33.452295+00	t	\\xb521121f3fd3a10c0de816977ac2a2ffb6118f34f8474ffb9058722abc0dc4cf5cbec83bc6ee49e79a68e6b715087f40	946271
20251221020000	remove host target	2026-03-26 01:31:33.453566+00	t	\\x77b5f8872705676ca81a5704bd1eaee90b9a52b404bdaa27a23da2ffd4858d3e131680926a5a00ad2a0d7a24ba229046	1088658
20251221030000	user network access	2026-03-26 01:31:33.454961+00	t	\\x5c23f5bb6b0b8ca699a17eee6730c4197a006ca21fecc79136a5e5697b9211a81b4cd08ceda70dace6a26408d021ff3a	8455218
20251221040000	interfaces table	2026-03-26 01:31:33.464032+00	t	\\xf7977b6f1e7e5108c614397d03a38c9bd9243fdc422575ec29610366a0c88f443de2132185878d8e291f06a50a8c3244	12880283
20251221050000	ports table	2026-03-26 01:31:33.477298+00	t	\\xdf72f9306b405be7be62c39003ef38408115e740b120f24e8c78b8e136574fff7965c52023b3bc476899613fa5f4fe35	11985440
20251221060000	bindings table	2026-03-26 01:31:33.489652+00	t	\\x933648a724bd179c7f47305e4080db85342d48712cde39374f0f88cde9d7eba8fe5fafba360937331e2a8178dec420c4	13791598
20251221070000	group bindings	2026-03-26 01:31:33.504112+00	t	\\x697475802f6c42e38deee6596f4ba786b09f7b7cd91742fbc5696dd0f9b3ddfce90dd905153f2b1a9e82f959f5a88302	8312930
20251222020000	tag cascade delete	2026-03-26 01:31:33.512841+00	t	\\xabfb48c0da8522f5c8ea6d482eb5a5f4562ed41f6160a5915f0fd477c7dd0517aa84760ef99ab3a5db3e0f21b0c69b5f	1509770
20251223232524	network remove default	2026-03-26 01:31:33.514671+00	t	\\x7099fe4e52405e46269d7ce364050da930b481e72484ad3c4772fd2911d2d505476d659fa9f400c63bc287512d033e18	1122192
20251225100000	color enum	2026-03-26 01:31:33.516102+00	t	\\x62cecd9d79a49835a3bea68a7959ab62aa0c1aaa7e2940dec6a7f8a714362df3649f0c1f9313672d9268295ed5a1cfa9	1637501
20251227010000	topology snapshot migration	2026-03-26 01:31:33.518057+00	t	\\xc042591d254869c0e79c8b52a9ede680fd26f094e2c385f5f017e115f5e3f31ad155f4885d095344f2642ebb70755d54	5490720
20251228010000	user api keys	2026-03-26 01:31:33.523903+00	t	\\xa41adb558a5b9d94a4e17af3f16839b83f7da072dbeac9251b12d8a84c7bec6df008009acf246468712a975bb36bb5f5	16389567
20251230160000	daemon version and maintainer	2026-03-26 01:31:33.540701+00	t	\\xafed3d9f00adb8c1b0896fb663af801926c218472a0a197f90ecdaa13305a78846a9e15af0043ec010328ba533fca68f	3636673
20260103000000	service position	2026-03-26 01:31:33.544716+00	t	\\x19d00e8c8b300d1c74d721931f4d771ec7bc4e06db0d6a78126e00785586fdc4bcff5b832eeae2fce0cb8d01e12a7fb5	2346906
20260106000000	interface mac index	2026-03-26 01:31:33.547382+00	t	\\xa26248372a1e31af46a9c6fbdaef178982229e2ceeb90cc6a289d5764f87a38747294b3adf5f21276b5d171e42bdb6ac	2189620
20260106204402	entity tags junction	2026-03-26 01:31:33.550079+00	t	\\xf73c604f9f0b8db065d990a861684b0dbd62c3ef9bead120c68431c933774de56491a53f021e79f09801680152f5a08a	16115461
20260108033856	fix entity tags json format	2026-03-26 01:31:33.566584+00	t	\\x197eaa063d4f96dd0e897ad8fd96cc1ba9a54dda40a93a5c12eac14597e4dea4c806dd0a527736fb5807b7a8870d9916	1791180
20260110000000	email verification	2026-03-26 01:31:33.568714+00	t	\\xb8da8433f58ba4ce846b9fa0c2551795747a8473ad10266b19685504847458ea69d27a0ce430151cfb426f5f5fb6ac3a	4544060
20260114145808	daemon user fk set null	2026-03-26 01:31:33.573614+00	t	\\x57b060be9fc314d7c5851c75661ca8269118feea6cf7ee9c61b147a0e117c4d39642cf0d1acdf7a723a9a76066c1b8ff	1574232
20260116010000	snmp credentials	2026-03-26 01:31:33.575543+00	t	\\x6f3971cf194d56883c61fa795406a8ab568307ed86544920d098b32a6a1ebb7effcb5ec38a70fdc9b617eff92d63d51e	8764724
20260116020000	host snmp fields	2026-03-26 01:31:33.584683+00	t	\\xf2f088c13ab0dd34e1cb1e5327b0b4137440b0146e5ce1e78b8d2dfa05d9b5a12a328eeb807988453a8a43ad8a1c95ba	5257729
20260116030000	if entries	2026-03-26 01:31:33.590268+00	t	\\xa58391708f8b21901ab9250af528f638a6055462f70ffddfd7c451433aacdabd62825546fa8be108f23a3cae78b8ae28	17642003
20260116100000	daemon api key link	2026-03-26 01:31:33.608331+00	t	\\x41088aa314ab173344a6b416280721806b2f296a32a8d8cae58c7e5717f389fe599134ed03980ed97e4b7659e99c4f82	4716745
20260131190000	add hubspot company id	2026-03-26 01:31:33.613586+00	t	\\x4326f95f4954e176157c1c3e034074a3e5c44da4d60bbd7a9e4b6238c9ef52a30f8b38d3c887864b6e4c1163dc062beb	1670477
20260201021238	fix service acronym capitalization	2026-03-26 01:31:33.615655+00	t	\\x88b010ac8f0223d880ea6a730f11dc6d27fa5de9d8747de3431e46d59f1dbf2f72ae4a87c2e52c32152549f5c1f96bb2	2325697
20260204004436	add entity tags to topology	2026-03-26 01:31:33.618329+00	t	\\x3eff1a1490e77065ec861ef1b9aad8c55de0170106a42720f7931b3929b179122b16e44390b2652771bf91bba32a7757	1779949
20260205120000	billing overhaul	2026-03-26 01:31:33.620533+00	t	\\xbf850cfa0c40a3c65f574efd15fd55a4b702296203d28077a09d1c22076fee8601f2b78345aef370ab9163657de767ab	3274343
20260205183207	rename hubspot to brevo	2026-03-26 01:31:33.624138+00	t	\\x4678a7d80215e5eafb5e80af0daa20e2868a3b4f2112e88cb1b2b9efc87d63de3fb96c133f359b224c658789ae4b0d13	1286230
20260221120000	add plan limit notifications	2026-03-26 01:31:33.625795+00	t	\\xef770dac07e1d80888832f33184dc46c1d3b8185b91c507cb404468d6ad8c29cacf455178801c67aa27b6a626d3ad82d	1498108
20260222120000	add pending email	2026-03-26 01:31:33.6277+00	t	\\xddd220f7602c44548d56849c0a8d081ecd1da1383374a11e3e227c7d9becb73a49f5e5bb09ed65901c16df4c16e913e5	1116800
20260301120000	add if name to if entries	2026-03-26 01:31:33.629122+00	t	\\xc9fc0a2b77ecbf0e1d5ab292c4fe162a26113468c878dfd26a3c63d89c0ee1957ca328ecfe25c611867a0e73780f0cb6	1086575
20260306002816	cleanup standby	2026-03-26 01:31:33.630541+00	t	\\x01b0c236a8a4d0d97f0f633b18f8cbdb92b6d72063289989b90a1b7b6b303e65e0557eb09927b2580dcb7e8ee5966c75	1175088
20260309120000	add org use case	2026-03-26 01:31:33.632005+00	t	\\xdb8c8a2f0f9416ba3b687fc75453d7c12c50a6f386b4784d21bd6adfc4a4a7556c637c25cf116118402bbd12c0d5aafe	1014749
20260313120000	snmp extended discovery	2026-03-26 01:31:33.633318+00	t	\\xc4e72539099de1b830d87a169bfbabba4b8fb378a3c4c4a1dfca698adf3e403d750040d784c26d9fa343be2908064c9d	2251860
20260315120000	universal credentials	2026-03-26 01:31:33.635957+00	t	\\x87dc6f39202e81d5555df78a9d056b143f11bd22e6d7f483065f605e242a360902c72c4d5a49717e7fcc24a366bb5ff5	35663288
20260315120001	discovery scan settings	2026-03-26 01:31:33.672102+00	t	\\xe9da183fdd8e04e574f553f61f6f33efa046cdae38c846c8077b06c5260446fb4aa39da2449bda7f1d8cf3aa9f16e158	1121641
20260315120002	backfill org created milestone	2026-03-26 01:31:33.673554+00	t	\\x14f886a19773cd2263d86f88479be460d21f071d5212e3789c5c40b6415c293fc7d06c7b138351cc42108f89a14fe745	1061447
20260316120000	fix jsonb null if entries	2026-03-26 01:31:33.674936+00	t	\\x65c358069710f7f86d6a3e257e658c2f241cc376433c3a0317b0ec9e1876a66f9738cb65c6ab1a5c197fe40d5aa2aa2b	2232530
20260319120000	rename snmp to snmpv2c	2026-03-26 01:31:33.677606+00	t	\\xdce5c9461f402e1672607078b2c571f0eb30b51d46f8e9414d8909efb40693f543e49e560cb7d703db274515043aa08e	8189515
20260321120000	add discovery scan count	2026-03-26 01:31:33.69692+00	t	\\x6c8201ab453a51632176d534c6604e0818e28a8a4a153e33e254f4dac0f9b67c9db394082cb663ff1b25941229cf96fc	40216617
\.


--
-- Data for Name: api_keys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.api_keys (id, key, network_id, name, created_at, updated_at, last_used, expires_at, is_enabled, plaintext) FROM stdin;
18bf961d-40ed-4417-a3f5-aaddf1fb51c6	2ce3771c175cd3901963c122a89ed6d0dd0250898a9287f6e1a8a4619d67fd51	51280042-881b-4774-b176-266d86e3c51a	scanopy-daemon-serverpoll API Key	2026-03-26 01:33:56.62179+00	2026-03-26 01:33:56.62179+00	2026-03-26 01:38:56.080745+00	\N	t	scp_d_FHp4tu6mUeBTgEcLn1eWkUM1aw4fprDy
aeabedc7-494f-4f76-8b7e-ff7e7e0f9ab5	7bd199b364280a67425da7552b3224830d885b617d8f2dab60a22d806390bca5	51280042-881b-4774-b176-266d86e3c51a	Compat Test API Key	2026-03-26 01:38:35.356391+00	2026-03-26 01:38:35.356391+00	2026-03-26 01:38:48.542553+00	\N	t	\N
4c699efa-f835-4112-b835-e187ed31a09b	6602471aac7fdfcd6dee526339efd411598942d4f5873211c4aa76a9f6ef5ae6	51280042-881b-4774-b176-266d86e3c51a	Integrated Daemon API Key	2026-03-26 01:31:39.612677+00	2026-03-26 01:31:39.612677+00	2026-03-26 01:38:44.911958+00	\N	t	\N
\.


--
-- Data for Name: bindings; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.bindings (id, network_id, service_id, binding_type, interface_id, port_id, created_at, updated_at) FROM stdin;
963a87f1-b6a8-4d9f-aab5-089ae500303f	51280042-881b-4774-b176-266d86e3c51a	abae1495-5c8e-4f13-8717-a64f5e857e0e	Port	e5612590-afaf-4ff7-973a-a22456210fd2	318b9ff2-ab97-490b-8ed5-19a195ca767d	2026-01-25 23:12:10.20886+00	2026-01-25 23:12:10.20886+00
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
19b97945-feeb-48f8-9e9a-fc1a3ef19a15	51280042-881b-4774-b176-266d86e3c51a	39d3169e-1a02-41e6-b3c6-db9716ae6ad4	2026-03-26 01:33:56.625634+00	2026-03-26 01:38:34.289781+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["fbd0da95-ba0a-4e1c-870e-b7b625f92659", "bc1667a9-c82b-402d-9eb7-a78e9a2b1f49"]}	2026-03-26 01:33:56.625634+00	"server_poll"	http://daemon-serverpoll:60074	scanopy-daemon-serverpoll	0.15.2	b94a67ba-8853-468c-b21e-0ae053879623	18bf961d-40ed-4417-a3f5-aaddf1fb51c6	f	f
99ac5804-c7b3-4ff2-9bf3-e00b891680c0	51280042-881b-4774-b176-266d86e3c51a	711a5546-8640-4ee2-bf0b-8b268c4e2044	2026-03-26 01:31:39.644098+00	2026-03-26 01:38:48.49574+00	{"has_docker_socket": false, "interfaced_subnet_ids": ["d721ac54-f1d3-4fd5-9009-10797ed60714"]}	2026-03-26 01:31:39.644098+00	"daemon_poll"		scanopy-daemon	0.13.6	b94a67ba-8853-468c-b21e-0ae053879623	\N	f	f
\.


--
-- Data for Name: discovery; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.discovery (id, network_id, daemon_id, run_type, discovery_type, name, created_at, updated_at, scan_count, force_full_scan, pending_credential_ids) FROM stdin;
888c3c39-5fc6-47a5-a7d5-1a349aead966	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Scheduled", "enabled": true, "last_run": "2026-03-26T01:31:39.652647154Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:31:39.650607+00	2026-03-26 01:33:56.209969+00	1	f	{}
6599c641-695d-4020-bc5c-3d6fee90b43d	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "cd493a46-8909-4378-b924-ae1650a0f5af", "started_at": "2026-03-26T01:31:45.021343211Z", "finished_at": "2026-03-26T01:33:56.195774325Z", "discovery_type": {"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "711a5546-8640-4ee2-bf0b-8b268c4e2044", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:31:45.021343+00	2026-03-26 01:33:56.207962+00	0	f	{}
a69907c1-b765-44fa-8f14-f1eb08bbe3d8	51280042-881b-4774-b176-266d86e3c51a	19b97945-feeb-48f8-9e9a-fc1a3ef19a15	{"type": "Scheduled", "enabled": true, "last_run": "2026-03-26T01:34:04.301649833Z", "timezone": null, "cron_schedule": "0 0 0 * * 0"}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:34:04.30012+00	2026-03-26 01:34:04.301651+00	0	f	{}
9f43a9f0-9f78-498c-bf73-2b1eff9cf5c9	51280042-881b-4774-b176-266d86e3c51a	19b97945-feeb-48f8-9e9a-fc1a3ef19a15	{"type": "AdHoc", "last_run": "2026-03-26T01:33:57.042159658Z"}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	ServerPoll Integration Test Discovery	2026-03-26 01:33:57.0335+00	2026-03-26 01:38:34.398538+00	1	f	{}
f401e5b4-9c97-499b-b1fd-669c4acccb78	51280042-881b-4774-b176-266d86e3c51a	19b97945-feeb-48f8-9e9a-fc1a3ef19a15	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "032af69a-4335-4c54-ac9c-8941c44050a3", "started_at": "2026-03-26T01:34:34.385635665Z", "finished_at": "2026-03-26T01:38:34.384998297Z", "discovery_type": {"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": 5, "estimated_remaining_secs": 30}}	{"type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:34:34.385635+00	2026-03-26 01:38:34.39681+00	0	f	{}
4d7d4da3-2704-4802-90ee-a98041ab6357	51280042-881b-4774-b176-266d86e3c51a	19b97945-feeb-48f8-9e9a-fc1a3ef19a15	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-03-26T01:38:49.275558843Z", "finished_at": "2026-03-26T01:38:49.282872838Z", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-03-26 01:38:49.275558+00	2026-03-26 01:38:49.289378+00	0	f	{}
388da2b3-6cad-4cc1-89e1-36f970a284ac	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "c10d2eae-1267-4023-997e-5a51fdf97281", "started_at": "2026-03-26T01:38:49.546481276Z", "finished_at": "2026-03-26T01:38:49.554071850Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:49.546481+00	2026-03-26 01:38:49.561356+00	0	f	{}
1800c75a-84f3-4966-86ba-8cd889b4c7fe	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "6845dcc8-6ea0-47af-9ce3-054b8cf1f667", "started_at": "2026-03-26T01:38:49.817882608Z", "finished_at": "2026-03-26T01:38:49.824649986Z", "discovery_type": {"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "1438e666-92b6-4fad-bc37-aa2717d9ba42"}	Self Report — My Network	2026-03-26 01:38:49.817882+00	2026-03-26 01:38:49.830881+00	0	f	{}
4a23fbb7-7132-4db8-9993-077578d74c05	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "0ffe5496-d64e-41a0-ab86-ff28eed28819", "started_at": "2026-03-26T01:38:50.087257173Z", "finished_at": "2026-03-26T01:38:50.094936173Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:50.087257+00	2026-03-26 01:38:50.101078+00	0	f	{}
142b86a9-7080-4e13-807d-725a47f87254	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "34c804c2-c01c-4104-a3b3-e31ae6dd0b6f", "started_at": "2026-03-26T01:38:50.639207585Z", "finished_at": "2026-03-26T01:38:50.646705749Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:50.639207+00	2026-03-26 01:38:50.65328+00	0	f	{}
3e0c1332-629a-4397-a49f-208c86d5af10	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "f2d5414b-3369-4280-929d-5422dd11a4b4", "started_at": "2026-03-26T01:38:50.908340126Z", "finished_at": "2026-03-26T01:38:50.915600351Z", "discovery_type": {"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "8f6b3991-b3ef-4d1d-9708-d2f57289a34f"}	Self Report — My Network	2026-03-26 01:38:50.90834+00	2026-03-26 01:38:50.922366+00	0	f	{}
a7dbf081-2a39-42c3-a59b-4f384e774d0e	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "969d115c-0112-4ba3-8757-bb1ece340112", "started_at": "2026-03-26T01:38:51.448412563Z", "finished_at": "2026-03-26T01:38:51.455783686Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:51.448412+00	2026-03-26 01:38:51.46186+00	0	f	{}
f509c44e-044e-450a-ab15-38e549cd6d59	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "4273fe18-d031-4977-950e-ad5a49d2ea8e", "started_at": "2026-03-26T01:38:52.267882165Z", "finished_at": "2026-03-26T01:38:52.276255202Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:52.267882+00	2026-03-26 01:38:52.282018+00	0	f	{}
a7e88b97-21c8-4307-9954-b94d3a4232a4	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "27ff5779-08b0-4970-aebd-04649d27c725", "started_at": "2026-03-26T01:38:52.812789626Z", "finished_at": "2026-03-26T01:38:52.821942688Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:52.812789+00	2026-03-26 01:38:52.827732+00	0	f	{}
436d5e1b-ee35-42a3-8f51-d7f34e445d74	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "d2b26f9b-0329-4191-8a60-58cf3c796dba", "started_at": "2026-03-26T01:38:53.188628416Z", "finished_at": "2026-03-26T01:38:53.195822997Z", "discovery_type": {"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9f1349e1-04dc-47e8-9a78-0c483e2a16a6", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:38:53.188628+00	2026-03-26 01:38:53.20119+00	0	f	{}
0432e49a-f5f7-43a7-8bc6-4d92a76f7d9d	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "2abdbee0-ac03-472c-b137-4321f33f03ca", "started_at": "2026-03-26T01:38:50.469444133Z", "finished_at": "2026-03-26T01:38:50.476059662Z", "discovery_type": {"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "d4cf5d4f-39ce-4bc8-9692-0398c5897364", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:38:50.469444+00	2026-03-26 01:38:50.481219+00	0	f	{}
f8e63f02-fa16-4e3c-ba82-6d2285bf2a66	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "c8c6530c-aaff-4d7c-a872-ba4457906d77", "started_at": "2026-03-26T01:38:51.179673148Z", "finished_at": "2026-03-26T01:38:51.186738108Z", "discovery_type": {"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "cc741d90-bcc0-4653-b38b-52b23f9e6a61"}	Self Report — My Network	2026-03-26 01:38:51.179673+00	2026-03-26 01:38:51.192606+00	0	f	{}
816eeb2e-0f71-48c0-b264-11442368b67c	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "10fdd8f4-03b6-44ea-adb6-27e74136b365", "started_at": "2026-03-26T01:38:51.994817147Z", "finished_at": "2026-03-26T01:38:52.003405066Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:51.994817+00	2026-03-26 01:38:52.009572+00	0	f	{}
e9e137b9-0869-4f29-a5c0-751a549fa9c1	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "abba33fb-bf1f-4e8b-985f-6a2d0b5d0380", "started_at": "2026-03-26T01:38:52.539727020Z", "finished_at": "2026-03-26T01:38:52.547543360Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:52.539727+00	2026-03-26 01:38:52.55412+00	0	f	{}
ad0b4333-f63c-48d9-b6f2-afedb35ae60d	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "857fbace-0358-4642-8ca0-b14bb999f353", "started_at": "2026-03-26T01:38:53.359183249Z", "finished_at": "2026-03-26T01:38:53.368442876Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:53.359183+00	2026-03-26 01:38:53.374544+00	0	f	{}
5b2f684d-ec7e-4492-b247-62008a2fa431	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "59fba01e-fefd-4c66-9ef2-a85c0e76a811", "started_at": "2026-03-26T01:38:54.186304884Z", "finished_at": "2026-03-26T01:38:54.194983975Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:54.186304+00	2026-03-26 01:38:54.200819+00	0	f	{}
155dd7a8-bcc2-464e-a87a-b7594032988d	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "5bd5e93e-a606-4c6e-b159-b04879bdd801", "started_at": "2026-03-26T01:38:54.734993886Z", "finished_at": "2026-03-26T01:38:54.744153198Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:54.734993+00	2026-03-26 01:38:54.749595+00	0	f	{}
14c9f005-b843-4f1b-94e7-b40a71bb3c16	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "5e741620-3de9-476c-86e7-7ce656d0a5a8", "started_at": "2026-03-26T01:38:55.008822927Z", "finished_at": "2026-03-26T01:38:55.017358001Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:55.008822+00	2026-03-26 01:38:55.022852+00	0	f	{}
ae3d259a-7679-49f1-9cc5-c1c3a84c3497	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "3b0868af-d558-45e3-b688-2cf55472b6ee", "started_at": "2026-03-26T01:38:55.543783811Z", "finished_at": "2026-03-26T01:38:55.552548044Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:55.543783+00	2026-03-26 01:38:55.558222+00	0	f	{}
202cb79d-569b-4bb2-a41f-b8b161c8e0dd	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "b64df0ca-f173-4ba8-a48b-edf7e372974a", "started_at": "2026-03-26T01:38:55.809708550Z", "finished_at": "2026-03-26T01:38:55.817958867Z", "discovery_type": {"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "09900acc-93fd-4af9-8a9b-9f45ace7475c"}	Self Report — My Network	2026-03-26 01:38:55.809708+00	2026-03-26 01:38:55.823448+00	0	f	{}
cab338ef-61f0-4077-962f-f0e98f430e30	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Cancelled", "progress": 0, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "ef1a4d64-afd3-4762-b6ae-2ff80e8dbc2f", "started_at": "2026-03-26T01:38:51.825984709Z", "finished_at": "2026-03-26T01:38:51.832821165Z", "discovery_type": {"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "Unified", "host_id": "9eae46ea-5edb-41bd-9955-7134bd96e594", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}	Discovery	2026-03-26 01:38:51.825984+00	2026-03-26 01:38:51.837858+00	0	f	{}
5f3249c1-eae8-4600-9e20-dabe2e6af768	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "5b19fece-cb82-45d8-a676-df53cc38a014", "started_at": "2026-03-26T01:38:55.274634600Z", "finished_at": "2026-03-26T01:38:55.282459186Z", "discovery_type": {"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "a9590643-88e0-45c1-8420-738ed98070ba"}	Self Report — My Network	2026-03-26 01:38:55.274634+00	2026-03-26 01:38:55.287922+00	0	f	{}
8a5b8651-79cd-43d7-a74c-5388428a8e89	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "7f54ecab-3771-4583-a7d0-f12569030e17", "started_at": "2026-03-26T01:38:56.071330839Z", "finished_at": "2026-03-26T01:38:56.079397755Z", "discovery_type": {"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "f738b076-a24e-4db2-800c-a0f10bb44b16"}	Self Report — My Network	2026-03-26 01:38:56.07133+00	2026-03-26 01:38:56.085227+00	0	f	{}
ea6e7ef0-a4cb-4694-bfac-6c1019f7230d	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "22ea697f-065c-4d2e-a81f-0809764aad01", "started_at": "2026-03-26T01:38:53.634992988Z", "finished_at": "2026-03-26T01:38:53.642592870Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:53.634992+00	2026-03-26 01:38:53.648274+00	0	f	{}
fec23d86-1947-4e01-83b2-2acb8002fefe	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "bbe36108-56c4-4d82-9087-e577842ff202", "started_at": "2026-03-26T01:38:53.910846081Z", "finished_at": "2026-03-26T01:38:53.921906581Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:53.910846+00	2026-03-26 01:38:53.928117+00	0	f	{}
bd891aee-7f0c-4dd1-af8e-b9f24c6b8f81	51280042-881b-4774-b176-266d86e3c51a	99ac5804-c7b3-4ff2-9bf3-e00b891680c0	{"type": "Historical", "results": {"error": null, "phase": "Complete", "progress": 100, "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "session_id": "ec9dc330-67d0-4ce5-94d8-506859c74940", "started_at": "2026-03-26T01:38:54.460336519Z", "finished_at": "2026-03-26T01:38:54.469831564Z", "discovery_type": {"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}, "hosts_discovered": null, "estimated_remaining_secs": null}}	{"type": "SelfReport", "host_id": "00000000-0000-0000-0000-000000000000"}	Self Report — My Network	2026-03-26 01:38:54.460336+00	2026-03-26 01:38:54.476158+00	0	f	{}
\.


--
-- Data for Name: entity_tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.entity_tags (id, entity_id, entity_type, tag_id, created_at) FROM stdin;
347afed6-7ce3-4d37-9e5e-d67e4d6689aa	27d3a8a1-c370-4b82-af62-4e65214aec80	"Service"	99bf2d38-dfb2-40bf-848c-9ea754616a34	2026-03-26 01:38:34.424856+00
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
cb2f10cd-bfec-487b-9bcd-cf6bc10f472b	51280042-881b-4774-b176-266d86e3c51a		\N	2026-03-26 01:38:34.42932+00	2026-03-26 01:38:34.42932+00	{"type": "Manual"}	Yellow	"SmoothStep"	RequestPath
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
7c51d243-60ef-4994-b7db-fa41b23b3644	51280042-881b-4774-b176-266d86e3c51a	843ded61a959	843ded61a959	Scanopy daemon	{"type": "Discovery", "metadata": [{"date": "2026-01-25T23:12:10.208844254Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0"}]}	null	2026-01-25 23:12:10.208845+00	2026-01-25 23:12:10.208845+00	f	\N	\N	\N	\N	\N	\N	\N	\N	\N	\N
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
e5612590-afaf-4ff7-973a-a22456210fd2	51280042-881b-4774-b176-266d86e3c51a	7c51d243-60ef-4994-b7db-fa41b23b3644	d721ac54-f1d3-4fd5-9009-10797ed60714	172.25.0.4	96:d5:6f:59:0f:cc	eth0	0	2026-01-25 23:12:10.202095+00	2026-01-25 23:12:10.202095+00
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
51280042-881b-4774-b176-266d86e3c51a	My Network	2026-03-26 01:31:39.585457+00	2026-03-26 01:31:39.585457+00	ccf220b7-6646-4911-8a31-9e93fc809c42
\.


--
-- Data for Name: organizations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.organizations (id, name, stripe_customer_id, plan, plan_status, created_at, updated_at, onboarding, brevo_company_id, has_payment_method, trial_end_date, plan_limit_notifications, use_case) FROM stdin;
ccf220b7-6646-4911-8a31-9e93fc809c42	My Organization	\N	{"rate": "Month", "type": "Community", "base_cents": 0, "host_cents": null, "seat_cents": null, "trial_days": 0, "network_cents": null, "included_hosts": null, "included_seats": null, "included_networks": null}	active	2026-03-26 01:31:39.571746+00	2026-03-26 01:31:39.571746+00	["OnboardingModalCompleted", "OrgCreated", "FirstDaemonRegistered", "FirstHostDiscovered", "FirstDiscoveryCompleted", "FirstTagCreated", "FirstGroupCreated", "FirstUserApiKeyCreated", "SecondNetworkCreated"]	\N	f	\N	{"hosts": "None", "seats": "None", "networks": "None"}	\N
\.


--
-- Data for Name: ports; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.ports (id, network_id, host_id, port_number, protocol, port_type, created_at, updated_at) FROM stdin;
318b9ff2-ab97-490b-8ed5-19a195ca767d	51280042-881b-4774-b176-266d86e3c51a	7c51d243-60ef-4994-b7db-fa41b23b3644	60073	Tcp	Custom	2026-01-25 23:12:10.207381+00	2026-01-25 23:12:10.207381+00
\.


--
-- Data for Name: services; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.services (id, network_id, created_at, updated_at, name, host_id, service_definition, virtualization, source, "position") FROM stdin;
abae1495-5c8e-4f13-8717-a64f5e857e0e	51280042-881b-4774-b176-266d86e3c51a	2026-01-25 23:12:10.212728+00	2026-01-25 23:12:10.212728+00	Scanopy Daemon	7c51d243-60ef-4994-b7db-fa41b23b3644	"Scanopy Daemon"	null	{"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-01-25T23:12:10.208864212Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0"}]}	0
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
d721ac54-f1d3-4fd5-9009-10797ed60714	51280042-881b-4774-b176-266d86e3c51a	2026-01-25 23:12:10.160483+00	2026-01-25 23:12:10.160483+00	"172.25.0.0/28"	172.25.0.0/28	\N	Lan	{"type": "Discovery", "metadata": [{"date": "2026-01-25T23:12:10.160444754Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0"}]}
be1513ab-b4f9-440b-b389-d5521ffcda61	51280042-881b-4774-b176-266d86e3c51a	2026-03-26 01:38:57.871763+00	2026-03-26 01:38:57.871763+00	"10.1.0.0/24"	Blocked Subnet	\N	Lan	{"type": "System"}
\.


--
-- Data for Name: tags; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tags (id, organization_id, name, description, created_at, updated_at, color) FROM stdin;
99bf2d38-dfb2-40bf-848c-9ea754616a34	ccf220b7-6646-4911-8a31-9e93fc809c42	Integration Test Tag	\N	2026-03-26 01:38:34.406642+00	2026-03-26 01:38:34.406642+00	Yellow
\.


--
-- Data for Name: topologies; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.topologies (id, network_id, name, edges, nodes, options, hosts, subnets, services, groups, is_stale, last_refreshed, is_locked, locked_at, locked_by, removed_hosts, removed_services, removed_subnets, removed_groups, parent_id, created_at, updated_at, tags, interfaces, removed_interfaces, ports, removed_ports, bindings, removed_bindings, if_entries, removed_if_entries, entity_tags) FROM stdin;
d9154ee7-7a06-461a-81d3-c0745c85d243	51280042-881b-4774-b176-266d86e3c51a	My Topology	[]	[]	{"local": {"tag_filter": {"hidden_host_tag_ids": [], "hidden_subnet_tag_ids": [], "hidden_service_tag_ids": []}, "show_minimap": true, "no_fade_edges": false, "hide_edge_types": ["HostVirtualization"], "left_zone_title": "Infrastructure", "hide_resize_handles": false}, "request": {"hide_ports": false, "hide_service_categories": ["OpenPorts"], "show_gateway_in_left_zone": true, "group_docker_bridges_by_host": true, "left_zone_service_categories": ["DNS", "ReverseProxy"], "hide_vm_title_on_docker_container": false}}	[]	[{"id": "d721ac54-f1d3-4fd5-9009-10797ed60714", "cidr": "172.25.0.0/28", "name": "172.25.0.0/28", "tags": [], "source": {"type": "Discovery", "metadata": [{"date": "2026-01-25T23:12:10.160444754Z", "type": "SelfReport", "host_id": "7c51d243-60ef-4994-b7db-fa41b23b3644", "daemon_id": "99ac5804-c7b3-4ff2-9bf3-e00b891680c0"}]}, "created_at": "2026-01-25T23:12:10.160483Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-01-25T23:12:10.160483Z", "description": null, "subnet_type": "Lan"}, {"id": "be1513ab-b4f9-440b-b389-d5521ffcda61", "cidr": "10.1.0.0/24", "name": "Blocked Subnet", "tags": [], "source": {"type": "System"}, "created_at": "2026-03-26T01:38:57.871763Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:38:57.871763Z", "description": null, "subnet_type": "Lan"}]	[{"id": "6c1ac910-6f29-41ed-b1ac-4987c4877a2f", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Scanopy Daemon self-report", "type": "reason"}, "confidence": "Certain"}, "metadata": [{"date": "2026-03-26T01:34:34.395324058Z", "type": "Unified", "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "scan_settings": {"arp_retries": null, "arp_rate_pps": null, "is_full_scan": false, "scan_rate_pps": null, "use_npcap_arp": false, "full_scan_interval": null, "port_scan_batch_size": null, "probe_raw_socket_ports": false}, "host_naming_fallback": "BestService", "scan_local_docker_socket": false}]}, "host_id": "39d3169e-1a02-41e6-b3c6-db9716ae6ad4", "bindings": [{"id": "8fc5a98e-6967-4bde-9546-45eee4d303a9", "type": "Port", "port_id": "cacc59cb-f718-4880-9729-8a82af93259d", "created_at": "2026-03-26T01:34:34.395318Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "6c1ac910-6f29-41ed-b1ac-4987c4877a2f", "updated_at": "2026-03-26T01:34:34.395318Z", "interface_id": "1cc15d21-b594-497e-bc33-9077a7160285"}, {"id": "8a2d2f9f-2a6f-4bdb-a2a9-dd995c176174", "type": "Port", "port_id": "cacc59cb-f718-4880-9729-8a82af93259d", "created_at": "2026-03-26T01:34:34.395320Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "6c1ac910-6f29-41ed-b1ac-4987c4877a2f", "updated_at": "2026-03-26T01:34:34.395320Z", "interface_id": "82e2398c-f51f-4c18-9d2f-0c9fbe0cb73e"}], "position": 0, "created_at": "2026-03-26T01:34:34.395325Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:34:34.395325Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "66c57b87-aa53-4115-8410-84e7c07d36a9", "name": "Scanopy Daemon", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.4:60073/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T01:36:28.460535889Z", "type": "Network", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "7c04fa1e-7681-4ebb-bc92-abbc4b31c615", "bindings": [{"id": "f9b9ac17-a774-437c-80f2-f4ec330dde80", "type": "Port", "port_id": "8bf78ca3-8f77-41eb-93e6-db1e4827cec4", "created_at": "2026-03-26T01:36:28.460554Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "66c57b87-aa53-4115-8410-84e7c07d36a9", "updated_at": "2026-03-26T01:36:28.460554Z", "interface_id": "d55ce985-8ff1-463a-b745-dec80429793b"}], "position": 0, "created_at": "2026-03-26T01:36:28.460558Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:36:28.460558Z", "virtualization": null, "service_definition": "Scanopy Daemon"}, {"id": "460356a2-36bf-4682-b746-a4c3f7a84871", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.3:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T01:36:37.142200559Z", "type": "Network", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "7d275c72-0b0d-45f9-af89-0a2603c982b0", "bindings": [{"id": "50707036-2af1-466c-8c4d-2c09b18af317", "type": "Port", "port_id": "bb60b22b-0e54-48ce-ba32-b9c9dde7312f", "created_at": "2026-03-26T01:36:37.142218Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "460356a2-36bf-4682-b746-a4c3f7a84871", "updated_at": "2026-03-26T01:36:37.142218Z", "interface_id": "c0aad36e-1c83-40d3-b050-678f329c6823"}], "position": 0, "created_at": "2026-03-26T01:36:37.142221Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:36:37.142221Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "47c4d443-6cd3-4a91-8822-62455ab2aa38", "name": "PostgreSQL", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 5432/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-03-26T01:37:06.896443658Z", "type": "Network", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "f25afec1-6b91-4acf-ad96-e20855aa2151", "bindings": [{"id": "b62b83b7-4427-4db9-b7c1-c40b9ef7231f", "type": "Port", "port_id": "6e90a29d-5203-4631-8117-35ac95806c9a", "created_at": "2026-03-26T01:37:06.896485Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "47c4d443-6cd3-4a91-8822-62455ab2aa38", "updated_at": "2026-03-26T01:37:06.896485Z", "interface_id": "5d96d23f-a825-44c1-832e-3e75dfa892ee"}], "position": 0, "created_at": "2026-03-26T01:37:06.896490Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:37:06.896490Z", "virtualization": null, "service_definition": "PostgreSQL"}, {"id": "27d3a8a1-c370-4b82-af62-4e65214aec80", "name": "Home Assistant", "tags": ["99bf2d38-dfb2-40bf-848c-9ea754616a34"], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:8123/ contained \\"home assistant\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T01:37:26.721865566Z", "type": "Network", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "520483e2-be2a-4959-8fe9-65cb9cc4c9b4", "bindings": [{"id": "0a4a96d1-ed73-4fed-af73-5217245b3e26", "type": "Port", "port_id": "9cb72ac3-a327-4edb-9add-abcb2e0ecc96", "created_at": "2026-03-26T01:37:26.721887Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "27d3a8a1-c370-4b82-af62-4e65214aec80", "updated_at": "2026-03-26T01:37:26.721887Z", "interface_id": "cecdd4f5-67bd-4785-9a64-142612c14115"}], "position": 0, "created_at": "2026-03-26T01:37:26.721892Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:37:26.721892Z", "virtualization": null, "service_definition": "Home Assistant"}, {"id": "dbe4bba8-94a2-47b6-86d1-9ee0b1c389d9", "name": "Scanopy Server", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": "Response for 172.25.0.1:60072/api/health contained \\"scanopy\\" in body", "type": "reason"}, "confidence": "High"}, "metadata": [{"date": "2026-03-26T01:37:30.691262627Z", "type": "Network", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "520483e2-be2a-4959-8fe9-65cb9cc4c9b4", "bindings": [{"id": "22003f04-4eeb-4e44-b043-c8baf071005b", "type": "Port", "port_id": "315f9db8-d709-4e08-9e21-5d06110073f1", "created_at": "2026-03-26T01:37:30.691279Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "dbe4bba8-94a2-47b6-86d1-9ee0b1c389d9", "updated_at": "2026-03-26T01:37:30.691279Z", "interface_id": "cecdd4f5-67bd-4785-9a64-142612c14115"}], "position": 1, "created_at": "2026-03-26T01:37:30.691283Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:37:30.691283Z", "virtualization": null, "service_definition": "Scanopy Server"}, {"id": "ee579a96-077d-40ee-b1e9-6990f3eb6190", "name": "SSH", "tags": [], "source": {"type": "DiscoveryWithMatch", "details": {"reason": {"data": ["Generic service", [{"data": "Port 22/tcp is open", "type": "reason"}]], "type": "container"}, "confidence": "NotApplicable"}, "metadata": [{"date": "2026-03-26T01:37:43.174674012Z", "type": "Network", "daemon_id": "19b97945-feeb-48f8-9e9a-fc1a3ef19a15", "subnet_ids": null, "snmp_credentials": {"ip_overrides": [], "default_credential": null}, "host_naming_fallback": "BestService"}]}, "host_id": "520483e2-be2a-4959-8fe9-65cb9cc4c9b4", "bindings": [{"id": "22910f8c-388d-4aee-ba44-696107feb612", "type": "Port", "port_id": "cddea114-4c80-4500-923d-a4b4a5eb1761", "created_at": "2026-03-26T01:37:43.174691Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "service_id": "ee579a96-077d-40ee-b1e9-6990f3eb6190", "updated_at": "2026-03-26T01:37:43.174691Z", "interface_id": "cecdd4f5-67bd-4785-9a64-142612c14115"}], "position": 2, "created_at": "2026-03-26T01:37:43.174695Z", "network_id": "51280042-881b-4774-b176-266d86e3c51a", "updated_at": "2026-03-26T01:37:43.174695Z", "virtualization": null, "service_definition": "SSH"}]	[]	t	2026-03-26 01:31:39.610021+00	f	\N	\N	{c6ba00c8-a937-45a3-9dc8-d697e516d260,0a8a820b-1795-4bd0-9269-7d17ea79a8f8,9d775dda-a8d4-4934-bb3c-b10a8f43e883}	{f311c79f-56cc-4900-96b4-1f5b0201ef49}	{544bd0d1-a57b-47f5-b816-d64abf855a09}	{739c5cf9-e27a-4623-9dd6-378318c43025}	\N	2026-03-26 01:31:39.594565+00	2026-03-26 01:31:39.594565+00	{}	[]	{}	[]	{}	[]	{}	[]	{}	[]
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
b94a67ba-8853-468c-b21e-0ae053879623	2026-03-26 01:31:39.574534+00	2026-03-26 01:31:39.574534+00	$argon2id$v=19$m=19456,t=2,p=1$OMvl0uHW8UyHaaiMNco18g$VC+J/CMTQRa20qfBuSoWJ1dFypia/SaX+QdIY00ib2I	\N	\N	\N	user@gmail.com	ccf220b7-6646-4911-8a31-9e93fc809c42	Owner	{}	\N	t	\N	\N	\N	\N	\N
99f69934-8e4b-453d-8729-cbdce7cc7be2	2026-03-26 01:38:57.506534+00	2026-03-26 01:38:57.506534+00	\N	\N	\N	\N	user@example.com	ccf220b7-6646-4911-8a31-9e93fc809c42	Owner	{}	\N	f	\N	\N	\N	\N	\N
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: tower_sessions; Owner: postgres
--

COPY tower_sessions.session (id, data, expiry_date) FROM stdin;
aCSgxs-xtiWaaLD5N2mSkg	\\x93c41092926937f9b0689a25b6b1cfc6a0246881a7757365725f6964d92462393461363762612d383835332d343638632d623231652d30616530353338373936323399cd07ea5c011f27ce27407e85000000	2026-04-02 01:31:39.658538+00
VZhqYu0zg1OSa8y2paNn7g	\\x93c410ee67a3a5b6cc6b92538333ed626a985582a7757365725f6964d92462393461363762612d383835332d343638632d623231652d306165303533383739363233ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92430393231396436612d393733332d346533362d386430652d653863376263346332333734ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea5c012623ce1516e6c3000000	2026-04-02 01:38:35.353822+00
UH8i3udjndBNP6RorPC2og	\\x93c410a2b6f0ac68a43f4dd09d63e7de227f5082a7757365725f6964d92462393461363762612d383835332d343638632d623231652d306165303533383739363233ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92432643830313265642d616264642d343538342d383066352d333538363836616666353737ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6e99cd07ea5c012631ce04a59148000000	2026-04-02 01:38:49.077959+00
wkigsz0gkPWSq0R2mB9LYA	\\x93c410604b1f987644ab92f590203db3a048c282ad70656e64696e675f736574757082a76e6574776f726b83a46e616d65aa4d79204e6574776f726baa6e6574776f726b5f6964d92433666539646431332d643232642d343139312d616263332d303431663366383734353230ac736e6d705f656e61626c6564c2a86f72675f6e616d65af4d79204f7267616e697a6174696f6ea7757365725f6964d92462393461363762612d383835332d343638632d623231652d30616530353338373936323399cd07ea5c012638ce26bc70a5000000	2026-04-02 01:38:56.649883+00
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

\unrestrict VmoNqUwCvOYuWVbj0w3AA5Zdd9yfsC291ve6mz8fUKXQkpycFH35v90tkHfCffa

