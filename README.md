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
# → écoute sur http://0.0.0.0:3000 ; tester /healthz
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
