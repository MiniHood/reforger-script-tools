export interface WorkbenchDiagnosticSpan {
	startCharacter: number;
	endCharacter: number;
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
