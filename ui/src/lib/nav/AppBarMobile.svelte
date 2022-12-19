<script lang="ts">
	import {
		Collapse,
		Nav,
		Navbar,
		NavbarBrand,
		NavbarToggler,
		NavItem,
		NavLink
	}             from "sveltestrap"
	import {BAND} from "../../util/constants.js"

	let isOpen = false
	const toggle = () => (isOpen = !isOpen)
</script>
<slot></slot>
<Navbar color="light" light>
	<NavbarBrand href="/" class="me-auto">{BAND}</NavbarBrand>
	<NavbarToggler on:click={toggle} class="me-2"/>
	<Collapse {isOpen} navbar>
		<Nav navbar>
			<NavItem>
				<NavLink href="#components/">Components</NavLink>
			</NavItem>
			<NavItem>
				<NavLink href="https://github.com/bestguy/sveltestrap">GitHub</NavLink>
			</NavItem>
		</Nav>
	</Collapse>
</Navbar>