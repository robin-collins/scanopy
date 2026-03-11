<script lang="ts">
	import type { Snippet } from 'svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import type { DaemonOS } from '../utils';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		common_binary,
		common_docker,
		common_linux,
		common_macos,
		common_windows,
		daemons_operatingSystem,
		daemons_requestOsSupport,
		onboarding_freebsdNotSupported,
		onboarding_notSupportedTitle,
		onboarding_openbsdNotSupported
	} from '$lib/paraglide/messages';

	type LinuxMethod = 'binary' | 'docker';

	interface Props {
		selectedOS: DaemonOS;
		onOsSelect: (os: DaemonOS) => void;
		linuxMethod?: LinuxMethod;
		onLinuxMethodChange?: (method: LinuxMethod) => void;
		afterLabel?: Snippet;
		afterButtons?: Snippet;
		children?: Snippet;
	}

	let {
		selectedOS,
		onOsSelect,
		linuxMethod = 'binary',
		onLinuxMethodChange,
		afterLabel,
		afterButtons,
		children
	}: Props = $props();

	let osOptions = $derived([
		{ id: 'linux' as DaemonOS, label: common_linux() },
		{ id: 'macos' as DaemonOS, label: common_macos() },
		{ id: 'windows' as DaemonOS, label: common_windows() },
		{ id: 'freebsd' as DaemonOS, label: 'FreeBSD' },
		{ id: 'openbsd' as DaemonOS, label: 'OpenBSD' }
	]);

	let linuxMethodOptions = $derived([
		{ id: 'binary' as LinuxMethod, label: common_binary() },
		{ id: 'docker' as LinuxMethod, label: common_docker() }
	]);

	const configQuery = useConfigQuery();
	let hasPosthog = $derived(!!configQuery.data?.posthog_key);
	let requestedOs = $state(new Set<string>());

	function handleRequestOsSupport(os: string) {
		trackEvent('daemon_os_support_requested', { os });
		requestedOs = new Set([...requestedOs, os]);
	}
</script>

<!-- OS Selector -->
<div role="group" aria-label={daemons_operatingSystem()}>
	<div class="flex items-baseline justify-between">
		<span class="text-secondary text-sm font-medium">{daemons_operatingSystem()}</span>
		{@render afterLabel?.()}
	</div>
	<div class="mt-2 flex items-center justify-between gap-2">
		<div class="flex items-center gap-2">
			{#each osOptions as option (option.id)}
				<button
					type="button"
					class="btn-secondary {selectedOS === option.id ? 'ring-primary ring-2' : ''}"
					onclick={() => onOsSelect(option.id)}
				>
					{option.label}
				</button>
			{/each}
		</div>
		{@render afterButtons?.()}
	</div>
</div>

{#if selectedOS === 'linux'}
	<!-- Linux: Install method sub-toggle, sized to match the Linux OS button -->
	<div class="flex gap-1" style="width: calc((100% - 4 * 0.5rem) / 5)">
		{#each linuxMethodOptions as option (option.id)}
			<button
				type="button"
				class="btn-secondary btn-sm flex-1 {linuxMethod === option.id ? 'ring-primary ring-2' : ''}"
				onclick={() => onLinuxMethodChange?.(option.id)}
			>
				{option.label}
			</button>
		{/each}
	</div>
{/if}

{#if selectedOS === 'freebsd'}
	<InlineWarning title={onboarding_notSupportedTitle()} body={onboarding_freebsdNotSupported()} />
	{#if hasPosthog}
		<button
			type="button"
			class="btn-secondary btn-sm"
			disabled={requestedOs.has('freebsd')}
			onclick={() => handleRequestOsSupport('freebsd')}
		>
			{requestedOs.has('freebsd') ? '✓' : ''}
			{daemons_requestOsSupport({ os: 'FreeBSD' })}
		</button>
	{/if}
{:else if selectedOS === 'openbsd'}
	<InlineWarning title={onboarding_notSupportedTitle()} body={onboarding_openbsdNotSupported()} />
	{#if hasPosthog}
		<button
			type="button"
			class="btn-secondary btn-sm"
			disabled={requestedOs.has('openbsd')}
			onclick={() => handleRequestOsSupport('openbsd')}
		>
			{requestedOs.has('openbsd') ? '✓' : ''}
			{daemons_requestOsSupport({ os: 'OpenBSD' })}
		</button>
	{/if}
{:else}
	{@render children?.()}
{/if}
