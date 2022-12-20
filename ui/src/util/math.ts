export function percent(lower: number, higher: number): number {
	return Math.min(lower / higher, 1)
}