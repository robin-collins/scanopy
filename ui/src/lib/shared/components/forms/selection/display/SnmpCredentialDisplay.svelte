<script lang="ts" module>
	export const SnmpCredentialDisplay: EntityDisplayComponent<SnmpCredential, object> = {
		getId: (credential) => credential.id,
		getLabel: (credential) => credential.name,
		getDescription: () => '',
		getIcon: () => entities.getIconComponent('SnmpCredential'),
		getIconColor: () => entities.getColorHelper('SnmpCredential').icon,
		getTags: (credential) => [
			{ label: credential.version ?? 'V2c', color: entities.getColorHelper('SnmpCredential').color }
		],
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { components } from '$lib/api/schema';
	type SnmpCredential = components['schemas']['SnmpCredential'];
	import { entities } from '$lib/shared/stores/metadata';

	interface Props {
		item: SnmpCredential;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={SnmpCredentialDisplay} />
