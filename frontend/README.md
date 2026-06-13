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
