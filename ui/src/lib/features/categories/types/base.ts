import type { components } from '$lib/api/schema';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Category = components['schemas']['Category'];

export function createDefaultCategory(organization_id: string): Category {
	return {
		name: '',
		description: null,
		color: 'Blue',
		icon: 'tag',
		skip_full_port_scan: false,
		preferred_ports: null,
		organization_id,
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel
	};
}
