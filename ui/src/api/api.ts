import {
	location,
	replace
}                         from "svelte-spa-router"
import {get_store_value}  from "svelte/internal"
import {urgent}           from "../util/alert"
import {HOST}             from "../util/constants"
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

const HANDLE_RESP = async function (it: Response) {
	if (it.status == 401) {
		const loc = get_store_value(location)
		if (!loc.startsWith('/login'))
			await replace("/login?next=" + loc)
		return Promise.reject(await it.text())
	} else {
		return it.json()
	}
}

async function HANDLE_RESP_BODY<T>(resp: Response): Promise<any | undefined> {
	if (resp.status == 401) {
		const loc = get_store_value(location)
		if (!loc.startsWith('/login'))
			await replace("/login?next=" + loc)
		return Promise.reject(await resp.text())
	} else if (resp.status >= 400 && resp.status < 500) {
		return Promise.reject("http." + resp.status)
	} else if (resp.status == 204) return undefined
	return resp.json()
}

export function raw_get<T>(path: string, method: "GET" | "DELETE" = "GET"): Promise<Result<T>> {
	return fetch(path, {
		mode          : 'cors',
		credentials   : 'include',
		method        : 'GET',
		cache         : 'no-cache',
		referrerPolicy: 'no-referrer',
		redirect      : "follow",
	})
		.then(HANDLE_RESP)
		.then(it => it as Result<T>)
}

export function raw_post<T>(path: string, body: any, method: "POST" | "PUT" | "PATCH" = "POST"): Promise<Result<T>> {
	return fetch(path, {
		mode          : 'cors',
		credentials   : 'include',
		method,
		cache         : 'no-cache',
		headers       : {
			'Content-Type': 'application/json'
		},
		redirect      : "follow",
		referrerPolicy: 'no-referrer',
		body          : typeof body == 'string' ? body : JSON.stringify(body)
	})
		.then(HANDLE_RESP)
		.then(it => it as Result<T>)
}

function handle_result<T>(res: Result<T>): Promise<T> {
	if (res.success) {
		return Promise.resolve(res.result)
	} else {
		return Promise.reject(res)
	}
}

export function get<T = any, E = string>(url: string): PromiseSafe<T, Error<E>> {
	return raw_get<T>(url)
		.then(handle_result)
}

export function post<T = any, E = string>(url: string, body: any): PromiseSafe<T, Error<E>> {
	return fetch(HOST + url, {
		mode          : 'cors',
		credentials   : 'include',
		method        : 'POST',
		cache         : 'no-cache',
		referrerPolicy: 'no-referrer',
		redirect      : "follow",
	}).then(HANDLE_RESP_BODY)
}

export function put<T = any, E = string>(url: string, body: any): Promise<T | undefined> {
	return fetch(HOST + url, {
		mode          : 'cors',
		credentials   : 'include',
		method        : 'PUT',
		cache         : 'no-cache',
		referrerPolicy: 'no-referrer',
		redirect      : "follow",
	}).then(HANDLE_RESP_BODY)
}

export function remove<T = any, E = string>(url: string): PromiseSafe<T, Error<E>> {
	return fetch(HOST + url, {
		mode          : 'cors',
		credentials   : 'include',
		method        : 'DELETE',
		cache         : 'no-cache',
		referrerPolicy: 'no-referrer',
		redirect      : "follow",
	}).then(HANDLE_RESP_BODY)
}

export async function check(): Promise<boolean> {
	return fetch(HOST + "/api", {mode: "no-cors", redirect: "follow", credentials: "same-origin"})
		.then(it => it.text())
		.then(() => true)
		.catch(() => (urgent('http.500', 'danger'), false))
}