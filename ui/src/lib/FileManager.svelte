<script lang="ts">
	import {onMount}        from "svelte"
	import {_}              from "svelte-i18n"
	import {
		Breadcrumb,
		BreadcrumbItem,
		Button,
		Table
	}                       from "sveltestrap"
	import type {
		RemoteDirEntry,
		RemoteFs
	}                       from "../util/remote_fs"
	import {PseudoRemoteFs} from "../util/remote_fs"

	export let fs: RemoteFs = new PseudoRemoteFs()
	let parent: string[] = []
	let current: string = "/"
	let contents: RemoteDirEntry[] = []

	onMount(async function () {

	})

	$:setTimeout(async () => contents = await fs.list(current))

	function go_up() {

	}
</script>
<div class="d-inline">Remote file at</div>
<div class="d-inline-block fw-bold">
	<Breadcrumb>
		{#each parent as p}
			<BreadcrumbItem>
				<a href="javascript:null">{p}</a>
			</BreadcrumbItem>
		{/each}
		<BreadcrumbItem active>{current}</BreadcrumbItem>
	</Breadcrumb>
</div>
<Table hover>
	<thead>
	<tr>
		<th>Name</th>
		<th>Filename</th>
		<th>{$_("instance.action._")}</th>
	</tr>
	</thead>
	<tbody>
	<tr id="btn-top">
		<th scope="row">ABCD</th>
		<td>ABCD-1.1.jar</td>
		<td>
			<Button color="secondary">Disable</Button>
			<Button color="danger">Delete</Button>
		</td>
	</tr>
	</tbody>
</Table>