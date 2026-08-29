import type { BorderStyle, CustomViewNode } from '$lib/features/custom-topology-views/queries';
import type { components } from '$lib/api/schema';
import { createColorHelper } from '$lib/shared/utils/styling';
import type { Host, IPAddress, Port } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import { getFontCssStack } from './fonts';

export function getTextFontSize(size: number | null | undefined): number {
	return size == null ? 16 : Math.max(10, Math.round(size));
}

/**
 * Canvas-level typography, which every node inherits unless it overrides.
 *
 * Confirmed item 1: canvas settings are defaults, each element overrides them
 * independently, and clearing an override restores inheritance. A null on the
 * node therefore has to fall through to the canvas value, not to a constant -
 * otherwise clearing an override silently jumps to a hardcoded 16px/system
 * font rather than back to what the canvas is set to.
 */
export interface CanvasTypographyDefaults {
	fontFamily?: string | null;
	fontSize?: number | null;
	textColor?: components['schemas']['Color'] | null;
	fontBold?: boolean | null;
	fontItalic?: boolean | null;
	fontUnderline?: boolean | null;
	textAlign?: components['schemas']['TextAlign'] | null;
}

export function getNodeAppearance(
	node: CustomViewNode,
	canvasDefaults: CanvasTypographyDefaults = {}
) {
	const primary = createColorHelper(node.primary_color ?? node.color ?? 'Gray').rgb;
	const secondary = createColorHelper(
		node.secondary_color ?? node.primary_color ?? node.color ?? 'Gray'
	).rgb;
	const background = createColorHelper(node.background_color ?? 'Gray').rgb;
	const textColor = createColorHelper(node.text_color ?? canvasDefaults.textColor ?? 'Gray').rgb;
	return {
		primary,
		secondary,
		background,
		textColor,
		opacity: Math.min(100, Math.max(0, node.opacity ?? 100)) / 100,
		fontFamily: getFontCssStack(node.font_family ?? canvasDefaults.fontFamily),
		fontSize: getTextFontSize(node.font_size ?? canvasDefaults.fontSize),
		fontWeight: (node.font_bold ?? canvasDefaults.fontBold ?? false) ? '700' : '400',
		fontStyle: (node.font_italic ?? canvasDefaults.fontItalic ?? false) ? 'italic' : 'normal',
		textDecoration:
			(node.font_underline ?? canvasDefaults.fontUnderline ?? false) ? 'underline' : 'none',
		textAlign: (node.text_align ?? canvasDefaults.textAlign ?? 'Left').toLowerCase(),
		borderStyle: ((node.border_style ?? 'Solid') as BorderStyle).toLowerCase(),
		borderRadius: node.corner_style === 'Square' ? '0' : '0.5rem'
	};
}

/** Only allow browser-safe links on canvas objects and joins. */
export function getSafeCanvasLink(link: string | null | undefined): string | null {
	if (!link?.trim()) return null;
	try {
		const url = new URL(link);
		return url.protocol === 'http:' || url.protocol === 'https:' ? url.href : null;
	} catch {
		return null;
	}
}

export function getHostServices(services: Service[], hostId: string): Service[] {
	return services.filter((service) => service.host_id === hostId);
}

/** A host is usable on the canvas only if it has a hostname, an IP address, or at least one service. */
export function hasHostIdentifier(
	host: Host,
	ipAddresses: IPAddress[],
	services: Service[]
): boolean {
	if (host.hostname?.trim()) return true;
	if (ipAddresses.some((ip) => ip.host_id === host.id)) return true;
	if (services.some((service) => service.host_id === host.id)) return true;
	return false;
}

/** Excludes hosts with no hostname, IP address, or service from the custom-view palette. */
export function filterIdentifiedHosts(
	hosts: Host[],
	ipAddresses: IPAddress[],
	services: Service[]
): Host[] {
	return hosts.filter((host) => hasHostIdentifier(host, ipAddresses, services));
}

/** Preferred display identifier for a host: hostname, falling back to its first known IP address. */
export function getHostIdentifier(host: Host, ipAddresses: IPAddress[]): string | null {
	if (host.hostname?.trim()) return host.hostname;
	return ipAddresses.find((ip) => ip.host_id === host.id)?.ip_address ?? null;
}

/**
 * Palette label for a service, qualified with its host so identical service
 * names on different hosts are distinguishable, e.g. `Samba@HOSTNAME`. When
 * a service name repeats on a host — either as multiple bindings on one
 * service record, or as multiple service records sharing a name (both occur
 * in practice) — the port/protocol from the first binding is appended, e.g.
 * `DNS@192.168.1.1:53/UDP`.
 */
export function formatServiceLabel(
	service: Service,
	hosts: Host[],
	ipAddresses: IPAddress[],
	ports: Port[],
	allServices: Service[]
): string {
	const host = hosts.find((h) => h.id === service.host_id);
	const hostIdentifier = host ? getHostIdentifier(host, ipAddresses) : null;
	if (!hostIdentifier) return service.name;

	const sameNameOnHost = allServices.filter(
		(s) => s.host_id === service.host_id && s.name === service.name
	);
	const hasDuplicateInstances = sameNameOnHost.length > 1 || service.bindings.length > 1;
	if (!hasDuplicateInstances) return `${service.name}@${hostIdentifier}`;

	const portBinding = service.bindings.find((b) => b.type === 'Port');
	const port = portBinding ? ports.find((p) => p.id === portBinding.port_id) : undefined;
	if (!port) return `${service.name}@${hostIdentifier}`;

	const protocolSuffix = port.protocol === 'Udp' ? '/UDP' : '';
	return `${service.name}@${hostIdentifier}:${port.number}${protocolSuffix}`;
}

export function filterPaletteHosts(hosts: Host[], services: Service[], search: string): Host[] {
	const term = search.trim().toLowerCase();
	if (!term) return hosts;

	return hosts.filter(
		(host) =>
			host.name.toLowerCase().includes(term) ||
			(host.hostname?.toLowerCase().includes(term) ?? false) ||
			services.some(
				(service) => service.host_id === host.id && service.name.toLowerCase().includes(term)
			)
	);
}
