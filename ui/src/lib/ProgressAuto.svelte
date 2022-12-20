<script lang="ts">
	import {Progress} from "sveltestrap"

	export let value = 0
	let color: string
	$:color = value < 80 ? 'success' : value < 95 ? 'warning' : 'danger'
</script>
<Progress {value} {color}/>