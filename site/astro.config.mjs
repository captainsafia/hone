// @ts-check
import { defineConfig } from 'astro/config';

import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  site: 'https://hone.safia.dev',
  vite: {
    plugins: [tailwindcss()]
  },
  redirects: {
    '/docs': '/docs/installation'
  }
});