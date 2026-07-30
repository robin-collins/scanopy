export function missingDaemonFeature(
	required: string[] | undefined,
	available: string[] | null
): string | undefined {
	if (available === null) return undefined;
	return (required ?? []).find((feature) => !available.includes(feature));
}
