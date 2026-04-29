// Electrobun loads the SvelteKit build through views://, so the app is built
// as a static SPA with an index.html fallback.
// See: https://svelte.dev/docs/kit/single-page-apps
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    paths: {
      relative: true,
    },
    adapter: adapter({
      pages: "web-build",
      assets: "web-build",
      fallback: "index.html",
    }),
  },
};

export default config;
