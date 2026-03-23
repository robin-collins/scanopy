var TEST_PLANS = [
{
  "branch": "fix/loopback-interface-handling",
  "tests": []
}
,
{
  "branch": "fix/list-config-editor",
  "tests": []
}
,
{
  "branch": "fix/subnet-tags-multi-target-ips",
  "tests": []
}
,
{
  "branch": "test/credential-seed-e2e",
  "tests": []
}
,
{
  "branch": "fix/entity-deletion-ux",
  "tests": []
}
,
{
  "branch": "fix/docker-service-reconciliation",
  "tests": []
}
,
{
  "branch": "tooling/docker-proxy-test-env",
  "tests": []
}
,
{
  "branch": "feat/discovery-scan-settings",
  "tests": []
}
,
{
  "branch": "fix/credential-service-port-binding",
  "tests": [
    {
      "id": "remote-docker-proxy-port-binding",
      "category": "Credential Port Binding",
      "description": "Remote Docker proxy service should bind to the probed port",
      "steps": [
        "Navigate to a host that has a Docker proxy credential targeting a remote IP on port 2376",
        "Run a network discovery scan that covers that host",
        "After scan completes, navigate to the discovered host's services",
        "Verify the Docker service has a port 2376 binding (not just an interface binding)",
        "Verify port 2376 does NOT appear in 'Unclaimed Open Ports'"
      ],
      "setup": "Create a DockerProxy credential targeting a remote host IP with port 2376. Ensure the remote host has Docker API accessible on that port.",
      "expected": "Docker service shows with both an interface binding and a port 2376 binding. Port 2376 is not listed under Unclaimed Open Ports.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "custom-port-docker-proxy-binding",
      "category": "Credential Port Binding",
      "description": "Docker proxy on a custom port should bind to that custom port, not 2376",
      "steps": [
        "Navigate to a host that has a Docker proxy credential targeting a remote IP on port 8008",
        "Run a network discovery scan that covers that host",
        "After scan completes, navigate to the discovered host's services",
        "Verify the Docker service has a port 8008 binding",
        "Verify port 8008 does NOT appear in 'Unclaimed Open Ports'"
      ],
      "setup": "Create a DockerProxy credential targeting a remote host IP with port 8008 (custom port). Ensure Docker API is accessible on that port.",
      "expected": "Docker service shows with port 8008 binding (not 2376). Port 8008 is not listed under Unclaimed Open Ports.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "local-docker-socket-no-port",
      "category": "Credential Port Binding",
      "description": "Local Docker socket service should have interface binding only (no port)",
      "steps": [
        "Run a unified discovery scan on a daemon host that uses a local Docker socket (no proxy credential)",
        "After scan completes, navigate to the daemon host's services",
        "Verify the Docker service has an interface binding only",
        "Verify no spurious port bindings were added to the Docker service"
      ],
      "setup": "Ensure the daemon is configured to use local Docker socket (default /var/run/docker.sock), not a proxy credential.",
      "expected": "Docker service shows with interface binding only, no port binding.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "local-docker-proxy-port-binding",
      "category": "Credential Port Binding",
      "description": "Local Docker proxy service should bind to the probed port",
      "steps": [
        "Run a unified discovery scan on a daemon host that uses a Docker proxy credential on port 2376",
        "After scan completes, navigate to the daemon host's services",
        "Verify the Docker service has a port 2376 binding"
      ],
      "setup": "Configure a DockerProxy credential for localhost/127.0.0.1 targeting port 2376.",
      "expected": "Docker service shows with both an interface binding and a port 2376 binding.",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "port-not-in-unclaimed-after-credential-match",
      "category": "Credential Port Binding",
      "description": "Port claimed by credential service should not appear in Unclaimed Open Ports",
      "steps": [
        "Run a network discovery scan on a host with a Docker proxy credential on port 2376",
        "After scan completes, navigate to the host's services",
        "Check the 'Unclaimed Open Ports' service",
        "Verify port 2376 is NOT listed there"
      ],
      "setup": "Create a DockerProxy credential targeting a host with port 2376 open.",
      "expected": "Port 2376 is bound to the Docker service and does not appear in Unclaimed Open Ports.",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/non-interfaced-host-creation",
  "tests": []
}
,
{
  "branch": "fix/topology-creation-422",
  "tests": []
}
,
{
  "branch": "docs/unified-discovery-migration",
  "tests": []
}
,
{
  "branch": "fix/stalled-session-orphan",
  "tests": []
}
];
