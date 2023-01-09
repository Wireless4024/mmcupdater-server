import {HOST} from "../util/constants"
import {
	get,
	post,
	remove
}             from "./api"

export type Instance = {
	config: InstanceConfig
	mod_type: string
	name: string
	version: string
}

export type InstanceConfig = {
	java: string
	max_ram: number
	jvm_args: string[]
	server_file: string
	args: string[]
	dist_folder: string[]
}

export type InstanceType = { forge: string } | "Vanilla" | "Purpur"

export function get_all_instances(): Promise<string[]> {
	return get<string[]>(`${HOST}/api/v1/instance/`).catch(() => undefined!)
}

export function get_instances_info(name: string): Promise<Instance> {
	return get<Instance>(`${HOST}/api/v1/instance/${name}`).catch(() => undefined!)
}

/**
 * @param name Instance name
 * @param typ Minecraft mod type {@link InstanceType}
 * @param version Minecraft version eg. 1.19
 */
export function new_instance(name: string, typ: InstanceType, version: string) {
	return post<Instance>(`${HOST}/api/v1/instance/${name}`, {typ, version})
}

export function delete_instance(name: string): Promise<void> {
	return remove(`${HOST}/api/v1/instance/${name}`)
}