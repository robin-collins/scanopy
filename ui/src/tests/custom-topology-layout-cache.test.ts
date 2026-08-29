import { describe, expect, it } from 'vitest';
import { applyLayoutEntityChanges } from '$lib/features/custom-topology-views/layout-cache';

describe('custom topology layout cache', () => {
	it('replaces parent and relative coordinates as one immutable entity change', () => {
		const current = [
			{ id: 'child', parent_node_id: 'old-group', x: 35, y: 25 },
			{ id: 'untouched', parent_node_id: null, x: 400, y: 300 }
		];
		const changed = [{ id: 'child', parent_node_id: 'new-group', x: 10, y: 15 }];

		const result = applyLayoutEntityChanges(current, changed);

		expect(result).toEqual([changed[0], current[1]]);
		expect(current[0]).toEqual({ id: 'child', parent_node_id: 'old-group', x: 35, y: 25 });
	});

	it('appends server-created entities without disturbing cached z-order', () => {
		const current = [{ id: 'group' }, { id: 'child' }];
		const created = [{ id: 'new-node' }];

		expect(applyLayoutEntityChanges(current, created)).toEqual([...current, created[0]]);
	});
});
