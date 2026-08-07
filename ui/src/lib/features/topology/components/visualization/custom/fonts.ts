/**
 * Self-hosted curated Google Fonts subset for the custom topology canvas.
 * Font files live in ui/static/fonts/<slug>/{400,700}.woff2 (fetched via
 * scripts/fetch-curated-fonts.mjs); this module is the runtime catalog,
 * on-demand @font-face loader, and safe CSS fallback stack.
 */
import fontCatalogData from './font-catalog.json';

export interface FontCatalogEntry {
	/** The font id stored on nodes/canvases (matches the Google Fonts family name). */
	id: string;
	slug: string;
	category: 'sans-serif' | 'serif' | 'monospace' | 'display';
	files: Partial<Record<'400' | '700', string>>;
}

export const FONT_CATALOG: FontCatalogEntry[] = fontCatalogData as FontCatalogEntry[];

const FALLBACK_STACKS: Record<FontCatalogEntry['category'], string> = {
	'sans-serif': 'ui-sans-serif, system-ui, sans-serif',
	serif: 'Georgia, Cambria, "Times New Roman", serif',
	monospace: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
	display: 'ui-sans-serif, system-ui, sans-serif'
};

/** Safe fallback used when no font is selected, or a selected font is unknown/fails to load. */
export const SAFE_FALLBACK_FONT_STACK = FALLBACK_STACKS['sans-serif'];

export function getFontCatalogEntry(fontId: string | null | undefined): FontCatalogEntry | null {
	if (!fontId) return null;
	return FONT_CATALOG.find((f) => f.id === fontId) ?? null;
}

/** CSS font-family value for a font id: the curated font quoted first, then its category's safe fallback stack. */
export function getFontCssStack(fontId: string | null | undefined): string {
	const entry = getFontCatalogEntry(fontId);
	if (!entry) return SAFE_FALLBACK_FONT_STACK;
	return `"${entry.id}", ${FALLBACK_STACKS[entry.category]}`;
}

export function searchFonts(query: string): FontCatalogEntry[] {
	const term = query.trim().toLowerCase();
	if (!term) return FONT_CATALOG;
	return FONT_CATALOG.filter((f) => f.id.toLowerCase().includes(term));
}

const loadedFontIds = new Set<string>();

/**
 * Injects @font-face rules for the given font id, once. Only fonts actually
 * placed on a canvas are loaded — the full curated catalog is never
 * eagerly fetched.
 */
export function loadFont(fontId: string | null | undefined): void {
	if (!fontId || typeof document === 'undefined') return;
	if (loadedFontIds.has(fontId)) return;
	const entry = getFontCatalogEntry(fontId);
	if (!entry) return;

	loadedFontIds.add(fontId);
	const style = document.createElement('style');
	style.setAttribute('data-canvas-font', entry.slug);
	style.textContent = Object.entries(entry.files)
		.map(
			([weight, url]) => `
@font-face {
	font-family: "${entry.id}";
	font-weight: ${weight};
	font-style: normal;
	font-display: swap;
	src: url("${url}") format("woff2");
}`
		)
		.join('\n');
	document.head.appendChild(style);
}

/** Load every distinct font family currently referenced on a canvas (nodes + canvas default). */
export function loadFontsInUse(fontIds: Array<string | null | undefined>): void {
	const distinct = new Set(fontIds.filter((id): id is string => !!id));
	for (const id of distinct) loadFont(id);
}
