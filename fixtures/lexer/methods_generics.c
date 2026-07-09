//! Fixture truth: game-data-derived from common method and generic type shapes; not Workbench-confirmed.
class SCR_GenericExample : Managed
{
	protected ref array<ref SCR_GenericExample> m_aChildren;
	protected ref map<typename, ref array<string>> m_mLogBuffer = new map<typename, ref array<string>>();

	private static void Configure(notnull array<ref SCR_GenericExample> children, bool enabled = false)
	{
	}

	proto external void RpcDo(int value = 10, string label = "default");
}
