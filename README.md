# pompote-view

Interface de **visualisation et monitoring live** des stratégies de trading du projet PomPotRobot.

- **Backend** : Rust 2024 — axum + sqlx + utoipa (architecture hexagonale, read-only sur TimescaleDB)
- **Frontend** : SvelteKit (Svelte 5) — Vite + Vitest + Playwright + TradingView Lightweight Charts
- **Licence** : [Apache 2.0](./LICENSE)

## Liens

- 📋 [Conventions, architecture et garde-fous → `CLAUDE.md`](./CLAUDE.md)
- 🎯 [Epic GitHub — Visualisation & monitoring live](https://github.com/PierreAttard/pompote-view/issues/1)
- 📐 [Document de cadrage complet](https://github.com/PierreAttard/robot_rust/blob/main/docs/epic_visualisation_monitoring.md) (repo `robot_rust`)

## Démarrage rapide

### Backend

```bash
cd backend
cargo build
cargo run -p viz_api
# → écoute sur http://0.0.0.0:3100 ; tester /healthz
```

### Frontend

```bash
cd frontend
npm install
npm run dev
# → http://localhost:5173
```

### Stack complète (Timescale + backend + frontend)

```bash
docker compose up
```

## Garde-fous

`pompote-view` est **strictement read-only** sur la base TimescaleDB. Aucune écriture, aucune migration de schéma (propriété de [`robot_rust`](https://github.com/PierreAttard/robot_rust)), **aucune interaction avec un compte exchange**. Détails dans [`CLAUDE.md`](./CLAUDE.md).

## CI & schéma de la DB

La CI GitHub Actions est composée pour l'instant d'un seul workflow :

- [`/.github/workflows/check.yml`](./.github/workflows/check.yml) — déclenché sur chaque pull request vers `main` et sur chaque push sur `main`. Deux jobs en parallèle :
  - **`backend`** : `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --workspace --locked`, `cargo test --workspace --locked`. Cache géré par [`Swatinem/rust-cache@v2`](https://github.com/Swatinem/rust-cache).
  - **`frontend`** : `npm ci --legacy-peer-deps`, installation Chromium (Vitest browser provider), `npm run lint`, `npm run check`, `npm run build`, `npm run test:unit -- --run`.

> ℹ️ Le flag `--legacy-peer-deps` est nécessaire à cause d'`openapi-typescript` qui n'a pas encore validé sa compatibilité avec TypeScript 6. Cf. PR #38 pour le contexte.

### Récupération du schéma `robot_rust` (option retenue : **A — clone read-only via PAT GitHub**)

Le schéma de la base TimescaleDB est la propriété exclusive de [`robot_rust`](https://github.com/PierreAttard/robot_rust) (repo privé). Aucun fichier `.sql` n'est dupliqué dans `pompote-view`.

Pour les tests d'intégration backend (qui spinnent un Postgres jetable et appliquent le schéma cloné), trois options ont été évaluées dans l'[Issue #6](https://github.com/PierreAttard/pompote-view/issues/6) :

- **Option A — Clone CI de `robot_rust` via PAT GitHub** ✅ retenue.
- **Option B — Artifact `.sql` publié par `robot_rust`** : nécessite un workflow côté `robot_rust` et discipline de release. À reconsidérer si `pompote-view` passe open-source et qu'on veut retirer le PAT.
- **Option C — Submodule git** : pratique en local mais complique la CI et le pin de version.

#### Mécanique de l'option A

1. Un mainteneur du repo crée manuellement un **PAT GitHub fine-grained** ayant la portée minimale `Contents: read` sur `PierreAttard/robot_rust` uniquement.
2. Le PAT est stocké dans le secret de repo **`ROBOT_RUST_READ_PAT`** (`Settings → Secrets and variables → Actions → New repository secret`).
3. Le futur workflow d'intégration (livré par l'[Issue #12](https://github.com/PierreAttard/pompote-view/issues/12), pas par celle-ci) consommera ce secret pour faire un `git clone --depth=1` en lecture seule du repo `robot_rust`, appliquera les migrations sur un Postgres de service, puis lancera les tests d'intégration backend.

> ⚠️ À ce stade, **aucun workflow ne consomme `ROBOT_RUST_READ_PAT`**. Le secret peut être créé en amont mais sa création n'est pas bloquante pour le merge de cette PR. Il deviendra bloquant lorsque l'Issue #12 sera implémentée.

### Hors scope (à venir dans d'autres issues)

- `integration.yml` — Postgres jetable + schéma `robot_rust` (Issue #12).
- `e2e.yml` — Playwright smoke tests (Lot 7 ou issue dédiée).
- Dockerfiles + `docker-compose.yml` (Lot 8 — déploiement local).
