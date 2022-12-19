<script lang="ts">
	import {onMount}     from "svelte"
	import {_}           from "svelte-i18n"
	import {replace}     from "svelte-spa-router"
	import {
		Alert,
		Button,
		FormGroup,
		Input,
		Label
	}                    from "sveltestrap"
	import {USER}        from "../api/shared_state"
	import {
		get_user,
		login
	}                    from "../api/user"
	import ContainerForm from "../lib/ContainerForm.svelte"

	let username = ''
	let password = ''

	let form: HTMLFormElement

	let msg = ''

	function do_login(ev: SubmitEvent) {
		if (form.checkValidity()) {
			ev.stopPropagation()
			ev.stopImmediatePropagation()
			ev.preventDefault()

			setTimeout(async function () {
				const m = await login(username, password)
				if (m == 'auth.success') {
					let user = await get_user()
					USER.set(user)
					await replace("/")
				} else {
					msg = m
				}
			})
		}
	}

	onMount(function () {
		if ($USER) replace("/")
	})
</script>
<ContainerForm>
	<form bind:this={form}>
		<FormGroup>
			<Label for="username">{$_("form.username")}</Label>
			<Input type="text"
			       name="username"
			       id="username"
			       placeholder="Username"
			       autocomplete="username"
			       bind:value={username}
			       required
			/>
		</FormGroup>
		<FormGroup>
			<Label for="password">{$_("form.password")}</Label>
			<Input type="password"
			       name="password"
			       id="password"
			       placeholder="Password"
			       autocomplete="current-password"
			       bind:value={password}
			       required
			/>
		</FormGroup>
		{#if msg}
			<Alert color="danger">
				{$_(msg)}
			</Alert>
		{/if}
		<div style="text-align:center">
			<Button outline dark on:click={do_login}>{$_("form.login")}</Button>
		</div>
	</form>
</ContainerForm>