# Backend `pompote-view` — `viz_api`

Backend Rust **read-only** qui expose les bougies, indicateurs et décisions
de trading lus dans la TimescaleDB de `robot_rust`. Architecture hexagonale
stricte : voir [`CLAUDE.md`](../CLAUDE.md).

## Structure du workspace

```
backend/crates/
  domain/        # entités, value objects, règles métier (aucune dep d'I/O)
  application/   # use cases + ports (traits)
  adapters/      # axum (inbound/http), sqlx (outbound/persistence), clock (placeholder)
  viz_api/       # composition root — fn main() + lecture de la config + wiring
```

Le binaire produit reste `viz_api` (`cargo run -p viz_api`). Le crate
`viz_api` joue le rôle de "bootstrap" décrit dans `CLAUDE.md` : il ne
contient aucune logique métier, uniquement le câblage des adapters dans les
use cases de `application`.

## Variables d'environnement

| Variable            | Requise | Défaut           | Description                                                                 |
|---------------------|---------|------------------|-----------------------------------------------------------------------------|
| `DATABASE_URL`      | oui     | —                | URL Postgres du rôle `pompote_viz_reader` (SELECT only).                    |
| `VIZ_API_KEY`       | oui     | —                | Valeur attendue du header `X-API-Key`. Minimum 16 octets (refus au boot).   |
| `VIZ_API_BIND_ADDR` | non     | `0.0.0.0:3100`   | Adresse d'écoute du serveur axum.                                           |
| `RUST_LOG`          | non     | `info`           | Filtre `tracing-subscriber` (ex. `info,sqlx=warn`).                         |

> ⚠️ Aucune valeur par défaut n'est fournie pour `DATABASE_URL` ni pour
> `VIZ_API_KEY` : le serveur refuse de démarrer si elles sont absentes ou
> vides. Aucun fichier `.env.example` n'est commité afin d'éviter qu'une
> valeur d'exemple ne soit prise pour une valeur de prod.

## Lancer localement

```bash
cd backend
DATABASE_URL=postgres://pompote_viz_reader:password@localhost:5432/pompote \
VIZ_API_KEY=dev-key-please-change-0123 \
cargo run -p viz_api
```

Endpoints exposés à ce stade :

- `GET /healthz` — 200, non authentifié, vivacité du process.
- `GET /readyz`  — 200 si `SELECT 1` réussit, 503 sinon. Non authentifié
  (Kubernetes ne propage pas le header `X-API-Key` sur ses probes).
- `/api/v1/monitoring/*` — protégé par le middleware `X-API-Key`. Les routes
  concrètes (candles, decisions, markers…) arrivent dans les issues #8 et
  suivantes ; pour l'instant toute requête sous ce préfixe renvoie 401 si
  le header est absent ou invalide, 404 sinon.

## Commandes Cargo

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --locked
cargo test --workspace --locked
```

Reproduit exactement la matrice CI (`.github/workflows/check.yml`, job
`backend`). Les tests d'intégration backed par Postgres seront ajoutés dans
l'issue #12 ; tous les tests actuels passent sans Postgres ni variable
d'env.

## Garde-fous

- ❌ Pas de migration, pas d'`INSERT` / `UPDATE` / `DELETE` (le rôle
  `pompote_viz_reader` rejette de toute façon ces requêtes côté DB).
- ❌ Pas de dépendance directe entre `domain` et `axum` / `sqlx` / `tokio`.
- ❌ Pas d'interaction exchange (cf. interdiction absolue dans
  [`CLAUDE.md`](../CLAUDE.md)).
- ✅ Le crate `domain` ne dépend que de `serde` + `chrono` + `thiserror`.
