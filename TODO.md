# TODO — Visualisation backtest (Lot 10) + `statement_timeout`

> Plan établi le 2026-06-13. Objectif : visualiser dans pompote-view les stratégies
> rejouées en **backtest** par `robot_rust`, en réutilisant un maximum du travail déjà
> prévu (chart, markers, cap 5000, auth). Tout reste **read-only / Postgres**, conforme
> à l'archi hexagonale et aux interdictions du repo (pas de `robot_rust`, pas d'exchange).
>
> Cadrage formel (le « pourquoi » + décisions verrouillées) :
> [`docs/epic_backtest_visualisation.md`](docs/epic_backtest_visualisation.md).
> Ce fichier-ci porte le **plan d'exécution** (cases à cocher, découpage en issues).

## Contexte (tables backtest côté `robot_rust`, migration `20260605`)

`backtest_runs` (racine, 1 ligne/run) + miroirs `strategy_decisions_backtest`,
`decision_market_context_backtest`, `orders_backtest`, `fills_backtest`,
`positions_backtest`, `position_stops_backtest`.

## Décisions verrouillées

- [x] **`GRANT SELECT` sur les `*_backtest` à `pompote_viz_reader`** — fait par l'utilisateur.
- **Axe temps = `market_ts`** (horloge marché rejouée), PAS `created_at` (= instant
  d'insertion, sans rapport). Trier/filtrer tous les markers backtest sur `market_ts`.
- **Modèle run-centric** : tout est clé par `run_id` ; le sélecteur UI = une liste de runs
  (pas stratégie/symbole). `backtest_runs.strategy_id` est nullable/synthétique, sans FK.
- **Pas de table `candles_backtest`, pas d'accès S3, pas de réutilisation du `.env`
  robot_rust** (les clés S3 du `trade_archiver` sont write-capable → casserait l'isolation).
  Les bougies viennent de `candles_5s` uniquement.
- **Scope V1 = dégradation propre** : chart OHLC quand `candles_5s` couvre la fenêtre
  (runs récents) ; sinon **timeline-only** (runs C0 synthétiques + vieilles fenêtres
  froides) + bandeau biais (cf. `robot_rust/docs/backtester.md` §3).
- **DB séparée / réplica lecture = différé** (coûteux) ; on reste sur le primaire +
  `statement_timeout` comme garde-fou anti-contention.

---

## Chantier A — `statement_timeout` (à faire en 1er, isolé, mergeable seul)

- [x] `viz_api/src/config.rs` : champ optionnel `db_statement_timeout_ms: u64`, lu depuis
      `VIZ_DB_STATEMENT_TIMEOUT_MS` (défaut `5000`).
- [x] `viz_api/src/main.rs` : sur `PgPoolOptions`, `.after_connect(...)` exécutant
      `SELECT set_config('statement_timeout', $1, false)` (valeur ms→texte ; `set_config`
      car `SET` n'accepte pas de bind param).
- [ ] (optionnel, ton côté) `ALTER ROLE pompote_viz_reader SET statement_timeout = '5s';`
      (ceinture + bretelles, couvre toute connexion du rôle).
- [ ] Test : `SELECT pg_sleep(10)` doit échouer ~5 s (test d'intégration #12, ou vérif manuelle).

---

## Chantier B — Backend : endpoints backtest (hexagonal, calqué sur candles/orders)

Montés sous `/api/v1/monitoring/backtests` (déjà derrière le middleware `api_key`).

### Endpoints

- [ ] `GET /backtests` — liste/sélecteur (filtres `status`/`strategy_kind`/`symbol`/`exchange`),
      tri `started_at DESC`, cap `MAX_BACKTEST_RUNS` (~500).
- [ ] `GET /backtests/{run_id}` — détail (métadonnées + `config_snapshot` + `scenario_name`).
- [ ] `GET /backtests/{run_id}/candles?timeframe=` — délègue à l'agrégation `candles_5s` sur la
      fenêtre du run ; renvoie un hint `source: "candles_5s" | "none"` ; cap `MAX_CANDLE_POINTS`.
- [ ] `GET /backtests/{run_id}/decisions` — `market_ts ASC` + snapshot JSONB ; cap `MAX_ORDER_ROWS`.
- [ ] `GET /backtests/{run_id}/orders` — `market_ts ASC` ; cap `MAX_ORDER_ROWS`.
- [ ] `GET /backtests/{run_id}/fills` — `market_ts ASC` ; cap `MAX_ORDER_ROWS`.

### Par couche

- [ ] `domain/backtest/` : entité `BacktestRun` + VO `BacktestStatus` (whitelist
      `running|completed|failed|aborted`, patron `OrderStatus`). Const `MAX_BACKTEST_RUNS`.
      Réutilise `OrderSide`/`OrderStatus`.
- [ ] Entités partagées `domain::decision::Decision` + `domain::fill::Fill` (n'existent pas
      encore : live #9/#11 non faits) → les introduire ici, le live les réutilisera.
      Le `snapshot` est transporté en `serde_json::Value` opaque (display-only).
- [ ] `application/ports/` : `BacktestRepository` (un seul port, tout run-scoped) :
      `list_runs`, `get_run`, `fetch_orders/fills/decisions(run_id, from, to, limit)`.
- [ ] `application/use_cases/` : `ListBacktestRuns`, `GetBacktestRun`,
      `GetBacktestRun{Orders,Fills,Decisions}`, `GetBacktestRunCandles` (compose : charge le
      run → `CandleRepository.fetch_aggregated` sur `exchange/symbol/[data_range_start,
      ended_at∨now]` → réutilise le cap bucket ; `CandleSeries` éventuellement vide).
- [ ] `adapters/outbound/persistence/` : repos sqlx sur `*_backtest`,
      `WHERE run_id=$ AND market_ts >= $from AND market_ts < $to ORDER BY market_ts ASC LIMIT $`.
      Réutilise `parse_side`/`parse_status` (défense schema-drift) + `map_sqlx_error`.
- [ ] `adapters/inbound/http/` : DTOs `BacktestRunDto`, `BacktestRunSummaryDto`,
      `BacktestOrderDto`/`BacktestFillDto`/`BacktestDecisionDto` (**`market_ts`** au lieu de
      `created_at`, `snapshot` en JSON brut). Annotations `utoipa` + montage `router.rs` +
      enregistrement `openapi.rs` (`paths` + `schemas`).
- [ ] Tests d'intégration (Postgres jetable + schéma `robot_rust`, dépend de #12).

### Différé V1+1

- [ ] Métriques de run (PnL net / frais / hit-rate recalculés depuis `fills_backtest`,
      recette SQL `GROUP BY side` dans `backtester.md` §5).

---

## Chantier C — OpenAPI → client TS (#13, prérequis frontend)

- [ ] Les endpoints backtest annotés `utoipa` (chantier B) tombent dans le client TS généré
      par #13. **#13 doit être fait AVANT la vue frontend backtest.**

---

## Chantier D — Frontend : vue backtest (réutilise Lots 4-6)

> Principe transverse à imposer aux Lots 4-6 : **chart agnostique de la source** (prend
> `candles` + `markers` + `overlays` en props). La vue backtest = un data-loader différent.

- [ ] Route liste `/backtests` : tableau des runs (kind, symbole, exchange, fenêtre, `status`,
      `started_at`, `scenario_name` pour C0) + filtres. = le sélecteur.
- [ ] Route détail `/backtests/[run_id]` :
  - [ ] réutilise chart (Lot 4) + markers buy/sell + tooltip `reason`+snapshot (Lot 5) +
        overlays (Lot 6) ;
  - [ ] panneau résumé run (params `slippage_bps`/`latency_ms`/frais, `status`, fenêtre) ;
  - [ ] bandeau « résultats indicatifs » (5 biais, `backtester.md` §3) ;
  - [ ] état dégradé si `source: "none"` : masquer le fond de bougies, timeline seule + message.
- [ ] Pas de polling 10 s (run `completed` statique). Polling d'un run `running` = V1+1.
- **Dépend de** : Lots 4-6 construits (source-agnostic) + #13.

---

## Séquencement

```
A. statement_timeout ──────────────► (indépendant, maintenant)

B. Backend backtest ─┬─ introduit domain Decision/Fill (réutilisés par live #9/#11)
                     └─► C. OpenAPI #13 ─► D. Frontend backtest
                                            ▲
            Lots 4-6 (chart/markers/overlays, source-agnostic) ┘
```

Chemin : **A** (maintenant) → **B** (en parallèle de la fin du Lot 1 live) → **#13** →
construire **Lots 4-6** source-agnostic → **D**.

---

## Découpage en issues — Lot 10 (label `view`)

1. [ ] [infra] `statement_timeout` sur le pool viz (Chantier A) — `p1`, autonome.
2. [ ] [backend] Domain `BacktestRun`/`BacktestStatus` + `Decision`/`Fill` partagés + ports — `p1`.
3. [ ] [backend] `GET /backtests` + `/{run_id}` (liste + détail) — `p1`.
4. [ ] [backend] `GET /{run_id}/orders` + `/fills` + `/decisions` (séries, `market_ts`) — `p1`.
5. [ ] [backend] `GET /{run_id}/candles` (compose + hint `source`) — `p2`.
6. [ ] [backend] Tests d'intégration backtest — `p1`, dépend de #12.
7. [ ] [frontend] Route liste `/backtests` (sélecteur de run) — `p2`, dépend de #13.
8. [ ] [frontend] Vue détail run (chart réutilisé + résumé + bandeau biais + état dégradé) —
       `p2`, dépend de Lots 4-6 + #13.
9. [ ] [backend, V1+1] Métriques de run (PnL/hit-rate depuis `fills_backtest`) — `p3`.
10. [ ] (chapeau) Lot 10 — Visualisation backtest : issue parent listant 1-9.

> Pour créer ces issues : lancer l'agent **`pompote`** (PM) sur ce découpage.

---

## Différé / côté humain (hors V1, non bloquant)

- [ ] Bougies des fenêtres froides/anciennes : décider *backfill `candles_5s`* vs
      *timeline-only définitif* — décision `robot_rust`/ops, quand le cas froid deviendra réel.
- [ ] Réplica en lecture : quand la charge le justifiera. pompote-view : zéro changement de
      code (swap de `DATABASE_URL`).
