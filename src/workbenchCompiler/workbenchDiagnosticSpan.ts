export interface WorkbenchDiagnosticSpan {
	startCharacter: number;
	endCharacter: number;
}

export interface WorkbenchDiagnosticRange {
	startLine: number;
	startCharacter: number;
	endLine: number;
	endCharacter: number;
}

export interface WorkbenchDiagnosticProjection {
	primaryRange: WorkbenchDiagnosticRange;
	recoveryContextRange?: WorkbenchDiagnosticRange;
}

export function workbenchDiagnosticProjection(
	sourceLines: readonly string[],
	reportedLine: number,
	message: string,
): WorkbenchDiagnosticProjection {
	if (message === "Broken expression (missing ';'?)") {
		const reportedSpan = lineContentSpan(sourceLines[reportedLine] ?? '');
		const precedingLine = precedingContentLine(sourceLines, reportedLine);
		return {
			primaryRange: singleLineRange(reportedLine, reportedSpan),
			...(precedingLine === undefined
				? {}
				: {
					recoveryContextRange: singleLineRange(
						precedingLine,
						lineContentSpan(sourceLines[precedingLine] ?? ''),
					),
				}),
		};
	}

	const span = workbenchDiagnosticSpan(sourceLines[reportedLine] ?? '', message);
	return {
		primaryRange: singleLineRange(reportedLine, span),
	};
}

export function workbenchDiagnosticSpan(
	lineText: string,
	message: string,
): WorkbenchDiagnosticSpan {
	for (const fragment of quotedFragments(message)) {
		const occurrences = exactOccurrences(lineText, fragment);
		if (occurrences.length === 1) {
			return occurrences[0];
		}
	}

	return lineContentSpan(lineText);
}

function singleLineRange(
	line: number,
	span: WorkbenchDiagnosticSpan,
): WorkbenchDiagnosticRange {
	return {
		startLine: line,
		startCharacter: span.startCharacter,
		endLine: line,
		endCharacter: span.endCharacter,
	};
}

function lineContentSpan(lineText: string): WorkbenchDiagnosticSpan {
	const firstContentCharacter = lineText.search(/\S/u);
	if (firstContentCharacter < 0) {
		return {
			startCharacter: lineText.length,
			endCharacter: lineText.length,
		};
	}

	let endCharacter = lineText.length;
	while (endCharacter > firstContentCharacter
		&& /\s/u.test(lineText[endCharacter - 1])) {
		endCharacter -= 1;
	}
	return {
		startCharacter: firstContentCharacter,
		endCharacter,
	};
}

function precedingContentLine(
	sourceLines: readonly string[],
	reportedLine: number,
): number | undefined {
	for (let line = reportedLine - 1; line >= 0; line -= 1) {
		if (/\S/u.test(sourceLines[line] ?? '')) {
			return line;
		}
	}
	return undefined;
}

function quotedFragments(message: string): string[] {
	return [...message.matchAll(/'([^'\r\n]+)'/gu)]
		.map(match => match[1])
		.filter(fragment => /\S/u.test(fragment));
}

function exactOccurrences(
	lineText: string,
	fragment: string,
): WorkbenchDiagnosticSpan[] {
	const occurrences: WorkbenchDiagnosticSpan[] = [];
	const identifier = /^[A-Za-z_][A-Za-z0-9_]*$/u.test(fragment);
	let startCharacter = lineText.indexOf(fragment);
	while (startCharacter >= 0) {
		const endCharacter = startCharacter + fragment.length;
		if (!identifier
			|| (!isIdentifierCharacter(lineText[startCharacter - 1])
				&& !isIdentifierCharacter(lineText[endCharacter]))) {
			occurrences.push({ startCharacter, endCharacter });
		}
		startCharacter = lineText.indexOf(fragment, startCharacter + 1);
	}
	return occurrences;
}

function isIdentifierCharacter(character: string | undefined): boolean {
	return character !== undefined && /[A-Za-z0-9_]/u.test(character);
}
