import type { TextFont } from '$lib/features/custom-topology-views/queries';
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
