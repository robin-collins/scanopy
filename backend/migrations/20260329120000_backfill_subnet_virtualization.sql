-- Add virtualization column to subnets table and backfill DockerBridge subnets.
-- SubnetVirtualization tracks which service owns a virtual subnet (e.g., Docker daemon).
-- Docker bridge subnets need this for per-host dedup (same CIDR on different hosts = distinct).

-- 1. Add virtualization column
ALTER TABLE subnets ADD COLUMN IF NOT EXISTS virtualization jsonb;

-- 2. Backfill subnets table: join discovery metadata host_id → Docker daemon service
UPDATE subnets s
SET virtualization = jsonb_build_object(
    'type', 'Docker',
    'service_id', docker_svc.id::text
)
FROM (
    SELECT s2.id AS subnet_id,
           (s2.source->'metadata'->0->>'host_id')::uuid AS host_id
    FROM subnets s2
    WHERE s2.subnet_type = 'DockerBridge'
      AND s2.virtualization IS NULL
      AND s2.source->>'type' = 'Discovery'
      AND s2.source->'metadata'->0->>'host_id' IS NOT NULL
) bridge
JOIN services docker_svc
    ON docker_svc.host_id = bridge.host_id
    AND docker_svc.service_definition = '"Docker"'
WHERE s.id = bridge.subnet_id;

-- 3. Backfill topology snapshots (self-contained — uses services from same snapshot)
UPDATE topologies t
SET subnets = (
    SELECT jsonb_agg(
        CASE
            WHEN (subnet_elem->>'subnet_type') = 'DockerBridge'
                 AND subnet_elem->'virtualization' IS NULL
            THEN subnet_elem || jsonb_build_object(
                'virtualization',
                COALESCE(
                    (
                        SELECT jsonb_build_object('type', 'Docker', 'service_id', svc_elem->>'id')
                        FROM jsonb_array_elements(t.services) AS svc_elem
                        WHERE svc_elem->>'name' = 'Docker'
                          AND svc_elem->>'host_id' = (
                              subnet_elem->'source'->'metadata'->0->>'host_id'
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
    WHERE (elem->>'subnet_type') = 'DockerBridge'
      AND elem->'virtualization' IS NULL
);
