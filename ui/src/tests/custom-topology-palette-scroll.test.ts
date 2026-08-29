import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * Scroll isolation for the object palette (confirmed tasklist item 2).
 *
 * These assert the structural contract rather than simulating a fling: the
 * behaviour is produced entirely by CSS containment plus which element owns
 * the scrollport, and jsdom implements neither scrolling nor
 * overscroll-behavior. A rendered test here would pass while the real
 * behaviour was broken, which is worse than no test.
 */
const palette = readFileSync(
	resolve(
		__dirname,
		'../lib/features/topology/components/visualization/custom/CustomViewPalette.svelte'
	),
	'utf-8'
);
const page = readFileSync(resolve(__dirname, '../routes/+page.svelte'), 'utf-8');
const canvas = readFileSync(
	resolve(
		__dirname,
		'../lib/features/topology/components/visualization/custom/CustomViewCanvas.svelte'
	),
	'utf-8'
);

describe('object palette scroll isolation', () => {
	it('does not let the palette root scroll, so the search field cannot scroll away', () => {
		const root = palette.slice(palette.indexOf('<div'), palette.indexOf('</div>'));
		expect(root).toContain('overflow-hidden');
		expect(root).not.toContain('overflow-y-auto');
	});

	it('gives the results region its own scrollport with contained overscroll', () => {
		expect(palette).toContain('overflow-y-auto overscroll-contain');
	});

	it('keeps the results region reachable and scrollable by keyboard', () => {
		expect(palette).toContain('role="region"');
		expect(palette).toContain('tabindex="0"');
		expect(palette).toContain('aria-label=');
	});
});

describe('topology workspace sizing', () => {
	it('no longer pins a 600px minimum on the tab wrapper or the canvas', () => {
		expect(page).not.toContain('min-h-[600px]');
		expect(canvas).not.toContain('min-h-[600px]');
	});

	it('keeps min-h-0 so the flex chain can shrink below content height', () => {
		expect(page).toContain('flex h-full min-h-0 flex-1 flex-col');
		expect(canvas).toContain('min-h-0');
	});

	it('retains the conditional overflow isolation that protects other tabs', () => {
		expect(page).toContain("class:overflow-hidden={activeTab === 'topology'}");
		expect(page).toContain("class:overflow-auto={activeTab !== 'topology'}");
	});
});
