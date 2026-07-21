## ADDED Requirements

### Requirement: Native unbraced if indentation
The extension SHALL declare a native VS Code indentation rule that indents
only the immediately following line after a complete standalone unbraced
Enfusion `if (...)`, `else if (...)`, or `else` header. The rule SHALL not
insert braces or inspect the subsequent statement.

#### Scenario: Enter after a complete if header
- **WHEN** the user presses Enter after `if (condition)` with no trailing body
- **THEN** VS Code places the next line one indentation level deeper without a
  language-client edit

#### Scenario: Enter after a completed body line
- **WHEN** the user presses Enter after the immediately following body line
- **THEN** VS Code returns the new line to the header's normal indentation
  according to its native indentation rules

#### Scenario: Ineligible header text
- **WHEN** a line is incomplete, has a trailing body, brace, semicolon, or
  comment, or is in a non-header context
- **THEN** the extension does not apply the unbraced-if indentation rule

### Requirement: No deferred scope correction
The extension SHALL NOT use a post-document-change language-client request to
outdent a line or move the caret for unbraced control-body indentation.

#### Scenario: Native Enter completion
- **WHEN** a user presses Enter after an eligible if-family header or body
- **THEN** no deferred scope-layout edit is applied after VS Code renders the
  native Enter result

### Requirement: Separate semicolon assistance
The extension SHALL keep semicolon-on-Enter classification independent from
native control indentation.

#### Scenario: Body content is irrelevant
- **WHEN** the one following statement is a call, assignment, declaration,
  return, nested statement, or another valid statement shape
- **THEN** native control indentation does not classify that statement to
  decide the following line's indentation
