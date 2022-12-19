<svelte:head>
	<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap@5.2.1/dist/css/bootstrap.min.css">
</svelte:head>
<script lang="ts">
	import {onMount}   from "svelte"
	import {isLoading} from "svelte-i18n"
	import Router      from "svelte-spa-router"
	import {Progress}  from "sveltestrap"
	import {USER}      from "./api/shared_state"
	import {get_user}  from "./api/user"
	import "./lang/lang"
	import AppBar      from "./lib/AppBar.svelte"
	import routes      from "./util/routes.js"

	onMount(function () {
		get_user().then(user => USER.set(user))
	})
</script>
{#if $isLoading}
	<Progress animated color="info" value={100}/>
{:else}
	<AppBar>
		<main>
			<Router {routes}/>
		</main>
	</AppBar>
{/if}