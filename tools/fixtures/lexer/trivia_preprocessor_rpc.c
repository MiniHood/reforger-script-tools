// Fixture truth: game-data-derived for comments, preprocessor, and Rpl-looking syntax; not Workbench-confirmed.
#define SCR_FEATURE_FLAG
#ifdef SCR_FEATURE_FLAG

/*!
	Block documentation comment.
*/
class SCR_RpcExample : Managed
{
	// Regular line comment.
	[RplRpc(RplChannel.Reliable)]
	protected void Rpc_RequestSync(RplId id)
	{
		string escaped = "quote: \" and slash: \\";
	}
}

#endif
