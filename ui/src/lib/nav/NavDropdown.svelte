<script lang="ts">
	import {_} from "svelte-i18n"
	import {
		Dropdown,
		DropdownItem,
		DropdownMenu,
		DropdownToggle
	}          from "sveltestrap"

	let isOpen = false
	export let name: string = "nav.dropdown"
</script>
<Dropdown nav {isOpen} toggle={() => (isOpen = !isOpen)}>
	<DropdownToggle nav caret>{$_(name)}</DropdownToggle>
	<DropdownMenu>
		<DropdownItem header>Header</DropdownItem>
		<DropdownItem disabled>Action</DropdownItem>
		<DropdownItem>Another Action</DropdownItem>
		<DropdownItem divider/>
		<DropdownItem>Another Action</DropdownItem>
	</DropdownMenu>
</Dropdown>