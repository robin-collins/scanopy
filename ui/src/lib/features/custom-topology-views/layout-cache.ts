/**
 * Apply a layout response/request to one cached entity list without mutating
 * either input. Each entity is replaced as a whole, so parent and relative
 * coordinates can never appear in the cache as separate partial changes.
 */
export function applyLayoutEntityChanges<T extends { id: string }>(
	current: T[] | undefined,
	changes: T[]
): T[] | undefined {
	if (!current) return current;
	const byId = new Map(changes.map((entity) => [entity.id, entity]));
	const merged = current.map((entity) => byId.get(entity.id) ?? entity);
	const existingIds = new Set(current.map((entity) => entity.id));
	for (const entity of changes) {
		if (!existingIds.has(entity.id)) merged.push(entity);
	}
	return merged;
}
