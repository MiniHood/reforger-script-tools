export function tracePerformance<T>(name: string, detail: string, operation: () => T): T {
	void name;
	void detail;
	return operation();
}

export async function tracePerformanceAsync<T>(name: string, detail: string, operation: () => Promise<T>): Promise<T> {
	void name;
	void detail;
	return operation();
}
