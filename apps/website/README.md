# ferrocodex.com

Static marketing site for Ferrocodex, served as static assets from a Cloudflare Worker.

There is no build step. Everything under `public/` is deployed as-is.

## Layout

- `public/index.html`, `public/styles.css`, `public/site.js`: the single page.
- `public/site.js` asks the GitHub API for the latest release and rewrites the
  download links. Without JavaScript, or if the request fails, the page keeps
  the links to the release version fixed in the HTML. Update those when the
  app version changes.
- `public/_headers`: security headers and caching, applied by Cloudflare.
- `public/.well-known/security.txt`: vulnerability contact. The `Expires`
  date must be renewed yearly.
- `public/assets/fonts/`: Archivo (SIL Open Font License), self-hosted so the
  site makes no third-party requests.
- `wrangler.jsonc`: Worker name, assets directory, and the custom domains.

## Commands

Run from the repository root so the workspace install is used.

```bash
npm run dev -w apps/website            # local preview on http://localhost:8787
npm run deploy:dry-run -w apps/website # validate config and packaging
npm run deploy -w apps/website         # publish to Cloudflare
```

## First deployment

1. Add `ferrocodex.com` as a zone on the Cloudflare account and point the
   registrar's nameservers at it.
2. Log in: `npx wrangler login` (or set `CLOUDFLARE_API_TOKEN` and
   `CLOUDFLARE_ACCOUNT_ID` for CI).
3. `npm run deploy -w apps/website`. Wrangler creates the Worker and attaches
   the custom domains listed in `wrangler.jsonc`, creating DNS records as needed.

To preview before the domain is ready, remove or comment out the `routes`
block and deploy; the Worker answers on its `workers.dev` subdomain.
