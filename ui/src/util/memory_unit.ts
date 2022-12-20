export const KiB = (n: number): number => n / 1024
export const MiB = (n: number): number => KiB(n) / 1024
export const GiB = (n: number): number => MiB(n) / 1024

const SCALE = [" KiB", " MiB", " GiB", " TiB"]

export function memory_unit_from_k(n: number) {
	if (!n) return "0 KiB"
	let scale = 0
	while (n > 1024) {
		n /= 1024
		++scale
	}
	return n.toFixed(3) + SCALE[scale]
}