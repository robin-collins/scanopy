var TEST_PLANS = [
{
  "branch": "feat/brevo-doi",
  "tests": []
}
,
{
  "branch": "fix/host-edit-validation",
  "tests": [
    {
      "id": "save-after-service-edit",
      "category": "Service Edit",
      "description": "Verify editing a service name in the standalone Service modal saves and displays correctly",
      "steps": [
        "Go to the Services tab",
        "Click on a service to open the Service edit modal",
        "Change the service name to a new valid name",
        "Click Update Service"
      ],
      "expected": "The modal closes and the Services tab list immediately shows the updated service name without needing a page refresh.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "validation-error-shows-entity-name",
      "category": "Error Messages",
      "description": "Verify validation errors show human-readable entity names instead of array indices",
      "steps": [
        "Open the host edit modal for a host with an interface",
        "Navigate to the Interfaces tab",
        "Select the interface and clear the IP address field",
        "Click Update Host"
      ],
      "expected": "Error toast shows 'Interface 1: ip address' instead of '#1: ip address'.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/disposable-email-check",
  "tests": []
}
,
{
  "branch": "feat/topology-pdf-html-export",
  "tests": []
}
,
{
  "branch": "fix/vlan-interface-dedup",
  "tests": []
}
,
{
  "branch": "feat/discovery-integrations-v2",
  "tests": [
    {
      "id": "discovery-no-credentials",
      "category": "Discovery Pipeline",
      "description": "Run discovery without any credentials configured — basic network scan only",
      "steps": [
        "Navigate to a network with a daemon",
        "Ensure no credentials are configured on the network",
        "Click 'Run Discovery'",
        "Wait for discovery to complete"
      ],
      "expected": "Discovery completes successfully. Hosts are discovered via ARP + TCP port scanning. Progress bar advances smoothly from 0-100%. No errors in daemon logs.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-snmp-credentials",
      "category": "Discovery Pipeline",
      "description": "Run discovery with SNMP credentials — SNMP hosts should be enriched",
      "steps": [
        "Navigate to a network with a daemon and SNMP-enabled devices",
        "Add an SNMP v2c credential with the correct community string",
        "Assign credential to the network (broadcast scope)",
        "Click 'Run Discovery'",
        "Wait for discovery to complete",
        "Open a host that has SNMP enabled"
      ],
      "setup": "Ensure at least one host on the network has SNMP enabled and responds to the configured community string.",
      "expected": "SNMP-enabled hosts show enrichment: sys_descr, sys_name, manufacturer, model visible in host details. SNMP service appears on the host. If the host is a router, remote subnets discovered via ipAddrTable should appear. Interface table (ifEntries) populated.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-docker-credentials",
      "category": "Discovery Pipeline",
      "description": "Run discovery with Docker proxy credentials — containers should appear as services",
      "steps": [
        "Navigate to a network with a daemon and a Docker host",
        "Add a Docker Proxy credential targeting the Docker host IP",
        "Click 'Run Discovery'",
        "Wait for discovery to complete",
        "Open the Docker host"
      ],
      "setup": "Ensure at least one host on the network has a Docker proxy running on the configured port with containers running.",
      "expected": "Docker daemon service appears on the host. Docker containers appear as services with Docker virtualization metadata (container name, container ID). Container ports and interfaces are visible.",
      "flow": "setup",
      "sequence": 3,
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-localhost-docker",
      "category": "Discovery Pipeline",
      "description": "Run discovery with Docker proxy credential targeting localhost",
      "steps": [
        "Navigate to a network with a daemon that has Docker running locally",
        "Add a Docker Proxy credential with IP override targeting 127.0.0.1",
        "Click 'Run Discovery'",
        "Wait for discovery to complete",
        "Open the daemon host"
      ],
      "setup": "Ensure the daemon host has Docker running with containers. Configure a Docker Proxy credential targeting 127.0.0.1.",
      "expected": "Daemon host shows Docker daemon service and container services from the localhost integration phase (0-5% progress range).",
      "flow": "setup",
      "sequence": 4,
      "status": null,
      "feedback": null
    },
    {
      "id": "snmp-if-entries-created",
      "category": "SNMP Integration",
      "description": "SNMP ifTable entries are created as IfEntry entities",
      "steps": [
        "Run a network discovery on a subnet with an SNMP-enabled switch",
        "Open the discovered switch's detail page",
        "Check that interface entries (ifTable) are listed"
      ],
      "setup": "Ensure SNMP credentials are configured. Target subnet must contain a managed switch.",
      "expected": "IfEntry entities created with correct if_descr, if_name, if_type, speed, admin/oper status",
      "flow": "setup",
      "sequence": 5,
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-progress-not-stalled",
      "category": "Progress Reporting",
      "description": "Discovery progress advances smoothly and doesn't trigger stall detection",
      "steps": [
        "Navigate to a network with a daemon",
        "Configure SNMP and/or Docker credentials",
        "Click 'Run Discovery'",
        "Observe the progress bar during discovery"
      ],
      "setup": "Configure credentials that will match hosts on the network so integrations run during scanning.",
      "expected": "Progress bar advances from 0% through 5% (daemon host phase) to 5-100% (network scan). Progress should not stall for more than 30 seconds at any point. Discovery should not be killed by the 5-minute stall detector.",
      "flow": "setup",
      "sequence": 6,
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-cancel",
      "category": "Discovery Pipeline",
      "description": "Cancelling a running discovery session works correctly",
      "steps": [
        "Navigate to a network with a daemon",
        "Click 'Run Discovery'",
        "While discovery is in progress, click 'Cancel Discovery'",
        "Verify the session is cancelled"
      ],
      "expected": "Discovery session is cancelled. Progress stops. Session status shows 'Cancelled'. No orphaned sessions.",
      "status": null,
      "feedback": null
    },
    {
      "id": "discovery-multiple-credential-types",
      "category": "Discovery Pipeline",
      "description": "Discovery with both SNMP and Docker credentials configured simultaneously",
      "steps": [
        "Navigate to a network with a daemon",
        "Add both SNMP and Docker Proxy credentials",
        "Click 'Run Discovery'",
        "Wait for discovery to complete",
        "Check hosts with SNMP and hosts with Docker"
      ],
      "setup": "Configure both SNMP (broadcast) and Docker Proxy (per-host) credentials. Ensure at least one SNMP device and one Docker host exist on the network.",
      "expected": "SNMP hosts show enrichment. Docker hosts show container services. Both credential types work in the same discovery session without interference.",
      "flow": "setup",
      "sequence": 7,
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/onboarding-mobile",
  "tests": [
    {
      "id": "email-install-command-button",
      "category": "Email Install Command",
      "description": "Email button appears in modal footer when email is configured",
      "steps": [
        "Open daemon install step",
        "Look for 'Email me this command' button in the modal footer",
        "Click it"
      ],
      "setup": "Ensure email service is configured (has_email_service config flag is true)",
      "expected": "Button with mail icon appears in the modal footer alongside 'I've run the install command'. Clicking it shows a success toast. Button is disabled while sending.",
      "flow": "setup",
      "sequence": 1,
      "status": null,
      "feedback": null
    },
    {
      "id": "email-per-os-command",
      "category": "Email Install Command",
      "description": "Emailed command identifies the correct OS",
      "steps": [
        "Select Windows tab in the install step",
        "Click 'Email me this command'",
        "Check the email references Windows specifically"
      ],
      "setup": "Ensure email service is configured",
      "expected": "The email instructions mention the specific OS (e.g., 'Run it on your Windows server') matching the currently selected OS tab.",
      "flow": "setup",
      "sequence": 2,
      "status": null,
      "feedback": null
    },
    {
      "id": "os-selector-mobile-label",
      "category": "Daemon Install Modal",
      "description": "Mobile OS selector dropdown label says 'Select operating system'",
      "steps": [
        "Open daemon install step on mobile width",
        "Check the OS selector label"
      ],
      "expected": "Label reads 'Select operating system' above the dropdown.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/daemon-connect-ux",
  "tests": [
    {
      "id": "copyable-command-click-to-copy",
      "category": "Components",
      "description": "CopyableCommand copies text on click and shows inline (not full width)",
      "steps": [
        "Trigger trouble state, expand a checklist item with a command",
        "Click the command strip",
        "Paste somewhere to verify"
      ],
      "expected": "Command copied to clipboard, success toast shown. Command strip is inline width (wraps to content), not full-width.",
      "status": null,
      "feedback": null
    },
    {
      "id": "homepage-checklist-no-regression",
      "category": "Component Reuse",
      "description": "Homepage GettingStartedChecklist renders correctly with shared ChecklistItem",
      "steps": [
        "Navigate to homepage with a fresh org (not all steps complete)",
        "Observe the Getting Started checklist"
      ],
      "expected": "Green checkmarks for completed steps, empty circles for pending. Descriptions visible for actionable steps. 'While you wait' suggestions appear during active discovery. No visual regressions from the timeline layout change.",
      "status": null,
      "feedback": null
    },
    {
      "id": "daemon-dns-failure-retry",
      "category": "Backend Logging",
      "description": "DNS resolution failure retries and gives up cleanly",
      "steps": [
        "Start daemon with nonexistent hostname"
      ],
      "setup": "Run daemon with --server-url http://nonexistent.invalid:60072",
      "expected": "Initial failure logged, retries with backoff (5s, 10s, 20s, 40s, 60s), then 'Daemon NOT ready — fix the issue and restart (Ctrl+C)'. No polling loop started.",
      "status": null,
      "feedback": null
    },
    {
      "id": "modal-close-resets-form-and-checklist",
      "category": "Modal Behavior",
      "description": "Closing and reopening modal resets everything including troubleshooting checklist state",
      "steps": [
        "Open wizard, change advanced settings, trigger trouble state, check off some troubleshooting items",
        "Close the modal",
        "Reopen the wizard"
      ],
      "expected": "Configure tab shown, all form fields at defaults, no trouble state. If you trigger trouble again, all checklist items are unchecked.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "fix/honeypot-selfhosted",
  "tests": []
}
,
{
  "branch": "fix/daemon-oom",
  "tests": [
    {
      "id": "arp-scan-cutoff-setting",
      "category": "Scan Settings",
      "description": "Verify arp_scan_cutoff warning shows inline with the field when lowered",
      "steps": [
        "Open a discovery modal and go to the Speed tab",
        "Find the ARP Scan Cutoff field in the ARP section",
        "Set it to a value below 15 (e.g., 12)",
        "Observe the amber warning text in the same row as the field"
      ],
      "expected": "Amber warning text appears in the same grid row as the cutoff input, showing IP count and estimated scan time at 50 pps.",
      "status": null,
      "feedback": null
    }
  ]
}
,
{
  "branch": "docs/unified-discovery-migration",
  "tests": []
}
,
{
  "branch": "fix/open-ports-reclaim",
  "tests": []
}
,
{
  "branch": "fix/pagination-counter",
  "tests": []
}
,
{
  "branch": "feat/discovery-tab-consolidation",
  "tests": []
}
,
{
  "branch": "feat/n8n-service",
  "tests": []
}
];
