import * as assert from 'node:assert';
import {
	workbenchDiagnosticRange,
	workbenchDiagnosticSpan,
} from '../workbenchCompiler/workbenchDiagnosticSpan';

suite('Workbench compiler diagnostic span', () => {
	test('traces a broken expression across blank lines to the preceding source expression', () => {
		const sourceLines = [
			'        int testnum45 = 5',
			'        ',
			'        int testnum = 5;',
		];

		const range = workbenchDiagnosticRange(
			sourceLines,
			2,
			"Broken expression (missing ';'?)",
		);

		assert.deepStrictEqual(range, {
			startLine: 0,
			startCharacter: sourceLines[0].indexOf('int'),
			endLine: 2,
			endCharacter: sourceLines[2].length,
		});
	});

	test('uses the reported line when a broken expression has no preceding source expression', () => {
		const sourceLines = [
			'        ',
			'        int testnum = 5;',
		];

		const range = workbenchDiagnosticRange(
			sourceLines,
			1,
			"Broken expression (missing ';'?)",
		);

		assert.deepStrictEqual(range, {
			startLine: 1,
			startCharacter: sourceLines[1].indexOf('int'),
			endLine: 1,
			endCharacter: sourceLines[1].length,
		});
	});

	test('highlights the named variable instead of leading indentation', () => {
		const lineText = '    int testnum = 5;';

		const span = workbenchDiagnosticSpan(
			lineText,
			"Variable 'testnum' is not used",
		);

		assert.deepStrictEqual(span, {
			startCharacter: lineText.indexOf('testnum'),
			endCharacter: lineText.indexOf('testnum') + 'testnum'.length,
		});
	});

	test('uses all non-whitespace line content when no named subject is present', () => {
		const lineText = '    value = Call();   ';

		const span = workbenchDiagnosticSpan(lineText, 'Statement has no effect');

		assert.strictEqual(
			lineText.slice(span.startCharacter, span.endCharacter),
			'value = Call();',
		);
	});

	test('does not underline whitespace on a whitespace-only reported line', () => {
		const lineText = '    ';

		const span = workbenchDiagnosticSpan(lineText, 'Unexpected end of statement');

		assert.deepStrictEqual(span, {
			startCharacter: lineText.length,
			endCharacter: lineText.length,
		});
	});

	test('falls back to line content when the named subject is ambiguous', () => {
		const lineText = '    result = testnum + testnum;';

		const span = workbenchDiagnosticSpan(
			lineText,
			"Variable 'testnum' is not used",
		);

		assert.strictEqual(
			lineText.slice(span.startCharacter, span.endCharacter),
			'result = testnum + testnum;',
		);
	});

	test('does not match a named identifier inside a longer identifier', () => {
		const lineText = '    int testnumber = 5;';

		const span = workbenchDiagnosticSpan(
			lineText,
			"Variable 'testnum' is not used",
		);

		assert.strictEqual(
			lineText.slice(span.startCharacter, span.endCharacter),
			'int testnumber = 5;',
		);
	});
});
