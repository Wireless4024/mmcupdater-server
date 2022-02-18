<script lang="ts">
	import {onDestroy, onMount} from "svelte";
	import {
		Alert,
		Button,
		Col,
		Container,
		FormGroup,
		FormText,
		Input,
		Label,
		Row,
		TabContent,
		TabPane
	}                           from "sveltestrap";

	let server_status = 'UNKNOWN'
	let status_color: string

	let refresh_interval

	const SERVER_URL = 'http://localhost:8888'

	function refresh_status() {
		fetch(`${SERVER_URL}/status`, {}).then(it => it.text()).then(it => server_status = it).catch(_ => server_status = 'UNKNOWN')
	}

	type ServerInfo = {
		mc_version: string
		forge_version: string
	}

	type ModInfo = {
		name: string
		version: string
		file_name: string
	}

	type ConfigJson = {
		config: ServerInfo
		mods: ModInfo[]
	}

	let server_config: ConfigJson

	let admin_mode = false

	async function loadConfig() {
		server_config = await fetch(`${SERVER_URL}/config.json`, {}).then(it => it.json()).catch(_ => {
			return {
				config: {},
				mods  : {}
			}
		})
	}

	let auth_code: string = localStorage.getItem("auth_code") || ""

	async function check_admin(code?: string) {
		try {
			auth_code = code ? code : prompt("enter auth env from .env file")
			if (!auth_code?.length) return false
			await fetch(`${SERVER_URL}/update`, {
				headers: {
					Authorization: auth_code
				},
				method : 'POST',
				body   : new FormData()
			})
			localStorage.setItem("auth_code", auth_code)
			return admin_mode = true
		} catch (e) {
			localStorage.removeItem("auth_code")
			auth_code = ''
			return admin_mode = false
		}
	}

	onMount(async function () {
		refresh_interval = setInterval(refresh_status, 5000)
		refresh_status()
		await loadConfig()
		if (auth_code) {
			await check_admin(auth_code)
		}
	})

	onDestroy(function () {
		clearInterval(refresh_interval)
	})

	$:{
		switch (server_status) {
			case 'STARTING':
				status_color = 'info'
				break
			case 'RUNNING':
				status_color = 'success'
				break
			case 'CRASHED':
				status_color = 'danger'
				break
			case 'STOPPED':
				status_color = 'secondary'
				break
			default:
				status_color = 'light'
		}
	}

	async function restart() {
		if (auth_code) {
			if (!(server_status == 'STOPPED' || server_status == 'CRASHED') && !confirm("do you really want to restart")) return
			await fetch(`${SERVER_URL}/restart`, {
				headers: {
					Authorization: auth_code
				}
			})
		}
	}

	async function handle_file(ev) {
		const target: HTMLInputElement = ev.target
		const files = target.files
		const file: File = files?.[0]
		if (file) {
			if (!file.name.endsWith(".jar")) {
				alert("only accept .jar file!")
			} else {
				const body = new FormData();
				body.append('file', file)
				const mod_info: ModInfo = await fetch(`${SERVER_URL}/update`, {
					headers: {
						Authorization: auth_code
					},
					method : 'POST',
					body
				}).then(it => it.json())

				await loadConfig()
				alert(`${mod_info.name} has been updated to version ${mod_info.version}`)
			}
		}

		target.value = ''
	}
</script>
<svelte:head>
	<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bootstrap@5.1.0/dist/css/bootstrap.min.css">
</svelte:head>
<main>
	<Container lg>
		<Row>
			<Col>
				<Alert color={status_color}>
					Server status : {server_status} |
					{#if !admin_mode}
						<a on:click={()=>check_admin()} href="javascript:void 0">turn on admin mode?</a>
					{:else}
						<a on:click={restart} href="javascript:void 0">restart</a>
					{/if}
				</Alert>
			</Col>
		</Row>
		<TabContent>
			<TabPane tabId="overview" tab="Overview" active>
				<h2>Mods</h2>
				{#if server_config?.mods}
					{#each server_config?.mods as mod}
						<Row>
							<Col md="5">
								{mod.name}
							</Col>
							<Col md="5">
								{mod.version}
							</Col>
							<Col md="2">
								<Button href={mod.file_name.startsWith("http")?mod.file_name:`${SERVER_URL}/mods/${mod.file_name}`}>
									Download
								</Button>
							</Col>
						</Row>
					{/each}
				{/if}
			</TabPane>
			{#if admin_mode}
				<TabPane tabId="admin_manager" tab="Mod Manager">
					<h2>Mod Manager</h2>
					<Row>
						<Col>
							<FormGroup>
								<Label for="exampleFile">Update mod</Label>
								<Input on:change={handle_file} type="file" name="file" id="exampleFile"/>
								<FormText color="muted">
									Drop jar file here.
								</FormText>
							</FormGroup>
						</Col>
					</Row>
				</TabPane>
			{/if}
		</TabContent>
	</Container>
</main>

<style></style>
