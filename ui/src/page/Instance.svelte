<script lang="ts">
	import {onMount} from "svelte"
	import {_}       from "svelte-i18n"
	import {replace} from "svelte-spa-router"
	import {
		Button,
		Card,
		CardBody,
		CardHeader,
		CardText,
		CardTitle,
		Col,
		Collapse,
		Row,
		Table
	}                from "sveltestrap"
	import {
		get_instances_info,
		type    Instance
	}                from "../api/instance"
	import Container from "../lib/Container.svelte"

	export let params = {name: ""}
	let instance: Instance

	onMount(async function () {
		if (!params.name) {
			await replace('/')
		}
		await get_instances_info(params.name)
			.then(it => console.log(instance = it))

		if (!instance) await replace('/')
	})

	function mod_type(obj: any): string {
		if (typeof obj == "string") {
			return obj
		} else {
			for (let k in obj) {
				return `${k} (${obj[k]})`
			}
		}
	}

	let jvm_args_open = false
</script>
{#if instance}
	<Container>
		<Card class="mb-3">
			<CardHeader>
				<CardTitle>{params.name} {$_("instance.cfg")}</CardTitle>
			</CardHeader>
			<CardBody>
				<CardText>
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
						<Col xs="6" md="3">
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
				</CardText>
			</CardBody>
		</Card>
		<Card class="mb-3">
			<CardHeader>
				<CardTitle>{$_("instance.action._")}</CardTitle>
			</CardHeader>
			<CardBody>
				<CardText>
					<Button color="primary">Start</Button>
					<Button color="warning">Restart</Button>
					<Button color="secondary">Stop</Button>
					<Button color="danger">Kill</Button>
					<Button color="danger">Delete</Button>
				</CardText>
			</CardBody>
		</Card>
		<Card class="mb-3">
			<CardHeader>
				<CardTitle>Mods / Plugins</CardTitle>
			</CardHeader>
			<CardBody>
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
			</CardBody>
		</Card>
	</Container>
{/if}