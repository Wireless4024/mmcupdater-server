import type {Writable} from "svelte/store"
import {writable}      from "svelte/store"

export const CLICK_EVENT: Writable<MouseEvent> = writable(null!)
export const CONFIG: Writable<Config> = writable(JSON.parse(localStorage.getItem("cfg") || "{}"))

type Config = {}

CONFIG.subscribe(it => {
	localStorage.setItem("cfg", JSON.stringify(it))
})