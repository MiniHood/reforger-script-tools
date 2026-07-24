// Fixture truth: game-data-derived excerpt copied from WorkbenchGame/ScriptEditor/SCR_BasicCodeFormatterPlugin.c in BI script data 1.7.0.54; not Workbench-confirmed in this repo.
#ifdef WORKBENCH

//! \brief A Basic Code Formatter - use Ctrl+Shift+K to trigger.
//! Ctrl+Alt+Shift+K can be used to force processing all the lines of the currently opened file.
//! \see SCR_BasicCodeFormatterForcedPlugin
[WorkbenchPluginAttribute(
	name: "Basic Code Formatter",
	shortcut: "Ctrl+Shift+K",
	wbModules: { "ScriptEditor" },
	awesomeFontCode: 0xF036)]
class SCR_BasicCodeFormatterPlugin : WorkbenchPlugin
{
	/*
		Category: Options
	*/

	[Attribute(defvalue: "0", desc: "Demo mode only logs possible fixes and does not modify the file at all", category: "Options")]
	protected bool m_bDemoMode;

	[Attribute(defvalue: "1", desc: "Display a dialog on execution if unchecked - dialog is always displayed when batch-processing is enabled", category: "Options")]
	protected bool m_bSilentExecution;

	[Attribute(defvalue: "1", desc: "Only format DiffCommand-detected modified lines instead of all of them", category: "Options")]
	protected bool m_bOnlyFormatModifiedLines;

	[Attribute(defvalue: "scripts/xxx/generated/", desc: "Directories in which to avoid formatting (case-insensitive, no wildcards)", category: "Options")]
	protected ref array<string> m_aExcludedDirectories;

	[Attribute(desc: "Spell check config - set it to null to reset its default values", category: "Options")]
	protected ref SCR_BasicCodeFormatterSpellCheckConfig m_SpellCheckConfig;

	[Attribute(defvalue: DEFAULT_DIFF_CMD, desc: "The command line used to generate the diff file\n%1 = absolute target filepath\n%2 = absolute destination filepath", category: "Advanced")]
	protected string m_sDiffCommand;

	protected ref array<ref array<string>> m_aGeneralFormatting_Start;
	protected ref array<ref array<string>> m_aGeneralFormatting_Middle;
	protected ref map<string, string> m_mVariableTypePrefixes;
	protected ref map<string, string> m_mVariableTypePrefixesStart;

	//! mistake-correction map; mistake can contain star(s)
	protected ref map<string, string> m_mForbiddenWords;		// detection only, no replacement due to casing

	protected static const int LINE_NUMBER_LIMIT = 12;			//!< used by JoinLineNumbers to limit the amount of shown line number groups
	protected static const string LINE_NUMBER_RANGE = "%1-%2";	//!< used by JoinLineNumbers to give a line range
	protected static const string LOG_SEPARATOR = "- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -";
	protected static const string BRACKET_OPEN = "{";			// avoids {} Script Editor indent shenanigans
	protected static const string BRACKET_CLOSE = "}";

	protected static const ref array<string> NATIVE_TYPES = {
		"bool", "float", "int", "string", "typename", "vector",
		"FactionKey", "LocalizedString", "ResourceName",
	};

	protected static const ref array<string> VARIABLE_NAME_ENDING = { SCR_StringHelper.SPACE, ",", "=", ";", SCR_StringHelper.SLASH };
}

#endif
