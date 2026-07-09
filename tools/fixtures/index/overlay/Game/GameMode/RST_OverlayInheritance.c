// truth-status: speculative; overlay index fixture, not Workbench-confirmed.

class RST_WorkspaceGameModeBase : SCR_BaseGameMode
{
	protected int m_iSharedOverlayValue;
	protected int m_iWorkspaceBaseOnly;

	void OverlayAction()
	{
	}

	void Begin()
	{
	}

	void Begin(string reason)
	{
	}
}

class RST_WorkspaceGameModeChild : RST_WorkspaceGameModeBase
{
	protected int m_iSharedOverlayValue;
	protected int m_iWorkspaceChildOnly;

	void OverlayAction()
	{
	}

	void Begin(int value)
	{
	}
}
