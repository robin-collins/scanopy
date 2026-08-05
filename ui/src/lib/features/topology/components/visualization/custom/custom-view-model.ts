import type {
	BorderStyle,
	CustomViewNode,
	TextFont
} from '$lib/features/custom-topology-views/queries';
import { createColorHelper } from '$lib/shared/utils/styling';
import type { Host } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';

export const TEXT_FONT_FAMILIES: Record<TextFont, string> = {
	Sans: 'ui-sans-serif, system-ui, sans-serif',
	Serif: 'Georgia, Cambria, "Times New Roman", serif',
	Monospace: '"Cascadia Code", "SFMono-Regular", Consolas, monospace'
};

export function getTextFontFamily(font: TextFont | null | undefined): string {
	return TEXT_FONT_FAMILIES[font ?? 'Sans'];
}

export function getTextFontSize(size: number | null | undefined): number {
	return size == null ? 16 : Math.min(72, Math.max(10, Math.round(size)));
}

export function getNodeAppearance(node: CustomViewNode) {
	const primary = createColorHelper(node.primary_color ?? node.color ?? 'Gray').rgb;
	const secondary = createColorHelper(
		node.secondary_color ?? node.primary_color ?? node.color ?? 'Gray'
	).rgb;
	const background = createColorHelper(node.background_color ?? 'Gray').rgb;
	const fontStyle = node.font_style ?? 'Normal';
	return {
		primary,
		secondary,
		background,
		opacity: Math.min(100, Math.max(0, node.opacity ?? 100)) / 100,
		fontFamily: getTextFontFamily(node.font_family),
		fontSize: getTextFontSize(node.font_size),
		fontWeight: fontStyle === 'Bold' || fontStyle === 'BoldItalic' ? '700' : '400',
		fontStyle: fontStyle === 'Italic' || fontStyle === 'BoldItalic' ? 'italic' : 'normal',
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
