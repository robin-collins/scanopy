-- Backfill SubnetVirtualization on DockerBridge subnets.
-- Docker bridge subnets need a service_id pointing to their Docker daemon service
-- for per-host deduplication (same CIDR on different hosts = distinct subnets).

-- 1. Backfill subnets table: join discovery metadata host_id → Docker daemon service
UPDATE subnets s
SET base = jsonb_set(
    s.base,
    '{virtualization}',
    jsonb_build_object(
        'type', 'Docker',
        'service_id', docker_svc.id::text
    )
)
FROM (
    SELECT s2.id AS subnet_id,
           (s2.base->'source'->'metadata'->0->>'host_id')::uuid AS host_id
    FROM subnets s2
    WHERE s2.base->>'subnet_type' = 'DockerBridge'
      AND s2.base->'virtualization' IS NULL
      AND s2.base->'source'->>'type' = 'Discovery'
) bridge
JOIN services docker_svc ON docker_svc.base->>'host_id' = bridge.host_id::text
    AND docker_svc.base->'service_definition'->>'name' = 'Docker'
WHERE s.id = bridge.subnet_id;

-- 2. Backfill topology snapshots: uses only data within the same snapshot
UPDATE topologies t
SET subnets = (
    SELECT jsonb_agg(
        CASE
            WHEN (subnet_elem->'base'->>'subnet_type') = 'DockerBridge'
                 AND subnet_elem->'base'->'virtualization' IS NULL
            THEN jsonb_set(
                subnet_elem,
                '{base,virtualization}',
                COALESCE(
                    (
                        SELECT jsonb_build_object('type', 'Docker', 'service_id', svc_elem->>'id')
                        FROM jsonb_array_elements(t.services) AS svc_elem
                        WHERE svc_elem->'base'->'service_definition'->>'name' = 'Docker'
                          AND svc_elem->'base'->>'host_id' = (
                              subnet_elem->'base'->'source'->'metadata'->0->>'host_id'
                          )
                        LIMIT 1
                    ),
                    'null'::jsonb
                )
            )
            ELSE subnet_elem
        END
    )
    FROM jsonb_array_elements(t.subnets) AS subnet_elem
)
WHERE EXISTS (
    SELECT 1 FROM jsonb_array_elements(t.subnets) AS elem
    WHERE (elem->'base'->>'subnet_type') = 'DockerBridge'
      AND elem->'base'->'virtualization' IS NULL
);
