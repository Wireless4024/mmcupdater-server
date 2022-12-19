import {writable}  from "svelte/store"
import type {User} from "./user"

export const USER = writable<User>()