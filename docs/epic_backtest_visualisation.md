# EPIC supplémentaire — Visualisation backtest (Lot 10)

> Statut : **planifié** (juin 2026). Cet EPIC étend l'[Epic #1 — Visualisation &
> monitoring live des stratégies](https://github.com/PierreAttard/pompote-view/issues/1).
> Le backtest était **hors-scope** de l'Epic d'origine ; il est traité ici comme un
> **Lot 10** autonome, greffé sur l'infrastructure déjà en place (chart, markers, cap
> 5000, auth `api_key`, archi hexagonale).
>
> Tout reste **read-only / Postgres**, conforme à l'architecture hexagonale et aux
> interdictions strictes du repo (pas de modification de `robot_rust`, aucune interaction
> exchange). Le plan d'exécution détaillé (cases à cocher, découpage en issues) vit dans
> [`../TODO.md`](../TODO.md) ; ce document porte le **cadrage** (le « pourquoi » et les
> décisions verrouillées).

## 1. Objectif

Visualiser dans `pompote-view` les stratégies **rejouées en backtest** par `robot_rust`,
au même niveau de lisibilité que le live : bougies OHLC, markers de décisions buy/sell
avec leur `reason`, ordres et fills, overlays d'indicateurs.

La valeur produit : permettre à une tradeuse (persona *Agathe*) de **rejouer et auditer**
le comportement d'une stratégie sur une fenêtre historique, run par run, sans rouvrir le
moteur d'exécution privé.

## 2. Contexte technique (source de vérité : `robot_rust`)

`robot_rust` a ajouté (migration `20260605`) un jeu de tables backtest, miroir des tables
live mais clé par run :

- `backtest_runs` — **racine**, une ligne par run (métadonnées, fenêtre, config snapshot).
- `strategy_decisions_backtest`
- `decision_market_context_backtest`
- `orders_backtest`
- `fills_backtest`
- `positions_backtest`
- `position_stops_backtest`

Le `GRANT SELECT` sur ces tables au rôle `pompote_viz_reader` a **déjà été fait** côté
`robot_rust` par l'humain. `pompote-view` n'ajoute **aucune** migration (cf. CLAUDE.md).

## 3. Décisions d'architecture verrouillées

Ces décisions sont **figées** ; toute remise en cause doit être remontée à l'humain.

1. **Axe temps = `market_ts`** (horloge marché rejouée), **jamais** `created_at` (= instant
   d'insertion en base, sans rapport avec le temps simulé). Tous les markers, tris et
   filtres backtest s'appuient sur `market_ts`.
2. **Modèle run-centric** : tout est clé par `run_id`. Le sélecteur UI est une **liste de
   runs**, pas un couple stratégie/symbole. `backtest_runs.strategy_id` est nullable /
   synthétique et **n'a pas de FK** vers `strategies`.
3. **Pas de table `candles_backtest`, pas d'accès S3, pas de réutilisation du `.env`
   `robot_rust`** : les clés S3 du `trade_archiver` sont write-capable et casseraient
   l'isolation read-only. Les bougies proviennent **uniquement** de `candles_5s`.
4. **Scope V1 = dégradation propre** :
   - chart OHLC complet quand `candles_5s` **couvre** la fenêtre du run (runs récents) ;
   - sinon **timeline-only** (markers sans fond de bougies) + **bandeau de biais** pour les
     runs C0 synthétiques et les vieilles fenêtres froides (cf. `robot_rust/docs/backtester.md` §3).
   - `candles_5s` n'a **aucune rétention** (table créée le 2026-05-19) : l'absence de
     bougies vient de la **non-couverture** par `candle_storer` (ou d'un run synthétique C0),
     pas d'une purge.
5. **DB séparée / réplica en lecture = différé** (coûteux). On reste sur le primaire avec un
   **`statement_timeout`** par connexion comme garde-fou anti-contention
   (`VIZ_DB_STATEMENT_TIMEOUT_MS`, défaut `5000`).
6. **Réutilisation maximale** : le chart et les markers (Lots 4-6) doivent devenir
   **agnostiques de la source** (props `candles` + `markers` + `overlays`). La vue backtest
   n'est alors qu'un *data-loader* différent.

## 4. Périmètre

### Dans le scope (V1)

- Backend : endpoints REST sous `/api/v1/monitoring/backtests` (derrière `api_key`) :
  liste de runs, détail d'un run, séries `orders` / `fills` / `decisions` (sur `market_ts`),
  bougies composées depuis `candles_5s` avec un hint `source: "candles_5s" | "none"`.
- Frontend : route liste `/backtests` (sélecteur de run) + route détail
  `/backtests/[run_id]` (chart réutilisé, résumé de run, bandeau biais, état dégradé).
- Domaine : introduction des entités partagées `Decision` et `Fill` (réutilisées ensuite
  par le live #9/#11), entité `BacktestRun` + VO `BacktestStatus`.

### Hors scope V1 (différé V1+1)

- Métriques de run recalculées (PnL net / frais / hit-rate depuis `fills_backtest`,
  recette SQL `GROUP BY side` dans `backtester.md` §5).
- Polling d'un run `running` (un run `completed` est statique → pas de polling 10 s).
- Backfill `candles_5s` des fenêtres froides (décision ops / `robot_rust`, côté humain).
- Réplica en lecture (zéro changement de code côté `pompote-view` : swap de `DATABASE_URL`).

## 5. Découpage en chantiers

| Chantier | Contenu | Dépendances |
|---|---|---|
| **A** | `statement_timeout` sur le pool viz (garde-fou anti-contention) | aucune — *maintenant* |
| **B** | Backend : domaine `BacktestRun`/`Decision`/`Fill` + ports + use cases + repos sqlx + DTOs/utoipa | tests d'intégration → #12 |
| **C** | Pipeline OpenAPI → client TS (#13) — **prérequis** de la vue frontend | B |
| **D** | Frontend : vue liste + vue détail (réutilise Lots 4-6 source-agnostic) | Lots 4-6 + #13 |

### Séquencement

```
A. statement_timeout ──────────────► (indépendant, maintenant)

B. Backend backtest ─┬─ introduit domain Decision/Fill (réutilisés par live #9/#11)
                     └─► C. OpenAPI #13 ─► D. Frontend backtest
                                            ▲
            Lots 4-6 (chart/markers/overlays, source-agnostic) ┘
```

Chemin : **A** (maintenant) → **B** (en parallèle de la fin du Lot 1 live) → **#13** →
construire **Lots 4-6** source-agnostic → **D**.

## 6. Endpoints backend (cible)

Tous montés sous `/api/v1/monitoring/backtests`, derrière le middleware `api_key`, avec
annotations `utoipa` (donc inclus dans le client TS généré par #13).

| Méthode | Route | Rôle | Cap |
|---|---|---|---|
| `GET` | `/backtests` | liste / sélecteur (filtres `status`/`strategy_kind`/`symbol`/`exchange`, tri `started_at DESC`) | `MAX_BACKTEST_RUNS` (~500) |
| `GET` | `/backtests/{run_id}` | détail (métadonnées + `config_snapshot` + `scenario_name`) | — |
| `GET` | `/backtests/{run_id}/candles?timeframe=` | agrégation `candles_5s` sur la fenêtre du run + hint `source` | `MAX_CANDLE_POINTS` |
| `GET` | `/backtests/{run_id}/decisions` | `market_ts ASC` + snapshot JSONB | `MAX_ORDER_ROWS` |
| `GET` | `/backtests/{run_id}/orders` | `market_ts ASC` | `MAX_ORDER_ROWS` |
| `GET` | `/backtests/{run_id}/fills` | `market_ts ASC` | `MAX_ORDER_ROWS` |

Les repos sqlx filtrent en `WHERE run_id = $ AND market_ts >= $from AND market_ts < $to
ORDER BY market_ts ASC LIMIT $`, réutilisent `parse_side` / `parse_status` (défense contre
le schema-drift) et `map_sqlx_error`. Le `snapshot` est transporté en `serde_json::Value`
opaque (display-only). Les DTOs HTTP exposent **`market_ts`** (pas `created_at`).

## 7. Garde-fous & conformité

- **Read-only strict** : rôle `pompote_viz_reader` (SELECT only) ; aucune requête mutative.
- **Aucune migration / aucun schéma** dans ce repo (propriété `robot_rust`).
- **Interdiction `robot_rust`** : si une table/colonne/grant manque, **STOP** et remontée à
  l'humain (jamais de PR côté `robot_rust`).
- **Interdiction exchange** : aucune connexion API, aucune clé exchange, aucun ordre réel.
- **Cap 5000 points** réutilisé (`MAX_CANDLE_POINTS` / `MAX_ORDER_ROWS`).
- **Hexagonal** : `adapters → application → domain` ; DTOs HTTP distincts des entités.

## 8. Issues

Le découpage en issues GitHub (label `view`, priorités `p1`/`p2`/`p3`, issue chapeau) est
maintenu dans [`../TODO.md`](../TODO.md) §« Découpage en issues ». Création via l'agent
`pompote` (PM). Les `kind:strategy-idea` éventuelles mentionnent explicitement la
dépendance `robot_rust` (à porter par l'humain).
