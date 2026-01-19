# Hone Website

The marketing and documentation site for Hone, deployed at [hone.safia.dev](https://hone.safia.dev).

## Development

```sh
# Install dependencies
npm install

# Start dev server
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview
```

## Structure

```
src/
├── components/     # Reusable UI components
├── layouts/        # Page layouts
├── pages/
│   ├── index.astro         # Landing page
│   ├── examples.astro      # Examples showcase
│   └── docs/               # Documentation pages
└── styles/         # Global styles
```

## Deployment

The site is deployed to Cloudflare Pages. Connect the repository and set:

- **Build command**: `npm run build`
- **Build output directory**: `dist`
- **Root directory**: `site/site`
