import type {Writable} from "svelte/store"
import {writable}      from "svelte/store"

type AlertType = 'primary' | 'secondary' | 'success' | 'danger' | 'warning' | 'info' | 'light' | 'dark'

type MessageAlert = {
	id?: number
	message: string
	typ: AlertType
	duration: number
}

class AlertQueue {
	private queue: MessageAlert[] = []
	private poll_queue = true
	private id = 0
	public CURRENT: Writable<MessageAlert> = writable()

	poll(msg?: MessageAlert) {
		let m = msg || this.queue.shift()
		if (!m) {
			this.CURRENT.set(undefined!)
			return
		}
		const id = ++this.id
		this.poll_queue = false
		const self = this
		setTimeout(function () {
			if (id != self.id) return
			self.poll_queue = true
			self.poll()
		}, m.duration * 1000)
		this.CURRENT.set(m)
	}

	push(msg: MessageAlert) {
		if (this.poll_queue) {
			this.poll(msg)
		} else
			this.queue.push(msg)
	}
}

const ALERT_QUEUE = new AlertQueue()
export const ALERT: Writable<MessageAlert> = ALERT_QUEUE.CURRENT

export function notify(message: string, typ: AlertType = "info", duration: number = 30) {
	ALERT_QUEUE.push({message, typ, duration})
}

export function notify_fast(message: string, typ: AlertType = "info", duration: number = 10) {
	ALERT_QUEUE.push({message, typ, duration})
}

export function urgent(message: string, typ: AlertType = "warning", duration: number = 10) {
	ALERT_QUEUE.poll({message, typ, duration})
}

export function consume_alert() {
	ALERT_QUEUE.poll()
}