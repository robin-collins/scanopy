/**
 * Downloads a curated, self-hosted subset of Google Fonts (Regular + Bold,
 * Latin subset, woff2) into ui/static/fonts/<slug>/ and writes the catalog
 * manifest consumed by the custom-canvas font picker
 * (ui/src/lib/features/topology/components/visualization/custom/fonts.ts).
 *
 * Re-run manually when the curated list changes — this is a one-off asset
 * fetch, not part of the normal build (same pattern as the service-logo
 * downloader in `make generate-fixtures`).
 */
import { writeFile, mkdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FONTS_DIR = path.join(__dirname, '..', 'static', 'fonts');
const MANIFEST_PATH = path.join(
	__dirname,
	'..',
	'src',
	'lib',
	'features',
	'topology',
	'components',
	'visualization',
	'custom',
	'font-catalog.json'
);

// Modern browser UA so Google's CSS2 API returns woff2 URLs.
const UA =
	'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36';

const CURATED_FONTS = [
	{ family: 'Inter', category: 'sans-serif' },
	{ family: 'Roboto', category: 'sans-serif' },
	{ family: 'Open Sans', category: 'sans-serif' },
	{ family: 'Lato', category: 'sans-serif' },
	{ family: 'Montserrat', category: 'sans-serif' },
	{ family: 'Poppins', category: 'sans-serif' },
	{ family: 'Nunito', category: 'sans-serif' },
	{ family: 'Work Sans', category: 'sans-serif' },
	{ family: 'Merriweather', category: 'serif' },
	{ family: 'Playfair Display', category: 'serif' },
	{ family: 'Lora', category: 'serif' },
	{ family: 'PT Serif', category: 'serif' },
	{ family: 'Roboto Mono', category: 'monospace' },
	{ family: 'JetBrains Mono', category: 'monospace' },
	{ family: 'Source Code Pro', category: 'monospace' },
	{ family: 'IBM Plex Mono', category: 'monospace' },
	{ family: 'Quicksand', category: 'display' },
	{ family: 'Comfortaa', category: 'display' }
];

const WEIGHTS = [400, 700];

function slugify(family) {
	return family.toLowerCase().replace(/\s+/g, '-');
}

async function fetchFontFaceCss(family, weight) {
	const url = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(family)}:wght@${weight}&display=swap`;
	const res = await fetch(url, { headers: { 'User-Agent': UA } });
	if (!res.ok) throw new Error(`CSS fetch failed for ${family}@${weight}: ${res.status}`);
	return res.text();
}

function extractWoff2Url(css) {
	const match = css.match(/src:\s*url\((https:\/\/[^)]+\.woff2)\)/);
	if (!match) throw new Error(`No woff2 URL found in CSS: ${css.slice(0, 200)}`);
	return match[1];
}

async function downloadFont(family, weight) {
	const css = await fetchFontFaceCss(family, weight);
	const fontUrl = extractWoff2Url(css);
	const res = await fetch(fontUrl);
	if (!res.ok) throw new Error(`Font file fetch failed for ${family}@${weight}: ${res.status}`);
	const buffer = Buffer.from(await res.arrayBuffer());

	const slug = slugify(family);
	const dir = path.join(FONTS_DIR, slug);
	await mkdir(dir, { recursive: true });
	const filename = `${weight}.woff2`;
	await writeFile(path.join(dir, filename), buffer);
	return `/fonts/${slug}/${filename}`;
}

async function main() {
	const manifest = [];
	for (const font of CURATED_FONTS) {
		const slug = slugify(font.family);
		console.log(`Fetching ${font.family}...`);
		const files = {};
		for (const weight of WEIGHTS) {
			try {
				files[weight] = await downloadFont(font.family, weight);
			} catch (e) {
				console.error(`  FAILED ${font.family}@${weight}: ${e.message}`);
			}
		}
		if (Object.keys(files).length === 0) {
			console.error(`  Skipping ${font.family} — no weights downloaded`);
			continue;
		}
		manifest.push({ id: font.family, slug, category: font.category, files });
	}

	await writeFile(MANIFEST_PATH, JSON.stringify(manifest, null, '\t') + '\n');
	console.log(`\nWrote ${manifest.length} fonts to ${MANIFEST_PATH}`);
}

main();
