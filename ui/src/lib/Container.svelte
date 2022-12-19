<script lang="ts">
	import {fly} from "svelte/transition"
</script>
<div in:fly class="container-wrap">
	<slot></slot>
</div>
<style>
	.container-wrap {
		padding-top: 12px;
	}

	.container-wrap {
		margin: 0 auto;
		max-width: 1280px;
		width: 98%;
	}

	@media only screen and (min-width: 601px) {
		.container-wrap {
			width: 95%;
		}
	}

	@media only screen and (min-width: 993px) {
		.container-wrap {
			width: 90%;
		}
	}
</style>