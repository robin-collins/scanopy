<script lang="ts">
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { Braces, X } from 'lucide-svelte';
	import Prism from '@magidoc/plugin-svelte-prismjs';
	import 'prismjs/components/prism-yaml';
	import 'prismjs/components/prism-json';
	import 'prismjs/components/prism-bash';
	import 'prismjs/components/prism-powershell';
	import 'prismjs/themes/prism-twilight.css';
	import { common_copied, common_copy, common_failedToCopy } from '$lib/paraglide/messages';

	export let code: string;
	export let expandable: boolean = true;
	export let expanded: boolean = true;
	export let language: string = 'json';
	export let maxHeight: string = 'max-h-80';
	export let onCopy: (() => void) | undefined = undefined;
	export let hideCopyButton: boolean = false;
	export let preventSelect: boolean = false;

	const isLocalhost =
		window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';

	// Copy JSON to clipboard
	async function copyJson() {
		try {
			await navigator.clipboard.writeText(code);
			if (onCopy) {
				onCopy();
			} else {
				pushSuccess(common_copied());
			}
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	const isSecureContext =
		window.isSecureContext ||
		window.location.hostname === 'localhost' ||
		window.location.hostname === '127.0.0.1';

	function toggleJson() {
		expanded = !expanded;
	}
</script>

<div>
	{#if expandable}
		<div class={`flex items-center justify-between  ${expanded ? 'mb-1' : ''}`}>
			<button
				type="button"
				class="text-tertiary hover:text-secondary flex items-center gap-1 p-1 text-xs transition-colors"
				on:click={toggleJson}
			>
				<Braces class="h-3 w-3" />
				<span>JSON</span>
			</button>
			{#if expanded}
				<button
					type="button"
					class="text-tertiary hover:text-secondary p-1 transition-colors"
					on:click={toggleJson}
					title="Collapse"
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>
	{/if}

	{#if expanded}
		<div translate="no" class="code-wrapper {maxHeight ? maxHeight + ' overflow-y-auto' : ''}">
			<div class="min-w-0 flex-1 {preventSelect && !isLocalhost ? 'prevent-select' : ''}">
				<Prism {language} showCopyButton={false} source={code} showLineNumbers={true} />
			</div>
			{#if isSecureContext && !hideCopyButton}
				<div class="copy-column">
					<button type="button" class="btn-icon" title={common_copy()} on:click={copyJson}>
						{common_copy()}
					</button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	/* Wrapper provides uniform background + border for code and button */
	.code-wrapper {
		display: flex;
		align-items: stretch;
		background: hsl(0, 0%, 8%);
		border: 2px solid #6b7280;
		border-radius: 0.375rem;
		min-width: 0;
		max-width: 100%;
	}

	/* Strip border/background/margin from Prism — the wrapper handles it */
	:global(.prism--code-container) {
		margin: 0 !important;
		border: none !important;
		background: transparent !important;
		max-width: 100% !important;
		overflow-x: auto !important;
		border-radius: 0 !important;
	}

	/* Horizontal scroll for code blocks instead of wrapping */
	:global(.prism--code-container pre),
	:global(.prism--code-container code) {
		white-space: pre !important;
		font-size: 0.75rem;
	}

	:global(.prism--code-container pre) {
		max-width: 100% !important;
		overflow-x: auto !important;
		background: transparent !important;
	}

	@media (min-width: 640px) {
		:global(.prism--code-container pre),
		:global(.prism--code-container code) {
			font-size: 0.875rem;
		}
	}

	.copy-column {
		display: flex;
		align-items: flex-start;
		padding: 0.5rem;
	}

	.prevent-select :global(*) {
		user-select: none !important;
		-webkit-user-select: none !important;
	}
</style>
