import { describe, expect, it } from 'vitest';
import type { Host } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import {
	filterPaletteHosts,
	getHostServices,
	getTextFontFamily,
	getTextFontSize
} from '$lib/features/topology/components/visualization/custom/custom-view-model';

const hosts = [
	{ id: 'host-a', name: 'Alpha', hostname: 'alpha.example.test' },
	{ id: 'host-b', name: 'Bravo', hostname: null }
] as Host[];

const services = [
	{ id: 'service-a', host_id: 'host-a', name: 'PostgreSQL' },
	{ id: 'service-b', host_id: 'host-b', name: 'Secure Shell' }
] as Service[];

describe('custom topology view model', () => {
	it('uses readable typography defaults and clamps persisted sizes', () => {
		expect(getTextFontFamily(null)).toContain('ui-sans-serif');
		expect(getTextFontFamily('Serif')).toContain('Georgia');
		expect(getTextFontFamily('Monospace')).toContain('Cascadia Code');
		expect(getTextFontSize(null)).toBe(16);
		expect(getTextFontSize(4)).toBe(10);
		expect(getTextFontSize(100)).toBe(72);
	});

	it('filters host cards by name, hostname, and their services', () => {
		expect(filterPaletteHosts(hosts, services, 'bravo')).toEqual([hosts[1]]);
		expect(filterPaletteHosts(hosts, services, 'alpha.example')).toEqual([hosts[0]]);
		expect(filterPaletteHosts(hosts, services, 'postgres')).toEqual([hosts[0]]);
	});

	it('associates service preview rows with the selected host only', () => {
		expect(getHostServices(services, 'host-a')).toEqual([services[0]]);
		expect(getHostServices(services, 'missing')).toEqual([]);
	});
});
