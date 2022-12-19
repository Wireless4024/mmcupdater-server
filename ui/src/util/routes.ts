import type {
	AsyncSvelteComponent,
	WrappedComponent
}                        from "svelte-spa-router"
import wrap              from "svelte-spa-router/wrap"
import {get_store_value} from "svelte/internal"
import type {Writable}   from "svelte/store"
import {writable}        from "svelte/store"
import Loading           from "../page/Loading.svelte"

function component(asyncComponent: AsyncSvelteComponent) {
	return wrap({
		asyncComponent,
		loadingComponent: Loading
	})
}

export type RouteInfo = {
	name: string
	hidden?: Writable<boolean>
	disabled?: Writable<boolean>
}

class RouteBuilder {
	routes: Record<string, WrappedComponent> = {}
	private metadata: Record<string, RouteInfo> = {}

	add(path: string, comp: WrappedComponent, meta: RouteInfo): this {
		this.routes[path] = comp
		this.metadata[path] = meta
		return this
	}

	get_routes(): Record<string, WrappedComponent> {
		const routes: Record<string, WrappedComponent> = {}
		for (let path in this.routes) {
			let hidden = this.metadata[path].hidden
			if (hidden && !get_store_value(hidden)) {
				routes[path] = this.routes[path]
			}
		}
		return routes
	}
}

const builder = new RouteBuilder()
	.add(
		"/",
		component(() => import("../page/Home.svelte")),
		{name: "nav.home"}
	)
	.add(
		"/login",
		component(() => import("../page/Login.svelte")),
		{name: "form.login", hidden: writable(true)}
	)

export default builder.routes