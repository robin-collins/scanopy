import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const read = (relativePath: string) =>
	readFileSync(new URL(`../lib/${relativePath}`, import.meta.url), 'utf8');

const tab = read('features/known_ports/components/KnownPortsTab.svelte');
const modal = read('features/known_ports/components/KnownPortModal.svelte');
const queries = read('features/known_ports/queries.ts');
const sidebar = read('shared/components/layout/Sidebar.svelte');

describe('Known Ports catalogue', () => {
	it('is exposed under Platform and displays the catalogue source', () => {
		expect(sidebar).toContain("id: 'known-ports'");
		expect(sidebar).toContain('component: KnownPortsTab');
		expect(tab).toContain("port.source === 'BuiltIn'");
		expect(tab).toContain('common_builtin()');
		expect(tab).toContain('common_custom()');
	});

	it('keeps built-ins read-only independently of caller permissions', () => {
		expect(modal).toContain("let isBuiltin = $derived(port?.source === 'BuiltIn')");
		expect(modal).toContain('let isProtected = $derived(isBuiltin || readOnly)');
		expect(modal).toContain('{#if port && !isProtected}');
	});

	it('uses the custom-only namespace for mutation endpoints', () => {
		expect(queries).toContain("PUT('/api/v1/known-ports/custom/{id}'");
		expect(queries).toContain("DELETE('/api/v1/known-ports/custom/{id}'");
		expect(queries).not.toContain("PUT('/api/v1/known-ports/{id}'");
		expect(queries).not.toContain("DELETE('/api/v1/known-ports/{id}'");
	});
});
