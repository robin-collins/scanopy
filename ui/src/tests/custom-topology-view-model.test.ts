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

describe('canvas typography inheritance', () => {
	const node = (overrides: Record<string, unknown> = {}) =>
		({ ...overrides }) as Parameters<typeof getNodeAppearance>[0];

	it('inherits the canvas font size when the node does not override it', () => {
		const appearance = getNodeAppearance(node(), { fontSize: 28 });
		expect(appearance.fontSize).toBe(28);
	});

	it('lets a node override the canvas default', () => {
		const appearance = getNodeAppearance(node({ font_size: 40 }), { fontSize: 28 });
		expect(appearance.fontSize).toBe(40);
	});

	it('falls back to the canvas default once an override is cleared', () => {
		const overridden = getNodeAppearance(node({ font_size: 40 }), { fontSize: 28 });
		const cleared = getNodeAppearance(node({ font_size: null }), { fontSize: 28 });

		expect(overridden.fontSize).toBe(40);
		expect(cleared.fontSize).toBe(28);
	});

	it('still applies the 10px floor to an inherited value', () => {
		expect(getNodeAppearance(node(), { fontSize: 4 }).fontSize).toBe(10);
	});

	it('inherits the canvas font family, and a node override wins', () => {
		// Font ids come from the catalog; getFontCssStack falls back to the safe
		// stack for anything it does not recognise, so the assertion has to use
		// real catalog entries or it proves nothing.
		const inherited = getNodeAppearance(node(), { fontFamily: 'Roboto' });
		const overridden = getNodeAppearance(node({ font_family: 'Inter' }), {
			fontFamily: 'Roboto'
		});

		expect(inherited.fontFamily).toContain('Roboto');
		expect(overridden.fontFamily).toContain('Inter');
		expect(overridden.fontFamily).not.toContain('Roboto');
	});

	it('uses the built-in default only when neither node nor canvas specifies one', () => {
		expect(getNodeAppearance(node()).fontSize).toBe(16);
	});
});
