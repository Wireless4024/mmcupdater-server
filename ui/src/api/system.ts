import {HOST} from "../util/constants"
import {get}  from "./api"

export type SystemInfo = {
	os: string
	arch: string
	hostname: string
	cpus: number
	cpu_clock: number
	mem_total: number
	mem_free: number
	mem_avail: number
	mem_buff: number
	mem_cache: number
	mem_shm: number
	mem_used: number
	swap_total: number
	swap_free: number
	load_1: number
	load_5: number
	load_15: number
}

export function info(): Promise<SystemInfo> {
	return get<SystemInfo>(HOST + "/api/v1/info")
}