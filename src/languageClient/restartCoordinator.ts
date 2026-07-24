export type RestartOperation = () => Promise<void>;

export class RestartCoordinator {
	private active: Promise<void> | undefined;
	private pending: RestartOperation | undefined;

	public run(operation: RestartOperation): Promise<void> {
		if (this.active) {
			this.pending = operation;
			return this.active;
		}

		const active = this.drain(operation).finally(() => {
			if (this.active === active) {
				this.active = undefined;
			}
		});
		this.active = active;
		return active;
	}

	private async drain(initial: RestartOperation): Promise<void> {
		let next: RestartOperation | undefined = initial;
		while (next) {
			this.pending = undefined;
			await next();
			next = this.pending;
		}
	}
}
