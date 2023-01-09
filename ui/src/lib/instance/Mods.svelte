<script lang="ts">
	import {_} from "svelte-i18n"
	import {
		Button,
		Table
	}          from "sveltestrap"

	function on_drop(ev: DragEvent) {
		console.log(ev)
	}
</script>
<div on:drop|preventDefault={on_drop} on:dragover|preventDefault={on_drop}>
	<Table hover>
		<thead>
		<tr>
			<th>Name</th>
			<th>Version</th>
			<th>Filename</th>
			<th>{$_("instance.action._")}</th>
		</tr>
		</thead>
		<tbody>
		<tr>
			<th scope="row">ABCD</th>
			<td>1.1</td>
			<td>ABCD-1.1.jar</td>
			<td>
				<Button color="secondary">Disable</Button>
				<Button color="danger">Delete</Button>
			</td>
		</tr>
		</tbody>
	</Table>
</div>