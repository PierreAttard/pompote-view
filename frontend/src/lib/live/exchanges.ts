/**
 * Exchanges offered by the live monitoring selector.
 *
 * The backend has no "list exchanges" endpoint, so this is a curated front-end
 * constant. `binance` is the default (the robot's primary venue); the candles
 * endpoint requires an `exchange` value, hence the selector. Extend this list
 * when the robot starts trading another venue.
 */
export const EXCHANGES = ['binance', 'kraken', 'coinbase', 'bybit', 'okx'] as const;

export type Exchange = (typeof EXCHANGES)[number];

/** Default venue when the URL carries no (or an unknown) `exchange`. */
export const DEFAULT_EXCHANGE: Exchange = 'binance';

/** `true` if `value` is a known exchange id. */
export function isExchange(value: string | null | undefined): value is Exchange {
	return EXCHANGES.includes(value as Exchange);
}
