<script lang="ts">
	import ChecklistItem from '$lib/shared/components/data/ChecklistItem.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import SupportOptions from '$lib/features/support/SupportOptions.svelte';
	import InlineSuccess from '$lib/shared/components/feedback/InlineSuccess.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import { Loader2 } from 'lucide-svelte';
	import type { DaemonOS } from '../../../utils';
	import {
		daemons_troubleshoot_isDaemonRunning,
		daemons_troubleshoot_isDaemonRunningDesc,
		daemons_troubleshoot_isListening,
		daemons_troubleshoot_isListeningDesc,
		daemons_troubleshoot_processNotFound,
		daemons_troubleshoot_canReachServer,
		daemons_troubleshoot_canReachServerDesc,
		daemons_troubleshoot_canReachServerFail,
		daemons_troubleshoot_canReachServerStep1,
		daemons_troubleshoot_canReachServerStep3,
		daemons_troubleshoot_canServerReach,
		daemons_troubleshoot_canServerReachDesc,
		daemons_troubleshoot_firewallNat,
		daemons_troubleshoot_firewallNatDesc,
		daemons_troubleshoot_checkLogs,
		daemons_troubleshoot_checkLogsDesc,
		daemons_troubleshoot_logFileAt,
		daemons_troubleshoot_logFileCustom,
		daemons_troubleshoot_logFileOrJournal,
		daemons_troubleshoot_logFileDocker,
		daemons_troubleshoot_logFileMounted,
		daemons_troubleshoot_commonErrors,
		daemons_troubleshoot_errorConnectionRefused,
		daemons_troubleshoot_errorConnectionTimeout,
		daemons_troubleshoot_errorApiKeyInactive,
		daemons_troubleshoot_errorApiKeyInvalid,
		daemons_troubleshoot_errorCertificate,
		daemons_troubleshoot_stillStuck,
		daemons_troubleshoot_stillStuckDesc,
		daemons_troubleshoot_healthReachable,
		daemons_troubleshoot_healthReachableDesc,
		daemons_troubleshoot_healthPartial,
		daemons_troubleshoot_healthPartialDesc,
		daemons_troubleshoot_healthUnreachable,
		daemons_troubleshoot_healthUnreachableDesc,
		daemons_troubleshoot_testReachability,
		daemons_troubleshoot_reviewInstallCommand,
		daemons_troubleshoot_targetingServer,
		daemons_troubleshoot_wrongServerUrl,
		daemons_troubleshoot_enableSelfSigned
	} from '$lib/paraglide/messages';

	type LinuxMethod = 'binary' | 'docker';

	interface Props {
		mode: 'daemon_poll' | 'server_poll';
		serverUrl: string;
		daemonUrl?: string;
		daemonName?: string;
		selectedOS: DaemonOS;
		linuxMethod?: LinuxMethod;
		hasEmailSupport?: boolean;
		logFilePath?: string;
		onHealthCheck?: () => void;
		isCheckingHealth?: boolean;
		healthResult?: { reachable: boolean; health?: boolean; error?: string } | null;
		onReviewCommands?: () => void;
		onEnableSelfSigned?: () => void;
	}

	let {
		mode,
		serverUrl,
		daemonUrl = '',
		daemonName = 'scanopy-daemon',
		selectedOS,
		linuxMethod = 'binary',
		hasEmailSupport = false,
		logFilePath = '',
		onHealthCheck,
		isCheckingHealth = false,
		healthResult = null,
		onReviewCommands,
		onEnableSelfSigned
	}: Props = $props();

	let isServerPoll = $derived(mode === 'server_poll');
	let isDocker = $derived(selectedOS === 'linux' && linuxMethod === 'docker');

	// Checklist state (local, not persisted)
	let checked = $state<Record<string, boolean>>({});
	function toggle(id: string) {
		checked[id] = !checked[id];
	}

	// OS-specific process check commands
	let processCheckCommand = $derived.by(() => {
		if (isDocker) return 'docker ps | grep scanopy';
		switch (selectedOS) {
			case 'linux':
				return 'systemctl status scanopy-daemon';
			case 'macos':
				return 'ps aux | grep scanopy-daemon';
			case 'windows':
				return 'Get-Process scanopy-daemon*';
			default:
				return 'systemctl status scanopy-daemon';
		}
	});

	// Port check command for ServerPoll
	let portCheckCommand = $derived.by(() => {
		if (isDocker) return 'docker ps | grep scanopy';
		switch (selectedOS) {
			case 'linux':
				return 'ss -tlnp | grep 60073';
			case 'macos':
				return 'lsof -i :60073';
			case 'windows':
				return 'netstat -an | findstr 60073';
			default:
				return 'ss -tlnp | grep 60073';
		}
	});

	// Effective log file path — custom if set, otherwise OS default
	let hasCustomLogPath = $derived(!!logFilePath && logFilePath !== 'none');
	let effectiveLogPath = $derived.by(() => {
		if (hasCustomLogPath) {
			// If custom path looks like a directory, append daemon filename
			if (logFilePath.endsWith('/') || logFilePath.endsWith('\\') || !logFilePath.includes('.')) {
				const name = daemonName || 'scanopy-daemon';
				const sep = logFilePath.endsWith('/') || logFilePath.endsWith('\\') ? '' : '/';
				return `${logFilePath}${sep}${name}.log`;
			}
			return logFilePath;
		}
		const name = daemonName || 'scanopy-daemon';
		switch (selectedOS) {
			case 'linux':
				return `/var/log/scanopy/${name}.log`;
			case 'macos':
				return `~/Library/Logs/scanopy/${name}.log`;
			case 'windows':
				return `%ProgramData%\\scanopy\\${name}.log`;
			default:
				return `/var/log/scanopy/${name}.log`;
		}
	});

	// Docker-specific: mounted volume path on host
	let dockerHostLogPath = $derived.by(() => {
		const name = daemonName || 'scanopy-daemon';
		return `/var/log/scanopy/${name}.log`;
	});

	// OS-specific log commands
	let logCommand = $derived.by(() => {
		if (isDocker) return `docker logs ${daemonName || 'scanopy-daemon'} --tail 50`;
		switch (selectedOS) {
			case 'linux':
			case 'macos':
				return `tail -50 ${effectiveLogPath}`;
			case 'windows':
				return `Get-Content "${effectiveLogPath}" -Tail 50`;
			default:
				return `tail -50 ${effectiveLogPath}`;
		}
	});

	let journalCommand = $derived(
		selectedOS === 'linux' && !isDocker ? 'journalctl -u scanopy-daemon -n 50 --no-pager' : ''
	);

	let processCheckLanguage = $derived(selectedOS === 'windows' ? 'powershell' : 'bash');
	let logLanguage = $derived(selectedOS === 'windows' ? 'powershell' : 'bash');

	// Health check command for DaemonPoll
	let healthCheckCommand = $derived(`curl -s ${serverUrl}/api/health`);

	// Extract hostname and port info from server URL
	let serverHostname = $derived.by(() => {
		try {
			return new URL(serverUrl).hostname;
		} catch {
			return serverUrl;
		}
	});

	let serverPortDesc = $derived.by(() => {
		try {
			const url = new URL(serverUrl);
			if (url.port) return `port ${url.port}`;
			return url.protocol === 'https:' ? 'port 443 (HTTPS)' : 'port 80 (HTTP)';
		} catch {
			return 'port 443 (HTTPS)';
		}
	});
</script>

<div class="space-y-1">
	{#if isServerPoll}
		<!-- ServerPoll troubleshooting steps -->
		<ChecklistItem
			card
			label={daemons_troubleshoot_isListening()}
			description={daemons_troubleshoot_isListeningDesc()}
			checked={!!checked['listening']}
			onToggle={() => toggle('listening')}
		>
			{#snippet detail()}
				<CodeContainer
					language={processCheckLanguage}
					expandable={false}
					code={processCheckCommand}
				/>
				{#if !isDocker}
					<CodeContainer
						language={processCheckLanguage}
						expandable={false}
						code={portCheckCommand}
					/>
				{/if}
				<p class="text-tertiary text-xs">{daemons_troubleshoot_processNotFound()}</p>
			{/snippet}
		</ChecklistItem>

		<ChecklistItem
			card
			label={daemons_troubleshoot_canServerReach()}
			description={daemons_troubleshoot_canServerReachDesc()}
			checked={!!checked['server-reach']}
			onToggle={() => toggle('server-reach')}
		>
			{#snippet detail()}
				{#if daemonUrl && onHealthCheck}
					{#if healthResult}
						{#if healthResult.reachable && healthResult.health}
							<InlineSuccess
								title={daemons_troubleshoot_healthReachable()}
								body={daemons_troubleshoot_healthReachableDesc()}
							/>
						{:else if healthResult.reachable}
							<InlineWarning
								title={daemons_troubleshoot_healthPartial()}
								body={daemons_troubleshoot_healthPartialDesc()}
							/>
						{:else}
							<InlineDanger
								title={healthResult.error ?? daemons_troubleshoot_healthUnreachable()}
								body={daemons_troubleshoot_healthUnreachableDesc()}
							/>
						{/if}
					{/if}
					<button
						type="button"
						class="btn-primary text-sm"
						disabled={isCheckingHealth}
						onclick={onHealthCheck}
					>
						{#if isCheckingHealth}
							<Loader2 class="h-4 w-4 animate-spin" />
						{/if}
						{daemons_troubleshoot_testReachability()}
					</button>
				{/if}
			{/snippet}
		</ChecklistItem>

		<ChecklistItem
			card
			label={daemons_troubleshoot_firewallNat()}
			description={daemons_troubleshoot_firewallNatDesc()}
			checked={!!checked['firewall']}
			onToggle={() => toggle('firewall')}
		/>
	{:else}
		<!-- DaemonPoll troubleshooting steps -->
		<ChecklistItem
			card
			label={daemons_troubleshoot_isDaemonRunning()}
			description={daemons_troubleshoot_isDaemonRunningDesc()}
			checked={!!checked['running']}
			onToggle={() => toggle('running')}
		>
			{#snippet detail()}
				<CodeContainer
					language={processCheckLanguage}
					expandable={false}
					code={processCheckCommand}
				/>
				<p class="text-tertiary text-xs">{daemons_troubleshoot_processNotFound()}</p>
				{#if onReviewCommands}
					<button type="button" class="btn-secondary text-xs" onclick={onReviewCommands}>
						{daemons_troubleshoot_reviewInstallCommand()}
					</button>
				{/if}
			{/snippet}
		</ChecklistItem>

		<ChecklistItem
			card
			label={daemons_troubleshoot_canReachServer()}
			description={daemons_troubleshoot_canReachServerDesc()}
			checked={!!checked['reach-server']}
			onToggle={() => toggle('reach-server')}
		>
			{#snippet detail()}
				<p class="text-secondary text-xs">{daemons_troubleshoot_targetingServer()}</p>
				<code class="text-primary block rounded bg-gray-100 px-2 py-1 text-xs dark:bg-gray-800"
					>{serverUrl}</code
				>
				<p class="text-tertiary mt-1 text-xs">{daemons_troubleshoot_wrongServerUrl()}</p>
				{#if onReviewCommands}
					<button type="button" class="btn-secondary text-xs" onclick={onReviewCommands}>
						{daemons_troubleshoot_reviewInstallCommand()}
					</button>
				{/if}
				<CodeContainer language="bash" expandable={false} code={healthCheckCommand} />
				<p class="text-tertiary mt-1 text-xs">{daemons_troubleshoot_canReachServerFail()}</p>
				<div class="space-y-1">
					<p class="text-tertiary text-xs">{daemons_troubleshoot_canReachServerStep1()}</p>
					<CodeContainer language="bash" expandable={false} code={`nslookup ${serverHostname}`} />
					<p class="text-tertiary text-xs">
						{daemons_troubleshoot_canReachServerStep3({ portDesc: serverPortDesc })}
					</p>
				</div>
			{/snippet}
		</ChecklistItem>
	{/if}

	<!-- Shared: Check logs (both modes) -->
	<ChecklistItem
		card
		label={daemons_troubleshoot_checkLogs()}
		description={daemons_troubleshoot_checkLogsDesc()}
		checked={!!checked['logs']}
		onToggle={() => toggle('logs')}
	>
		{#snippet detail()}
			{#if isDocker}
				<p class="text-secondary text-xs font-medium">{daemons_troubleshoot_logFileDocker()}</p>
				<CodeContainer language="bash" expandable={false} code={logCommand} />
				<p class="text-secondary mt-2 text-xs font-medium">
					{daemons_troubleshoot_logFileMounted()}
				</p>
				<CodeContainer language="bash" expandable={false} code={`tail -50 ${dockerHostLogPath}`} />
			{:else}
				{#if hasCustomLogPath}
					<p class="text-secondary text-xs font-medium">
						{daemons_troubleshoot_logFileCustom({ path: effectiveLogPath })}
					</p>
				{:else}
					<p class="text-secondary text-xs font-medium">
						{daemons_troubleshoot_logFileAt({ path: effectiveLogPath })}
					</p>
				{/if}
				<CodeContainer language={logLanguage} expandable={false} code={logCommand} />
				{#if journalCommand}
					<p class="text-secondary mt-2 text-xs font-medium">
						{daemons_troubleshoot_logFileOrJournal()}
					</p>
					<CodeContainer language="bash" expandable={false} code={journalCommand} />
				{/if}
			{/if}

			<div class="mt-2">
				<p class="text-secondary text-xs font-medium">{daemons_troubleshoot_commonErrors()}</p>
				<ul class="text-tertiary mt-1 space-y-1 text-xs">
					<li>{daemons_troubleshoot_errorConnectionRefused()}</li>
					<li>{daemons_troubleshoot_errorConnectionTimeout()}</li>
					<li>{daemons_troubleshoot_errorApiKeyInactive()}</li>
					<li>{daemons_troubleshoot_errorApiKeyInvalid()}</li>
					<li>
						{daemons_troubleshoot_errorCertificate()}
						{#if onEnableSelfSigned}
							<button
								type="button"
								class="ml-1 text-xs text-blue-400 underline hover:text-blue-300"
								onclick={onEnableSelfSigned}
							>
								{daemons_troubleshoot_enableSelfSigned()}
							</button>
						{/if}
					</li>
				</ul>
			</div>
		{/snippet}
	</ChecklistItem>

	<!-- Shared: Still stuck? (both modes) -->
	<ChecklistItem
		card
		label={daemons_troubleshoot_stillStuck()}
		description={daemons_troubleshoot_stillStuckDesc()}
		checked={!!checked['stuck']}
		onToggle={() => toggle('stuck')}
	>
		{#snippet detail()}
			<SupportOptions isTroubleshooting={true} {hasEmailSupport} />
		{/snippet}
	</ChecklistItem>
</div>
