-- Community self-hosted installations may create and scan unlimited networks.
-- Plan configuration is stored with each organization, so update existing rows
-- as well as changing the canonical plan used for newly provisioned orgs.
SET lock_timeout = '5s';
SET statement_timeout = '5s';

UPDATE organizations
SET plan = jsonb_set(plan, '{included_networks}', 'null'::jsonb, true)
WHERE plan->>'type' = 'Community'
  AND plan->'included_networks' IS DISTINCT FROM 'null'::jsonb;
