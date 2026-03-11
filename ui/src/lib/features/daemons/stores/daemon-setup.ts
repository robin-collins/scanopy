import { writable } from 'svelte/store';

export type DaemonConnectionStatus = 'idle' | 'waiting' | 'connected' | 'trouble';

export const daemonSetupState = writable<{
	connectionStatus: DaemonConnectionStatus;
}>({ connectionStatus: 'idle' });
