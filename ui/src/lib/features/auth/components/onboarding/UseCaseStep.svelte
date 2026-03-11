<script lang="ts">
	import { tick } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { Home, Building2, Users } from 'lucide-svelte';
	import { type UseCase, getUseCases } from '../../types/base';
	import { useConfigQuery, isCloud, isCommunity } from '$lib/shared/stores/config-query';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { required } from '$lib/shared/components/forms/validators';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import {
		auth_scanopyLogo,
		common_continue,
		common_other,
		common_reddit,
		common_youtube,
		onboarding_alreadyHaveAccount,
		onboarding_commercialNoticeBody,
		onboarding_commercialNoticeTitle,
		onboarding_howDidYouHear,
		onboarding_howWillYouUse,
		onboarding_logInHere,
		onboarding_referralSource_blogArticle,
		onboarding_referralSource_hackerNews,
		onboarding_referralSource_otherPlaceholder,
		onboarding_referralSource_searchEngine,
		onboarding_referralSource_selfHosted,
		onboarding_referralSource_socialMedia,
		onboarding_referralSource_preferNotToSay,
		onboarding_referralSource_wordOfMouth,
		onboarding_tailorSetup,
		onboarding_understandContinue
	} from '$lib/paraglide/messages';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';

	let {
		isOpen,
		onNext,
		onClose,
		onSwitchToLogin = null
	}: {
		isOpen: boolean;
		onNext: () => void;
		onClose: () => void;
		onSwitchToLogin?: (() => void) | null;
	} = $props();

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let selectedUseCase = $state<UseCase | null>(null);
	let showLicenseWarning = $state(false);
	let scrollContainerEl = $state<HTMLElement | undefined>(undefined);

	// Referral source options
	const referralSourceOptions = [
		{ value: '', label: onboarding_howDidYouHear(), disabled: true },
		{ value: 'search_engine', label: onboarding_referralSource_searchEngine() },
		{ value: 'youtube', label: common_youtube() },
		{ value: 'blog_article', label: onboarding_referralSource_blogArticle() },
		{ value: 'reddit', label: common_reddit() },
		{ value: 'hacker_news', label: onboarding_referralSource_hackerNews() },
		{ value: 'social_media', label: onboarding_referralSource_socialMedia() },
		{ value: 'word_of_mouth', label: onboarding_referralSource_wordOfMouth() },
		{ value: 'self_hosted', label: onboarding_referralSource_selfHosted() },
		{ value: 'other', label: common_other() },
		{ value: 'prefer_not_to_say', label: onboarding_referralSource_preferNotToSay() }
	];

	// Icons for each use case (kept separate from types for flexibility)
	const useCaseIcons: Record<UseCase, typeof Home> = {
		homelab: Home,
		company: Building2,
		msp: Users
	};

	// Use case IDs for iteration
	const useCaseIds: UseCase[] = ['homelab', 'company', 'msp'];

	// Show Cloud-only qualification fields
	let showCloudFields = $derived(configData && isCloud(configData));

	// Form for qualification fields
	const form = createForm(() => ({
		defaultValues: {
			referralSource: '',
			referralSourceOther: ''
		}
	}));

	// Track referral source value reactively for the "other" text field
	// (form.state.values is NOT tracked by $derived)
	let referralSourceValue = $state('');
	$effect(() => {
		return form.store.subscribe(() => {
			referralSourceValue = form.state.values.referralSource;
		});
	});

	function selectUseCase(useCase: UseCase) {
		selectedUseCase = useCase;

		// Reset fields when switching to homelab
		if (useCase === 'homelab') {
			form.reset({ referralSource: '', referralSourceOther: '' });
		}

		// For Community self-hosted + Company/MSP: show license warning
		if (configData && isCommunity(configData) && (useCase === 'company' || useCase === 'msp')) {
			showLicenseWarning = true;
		} else {
			showLicenseWarning = false;
		}

		// Auto-scroll to bottom of modal after DOM updates
		tick().then(() => {
			if (scrollContainerEl) {
				scrollContainerEl.scrollTo({ top: scrollContainerEl.scrollHeight, behavior: 'smooth' });
			}
		});
	}

	function handleLicenseAcknowledge() {
		showLicenseWarning = false;
	}

	function saveFields() {
		const values = form.state.values;
		if (showCloudFields) {
			onboardingStore.setReferralSource(
				values.referralSource || null,
				values.referralSourceOther || null
			);
		}
	}

	function submitAndProceed() {
		if (!selectedUseCase) return;
		saveFields();
		const values = form.state.values;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase,
			referral_source: values.referralSource || undefined,
			referral_source_other: values.referralSourceOther || undefined
		});
		onboardingStore.setUseCase(selectedUseCase);
		onNext();
	}

	async function handleContinue() {
		if (!selectedUseCase) return;
		if (showCloudFields) {
			const isValid = await validateForm(form);
			if (!isValid) return;
		}
		submitAndProceed();
	}

	let canProceed = $derived(selectedUseCase !== null && !showLicenseWarning);
</script>

<GenericModal
	{isOpen}
	title={onboarding_howWillYouUse()}
	{onClose}
	size="lg"
	centerTitle={true}
	showBackdrop={false}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-y-auto p-6" bind:this={scrollContainerEl}>
			<div class="space-y-6">
				<p class="text-secondary text-center text-sm">{onboarding_tailorSetup()}</p>

				<!-- Use Case Cards -->
				<div class="grid gap-3">
					{#each useCaseIds as useCaseId (useCaseId)}
						{@const useCaseConfig = getUseCases()[useCaseId]}
						{@const isSelected = selectedUseCase === useCaseId}
						{@const Icon = useCaseIcons[useCaseId]}
						<button
							type="button"
							class="card flex items-center gap-4 p-4 text-left transition-all {isSelected
								? `ring-2 ${useCaseConfig.colors.ring}`
								: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
							onclick={() => selectUseCase(useCaseId)}
						>
							<div
								class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {isSelected
									? `${useCaseConfig.colors.bg} ${useCaseConfig.colors.text}`
									: 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'}"
							>
								<Icon class="h-5 w-5" />
							</div>
							<div>
								<div class="text-primary font-medium">{useCaseConfig.label}</div>
								<div class="text-secondary text-sm">{useCaseConfig.description}</div>
							</div>
						</button>
					{/each}
				</div>

				<!-- Cloud-only qualification fields -->
				{#if showCloudFields}
					<!-- Referral Source (all use cases on Cloud) -->
					{#if selectedUseCase}
						<div class="card card-static">
							<form.Field name="referralSource">
								{#snippet children(field)}
									<SelectInput
										label={onboarding_howDidYouHear()}
										id="referral-source"
										{field}
										options={referralSourceOptions}
										required={false}
									/>
									{#if referralSourceValue === 'other'}
										<div class="mt-3">
											<form.Field
												name="referralSourceOther"
												validators={{
													onBlur: ({ value }) =>
														referralSourceValue === 'other' ? required(value) : undefined
												}}
											>
												{#snippet children(otherField)}
													<TextInput
														label=""
														id="referral-source-other"
														field={otherField}
														required={true}
														placeholder={onboarding_referralSource_otherPlaceholder()}
													/>
												{/snippet}
											</form.Field>
										</div>
									{/if}
								{/snippet}
							</form.Field>
						</div>
					{/if}
				{/if}

				<!-- License Warning (Community + Company/MSP) -->
				{#if showLicenseWarning}
					<InlineWarning
						title={onboarding_commercialNoticeTitle()}
						body={onboarding_commercialNoticeBody()}
					/>
					<button type="button" class="btn-primary mt-4" onclick={handleLicenseAcknowledge}>
						{onboarding_understandContinue()}
					</button>
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				<button
					type="button"
					class="btn-primary w-full"
					disabled={!canProceed}
					onclick={handleContinue}
				>
					{common_continue()}
				</button>

				{#if onSwitchToLogin}
					<p class="text-secondary text-center text-sm">
						{onboarding_alreadyHaveAccount()}
						<button
							type="button"
							onclick={onSwitchToLogin}
							class="font-medium text-blue-400 hover:text-blue-300"
						>
							{onboarding_logInHere()}
						</button>
					</p>
				{/if}
			</div>
		</div>
	</div>
</GenericModal>
