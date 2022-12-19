<script lang="ts">
	import {_}         from "svelte-i18n"
	import {
		Nav,
		Navbar,
		NavItem,
		NavLink
	}                  from "sveltestrap"
	import {USER}      from "../api/shared_state.js"
	import {logout}    from "../api/user.js"
	import NavDropdown from "./nav/NavDropdown.svelte"
	import NavLinkItem from "./nav/NavLinkItem.svelte"
</script>
<Navbar light="true" class="mb-2">
	<Nav pills>
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
	<Nav pills>
		{#if $USER}
			<NavItem>
				<NavLink href="#" on:click={logout}>{$_("form.logout")}</NavLink>
			</NavItem>
		{:else}
			<NavLinkItem href="/login" name="form.login"/>
		{/if}
	</Nav>
</Navbar>
<slot></slot>