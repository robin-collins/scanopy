<script lang="ts" module>
	export const CredentialDisplay: EntityDisplayComponent<Credential, object> = {
		getId: (credential) => credential.id,
		getLabel: (credential) => credential.name,
		getDescription: (credential) => getCredentialDescription(credential),
		getIcon: () => entities.getIconComponent('Credential'),
		getIconColor: () => entities.getColorHelper('Credential').icon,
		getTags: (credential) => {
			const typeId = credential.credential_type.type;
			return [
				{
					label: credentialTypes.getName(typeId),
					color: credentialTypes.getColorHelper(typeId).color
				}
			];
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import { type Credential, getCredentialDescription } from '$lib/features/credentials/types/base';
	import { entities, credentialTypes } from '$lib/shared/stores/metadata';

	interface Props {
		item: Credential;
		context?: object;
	}

	let { item, context = {} }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={CredentialDisplay} />
