export type Callback = (data, socket: SocketWrapper) => void;

export class SocketWrapper {
	private socket: WebSocket;
	private callbacks: { [key: string]: Callback };

	public constructor(url: string) {
		this.socket = new WebSocket(url);
		this.callbacks = {};

		this.socket.onmessage = this.onMessage.bind(this);
	}

	public onOpen(callback: (ev: Event, socket: SocketWrapper) => void): SocketWrapper {
		this.socket.onopen = (ev: Event) => {
			callback(ev, this);
		};
		return this;
	}

	public on(eventName: string, callback: Callback): SocketWrapper {
		this.callbacks[eventName] = callback;
		return this;
	}

	public send(data: { message: string, [key: string]: unknown }): SocketWrapper {
		this.socket.send(JSON.stringify(data));
		return this;
	}

	private onMessage(ev: MessageEvent): void {
		const data = JSON.parse(ev.data);
		const message: string = data.message;
		const callback = this.callbacks[message];
		if (callback) {
			delete data.message;
			callback(data, this);
		}
	}
}
