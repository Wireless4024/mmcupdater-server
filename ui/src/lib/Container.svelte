<div class="container-wrap" on:click>
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