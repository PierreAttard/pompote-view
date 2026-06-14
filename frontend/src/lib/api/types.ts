/**
 * Hand-friendly aliases over the generated OpenAPI types (`types.gen.ts`).
 *
 * Kept out of `$lib/server` so presentational components can import the shapes
 * without pulling in server-only code. Re-run `npm run codegen` when the
 * backend contract changes.
 */
import type { components } from './types.gen';

export type BacktestRunSummary = components['schemas']['BacktestRunSummaryDto'];
export type BacktestRunDetail = components['schemas']['BacktestRunDetailDto'];
export type BacktestCandles = components['schemas']['BacktestCandlesDto'];
export type Candle = components['schemas']['CandleDto'];
export type BacktestOrder = components['schemas']['BacktestOrderDto'];
export type Decision = components['schemas']['DecisionDto'];
export type BacktestMetrics = components['schemas']['BacktestMetricsDto'];
export type Strategy = components['schemas']['StrategyDto'];
export type LiveFill = components['schemas']['LiveFillDto'];
export type LiveDecision = components['schemas']['LiveDecisionDto'];
export type Order = components['schemas']['OrderDto'];
