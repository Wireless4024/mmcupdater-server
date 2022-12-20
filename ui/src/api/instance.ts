import {HOST} from "../util/constants"
import {get}  from "./api"

export type Instance = {
	config: InstanceConfig
	mod_type: string
	name: string
	version: string
}

export type InstanceConfig = {
	java: string
	max_ram: 1024
	jvm_args: string[]
	server_file: string
	args: string[]
	dist_folder: string[]
}

export function get_all_instances(): Promise<string[]> {
	return get<string[]>(`${HOST}/api/v1/instance/`).catch(() => undefined!)
}

export function get_instances_info(name: string): Promise<Instance> {
	return get<Instance>(`${HOST}/api/v1/instance/info?name=` + name).catch(() => undefined!)
}