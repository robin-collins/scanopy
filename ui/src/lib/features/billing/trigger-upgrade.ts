import { queryClient, queryKeys } from '$lib/api/query-client';
import type { PublicServerConfig } from '$lib/shared/stores/config-query';
import { trackEvent } from '$lib/shared/utils/analytics';
import { openModal } from '$lib/shared/stores/modal-registry';
import { upgradeContext, reopenSettingsAfterBilling } from '$lib/features/billing/stores';
import type { UpgradeFeature } from '$lib/shared/stores/metadata';

const PRICING_URL = 'https://scanopy.net/pricing';

export interface TriggerUpgradeOptions {
	/** Feature context for recommended plan selection. Null/undefined = generic upgrade. */
	feature?: UpgradeFeature | null;
	/** Source identifier for analytics (e.g., 'sidebar', 'export_modal'). */
	source: string;
	/** If true, reopens settings modal after billing modal closes. */
	reopenSettings?: boolean;
	/** Callback to run before opening the billing modal (e.g., close another modal). */
	beforeModal?: () => void;
}

/**
 * Single entry point for all upgrade actions.
 * Cloud: opens billing modal with feature context.
 * Self-hosted: opens pricing page in a new tab.
 */
export function triggerUpgrade(options: TriggerUpgradeOptions): void {
	const config = queryClient.getQueryData<PublicServerConfig>(queryKeys.config.all);
	const billingEnabled = config?.billing_enabled ?? false;

	trackEvent('upgrade_button_clicked', {
		feature: options.feature ?? options.source,
		source: options.source,
		external: !billingEnabled
	});

	if (!billingEnabled) {
		options.beforeModal?.();
		window.open(PRICING_URL, '_blank', 'noopener,noreferrer');
		return;
	}

	options.beforeModal?.();
	upgradeContext.set(options.feature ? { feature: options.feature } : null);

	if (options.reopenSettings) {
		reopenSettingsAfterBilling.set(true);
	}

	openModal('billing-plan');
}
