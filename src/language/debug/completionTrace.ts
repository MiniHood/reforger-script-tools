export type CompletionTraceKind = 'provider' | 'typed-change' | 'suggest-trigger';

interface CompletionTraceEntry {
	timestamp: string;
	kind: CompletionTraceKind;
	fields: Record<string, string | number | boolean | undefined>;
}

const maxTraceEntries = 200;
const completionTraceEntries: CompletionTraceEntry[] = [];

export function recordCompletionTrace(kind: CompletionTraceKind, fields: Record<string, string | number | boolean | undefined>): void {
	completionTraceEntries.push({
		timestamp: new Date().toISOString(),
		kind,
		fields,
	});
	if (completionTraceEntries.length > maxTraceEntries) {
		completionTraceEntries.splice(0, completionTraceEntries.length - maxTraceEntries);
	}
}

export function formatCompletionTrace(): string {
	if (completionTraceEntries.length === 0) {
		return 'traceEntries=0';
	}
	return [
		`traceEntries=${completionTraceEntries.length}`,
		...completionTraceEntries.map((entry, index) => [
			`${index + 1}.`,
			`timestamp=${entry.timestamp}`,
			`kind=${entry.kind}`,
			...Object.entries(entry.fields)
				.filter(([, value]) => value !== undefined)
				.map(([key, value]) => `${key}=${JSON.stringify(value)}`),
		].join(' ')),
	].join('\n');
}
