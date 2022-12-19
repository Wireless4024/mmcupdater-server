import type {PromiseSafe} from "../util/promise"

export type Result<T = any, M = string> = {
	success: boolean

	// Success result
	result: T


	// Error message
	message?: M

	// Error cause
	err_cause?: string
}

export type Error<T> = {
	message: T
	cause: string
}

export function raw_get<T>(path: string): Promise<Result<T>> {
	return fetch(path, {
		mode          : 'cors',
		credentials   : 'include',
		method        : 'GET',
		cache         : 'no-cache',
		referrerPolicy: 'no-referrer',
		redirect      : "error",
	})
		.then(it => it.json())
		.then(it => it as Result<T>)
}

export function raw_post<T>(path: string, body: any): Promise<Result<T>> {
	return fetch(path, {
		mode          : 'cors',
		credentials   : 'include',
		method        : 'POST',
		cache         : 'no-cache',
		headers       : {
			'Content-Type': 'application/json'
		},
		redirect      : "error",
		referrerPolicy: 'no-referrer',
		body          : typeof body == 'string' ? body : JSON.stringify(body)
	})
		.then(it => it.json())
		.then(it => it as Result<T>)
}

export function get<T, E = string>(url: string): PromiseSafe<T, Error<E>> {
	return raw_get<T>(url)
		.then(it => it.success ? Promise.resolve(it.result) : Promise.reject(it))
}