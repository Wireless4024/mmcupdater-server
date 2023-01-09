<script lang="ts">
	import {onDestroy}    from "svelte"
	import {ListGroup}    from "sveltestrap"
	import {was_child_of} from "../util/dom_helper"
	import {CLICK_EVENT}  from "../util/shared"

	export let isOpen: boolean = false
	let x: number = 0
	let y: number = 0

	export function dispatch(ev: MouseEvent) {
		x = ev.clientX
		y = ev.clientY
		isOpen = true
	}

	let unsub

	$:{
		if (isOpen) sub()
		else unsub && unsub()
	}

	function sub() {
		(handle_close as any).__first__ = true
		unsub = CLICK_EVENT.subscribe(handle_close)
	}

	function handle_close(ev: MouseEvent) {
		if ((handle_close as any).__first__) {
			(handle_close as any).__first__ = false
			return
		}
		if (!ev || !ev.target || ev.button !== 0) return
		if (!was_child_of(self, ev.target as Node)) isOpen = false
	}

	onDestroy(function () {
		unsub && unsub()
	})

	let self: HTMLDivElement
</script>
{#if isOpen}
	<div bind:this={self} class="context-menu" style="top:{y}px;left:{x}px">
		<ListGroup>
			dsads
			<slot></slot>
		</ListGroup>
	</div>
{/if}
<style>
	.context-menu {
		z-index: 1000;
		position: absolute;
		padding: 2px;
		display: block;
		margin: 0;
	}
</style>