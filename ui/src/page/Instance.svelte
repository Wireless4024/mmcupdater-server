<script lang="ts">
	import {onMount}   from "svelte"
	import {_}         from "svelte-i18n"
	import {replace}   from "svelte-spa-router"
	import {
		Button,
		Card,
		Col,
		Collapse,
		Nav,
		NavLink,
		Row,
		TabContent,
		Table,
		TabPane,
		Tooltip
	}                  from "sveltestrap"
	import {
		get_instances_info,
		type    Instance
	}                  from "../api/instance"
	import Container   from "../lib/Container.svelte"
	import FileManager from "../lib/FileManager.svelte"
	import Mods        from "../lib/instance/Mods.svelte"

	export let params = {name: ""}
	let instance: Instance
	type ServerStatus = 'STOP' | 'RUNNING'
	let status: ServerStatus = 'STOP'
	onMount(async function () {
		if (!params.name) {
			await replace('/')
		}
		await get_instances_info(params.name)
			.then(it => console.log(instance = it))

		if (!instance) await replace('/')
	})

	function mod_type(obj: any): string {
		if (typeof obj == "string")
			return obj
		else for (const k in obj) return `${k} (${obj[k]})`
	}

	let ctx = false
</script>
{#if instance}
	<Container>
		<h1>{instance.name}</h1>
		<hr>
		<Nav>
			{#if status != 'RUNNING'}
				<NavLink href="javascript:null" on:click={()=>(status='RUNNING')}>Start</NavLink>
			{/if}
			{#if status != 'STOP'}
				<NavLink>Restart</NavLink>
				<NavLink href="javascript:null" on:click={()=>(status='STOP')}>Stop</NavLink>
				<NavLink href="javascript:null">Kill</NavLink>
			{/if}
			{#if status != 'RUNNING'}
				<NavLink href="javascript:null">Delete</NavLink>
				<NavLink href="javascript:null">Reinstall</NavLink>
				<NavLink href="javascript:null">Backup</NavLink>
			{/if}
		</Nav>
		<hr>
		<TabContent>
			<TabPane tabId="overview" tab="Overview" active>
				<Row>
					<Col xs="6" md="3" class="fw-bold mb-2">
						{$_("instance.version")}
					</Col>
					<Col xs="6" md="3">
						{instance.version}
					</Col>
					<Col xs="6" md="3" class="fw-bold mb-2">
						{$_("instance.mod_type")}
					</Col>
					<Col xs="6" md="3">
						{mod_type(instance.mod_type)}
					</Col>
					<Col xs="6" md="3" class="fw-bold mb-2">
						{$_("instance.server_file")}
					</Col>
					<Col xs="6" md="3">
						{instance.config.server_file}
					</Col>
					<Col xs="12" md="6" class="fw-bold mb-2">
						<div class="fw-bold" id="args_toggle">{$_("instance.args")} ({$_("action.toggle")})</div>
						<Collapse toggler="#args_toggle">
							<Card body>
								<ul>
									{#each instance.config.args as arg}
										<li>{arg}</li>
									{/each}
								</ul>
							</Card>
						</Collapse>
					</Col>
					<Col xs="6" md="3" class="fw-bold mb-2">
						{$_("instance.java")}
					</Col>
					<Col xs="6" md="3">
						{instance.config.java}
					</Col>
					<Col xs="6" md="3" class="fw-bold mb-2">
						{$_("instance.max_ram")}
					</Col>
					<Col xs="6" md="3" id="ram_alloc">
						{instance.config.max_ram} MiB
					</Col>
					<Col xs="12" md="6" class="fw-bold mb-2">
						<div class="fw-bold" id="jvm_args_toggle">{$_("instance.jvm_args")} ({$_("action.toggle")})
						</div>
						<Collapse toggler="#jvm_args_toggle">
							<Card body>
								<ul>
									{#each instance.config.jvm_args as arg}
										<li>{arg}</li>
									{/each}
								</ul>
							</Card>
						</Collapse>
					</Col>
				</Row>
				<Tooltip target="ram_alloc" placement="top">{$_("instance.ram_over_usage")}</Tooltip>
			</TabPane>
			<TabPane tabId="backup" tab="Backup">
				<Table hover>
					<thead>
					<tr>
						<th>Name</th>
						<th>Date</th>
						<th>{$_("instance.action._")}</th>
					</tr>
					</thead>
					<tbody>
					<tr>
						<th scope="row">ABCD.zip</th>
						<td>1.1</td>
						<td>
							<Button color="primary">Use</Button>
							<Button color="secondary">Download</Button>
							<Button color="danger">Delete</Button>
						</td>
					</tr>
					</tbody>
				</Table>
			</TabPane>
			<TabPane tabId="mods" tab="Mods / Plugins">
				<Mods></Mods>
			</TabPane>
			<TabPane tabId="files" tab="Files">
				<FileManager></FileManager>
			</TabPane>
		</TabContent>
	</Container>
{/if}