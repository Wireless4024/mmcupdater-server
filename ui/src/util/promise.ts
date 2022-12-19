export type PromiseSafe<T, E> = Omit<Promise<T>, "catch"> & {
	catch<TResult = never>(onrejected?: ((reason: E) => TResult | PromiseLike<TResult>) | undefined | null): Promise<T | TResult>;
}