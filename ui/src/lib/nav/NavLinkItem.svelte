<script lang="ts">
	import {_}        from "svelte-i18n"
	import {location} from "svelte-spa-router"
	import {
		NavItem,
		NavLink
	}                 from "sveltestrap"

	export let href: string = ''
	export let name: string = '_'
	
	let active: boolean
	$:active = (href.endsWith("/") && href.length > 2) ? $location.startsWith(href) : $location == href
</script>
<NavItem class="mx-1">
	<NavLink href="#{href}" {active}>{$_(name)}</NavLink>
</NavItem>