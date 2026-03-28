<script lang="ts">
	import { toPng, toSvg } from 'html-to-image';
	import { useSvelteFlow, type Node } from '@xyflow/svelte';
	import {
		FileImage,
		FileCode,
		FileText,
		FileOutput,
		AppWindow,
		Image,
		Sun,
		Moon
	} from 'lucide-svelte';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { isExporting } from '../interactions';
	import {
		topology_createdUsing,
		topology_export,
		topology_exportComplete,
		topology_exportFailed,
		topology_exportPng,
		topology_exportPngDesc,
		topology_exportSvg,
		topology_exportSvgDesc,
		topology_exportMermaid,
		topology_exportMermaidDesc,
		topology_exportConfluence,
		topology_exportConfluenceDesc,
		topology_exportPdf,
		topology_exportPdfDesc,
		topology_exportHtml,
		topology_exportHtmlDesc,
		topology_exportTheme,
		topology_flowNotFound,
		topology_noNodesToExport
	} from '$lib/paraglide/messages';
	import { getResolvedTheme } from '$lib/shared/stores/theme.svelte';
	import { common_light, common_dark } from '$lib/paraglide/messages';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { downloadTopologyExport } from '$lib/shared/utils/csvExport';
	import UpgradeBadge from '$lib/shared/components/UpgradeBadge.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { upgradeContext } from '$lib/features/billing/stores';
	import type { UpgradeFeature } from '$lib/shared/stores/metadata';

	let {
		topologyId,
		topologyName = '',
		isOpen = $bindable(false),
		isShareView = false
	}: {
		topologyId: string;
		topologyName?: string;
		isOpen: boolean;
		isShareView?: boolean;
	} = $props();

	const { getNodes, getEdges, getViewport, setViewport } = useSvelteFlow();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let hideCreatedWith = $derived.by(() => {
		if (organization && organization.plan && organization.plan.type) {
			return billingPlans.getMetadata(organization.plan.type).features.remove_created_with;
		} else {
			return false;
		}
	});

	let hasSvgExport = $derived(
		organization?.plan ? billingPlans.getMetadata(organization.plan.type).features.svg_export : true
	);

	let hasMermaidExport = $derived(
		organization?.plan
			? billingPlans.getMetadata(organization.plan.type).features.mermaid_export
			: true
	);

	let hasConfluenceExport = $derived(
		organization?.plan
			? billingPlans.getMetadata(organization.plan.type).features.confluence_export
			: true
	);

	let hasPdfExport = $derived(
		organization?.plan ? billingPlans.getMetadata(organization.plan.type).features.pdf_export : true
	);

	let hasHtmlExport = $derived(
		organization?.plan
			? billingPlans.getMetadata(organization.plan.type).features.html_export
			: true
	);

	let exportTheme = $state<'light' | 'dark'>(getResolvedTheme());

	function getAbsolutePosition(node: Node, nodes: Node[]) {
		if (node.parentId) {
			const parent = nodes.find((n) => n.id === node.parentId);
			if (parent) {
				return {
					x: parent.position.x + node.position.x,
					y: parent.position.y + node.position.y
				};
			}
		}
		return { x: node.position.x, y: node.position.y };
	}

	function calculateExportBounds() {
		const nodes = getNodes();
		const edges = getEdges();

		if (nodes.length === 0) {
			pushError(topology_noNodesToExport());
			return null;
		}

		const flowElement = document.querySelector('.svelte-flow') as HTMLElement;
		if (!flowElement) {
			pushError(topology_flowNotFound());
			return null;
		}

		const childNodes = nodes.filter((n) => n.parentId);
		const parentNodes = nodes.filter((n) => !n.parentId);
		const parentIdsWithChildren = new Set(childNodes.map((n) => n.parentId));
		const standaloneNodes = parentNodes.filter((n) => !parentIdsWithChildren.has(n.id));

		let minX = Infinity,
			minY = Infinity,
			maxX = -Infinity,
			maxY = -Infinity;

		childNodes.forEach((child) => {
			const absPos = getAbsolutePosition(child, nodes);
			const width = child.measured?.width || child.width || 150;
			const height = child.measured?.height || child.height || 50;
			minX = Math.min(minX, absPos.x);
			minY = Math.min(minY, absPos.y);
			maxX = Math.max(maxX, absPos.x + width);
			maxY = Math.max(maxY, absPos.y + height);
		});

		standaloneNodes.forEach((node) => {
			const x = node.position.x;
			const y = node.position.y;
			const width = node.measured?.width || node.width || 150;
			const height = node.measured?.height || node.height || 50;
			minX = Math.min(minX, x);
			minY = Math.min(minY, y);
			maxX = Math.max(maxX, x + width);
			maxY = Math.max(maxY, y + height);
		});

		const parentBorderMargin = 20;
		parentNodes
			.filter((n) => parentIdsWithChildren.has(n.id))
			.forEach((parent) => {
				minX = Math.min(minX, parent.position.x - parentBorderMargin);
				minY = Math.min(minY, parent.position.y - parentBorderMargin);
			});

		edges.forEach((edge) => {
			const sourceNode = nodes.find((n) => n.id === edge.source);
			const targetNode = nodes.find((n) => n.id === edge.target);
			if (sourceNode && targetNode) {
				const sourcePos = getAbsolutePosition(sourceNode, nodes);
				const targetPos = getAbsolutePosition(targetNode, nodes);
				const sourceCenterX =
					sourcePos.x + (sourceNode.measured?.width || sourceNode.width || 150) / 2;
				const sourceCenterY =
					sourcePos.y + (sourceNode.measured?.height || sourceNode.height || 50) / 2;
				const targetCenterX =
					targetPos.x + (targetNode.measured?.width || targetNode.width || 150) / 2;
				const targetCenterY =
					targetPos.y + (targetNode.measured?.height || targetNode.height || 50) / 2;
				minX = Math.min(minX, sourceCenterX, targetCenterX);
				minY = Math.min(minY, sourceCenterY, targetCenterY);
				maxX = Math.max(maxX, sourceCenterX, targetCenterX);
				maxY = Math.max(maxY, sourceCenterY, targetCenterY);
			}
		});

		const edgeMargin = 150;
		minX -= edgeMargin;
		minY -= edgeMargin;
		maxX += edgeMargin;
		maxY += edgeMargin;

		const boundsWidth = maxX - minX;
		const boundsHeight = maxY - minY;
		const targetZoom = 0.75;
		const imageWidth = Math.round(boundsWidth * targetZoom);
		const imageHeight = Math.round(boundsHeight * targetZoom);
		const boundsCenterX = minX + boundsWidth / 2;
		const boundsCenterY = minY + boundsHeight / 2;
		const x = imageWidth / 2 - boundsCenterX * targetZoom;
		const y = imageHeight / 2 - boundsCenterY * targetZoom;

		return { flowElement, imageWidth, imageHeight, viewport: { x, y, zoom: targetZoom } };
	}

	interface ExportCaptureContext {
		flowElement: HTMLElement;
		imageWidth: number;
		imageHeight: number;
	}

	async function withExportCapture(
		callback: (ctx: ExportCaptureContext) => Promise<void>
	): Promise<void> {
		isOpen = false;

		// Wait for modal to close so it's not captured in the image
		await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

		const bounds = calculateExportBounds();
		if (!bounds) return;

		const { flowElement, imageWidth, imageHeight, viewport: newViewport } = bounds;
		const originalViewport = getViewport();
		const originalWidth = flowElement.style.width;
		const originalHeight = flowElement.style.height;

		// Temporarily switch theme for export
		const currentTheme = getResolvedTheme();
		const needsThemeSwitch = exportTheme !== currentTheme;
		if (needsThemeSwitch) {
			document.documentElement.classList.toggle('dark', exportTheme === 'dark');
			document.documentElement.style.colorScheme = exportTheme;
		}

		flowElement.style.width = `${imageWidth}px`;
		flowElement.style.height = `${imageHeight}px`;
		setViewport(newViewport, { duration: 0 });
		flowElement.classList.add('hide-for-export');
		isExporting.set(true);

		let watermark = document.createElement('div');
		const watermarkColor =
			exportTheme === 'dark' ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.3)';

		if (!hideCreatedWith) {
			watermark = document.createElement('div');
			watermark.style.cssText = `
				position: absolute;
				bottom: 15px;
				right: 15px;
				display: flex;
				align-items: center;
				gap: 8px;
				color: ${watermarkColor};
				font-size: 14px;
				font-family: system-ui;
				pointer-events: none;
				z-index: 9999;
			`;

			const logo = document.createElement('img');
			logo.src = '/logos/scanopy-logo.png';
			logo.style.cssText = `
				height: 18px;
				width: auto;
			`;

			const text = document.createElement('span');
			text.textContent = topology_createdUsing();

			watermark.appendChild(logo);
			watermark.appendChild(text);
			flowElement.appendChild(watermark);

			await new Promise<void>((resolve) => {
				if (logo.complete) {
					resolve();
				} else {
					logo.onload = () => resolve();
					logo.onerror = () => resolve();
				}
			});
		}

		await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

		try {
			await callback({ flowElement, imageWidth, imageHeight });
		} finally {
			isExporting.set(false);
			watermark.remove();
			flowElement.classList.remove('hide-for-export');
			flowElement.style.width = originalWidth;
			flowElement.style.height = originalHeight;
			if (needsThemeSwitch) {
				document.documentElement.classList.toggle('dark', currentTheme === 'dark');
				document.documentElement.style.colorScheme = currentTheme;
			}
			setTimeout(() => setViewport(originalViewport, { duration: 0 }), 50);
		}
	}

	async function captureImage(format: 'png' | 'svg') {
		await withExportCapture(async ({ flowElement, imageWidth, imageHeight }) => {
			const options = { width: imageWidth, height: imageHeight, pixelRatio: 2 };
			const dataUrl =
				format === 'svg' ? await toSvg(flowElement, options) : await toPng(flowElement, options);

			const link = document.createElement('a');
			const date = new Date().toISOString().split('T')[0];
			link.download = `scanopy-topology-${date}.${format}`;
			link.href = dataUrl;
			link.click();
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format });
		});
	}

	function triggerDownload(blob: Blob, filename: string) {
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.download = filename;
		link.href = url;
		link.click();
		URL.revokeObjectURL(url);
	}

	async function handlePdfExport() {
		await withExportCapture(async ({ flowElement, imageWidth, imageHeight }) => {
			const options = { width: imageWidth, height: imageHeight, pixelRatio: 2 };
			const pngDataUrl = await toPng(flowElement, options);

			// Convert PNG data URL to JPEG via canvas for smaller PDF size
			const img = new window.Image();
			img.src = pngDataUrl;
			await new Promise<void>((resolve) => {
				img.onload = () => resolve();
			});

			const canvas = document.createElement('canvas');
			canvas.width = img.width;
			canvas.height = img.height;
			const ctx = canvas.getContext('2d')!;
			// Fill with background for JPEG (no transparency)
			ctx.fillStyle = exportTheme === 'dark' ? '#1a1a2e' : '#ffffff';
			ctx.fillRect(0, 0, canvas.width, canvas.height);
			ctx.drawImage(img, 0, 0);

			const jpegDataUrl = canvas.toDataURL('image/jpeg', 0.92);
			const jpegBase64 = jpegDataUrl.split(',')[1];
			const jpegBytes = Uint8Array.from(atob(jpegBase64), (c) => c.charCodeAt(0));

			const exportName = topologyName || 'Network Topology';
			const exportDate = new Date().toLocaleString();
			const pdfBytes = buildPdf(jpegBytes, canvas.width, canvas.height, exportName, exportDate);

			const date = new Date().toISOString().split('T')[0];
			const pdfBuffer = new ArrayBuffer(pdfBytes.byteLength);
			new Uint8Array(pdfBuffer).set(pdfBytes);
			triggerDownload(
				new Blob([pdfBuffer], { type: 'application/pdf' }),
				`scanopy-topology-${date}.pdf`
			);
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'pdf' });
		});
	}

	function buildPdf(
		jpegBytes: Uint8Array,
		imgWidth: number,
		imgHeight: number,
		title: string,
		dateStr: string
	): Uint8Array {
		// Minimal PDF with embedded JPEG image, title, and date
		const isDark = exportTheme === 'dark';
		const pageWidth = 842; // A4 landscape points
		const pageHeight = 595;
		const margin = 40;
		const titleHeight = 50; // space for title + date
		const availableWidth = pageWidth - margin * 2;
		const availableHeight = pageHeight - margin * 2 - titleHeight;
		const scale = Math.min(availableWidth / imgWidth, availableHeight / imgHeight);
		const drawWidth = Math.round(imgWidth * scale);
		const drawHeight = Math.round(imgHeight * scale);
		const drawX = margin + (availableWidth - drawWidth) / 2;
		const drawY = pageHeight - margin - titleHeight - drawHeight;

		const encoder = new TextEncoder();
		const parts: Uint8Array[] = [];
		const offsets: number[] = [];
		let pos = 0;

		function write(str: string) {
			const bytes = encoder.encode(str);
			parts.push(bytes);
			pos += bytes.length;
		}

		function writeBinary(bytes: Uint8Array) {
			parts.push(bytes);
			pos += bytes.length;
		}

		function recordOffset() {
			offsets.push(pos);
		}

		write('%PDF-1.4\n%\xFF\xFF\xFF\xFF\n');

		// Object 1: Catalog
		recordOffset();
		write('1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n');

		// Object 2: Pages
		recordOffset();
		write(`2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n`);

		// Object 3: Page
		recordOffset();
		write(
			`3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${pageWidth} ${pageHeight}] /Contents 4 0 R /Resources << /XObject << /Img 5 0 R >> /Font << /F1 6 0 R >> >> >>\nendobj\n`
		);

		// Object 4: Content stream (background, title, date, image)
		const escPdf = (s: string) =>
			s.replace(/\\/g, '\\\\').replace(/\(/g, '\\(').replace(/\)/g, '\\)');
		const escapedTitle = escPdf(title);
		const escapedDate = escPdf(dateStr);
		// Background fill (full page)
		const bgR = isDark ? 0.102 : 1;
		const bgG = isDark ? 0.102 : 1;
		const bgB = isDark ? 0.18 : 1;
		const textR = isDark ? 0.878 : 0.2;
		const textG = isDark ? 0.878 : 0.2;
		const textB = isDark ? 0.878 : 0.2;
		const bgRect = `${bgR} ${bgG} ${bgB} rg 0 0 ${pageWidth} ${pageHeight} re f\n`;
		const titleCmd = `${textR} ${textG} ${textB} rg BT /F1 16 Tf ${margin} ${pageHeight - margin - 16} Td (${escapedTitle}) Tj ET\n`;
		const dateCmd = `0.5 0.5 0.5 rg BT /F1 10 Tf ${margin} ${pageHeight - margin - 34} Td (${escapedDate}) Tj ET\n`;
		const imgCmd = `q ${drawWidth} 0 0 ${drawHeight} ${drawX} ${drawY} cm /Img Do Q\n`;
		const contentStream = bgRect + titleCmd + dateCmd + imgCmd;
		recordOffset();
		write(
			`4 0 obj\n<< /Length ${contentStream.length} >>\nstream\n${contentStream}endstream\nendobj\n`
		);

		// Object 5: Image XObject
		recordOffset();
		write(
			`5 0 obj\n<< /Type /XObject /Subtype /Image /Width ${imgWidth} /Height ${imgHeight} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${jpegBytes.length} >>\nstream\n`
		);
		writeBinary(jpegBytes);
		write('\nendstream\nendobj\n');

		// Object 6: Font
		recordOffset();
		write('6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n');

		// Cross-reference table
		const xrefPos = pos;
		write('xref\n');
		write(`0 ${offsets.length + 1}\n`);
		write('0000000000 65535 f \n');
		for (const offset of offsets) {
			write(`${String(offset).padStart(10, '0')} 00000 n \n`);
		}

		write('trailer\n');
		write(`<< /Size ${offsets.length + 1} /Root 1 0 R >>\n`);
		write('startxref\n');
		write(`${xrefPos}\n`);
		write('%%EOF\n');

		// Combine all parts
		const totalLength = parts.reduce((sum, p) => sum + p.length, 0);
		const result = new Uint8Array(totalLength);
		let offset = 0;
		for (const part of parts) {
			result.set(part, offset);
			offset += part.length;
		}
		return result;
	}

	async function handleHtmlExport() {
		await withExportCapture(async ({ flowElement, imageWidth, imageHeight }) => {
			const options = { width: imageWidth, height: imageHeight, pixelRatio: 2 };
			const pngDataUrl = await toPng(flowElement, options);

			const exportName = topologyName || 'Network Topology';
			const bgColor = exportTheme === 'dark' ? '#1a1a2e' : '#ffffff';
			const textColor = exportTheme === 'dark' ? '#e0e0e0' : '#333333';
			const watermarkHtml = !hideCreatedWith
				? `<p style="margin-top:20px;color:${exportTheme === 'dark' ? 'rgba(255,255,255,0.5)' : 'rgba(0,0,0,0.3)'};font-size:13px;">${topology_createdUsing()}</p>`
				: '';

			const escapedTitle = exportName
				.replace(/&/g, '&amp;')
				.replace(/</g, '&lt;')
				.replace(/>/g, '&gt;');

			const styleTag = 'style';
			const css = `body{margin:0;padding:40px;background:${bgColor};color:${textColor};font-family:system-ui,-apple-system,sans-serif;text-align:center;}h1{font-size:24px;font-weight:600;margin:0 0 24px;}img{max-width:100%;height:auto;border-radius:8px;}`;
			const html = [
				'<!DOCTYPE html>',
				'<html lang="en">',
				'<head>',
				'<meta charset="UTF-8">',
				'<meta name="viewport" content="width=device-width, initial-scale=1.0">',
				`<title>${escapedTitle}</title>`,
				`<${styleTag}>${css}</${styleTag}>`,
				'</head>',
				'<body>',
				`<h1>${escapedTitle}</h1>`,
				`<img src="${pngDataUrl}" alt="${escapedTitle}" />`,
				watermarkHtml,
				'</body>',
				'</html>'
			].join('\n');

			const date = new Date().toISOString().split('T')[0];
			triggerDownload(new Blob([html], { type: 'text/html' }), `scanopy-topology-${date}.html`);
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'html' });
		});
	}

	async function handleMermaidExport() {
		isOpen = false;
		try {
			await downloadTopologyExport(topologyId, 'mermaid');
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'mermaid' });
		} catch {
			pushError(topology_exportFailed());
		}
	}

	async function handleConfluenceExport() {
		isOpen = false;
		try {
			await downloadTopologyExport(topologyId, 'confluence');
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'confluence' });
		} catch {
			pushError(topology_exportFailed());
		}
	}

	function handleUpgrade(feature: UpgradeFeature) {
		isOpen = false;
		trackEvent('upgrade_button_clicked', { feature });
		upgradeContext.set({ feature });
		openModal('billing-plan');
	}
</script>

<GenericModal title={topology_export()} {isOpen} onClose={() => (isOpen = false)} size="sm">
	<div class="overflow-y-auto p-6">
		<!-- Export Theme Toggle -->
		<div class="mb-4 flex items-center justify-between">
			<span class="text-secondary text-sm font-medium">{topology_exportTheme()}</span>
			<div class="flex gap-1">
				<button
					class="btn-secondary gap-1 !px-2.5 !py-1 !text-xs {exportTheme === 'light'
						? 'list-item-selected'
						: ''}"
					onclick={() => (exportTheme = 'light')}
				>
					<Sun size={12} />
					{common_light()}
				</button>
				<button
					class="btn-secondary gap-1 !px-2.5 !py-1 !text-xs {exportTheme === 'dark'
						? 'list-item-selected'
						: ''}"
					onclick={() => (exportTheme = 'dark')}
				>
					<Moon size={12} />
					{common_dark()}
				</button>
			</div>
		</div>

		<p class="text-secondary mb-4 text-sm">Choose an export format:</p>

		<div class="space-y-3">
			<button
				class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
				onclick={() => captureImage('png')}
			>
				<Image class="text-tertiary h-6 w-6 shrink-0" />
				<div>
					<div class="text-primary font-medium">{topology_exportPng()}</div>
					<div class="text-tertiary text-sm">{topology_exportPngDesc()}</div>
				</div>
			</button>

			{#if hasSvgExport}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={() => captureImage('svg')}
				>
					<FileImage class="text-tertiary h-6 w-6 shrink-0" />
					<div>
						<div class="text-primary font-medium">{topology_exportSvg()}</div>
						<div class="text-tertiary text-sm">{topology_exportSvgDesc()}</div>
					</div>
				</button>
			{:else if !isShareView}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={() => handleUpgrade('svg_export')}
				>
					<FileImage class="text-muted h-6 w-6 shrink-0" />
					<div class="flex-1">
						<div class="text-tertiary flex items-center gap-2 font-medium">
							{topology_exportSvg()}
							<UpgradeBadge feature="svg_export" />
						</div>
						<div class="text-muted text-sm">{topology_exportSvgDesc()}</div>
					</div>
				</button>
			{/if}

			{#if hasMermaidExport}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={handleMermaidExport}
				>
					<FileCode class="text-tertiary h-6 w-6 shrink-0" />
					<div>
						<div class="text-primary font-medium">{topology_exportMermaid()}</div>
						<div class="text-tertiary text-sm">{topology_exportMermaidDesc()}</div>
					</div>
				</button>
			{:else if !isShareView}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={() => handleUpgrade('mermaid_export')}
				>
					<FileCode class="text-muted h-6 w-6 shrink-0" />
					<div class="flex-1">
						<div class="text-tertiary flex items-center gap-2 font-medium">
							{topology_exportMermaid()}
							<UpgradeBadge feature="mermaid_export" />
						</div>
						<div class="text-muted text-sm">{topology_exportMermaidDesc()}</div>
					</div>
				</button>
			{/if}

			{#if hasConfluenceExport}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={handleConfluenceExport}
				>
					<FileText class="text-tertiary h-6 w-6 shrink-0" />
					<div>
						<div class="text-primary font-medium">{topology_exportConfluence()}</div>
						<div class="text-tertiary text-sm">{topology_exportConfluenceDesc()}</div>
					</div>
				</button>
			{:else if !isShareView}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={() => handleUpgrade('confluence_export')}
				>
					<FileText class="text-muted h-6 w-6 shrink-0" />
					<div class="flex-1">
						<div class="text-tertiary flex items-center gap-2 font-medium">
							{topology_exportConfluence()}
							<UpgradeBadge feature="confluence_export" />
						</div>
						<div class="text-muted text-sm">{topology_exportConfluenceDesc()}</div>
					</div>
				</button>
			{/if}

			{#if hasPdfExport}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={handlePdfExport}
				>
					<FileOutput class="text-tertiary h-6 w-6 shrink-0" />
					<div>
						<div class="text-primary font-medium">{topology_exportPdf()}</div>
						<div class="text-tertiary text-sm">{topology_exportPdfDesc()}</div>
					</div>
				</button>
			{:else if !isShareView}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={() => handleUpgrade('pdf_export')}
				>
					<FileOutput class="text-muted h-6 w-6 shrink-0" />
					<div class="flex-1">
						<div class="text-tertiary flex items-center gap-2 font-medium">
							{topology_exportPdf()}
							<UpgradeBadge feature="pdf_export" />
						</div>
						<div class="text-muted text-sm">{topology_exportPdfDesc()}</div>
					</div>
				</button>
			{/if}

			{#if hasHtmlExport}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={handleHtmlExport}
				>
					<AppWindow class="text-tertiary h-6 w-6 shrink-0" />
					<div>
						<div class="text-primary font-medium">{topology_exportHtml()}</div>
						<div class="text-tertiary text-sm">{topology_exportHtmlDesc()}</div>
					</div>
				</button>
			{:else if !isShareView}
				<button
					class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
					onclick={() => handleUpgrade('html_export')}
				>
					<AppWindow class="text-muted h-6 w-6 shrink-0" />
					<div class="flex-1">
						<div class="text-tertiary flex items-center gap-2 font-medium">
							{topology_exportHtml()}
							<UpgradeBadge feature="html_export" />
						</div>
						<div class="text-muted text-sm">{topology_exportHtmlDesc()}</div>
					</div>
				</button>
			{/if}
		</div>
	</div>
</GenericModal>
