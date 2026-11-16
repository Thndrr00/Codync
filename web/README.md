# Codync website

Next.js static export. `npm run dev` serves it at http://localhost:3000.

## Web app

`/app` is Codync's web client. It talks to the host through `proxy/host-proxy.mjs`, a loopback
bridge that serves the export and adds the host token (the browser never holds it):

```sh
npm run build
node proxy/host-proxy.mjs          # http://localhost:4321
```

For live reload, run `npm run dev` and `node proxy/host-proxy.mjs --next http://localhost:3000`.
The client helpers have `node --test` coverage: `npm test`.

## Deploy on Vercel

With the Vercel project's Root Directory set to `web/`, `vercel.json` limits
automatic Git deployments to `main` and `dev`. Other branches, including normal
external PR branches, do not create previews. GitHub Actions tests still run.

This filters branch names, not repository owners. Keep Git Fork Protection
enabled: fork branches named `main` or `dev` still require authorization.
Existing PR checks are not removed; PR branches need to include this
configuration for subsequent events to use it.

See [Vercel Git configuration](https://vercel.com/docs/project-configuration/git-configuration).
