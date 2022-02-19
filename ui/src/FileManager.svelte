<script lang="ts">
	import {onMount}                                              from "svelte";
	import {Breadcrumb, BreadcrumbItem, Button, Offcanvas, Table} from "sveltestrap";

	let path = '/'
	export let SERVER_URL: string
	export let auth_code: string
	let contents: ServerFile[] = []

	type ServerFile = {
		name: string
		dir?: string
		isDir?: boolean
	}

	async function loadContents() {
		const load_path = path.endsWith('/') ? path.substring(0, path.length - 1) : path
		let files: any = await fetch(`${SERVER_URL}/mc/file`, {
			headers: {
				Authorization : auth_code,
				'Content-Type': 'application/json'
			},
			method : 'POST',
			body   : JSON.stringify({
				path: load_path
			})
		}).then(it => it.json())
		files.sort()

		files = files.map(it => {
			let names = it.split('/')
			while (names.length && !names[names.length - 1]) names.pop()
			let name = names[names.length - 1]
			names.pop()
			let dir = names.join("/")
			return {name, dir, isDir: it.endsWith("/")}
		})
		contents = files
	}

	async function loadFileContent(filename: string) {
		if (!filename) return ""
		return await fetch(`${SERVER_URL}/mc/file/` + filename, {
			headers: {
				Authorization: auth_code
			}
		}).then(it => it.text())
	}

	onMount(function () {
		loadContents()
	})

	function deleteFile() {

	}

	async function putFile(filename: ServerFile, content: BlobPart[]): Promise<Response> {
		await uploadFile(filename, new File(content, filename.name, {type: 'text/plain'}))
	}

	async function uploadFile(file: ServerFile, content: File): Promise<Response> {
		let form = new FormData()
		form.append(file.dir ? (file.dir + '/' + file.name) : file.name, content)
		await fetch(`${SERVER_URL}/mc/file`, {
			headers: {
				Authorization: auth_code
			},
			method : 'PUT',
			body   : form
		})
	}

	$:{
		if (path) {
			loadContents()
			const _dirs = []
			for (let content of path.split('/')) {
				if (content) {
					_dirs.push(content)
				}
			}
			dirs = _dirs
		}
	}

	let dirs: string[]

	let selected_file: ServerFile

	function loadCurrentFileContent() {
		loadFileContent(selected_file.dir ? (selected_file.dir + '/' + selected_file.name) : selected_file.name)
			.then(it => file_content = it)
	}

	function rm(file: ServerFile) {
		if (!confirm("Do you want to remove this file?")) return
		if (file.isDir) {
			if (!confirm("you are about to delete directory '" + (file.name) + "' and its content. this operation can't be undone!")) return;
		}
		fetch(`${SERVER_URL}/mc/file`, {
			headers: {
				Authorization : auth_code,
				'Content-Type': 'application/json'
			},
			method : 'DELETE',
			body   : JSON.stringify({
				paths: [file.dir ? (file.dir + '/' + file.name) : file.name]
			})
		}).then(_ => loadContents())
	}

	let copen
	$:{
		copen = selected_file != null
		if (selected_file) {
			loadCurrentFileContent()
		}
	}

	let file_content: string = ""

	function saveCurrentFile() {
		putFile(selected_file, [file_content])
			.then(_ => {
				alert("File saved successfully")
				copen = false
			})
			.catch(_ => alert("Error while saving file"))
	}

	async function onDrop(ev) {
		const data: DataTransfer = ev.dataTransfer
		if (data) {
			let files = data.files
			for (let file of files) {
				const file_data: ServerFile = {
					dir : path.startsWith('/') ? path.substring(1) : path,
					name: file.name
				}

				await uploadFile(file_data, file)
			}
			await loadContents()
		}
	}
</script>
<div on:drop|preventDefault={onDrop} ondragover="return false" style="height:100%;width:100%;min-height:50vh">
	<Breadcrumb>
		<BreadcrumbItem active={dirs.length==0}>
			<a href={"javascript:void 0"} on:click={()=>path = '/'}>root</a>
		</BreadcrumbItem>
		{#each dirs as p,idx}
			{#if idx != dirs.length - 1}
				<BreadcrumbItem>
					<a href={"javascript:void 0"} on:click={()=>path = dirs.slice(0,idx+1).join("/")}>{p}</a>
				</BreadcrumbItem>
			{:else}
				<BreadcrumbItem active={true}>{p}</BreadcrumbItem>
			{/if}

		{/each}
	</Breadcrumb>
	<Table>
		<thead>
		<tr>
			<th>File name</th>
			<th style="text-align:right">Action</th>
		</tr>
		</thead>
		<tbody>
		{#each contents as file}
			<tr>
				<th>
					{#if file.isDir}
						<a href={"javascript:void 0"}
						   on:click={()=>path =file.dir?file.dir+'/'+ file.name:file.name}>{file.name}</a>
					{:else}
						<div>
							{file.name}
						</div>
					{/if}
				</th>
				{#if !file.isDir}
					<td style="text-align:right">
						{#if !(/.+\.jar$/.test(file.name))}
							<Button on:click={()=>selected_file=file}>Edit</Button>
						{/if}
						<Button on:click={()=>rm(file)}>Delete</Button>
					</td>
				{:else}
					<td style="text-align:right">
						<Button on:click={()=>rm(file)}>Delete</Button>
					</td>
				{/if}
			</tr>
		{/each}
		</tbody>

	</Table>
	{#if selected_file}
		<Offcanvas bind:isOpen={copen} toggle={()=>copen=!copen} placement="bottom" style="min-height:80vh">
			<h3 slot="header">
				Editing '{selected_file.name}' <a href={"javascript:void 0"} on:click={saveCurrentFile}>save</a>
			</h3>
			<textarea style="height: 98%;width:100%" bind:value={file_content}
			          placeholder="file is empty"></textarea>
		</Offcanvas>
	{/if}
</div>