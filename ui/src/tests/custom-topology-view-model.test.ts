import { describe, expect, it } from 'vitest';
import type { Host } from '$lib/features/hosts/types/base';
import type { IPAddress } from '$lib/features/hosts/types/base';
import type { Service } from '$lib/features/services/types/base';
import {
	filterPaletteHosts,
	filterIdentifiedHosts,
	getNodeAppearance,
	getSafeCanvasLink,
	getServiceLabelPlacement,
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

	it('excludes hosts with no hostname, IP address, or service from the palette', () => {
		const unidentified = { id: 'host-empty', name: '', hostname: null } as Host;
		const hostnameOnly = { id: 'host-name', name: '', hostname: 'named.example.test' } as Host;
		const ipOnly = { id: 'host-ip', name: '', hostname: null } as Host;
		const serviceOnly = { id: 'host-service', name: '', hostname: null } as Host;
		const ipAddresses = [{ id: 'ip-a', host_id: ipOnly.id }] as IPAddress[];
		const identifyingServices = [
			{ id: 'service-only', host_id: serviceOnly.id, name: 'SSH' }
		] as Service[];

		expect(
			filterIdentifiedHosts(
				[unidentified, hostnameOnly, ipOnly, serviceOnly],
				ipAddresses,
				identifyingServices
			)
		).toEqual([hostnameOnly, ipOnly, serviceOnly]);
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

	it('freely combines service label anchors and bounds persisted offsets', () => {
		expect(getServiceLabelPlacement('Left', 'Top', 12, -8)).toEqual({
			justifyContent: 'flex-start',
			alignItems: 'flex-start',
			transform: 'translate(12px, -8px)'
		});
		expect(getServiceLabelPlacement('Right', 'Bottom', 5000, -5000)).toEqual({
			justifyContent: 'flex-end',
			alignItems: 'flex-end',
			transform: 'translate(1000px, -1000px)'
		});
		expect(getServiceLabelPlacement('Center', 'Middle', null, null)).toEqual({
			justifyContent: 'center',
			alignItems: 'center',
			transform: 'translate(0px, 0px)'
		});
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

	it('inherits canvas text colour, emphasis, and alignment', () => {
		const inherited = getNodeAppearance(node(), {
			textColor: 'Red',
			fontBold: true,
			fontItalic: true,
			fontUnderline: true,
			textAlign: 'Right'
		});
		const red = getNodeAppearance(node({ text_color: 'Red' }));

		expect(inherited.textColor).toBe(red.textColor);
		expect(inherited.fontWeight).toBe('700');
		expect(inherited.fontStyle).toBe('italic');
		expect(inherited.textDecoration).toBe('underline');
		expect(inherited.textAlign).toBe('right');
	});

	it('keeps text colour independent from decorative colours', () => {
		const appearance = getNodeAppearance(node({ primary_color: 'Blue', text_color: 'Red' }));
		const blue = getNodeAppearance(node({ text_color: 'Blue' }));
		const red = getNodeAppearance(node({ text_color: 'Red' }));

		expect(appearance.primary).toBe(blue.textColor);
		expect(appearance.textColor).toBe(red.textColor);
		expect(appearance.textColor).not.toBe(appearance.primary);
	});

	it('lets explicit node values override every canvas text default', () => {
		const overridden = getNodeAppearance(
			node({
				text_color: 'Blue',
				font_bold: false,
				font_italic: false,
				font_underline: false,
				text_align: 'Center'
			}),
			{
				textColor: 'Red',
				fontBold: true,
				fontItalic: true,
				fontUnderline: true,
				textAlign: 'Right'
			}
		);
		const blue = getNodeAppearance(node({ text_color: 'Blue' }));

		expect(overridden.textColor).toBe(blue.textColor);
		expect(overridden.fontWeight).toBe('400');
		expect(overridden.fontStyle).toBe('normal');
		expect(overridden.textDecoration).toBe('none');
		expect(overridden.textAlign).toBe('center');
	});

	it('restores canvas values when text appearance overrides are cleared', () => {
		const cleared = getNodeAppearance(
			node({
				text_color: null,
				font_bold: null,
				font_italic: null,
				font_underline: null,
				text_align: null
			}),
			{
				textColor: 'Purple',
				fontBold: true,
				fontItalic: true,
				fontUnderline: true,
				textAlign: 'Right'
			}
		);

		expect(cleared.textColor).toBe(getNodeAppearance(node({ text_color: 'Purple' })).textColor);
		expect(cleared.fontWeight).toBe('700');
		expect(cleared.fontStyle).toBe('italic');
		expect(cleared.textDecoration).toBe('underline');
		expect(cleared.textAlign).toBe('right');
	});

	it('uses built-in text appearance only when node and canvas are both unset', () => {
		const builtIn = getNodeAppearance(node());
		const gray = getNodeAppearance(node({ text_color: 'Gray' }));

		expect(builtIn.textColor).toBe(gray.textColor);
		expect(builtIn.fontWeight).toBe('400');
		expect(builtIn.fontStyle).toBe('normal');
		expect(builtIn.textDecoration).toBe('none');
		expect(builtIn.textAlign).toBe('left');
	});
});
