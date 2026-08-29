import { describe, expect, it } from 'vitest';
import type { Host } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import {
	filterPaletteHosts,
	getNodeAppearance,
	getSafeCanvasLink,
	getHostServices,
	getTextFontSize
} from '$lib/features/topology/components/visualization/custom/custom-view-model';
import { getFontCssStack } from '$lib/features/topology/components/visualization/custom/fonts';

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
		expect(getFontCssStack(null)).toContain('ui-sans-serif');
		expect(getFontCssStack('not-a-real-font')).toContain('ui-sans-serif');
		expect(getFontCssStack('Lora')).toContain('Georgia');
		expect(getFontCssStack('Roboto Mono')).toContain('Cascadia Code');
		expect(getTextFontSize(null)).toBe(16);
		expect(getTextFontSize(4)).toBe(10);
		expect(getTextFontSize(100)).toBe(100);
		expect(getTextFontSize(5)).toBe(10);
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

	it('normalizes shared appearance settings for every canvas object', () => {
		const appearance = getNodeAppearance({
			primary_color: 'Blue',
			secondary_color: 'Red',
			background_color: 'Gray',
			opacity: 35,
			font_bold: true,
			font_italic: true,
			font_underline: true,
			text_align: 'Center',
			border_style: 'Dashed',
			corner_style: 'Square'
		} as Parameters<typeof getNodeAppearance>[0]);

		expect(appearance.opacity).toBe(0.35);
		expect(appearance.fontWeight).toBe('700');
		expect(appearance.fontStyle).toBe('italic');
		expect(appearance.textDecoration).toBe('underline');
		expect(appearance.textAlign).toBe('center');
		expect(appearance.borderStyle).toBe('dashed');
		expect(appearance.borderRadius).toBe('0');
	});

	it('rejects unsafe canvas links', () => {
		expect(getSafeCanvasLink('https://scanopy.net/docs')).toBe('https://scanopy.net/docs');
		expect(getSafeCanvasLink('javascript:alert(1)')).toBeNull();
		expect(getSafeCanvasLink('not a url')).toBeNull();
	});
});
