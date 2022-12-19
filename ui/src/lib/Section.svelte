<script lang="ts">
	import AnimatedDiv from "./AnimatedDiv.svelte";

	export let style: string = ""
	export let order: number = 0
</script>
<AnimatedDiv {order}>
	<div class="border" style="margin-bottom: 8px">
		<div style="background-color: white;{style}">
			<slot></slot>
		</div>
	</div>
</AnimatedDiv>