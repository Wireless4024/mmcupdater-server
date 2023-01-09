export type RemoteDirEntry = {
	isDir: boolean
	name: string
}

export interface RemoteFs {
	list(path: string): Promise<RemoteDirEntry[]>

	delete(path: string): Promise<boolean>

	upload(path: string, file: File): Promise<boolean>

	get(path: string): Promise<Blob | undefined>

	move(src: string, dst: string): Promise<boolean>
}

type PseudoFile = {
	name: string
	blob?: Blob
}

export class PseudoRemoteFs implements RemoteFs {
	private files: PseudoFile[] = []

	async delete(path: string): Promise<boolean> {
		const f = this.files.findIndex(f => f.name == path);
		if (f !== -1) {
			this.files.splice(f, 1)
			return true
		}
		return false
	}

	async get(path: string): Promise<Blob | undefined> {
		const f = this.files.find(f => f.name == path)
		return f ? f.blob : f
	}

	async list(path: string): Promise<RemoteDirEntry[]> {
		return this.files.filter(f => f.name.startsWith(path)).map(it => ({
			name : it.name.substring(path.length),
			isDir: it.blob != undefined
		}))
	}

	async move(src: string, dst: string): Promise<boolean> {
		return false
	}

	async upload(path: string, file: File): Promise<boolean> {
		await this.delete(path)
		this.files.push({name: path, blob: file})
		return true
	}
}