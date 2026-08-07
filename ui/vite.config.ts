import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vitest/config';
import pkg from './package.json';

export default defineConfig({
	test: {
		include: ['src/tests/**/*.test.ts'],
		// Date-formatting tests assert against fixed UTC timestamps; pin the runner's
		// timezone so results don't depend on the machine running them.
		env: { TZ: 'UTC' }
	},
	plugins: [
		sveltekit(),
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide'
		})
	],
	define: {
		__APP_VERSION__: JSON.stringify(pkg.version)
	},
	server: {
		host: '0.0.0.0',
		allowedHosts: ['scanopy-dev.local'],
		port: 5173,
		proxy: {
			'/api': {
				target: 'http://localhost:60072',
				changeOrigin: true
			}
		}
	},

	build: {
		outDir: 'build'
	}
});
