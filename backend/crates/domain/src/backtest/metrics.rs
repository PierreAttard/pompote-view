//! Recomputed performance metrics for a backtest run.
//!
//! Derived **read-only** from `fills_backtest` via the `GROUP BY side` recipe
//! documented in `robot_rust/docs/backtester.md` §5:
//!
//! ```sql
//! SELECT side, count(*), sum(quantity * price) AS notional, sum(fee) AS fees
//! FROM fills_backtest WHERE run_id = $1 GROUP BY side;
//! ```
//!
//! From the per-side aggregates we derive a cash-flow PnL: money received from
//! sells minus money spent on buys, minus fees. This is exact when the run ends
//! flat (Σ buy qty == Σ sell qty); an open residual position is reflected as an
//! unrealised skew, which the front-end surfaces alongside the per-side
//! quantities so the figure is never read out of context.
//!
//! Note: §5 labels the aggregate "Hit-rate / PnL brut" but does **not** define
//! a round-trip win-rate formula; computing one would require pairing
//! entries/exits, a semantic owned by `robot_rust`. We therefore expose the
//! unambiguous aggregates (per-side counts, notionals, fees) and the derived
//! net PnL, and leave a formal hit-rate to a future `robot_rust`-defined recipe.

use rust_decimal::Decimal;

use crate::order::OrderSide;

/// One row of the `GROUP BY side` aggregate over a run's fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillAggregate {
    /// Side this aggregate covers.
    pub side: OrderSide,
    /// Number of fills on this side.
    pub fills: u64,
    /// Total executed base-asset quantity (`sum(quantity)`).
    pub quantity: Decimal,
    /// Total quote notional (`sum(price * quantity)`).
    pub notional: Decimal,
    /// Total fees charged (`sum(fee)`), summed across fee assets verbatim.
    pub fees: Decimal,
}

/// Recomputed metrics for a single run, folded from its per-side fill
/// aggregates. Money fields use [`Decimal`] to avoid float drift.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacktestMetrics {
    /// Number of buy fills.
    pub buy_fills: u64,
    /// Number of sell fills.
    pub sell_fills: u64,
    /// Total bought base-asset quantity.
    pub buy_quantity: Decimal,
    /// Total sold base-asset quantity.
    pub sell_quantity: Decimal,
    /// Quote spent on buys (`sum(price * quantity)` over buys).
    pub buy_notional: Decimal,
    /// Quote received from sells (`sum(price * quantity)` over sells).
    pub sell_notional: Decimal,
    /// Total fees over all fills.
    pub total_fees: Decimal,
    /// Gross cash-flow PnL: `sell_notional - buy_notional` (pre-fees).
    pub gross_pnl: Decimal,
    /// Net cash-flow PnL: `gross_pnl - total_fees`.
    pub net_pnl: Decimal,
}

impl BacktestMetrics {
    /// Folds the per-side aggregates (zero, one or two rows) into the run
    /// metrics. Missing sides default to zero, so a run with only buys (or no
    /// fills at all) yields a well-formed, all-zero-on-the-missing-side result.
    pub fn from_aggregates(aggregates: &[FillAggregate]) -> Self {
        let pick = |side: OrderSide| aggregates.iter().find(|a| a.side == side);
        let buy = pick(OrderSide::Buy);
        let sell = pick(OrderSide::Sell);

        let buy_notional = buy.map(|a| a.notional).unwrap_or_default();
        let sell_notional = sell.map(|a| a.notional).unwrap_or_default();
        let total_fees = aggregates.iter().map(|a| a.fees).sum();
        let gross_pnl = sell_notional - buy_notional;

        Self {
            buy_fills: buy.map(|a| a.fills).unwrap_or_default(),
            sell_fills: sell.map(|a| a.fills).unwrap_or_default(),
            buy_quantity: buy.map(|a| a.quantity).unwrap_or_default(),
            sell_quantity: sell.map(|a| a.quantity).unwrap_or_default(),
            buy_notional,
            sell_notional,
            total_fees,
            gross_pnl,
            net_pnl: gross_pnl - total_fees,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agg(
        side: OrderSide,
        fills: u64,
        quantity: Decimal,
        notional: Decimal,
        fees: Decimal,
    ) -> FillAggregate {
        FillAggregate {
            side,
            fills,
            quantity,
            notional,
            fees,
        }
    }

    #[test]
    fn folds_both_sides_into_cash_flow_pnl() {
        let metrics = BacktestMetrics::from_aggregates(&[
            agg(
                OrderSide::Buy,
                2,
                Decimal::new(15, 1),
                Decimal::from(100),
                Decimal::new(1, 1),
            ),
            agg(
                OrderSide::Sell,
                3,
                Decimal::new(15, 1),
                Decimal::from(130),
                Decimal::new(2, 1),
            ),
        ]);
        assert_eq!(metrics.buy_fills, 2);
        assert_eq!(metrics.sell_fills, 3);
        assert_eq!(metrics.gross_pnl, Decimal::from(30)); // 130 - 100
        assert_eq!(metrics.total_fees, Decimal::new(3, 1)); // 0.3
        assert_eq!(metrics.net_pnl, Decimal::new(297, 1)); // 29.7
    }

    #[test]
    fn missing_side_defaults_to_zero() {
        let metrics = BacktestMetrics::from_aggregates(&[agg(
            OrderSide::Buy,
            1,
            Decimal::from(1),
            Decimal::from(50),
            Decimal::new(5, 2), // 0.05
        )]);
        assert_eq!(metrics.sell_fills, 0);
        assert_eq!(metrics.sell_notional, Decimal::ZERO);
        assert_eq!(metrics.gross_pnl, Decimal::from(-50)); // 0 - 50
        assert_eq!(metrics.net_pnl, Decimal::new(-5005, 2)); // -50.05
    }

    #[test]
    fn no_fills_yields_all_zero() {
        let metrics = BacktestMetrics::from_aggregates(&[]);
        assert_eq!(
            metrics,
            BacktestMetrics {
                buy_fills: 0,
                sell_fills: 0,
                buy_quantity: Decimal::ZERO,
                sell_quantity: Decimal::ZERO,
                buy_notional: Decimal::ZERO,
                sell_notional: Decimal::ZERO,
                total_fees: Decimal::ZERO,
                gross_pnl: Decimal::ZERO,
                net_pnl: Decimal::ZERO,
            }
        );
    }
}
