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

## Endpoints

| Méthode | Chemin                              | Auth        | Description                                                            |
|---------|-------------------------------------|-------------|------------------------------------------------------------------------|
| GET     | `/healthz`                          | non         | Liveness — 200 dès que le process est démarré.                         |
| GET     | `/readyz`                           | non         | Readiness — 200 si `SELECT 1` réussit, 503 sinon.                      |
| GET     | `/api/v1/monitoring/candles`        | `X-API-Key` | OHLCV bucketées via `time_bucket()` sur `candles_5s`. Voir ci-dessous. |

Toutes les autres routes sous `/api/v1/monitoring/*` répondent `401` sans
`X-API-Key` valide, et `404` ensuite — les endpoints à venir (décisions,
markers, indicateurs) arriveront dans les issues #9, #10 et #13.

### `GET /api/v1/monitoring/candles`

**Query string** :

| Paramètre   | Type    | Requis | Description                                                                              |
|-------------|---------|--------|------------------------------------------------------------------------------------------|
| `exchange`  | string  | oui    | Identifiant exchange (ex. `binance`, `kraken`).                                          |
| `symbol`    | string  | oui    | Symbole (ex. `BTC/USDC`).                                                                |
| `timeframe` | string  | oui    | `5s`, `15s`, `30s`, `1m`, `3m`, `5m`, `15m`, `30m`, `1h`, `2h`, `4h`, `6h`, `12h`, `1d`. |
| `from`      | RFC3339 | oui    | Borne basse inclusive sur `open_time`.                                                   |
| `to`        | RFC3339 | non    | Borne haute exclusive. Défaut côté serveur : `Clock::now()`.                             |

**Cap profondeur** : la réponse contient au plus **5000 points**. Le serveur
calcule `(to - from) / timeframe` **avant** d'émettre la requête SQL ; toute
combinaison qui dépasserait ce plafond est refusée avec un `400
too_many_points`. C'est au client de soit raccourcir la fenêtre, soit choisir
un timeframe plus grossier. Une seconde ligne de défense est posée côté SQL
via `LIMIT 5001`.

**Réponse 200** — tableau JSON compatible TradingView Lightweight Charts :

```json
[
  { "ts": "2026-05-27T00:00:00Z", "o": 75851.4, "h": 76006.13, "l": 75794.59, "c": 76000.22, "v": 122.99619 },
  ...
]
```

**Codes d'erreur** :

| Code | Body `error`          | Cause                                                |
|------|-----------------------|------------------------------------------------------|
| 400  | `invalid_timeframe`   | Valeur hors whitelist (le body renvoie `allowed`).   |
| 400  | `invalid_range`       | `from >= to` (ou `to` absent et `from >= now`).      |
| 400  | `too_many_points`     | `(to - from) / timeframe > 5000`.                    |
| 401  | —                     | `X-API-Key` absent ou invalide.                      |
| 503  | `service_unavailable` | Pool Postgres injoignable.                           |
| 500  | `internal_error`      | Schema drift ou autre bug — toujours loggé.          |

**Exemple curl** :

```bash
curl -H "X-API-Key: $VIZ_API_KEY" \
  "http://localhost:3100/api/v1/monitoring/candles?exchange=binance&symbol=BTC/USDC&timeframe=1h&from=2026-05-27T00:00:00Z&to=2026-05-27T05:00:00Z"
```

## sqlx — mode offline

Le crate `sqlx` est utilisé en **compile-time checked queries** :
`sqlx::query!` parse chaque requête SQL **à la compilation** et la
type-check côté Postgres. Pour que la CI puisse builder sans accès à la
base, les métadonnées de vérification sont sérialisées dans
`backend/.sqlx/` et **commitées** dans le dépôt. `cargo build` consomme ce
dossier dès qu'on exporte `SQLX_OFFLINE=true` (la CI le fait au niveau du
job, cf. `.github/workflows/check.yml`).

**Quand regénérer `.sqlx/`** : à toute modification d'une requête
`sqlx::query!` (nouvelle requête, ajout/suppression de colonne…).

```bash
# Pré-requis : sqlx-cli installé.
cargo install sqlx-cli --no-default-features --features postgres,rustls

# Génère les métadonnées en se connectant à la DB de dev (source `.env`).
cd backend
set -a && . .env && set +a
cargo sqlx prepare --workspace

# Commit le dossier `.sqlx/` à la racine de `backend/`.
git add .sqlx/ && git commit -m "Refresh sqlx offline metadata"
```

> ⚠️ Le rôle utilisé dans `DATABASE_URL` doit avoir le `GRANT SELECT` sur les
> tables interrogées (`pompote_viz_reader`). `cargo sqlx prepare` ne mute
> jamais la base : il fait des `DESCRIBE` côté Postgres et écrit le résultat
> en JSON dans `backend/.sqlx/`.

## Commandes Cargo

```bash
# Reproduit exactement la matrice CI (consomme `.sqlx/`, pas besoin de DB locale).
SQLX_OFFLINE=true cargo fmt --all -- --check
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
SQLX_OFFLINE=true cargo build --workspace --locked
SQLX_OFFLINE=true cargo test --workspace --locked
```

Les tests unitaires actuels n'ouvrent aucune connexion Postgres. Les tests
d'intégration backed par une DB jetable seront ajoutés dans l'issue #12.

## Garde-fous

- ❌ Pas de migration, pas d'`INSERT` / `UPDATE` / `DELETE` (le rôle
  `pompote_viz_reader` rejette de toute façon ces requêtes côté DB).
- ❌ Pas de dépendance directe entre `domain` et `axum` / `sqlx` / `tokio`.
- ❌ Pas d'interaction exchange (cf. interdiction absolue dans
  [`CLAUDE.md`](../CLAUDE.md)).
- ✅ Le crate `domain` ne dépend que de `serde` + `chrono` + `thiserror`
  + `rust_decimal` (fixed-precision pour OHLCV — pas d'I/O).
