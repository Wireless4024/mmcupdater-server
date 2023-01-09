<script lang="ts">
	import {_}             from "svelte-i18n"
	import {
		Alert,
		Nav,
		Navbar,
		NavItem,
		NavLink
	}                      from "sveltestrap"
	import {USER}          from "../api/shared_state.js"
	import {logout}        from "../api/user.js"
	import {ALERT}         from "../util/alert"
	import {consume_alert} from "../util/alert.js"
	import Container       from "./Container.svelte"
	import NavDropdown     from "./nav/NavDropdown.svelte"
	import NavLinkItem     from "./nav/NavLinkItem.svelte"
</script>
<Navbar light="true" class="mb-2">
	<Nav tabs>
		<NavLinkItem href="/" name="nav.home"/>
		<NavDropdown/>
		<NavItem>
			<NavLink href="#">Link</NavLink>
		</NavItem>
		<NavItem>
			<NavLink href="#">Another Link</NavLink>
		</NavItem>
		<NavItem>
			<NavLink disabled href="#">Disabled Link</NavLink>
		</NavItem>
	</Nav>
	<Nav tabs>
		{#if $USER}
			<NavItem>
				<NavLink href="#" on:click={logout}>{$_("form.logout")}</NavLink>
			</NavItem>
		{:else}
			<NavLinkItem href="/login" name="form.login"/>
		{/if}
	</Nav>
</Navbar>

<Container>
	{#if $ALERT}
		<Alert color={$ALERT.typ} class="alert-dismissible">
			<button type="button" class="btn-close" aria-label="Close" on:click={consume_alert}></button>
			{$_($ALERT.message)}
		</Alert>
	{/if}
</Container>
<slot></slot>