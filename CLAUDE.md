# CLAUDE.md — pompote-view

> Contexte minimal pour Claude Code et les agents qui interviennent sur ce dépôt.
> Documentation en français. Code & messages de commit en anglais.

## Project

`pompote-view` est l'interface de **visualisation et monitoring live** des stratégies de trading
du projet PomPotRobot. Monorepo composé d'un **backend Rust read-only** (axum + sqlx) et d'un
**frontend SvelteKit** (Svelte 5) qui affichent bougies OHLC, indicateurs et annotations de
décisions (markers buy/sell + `reason`) lues depuis la TimescaleDB du robot.

Le repo est candidat à l'open-source : il ne contient **ni clé**, **ni secret**, **ni schéma DB**,
**ni logique métier de trading**. La propriété du schéma et du moteur d'exécution reste dans
[`robot_rust`](https://github.com/PierreAttard/robot_rust) (privé).

## Liens de cadrage

- Document de cadrage complet : [`docs/epic_visualisation_monitoring.md`](https://github.com/PierreAttard/robot_rust/blob/main/docs/epic_visualisation_monitoring.md) (côté `robot_rust`)
- Epic GitHub : [#1 — Visualisation & monitoring live des stratégies](https://github.com/PierreAttard/pompote-view/issues/1)
- Issue setup conventions : [#2](https://github.com/PierreAttard/pompote-view/issues/2)

## Architecture cible

```
┌─── Repo `pompote-view` (ce repo, open-source candidate) ─────────┐
│  Frontend (SvelteKit + Lightweight Charts)                       │
│    └─ HTTP + api_key → Backend viz (axum, read-only)             │
│                          └─ sqlx (rôle pompote_viz_reader)       │
└──────────────────────────────────────────────────────────────────┘
                            │ GRANT SELECT only
                            ▼
       TimescaleDB OVH (`pompote-db-service`)
                            ▲
                            │ accès r/w applicatif
┌─── Repo `robot_rust` (privé) ────────────────────────────────────┐
│  strategy_engine / trade_storer / candle_storer                  │
│  api_server (plan de contrôle, inchangé)                         │
│  Migrations DB (source de vérité du schéma)                      │
└──────────────────────────────────────────────────────────────────┘
```

### Architecture hexagonale rigoureuse (backend Rust)

Le backend suit une **architecture hexagonale (ports & adapters) stricte**. Chaque couche est
isolée dans son propre crate du workspace Cargo et **les dépendances ne traversent qu'une seule
direction** : `adapters → application → domain` (le domaine ne dépend de rien).

```
crates/
  domain/         # Entités, value objects, règles métier pures (no_std friendly)
                  # Aucune dépendance à axum, sqlx, serde-utilitaires d'I/O.
  application/    # Use cases (services applicatifs) + ports (traits)
                  # Orchestre le domaine. Définit les interfaces que les adapters implémentent.
  adapters/
    inbound/
      http/       # axum routes, handlers, DTOs HTTP, utoipa, middleware api_key
                  # Convertit Request → commande use-case → Response.
    outbound/
      persistence/  # sqlx repositories. Implémente les ports persistence du domaine.
      clock/        # Implémentation Clock réelle (système).
  bootstrap/      # main.rs : composition root (wire les adapters dans les use cases)
```

**Règles non négociables :**

- Le crate `domain` ne dépend **ni** de `axum`, **ni** de `sqlx`, **ni** d'aucun crate d'I/O.
- Les **ports** (traits) sont définis dans `application` (ou `domain` pour les ports métier purs),
  jamais dans les adapters.
- Les **DTOs HTTP** sont distincts des entités du domaine. La conversion se fait au bord
  (handler axum), via `From`/`TryFrom`.
- Les **DTOs Rust sont re-déclarés** dans ce repo (pas de crate Rust partagé avec `robot_rust`,
  on n'importe rien depuis le repo privé).
- Tests unitaires sur le `domain` et `application` **sans Postgres**.
- Tests d'intégration sur les adapters de persistance avec un Postgres jetable + le schéma cloné
  depuis `robot_rust`.

## Stack

### Backend

- **Rust 2024**, workspace Cargo multi-crates (cf. découpage hexagonal ci-dessus)
- **axum** pour le serveur HTTP (read-only)
- **sqlx** (Postgres + Timescale) en compile-time checked queries
- **utoipa** pour OpenAPI + Swagger UI + génération client TypeScript
- **tokio**, **tracing**, **serde**, **thiserror**, **anyhow** (au bord uniquement)

### Frontend

- **SvelteKit** (Svelte 5)
- **Vite** (build / dev)
- **Vitest** (tests unitaires & composants)
- **Playwright** (1-2 smokes E2E)
- **TradingView Lightweight Charts** (wrapper Svelte maison)
- Client TypeScript généré depuis l'OpenAPI du backend

### DB

- **TimescaleDB** consommée en **lecture seule** via le rôle `pompote_viz_reader` (`GRANT SELECT only`)
- **Aucune migration** dans ce repo : le schéma appartient à `robot_rust`
- Profondeur temporelle bornée côté backend : **cap explicite à 5000 points par requête**
- Mode live : **polling 10s** (pas SSE/WS sur cette première Epic)

## Conventions Git

- **Branches** : `feat/<n°-issue>-<description-courte>` (cohérent avec `robot_rust`)
  - Exemples : `feat/4-scaffold-workspace`, `feat/8-candles-endpoint`
- **Commits** : messages en **anglais**, impératif présent (`Add candles endpoint`, pas `Added`)
- **Comments dans le code** : **anglais**
- **Documentation** (`*.md`, docstrings utilisateur) : **français**
- **Label `view`** obligatoire sur **toutes** les issues de ce repo (filtrage sur le board
  `PomPotRobot` partagé avec `robot_rust`). Tout agent qui ouvre une issue ajoute le label dès
  la création.

## Commandes utiles

### Backend (Rust)

```bash
cargo build                                          # build workspace
cargo test                                           # tests unitaires + intégration
cargo clippy --workspace --all-targets -- -D warnings  # lint strict
cargo fmt --all                                      # formatage
```

### Frontend (SvelteKit)

```bash
npm run dev          # serveur de dev Vite
npm run build        # build production
npm run test         # Vitest (unit + composant)
npm run test:e2e     # Playwright
npm run check        # svelte-check + tsc
```

### Stack complète locale

```bash
docker compose up    # Timescale + backend viz + frontend
```

## Garde-fous d'isolation

- ❌ **Aucune migration DB** dans ce repo — le schéma est la propriété de `robot_rust`. Si une
  évolution du schéma est nécessaire, ouvrir une PR côté `robot_rust`, jamais ici.
- ❌ **Pas de crate Rust partagé** avec `robot_rust`. Les **DTOs sont re-déclarés** côté
  `pompote-view`. Cela autorise l'ouverture open-source sans fuite du code privé.
- ❌ **Pas d'accès r/w à la DB**. La connexion utilise **uniquement** le rôle
  `pompote_viz_reader` (SELECT only). Toute requête mutative doit échouer côté DB.
- ❌ **Pas de secret commité**. Clé API et URL DB passent par variables d'environnement (et
  Sealed Secret K8s plus tard).
- ❌ **Pas de dépendance directe** du `domain` Rust vers `axum`, `sqlx` ou tout autre crate d'I/O
  (cf. architecture hexagonale).

## Hors scope

- Édition de stratégies via l'UI (le front est **read-only**)
- Backtest interactif
- Placement d'ordres manuels
- Déploiement K8s distant (Lot 8 différé — local uniquement via docker-compose pour cette Epic)
