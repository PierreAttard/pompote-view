# sv

Everything you need to build a Svelte project, powered by [`sv`](https://github.com/sveltejs/cli).

## Creating a project

If you're seeing this, you've probably already done this step. Congrats!

```sh
# create a new project
npx sv create my-app
```

To recreate this project with the same configuration:

```sh
# recreate this project
npx sv@0.15.3 create --template minimal --types ts --no-install frontend
```

## Developing

Once you've created a project and installed dependencies with `npm install` (or `pnpm install` or `yarn`), start a development server:

```sh
npm run dev

# or start the server and open the app in a new browser tab
npm run dev -- --open
```

## Building

To create a production version of your app:

```sh
npm run build
```

You can preview the production build with `npm run preview`.

> To deploy your app, you may need to install an [adapter](https://svelte.dev/docs/kit/adapters) for your target environment.

## Client API typé (généré depuis l'OpenAPI)

Les types TypeScript de l'API sont **générés** depuis la spec OpenAPI du backend
(`openapi-typescript`), pas écrits à la main.

- Spec committée : `frontend/openapi.json` (produite par le backend, cf.
  `backend/README.md` → `dump_openapi`).
- Types générés committés : `src/lib/api/types.gen.ts` (ne pas éditer ; exclus
  de prettier/eslint).
- Régénérer les types après mise à jour de `openapi.json` :

  ```sh
  npm run codegen
  ```

La CI échoue si `types.gen.ts` n'est pas à jour vis-à-vis de `openapi.json`
(et le job backend échoue si `openapi.json` ne correspond plus aux annotations
`utoipa`). Le client `fetch` typé qui consomme ces types arrive avec l'issue
#16 (Lot 3).

## Configuration (variables d'environnement serveur)

Les pages backtest (`/backtests`) appellent le backend **côté serveur**
(`+page.server.ts`) pour que la clé API ne soit **jamais** exposée au
navigateur. Deux variables sont lues via `$env/dynamic/private` :

| Variable      | Défaut                  | Description                                      |
| ------------- | ----------------------- | ------------------------------------------------ |
| `BACKEND_URL` | `http://127.0.0.1:3100` | URL du backend viz.                              |
| `VIZ_API_KEY` | —                       | Clé envoyée dans `X-API-Key` (= clé du backend). |

> ⚠️ **Privées** : ces variables sont serveur-only (jamais `PUBLIC_*`). Aucune
> `.env.example` n'est commité (même raison que côté backend) ; en local, place
> ces valeurs dans `frontend/.env` (gitignoré). Sans `VIZ_API_KEY`, les pages
> backtest renvoient une erreur backend (401/500).

## Client HTTP & data fetching

La **clé API reste strictement côté serveur**. Le navigateur n'appelle donc
jamais le backend Rust directement :

- **Pages avec `load`** (ex. backtest) → `load` serveur (`+page.server.ts`)
  appelle `$lib/server/backend.ts` (clé injectée via `backendConfig()`).
- **Appels navigateur** (live, polling) → routes SvelteKit same-origin
  `src/routes/api/*/+server.ts` qui **proxient** vers le backend avec la clé
  serveur ; le client navigateur `$lib/api/client.ts` (`apiGet`) tape ces
  routes `/api/*` (aucun header secret), gère les erreurs (`ApiError`,
  `isUnauthorized` pour le 401) et parse le JSON typé.
- **Cache / dedup / polling 10s** : `@tanstack/svelte-query` via le
  `QueryClientProvider` monté à la racine (`src/routes/+layout.svelte`).
