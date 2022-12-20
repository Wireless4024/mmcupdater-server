export type PromiseSafe<T, E = any> = Promise<T> & {
	catch<TResult = never>(onrejected?: ((reason: E) => TResult | PromiseLike<TResult>) | undefined | null): Promise<T | TResult>;
}