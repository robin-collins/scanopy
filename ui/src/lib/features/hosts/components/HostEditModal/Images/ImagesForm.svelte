<script lang="ts">
	import { Upload, Trash2, Star } from 'lucide-svelte';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import {
		useHostImagesQuery,
		useUploadHostImageMutation,
		useDeleteHostImageMutation,
		hostImageContentUrl
	} from '$lib/features/host-images/queries';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import {
		common_images,
		common_upload,
		common_uploading,
		common_delete,
		hosts_images_subtitle,
		hosts_images_noImages,
		hosts_images_emptySubtitle,
		hosts_images_setAsIcon,
		hosts_images_unsetIcon,
		hosts_images_isIcon,
		hosts_images_confirmDelete
	} from '$lib/paraglide/messages';

	interface Props {
		formData: HostFormData;
	}

	let { formData = $bindable() }: Props = $props();

	let fileInput: HTMLInputElement | undefined = $state();
	const imagesQuery = useHostImagesQuery(() => formData.id);
	let images = $derived(imagesQuery.data ?? []);
	const uploadMutation = useUploadHostImageMutation();
	const deleteMutation = useDeleteHostImageMutation();

	function triggerUpload() {
		fileInput?.click();
	}

	async function handleFileSelected(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		await uploadMutation.mutateAsync({ hostId: formData.id, file });
	}

	function handleDelete(imageId: string) {
		if (!confirm(hosts_images_confirmDelete())) return;
		// Clear locally too — the DB's ON DELETE SET NULL only takes effect
		// server-side; formData holds the in-progress edit state until Save.
		if (formData.topology_icon_image_id === imageId) {
			formData.topology_icon_image_id = null;
		}
		deleteMutation.mutate({ imageId, hostId: formData.id });
	}

	function toggleTopologyIcon(imageId: string) {
		formData.topology_icon_image_id = formData.topology_icon_image_id === imageId ? null : imageId;
	}
</script>

<div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-6">
	<div class="flex items-start justify-between gap-4">
		<div>
			<h3 class="text-primary text-sm font-medium">{common_images()}</h3>
			<p class="text-tertiary mt-1 text-xs">{hosts_images_subtitle()}</p>
		</div>
		<button
			type="button"
			class="btn-secondary flex shrink-0 items-center gap-2"
			disabled={uploadMutation.isPending}
			onclick={triggerUpload}
		>
			<Upload class="h-4 w-4" />
			{uploadMutation.isPending ? common_uploading() : common_upload()}
		</button>
		<input
			bind:this={fileInput}
			type="file"
			accept="image/png,image/jpeg,image/gif,image/webp"
			class="hidden"
			onchange={handleFileSelected}
		/>
	</div>

	{#if images.length === 0}
		<EntityConfigEmpty title={hosts_images_noImages()} subtitle={hosts_images_emptySubtitle()} />
	{:else}
		<div class="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4">
			{#each images as image (image.id)}
				{@const isIcon = formData.topology_icon_image_id === image.id}
				<div class="card flex flex-col gap-2 p-3">
					<img
						src={hostImageContentUrl(image.id)}
						alt={image.filename}
						class="aspect-square w-full rounded-lg object-cover"
					/>
					<p class="text-secondary truncate text-xs" title={image.filename}>
						{image.filename}
					</p>
					<div class="flex items-center justify-between gap-2">
						<button
							type="button"
							class="flex items-center gap-1 text-xs {isIcon
								? 'text-amber-400'
								: 'text-tertiary hover:text-secondary'}"
							title={isIcon ? hosts_images_unsetIcon() : hosts_images_setAsIcon()}
							onclick={() => toggleTopologyIcon(image.id)}
						>
							<Star class="h-4 w-4" fill={isIcon ? 'currentColor' : 'none'} />
							{#if isIcon}{hosts_images_isIcon()}{/if}
						</button>
						<button
							type="button"
							class="text-tertiary hover:text-red-400"
							title={common_delete()}
							disabled={deleteMutation.isPending}
							onclick={() => handleDelete(image.id)}
						>
							<Trash2 class="h-4 w-4" />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
