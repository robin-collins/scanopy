<script lang="ts">
	import { untrack } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import { pushError } from '$lib/shared/stores/feedback';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { entities } from '$lib/shared/stores/metadata';
	import { Settings, Terminal, Loader2, ArrowRight, ArrowLeft } from 'lucide-svelte';
	import confetti from 'canvas-confetti';
	import {
		createEmptyApiKeyFormData,
		useCreateApiKeyMutation
	} from '$lib/features/daemon_api_keys/queries';
	import { useProvisionDaemonMutation } from '../../queries';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { getVisibleFieldIds } from '../../config';
	import {
		buildDefaultValues,
		buildRunCommand,
		buildDockerCompose,
		constructDaemonUrl,
		detectOS,
		slugifyNetworkName,
		type DaemonOS
	} from '../../utils';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { daemonSetupState, type DaemonConnectionStatus } from '../../stores/daemon-setup';
	import ConfigureStep from './steps/ConfigureStep.svelte';
	import InstallStep from './steps/InstallStep.svelte';
	import AdvancedStep from './steps/AdvancedStep.svelte';
	import {
		common_back,
		common_close,
		common_configure,
		common_failedGenerateApiKey,
		common_install,
		common_next,
		daemons_createDaemon,
		daemons_enterApiKey
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onNavigate?: (tab: string) => void;
		name?: string;
	}

	let { isOpen = false, onClose, onNavigate, name = undefined }: Props = $props();

	// Queries & mutations
	const configQuery = useConfigQuery();
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();
	const createApiKeyMutation = useCreateApiKeyMutation();
	const provisionDaemonMutation = useProvisionDaemonMutation();

	// Derived data
	let serverUrl = $derived(configQuery.data?.public_url ?? '');
	let currentUserId = $derived(currentUserQuery.data?.id ?? null);
	let org = $derived(organizationQuery.data);
	let hasDaemonPoll = $derived.by(() => {
		if (!org?.plan?.type) return true;
		return billingPlans.getMetadata(org.plan.type).features.daemon_poll;
	});
	let isFirstDaemon = $derived(!org?.onboarding?.includes('FirstDaemonRegistered'));
	// Snapshot: tracks whether wizard was opened as first-daemon flow.
	// Prevents reactivity from flipping showWaitingUI when FirstDaemonRegistered appears.
	let startedAsFirstDaemon = $state(false);
	let hasEmailSupport = $derived.by(() => {
		if (!org?.plan?.type) return false;
		return billingPlans.getMetadata(org.plan.type).features.email_support;
	});

	// Networks
	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// Network selection
	let selectedNetworkId = $state('');
	let nameManuallyEdited = $state(false);

	// API key state
	let keyState = $state<string | null>(null);
	let key = $derived(keyState);
	let keySet = $derived(!!key);

	// Auto-generation state (for first daemon flow)
	let isAutoGenerating = $state(false);

	// OS selection
	let selectedOS: DaemonOS = $state(detectOS());
	let linuxMethod = $state<'binary' | 'docker'>('binary');
	let isDockerInstall = $derived(selectedOS === 'linux' && linuxMethod === 'docker');
	let installCtaLabel = $derived(
		isDockerInstall ? "I've started the Docker container" : "I've run the install command"
	);

	// Connection waiting state
	let connectionStatus = $state<DaemonConnectionStatus>('idle');
	let troubleTimeoutId = $state<ReturnType<typeof setTimeout> | null>(null);
	let showTroubleshootingPanel = $state(false);

	function getDefaultDaemonName(networkId: string): string {
		const network = networksData.find((n) => n.id === networkId);
		if (network) {
			const slug = slugifyNetworkName(network.name);
			if (slug) return `scanopy-daemon-${slug}`;
		}
		return 'scanopy-daemon';
	}

	$effect(() => {
		if (selectedNetworkId && !nameManuallyEdited) {
			const defaultName = getDefaultDaemonName(selectedNetworkId);
			untrack(() => form.setFieldValue('name', defaultName));
		}
	});

	// TanStack Form
	const form = createForm(() => ({
		defaultValues: buildDefaultValues(),
		onSubmit: async () => {
			// No-op; submission is handled by step navigation
		}
	}));

	// Reactive form values (form.state.values is NOT tracked by $derived)
	let formValues = $state<Record<string, string | number | boolean>>(buildDefaultValues());

	$effect(() => {
		return form.store.subscribe(() => {
			formValues = { ...form.state.values } as Record<string, string | number | boolean>;
		});
	});

	// Derived commands
	let runCommand = $derived(
		buildRunCommand(serverUrl, selectedNetworkId, key, formValues, null, currentUserId, selectedOS)
	);
	let dockerCompose = $derived(
		key ? buildDockerCompose(serverUrl, selectedNetworkId, key, formValues, currentUserId) : ''
	);

	// Check for form validation errors (only visible fields)
	let visibleFields = $derived(getVisibleFieldIds(formValues));
	let hasErrors = $derived.by(() => {
		const fieldMeta = form.state.fieldMeta;
		for (const fieldKey of Object.keys(fieldMeta)) {
			if (!visibleFields.has(fieldKey)) continue;
			const meta = fieldMeta[fieldKey];
			if (meta?.errors && meta.errors.length > 0) {
				return true;
			}
		}
		return false;
	});

	// --- Tab / wizard state ---
	const mainFlow = ['configure', 'install'] as const;

	let activeTab = $state('configure');
	let furthestReached = $state(0);
	let showAdvanced = $state(false);

	let tabs: ModalTab[] = $derived([
		{ id: 'configure', label: common_configure(), icon: Settings },
		{
			id: 'install',
			label: common_install(),
			icon: Terminal,
			disabled: furthestReached < 1
		}
	]);

	function nextTab() {
		const idx = (mainFlow as readonly string[]).indexOf(activeTab);
		if (idx >= 0 && idx < mainFlow.length - 1) {
			activeTab = mainFlow[idx + 1];
		}
	}

	function previousTab() {
		const idx = (mainFlow as readonly string[]).indexOf(activeTab);
		if (idx > 0) {
			activeTab = mainFlow[idx - 1];
		}
	}

	function handleTabChange(tabId: string) {
		showAdvanced = false;
		activeTab = tabId;
	}

	// --- Key generation ---
	async function handleCreateNewApiKey() {
		const fields = getVisibleFieldIds(formValues);
		const isValid = await validateForm(form, fields);
		if (!isValid) return;

		const daemonName = (form.state.values['name'] as string) ?? 'daemon';
		const mode = (form.state.values['mode'] as string) ?? 'daemon_poll';
		const daemonUrlBase = (form.state.values['daemonUrl'] as string) ?? '';
		const daemonPort = (() => {
			const port = form.state.values['daemonPort'];
			return typeof port === 'number' ? port : 60073;
		})();

		if (mode === 'server_poll') {
			const fullDaemonUrl = constructDaemonUrl(daemonUrlBase, daemonPort);
			try {
				const result = await provisionDaemonMutation.mutateAsync({
					name: daemonName,
					network_id: selectedNetworkId,
					url: fullDaemonUrl
				});
				keyState = result.daemon_api_key;
			} catch {
				pushError(common_failedGenerateApiKey());
			}
		} else {
			let newApiKey = createEmptyApiKeyFormData(selectedNetworkId);
			newApiKey.name = `${daemonName} Api Key`;
			try {
				const result = await createApiKeyMutation.mutateAsync(newApiKey);
				keyState = result.keyString;
			} catch {
				pushError(common_failedGenerateApiKey());
			}
		}
	}

	async function handleUseExistingKey() {
		const fields = getVisibleFieldIds(formValues);
		const isValid = await validateForm(form, fields);
		if (!isValid) return;

		const trimmedKey = ((form.state.values['existingKeyInput'] as string) ?? '').trim();
		if (!trimmedKey) {
			pushError(daemons_enterApiKey());
			return;
		}
		keyState = trimmedKey;
	}

	// --- Navigation handlers ---
	async function handleNext() {
		if (activeTab === 'configure') {
			const fields = getVisibleFieldIds(formValues);
			const isValid = await validateForm(form, fields);

			if (!isValid) return;

			trackEvent('daemon_wizard_step_completed', { step: 'configure' });

			// Auto-generate key for: first daemon (any mode), or server_poll, or daemon_poll with generate source
			const mode = formValues.mode as string;
			const keySource = formValues.keySource as string;
			const needsAutoGenerate =
				!key && (isFirstDaemon || mode === 'server_poll' || keySource === 'generate');

			if (needsAutoGenerate) {
				isAutoGenerating = true;
				try {
					await handleCreateNewApiKey();
				} finally {
					isAutoGenerating = false;
				}
				if (!keyState) return; // generation failed
			}

			// For daemon_poll with existing key source, key must be set already
			if (!isFirstDaemon && mode === 'daemon_poll' && keySource === 'existing' && !key) {
				pushError(daemons_enterApiKey());
				return;
			}

			if (furthestReached < 1) furthestReached = 1;
			nextTab();
		}
	}

	// --- Connection waiting ---
	function handleInstalled() {
		if (!startedAsFirstDaemon) return;
		connectionStatus = 'waiting';
		daemonSetupState.set({ connectionStatus: 'waiting' });
		trackEvent('daemon_install_confirmed');

		// Set 2-minute timeout for trouble state
		troubleTimeoutId = setTimeout(() => {
			if (connectionStatus === 'waiting') {
				connectionStatus = 'trouble';
				daemonSetupState.set({ connectionStatus: 'trouble' });
				trackEvent('daemon_connection_timeout');
			}
		}, 120_000);
	}

	function handleReviewCommands() {
		connectionStatus = 'idle';
		// Don't update store — polling continues via org query
	}

	function handleViewDiscovery() {
		onNavigate?.('discovery-sessions');
		handleOnClose();
	}

	function handleTrouble() {
		showTroubleshootingPanel = true;
		trackEvent('daemon_install_trouble');
	}

	// Poll org query for FirstDaemonRegistered when waiting
	$effect(() => {
		if (connectionStatus === 'waiting' || connectionStatus === 'trouble') {
			const interval = setInterval(() => {
				organizationQuery.refetch();
			}, 5000);
			return () => clearInterval(interval);
		}
	});

	// Detect connection via org onboarding
	$effect(() => {
		if (
			(connectionStatus === 'waiting' || connectionStatus === 'trouble') &&
			org?.onboarding?.includes('FirstDaemonRegistered')
		) {
			connectionStatus = 'connected';
			daemonSetupState.set({ connectionStatus: 'connected' });
			if (troubleTimeoutId) {
				clearTimeout(troubleTimeoutId);
				troubleTimeoutId = null;
			}
			confetti({ particleCount: 100, spread: 70, origin: { y: 0.6 } });
			trackEvent('daemon_connected');
		}
	});

	// --- Close / Open ---
	function handleOnClose() {
		trackEvent('daemon_wizard_closed');

		// Keep store active if still waiting (so checklist can show "Having trouble")
		if (connectionStatus !== 'waiting' && connectionStatus !== 'trouble') {
			daemonSetupState.set({ connectionStatus: 'idle' });
		}

		if (troubleTimeoutId) {
			clearTimeout(troubleTimeoutId);
			troubleTimeoutId = null;
		}

		keyState = null;
		isAutoGenerating = false;
		nameManuallyEdited = false;
		activeTab = 'configure';
		furthestReached = 0;
		showAdvanced = false;
		connectionStatus = 'idle';
		showTroubleshootingPanel = false;
		onClose();
	}

	function handleOpen() {
		trackEvent('daemon_wizard_opened');
		nameManuallyEdited = false;
		activeTab = 'configure';
		furthestReached = 0;
		showAdvanced = false;
		connectionStatus = 'idle';
		startedAsFirstDaemon = isFirstDaemon;
		showTroubleshootingPanel = false;
	}

	let colorHelper = entities.getColorHelper('Daemon');
	let title = daemons_createDaemon();
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	size="full"
	fixedHeight={true}
	onClose={handleOnClose}
	onOpen={handleOpen}
	{tabs}
	{activeTab}
	tabStyle="stepper"
	onTabChange={handleTabChange}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Daemon')} color={colorHelper.color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			{#if showAdvanced}
				<AdvancedStep {form} {formValues} />
			{:else if activeTab === 'configure'}
				<ConfigureStep
					{form}
					{formValues}
					{selectedNetworkId}
					onNetworkChange={(id) => (selectedNetworkId = id)}
					onNameInput={() => (nameManuallyEdited = true)}
					{hasDaemonPoll}
					{keySet}
					{isFirstDaemon}
					onUseExistingKey={handleUseExistingKey}
				/>
			{:else if activeTab === 'install'}
				<InstallStep
					{selectedOS}
					onOsSelect={(os) => (selectedOS = os)}
					{linuxMethod}
					onLinuxMethodChange={(method) => (linuxMethod = method)}
					{runCommand}
					{dockerCompose}
					{hasErrors}
					isFirstDaemon={startedAsFirstDaemon}
					{connectionStatus}
					onReviewCommands={handleReviewCommands}
					onViewDiscovery={handleViewDiscovery}
					{hasEmailSupport}
					{showTroubleshootingPanel}
					onAdvanced={() => (showAdvanced = true)}
				/>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				{#if showAdvanced}
					<button type="button" class="btn-primary" onclick={() => (showAdvanced = false)}>
						<ArrowLeft class="h-4 w-4" />
						Back to install
					</button>
				{:else if activeTab === 'configure'}
					<button
						type="button"
						class="btn-primary btn-primary-lg"
						onclick={handleNext}
						disabled={isAutoGenerating}
					>
						{#if isAutoGenerating}
							<Loader2 class="h-4 w-4 animate-spin" />
						{:else}
							{common_next()}
							<ArrowRight class="h-4 w-4" />
						{/if}
					</button>
				{:else if activeTab === 'install'}
					{#if connectionStatus === 'connected'}
						<button type="button" class="btn-primary" onclick={handleOnClose}>
							{common_close()}
						</button>
					{:else if connectionStatus === 'waiting' || connectionStatus === 'trouble'}
						<button type="button" class="btn-secondary" onclick={handleOnClose}>
							{common_close()}
						</button>
					{:else if startedAsFirstDaemon}
						<button type="button" class="btn-secondary" onclick={handleTrouble}>
							I'm having trouble
						</button>
						<button type="button" class="btn-primary" onclick={handleInstalled}>
							{installCtaLabel}
						</button>
					{:else}
						<button type="button" class="btn-secondary" onclick={previousTab}>
							{common_back()}
						</button>
						<button type="button" class="btn-secondary" onclick={handleOnClose}>
							{common_close()}
						</button>
					{/if}
				{/if}
			</div>
		</div>
	</div>
</GenericModal>
