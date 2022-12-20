import {notify_fast} from "../util/alert"
import {HOST}        from "../util/constants"
import {
	get,
	raw_post
}                    from "./api"
import {USER}        from "./shared_state"

export type User = {
	name: string
	username: string
	permissions: string
}

export async function login(username: string, password: string): Promise<string> {
	const resp = await raw_post<"Ok">(`${HOST}/api/v1/auth/login`, {username, password, set: true})
	if (resp.success) {
		return "auth.success"
	} else {
		return resp.message || "auth.invalid"
	}
}

export function get_user(): Promise<User> {
	return get<User>(`${HOST}/api/v1/user`).catch(() => undefined!)
}

export function logout(): Promise<void> {
	return get(`${HOST}/api/v1/auth/logout`)
		.then(() => {
			notify_fast("auth.logout")
		USER.set(undefined!)
	})
}