import type { EnforceParserDiagnostic } from './ast';
import { tokenRangeToParserRange } from './source';
import type { EnforceToken } from './tokens';

export function collectRecoveryDiagnostics(tokens: EnforceToken[]): EnforceParserDiagnostic[] {
	return tokens
		.filter(token => token.unterminated)
		.map(token => ({
			message: token.kind === 'comment' ? 'Unterminated block comment.' : 'Unterminated string literal.',
			range: tokenRangeToParserRange(token),
			severity: 'warning',
		}));
}
