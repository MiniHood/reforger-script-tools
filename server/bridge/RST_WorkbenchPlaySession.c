#ifdef WORKBENCH
class RST_WorkbenchPlaySessionRequest : JsonApiStruct
{
	bool start;
	bool debugMode;
	bool fullScreen;
	void RST_WorkbenchPlaySessionRequest()
	{
		RegAll();
	}
}
class RST_WorkbenchPlaySessionResponse : JsonApiStruct
{
	bool accepted;
	string status;
	void RST_WorkbenchPlaySessionResponse()
	{
		RegAll();
	}
}
class RST_WorkbenchPlaySession : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchPlaySessionRequest();
	}
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchPlaySessionRequest typedRequest = RST_WorkbenchPlaySessionRequest.Cast(request);
		RST_WorkbenchPlaySessionResponse response = new RST_WorkbenchPlaySessionResponse();
		WorldEditor worldEditor = Workbench.GetModule(WorldEditor);
		if (!worldEditor)
		{
			response.status = "world-editor-unavailable";
			return response;
		}
		if (typedRequest.start)
		{
			if (!worldEditor.GetApi())
			{
				response.status = "play-session-already-running";
				return response;
			}
			worldEditor.SwitchToGameMode(typedRequest.debugMode, typedRequest.fullScreen);
			response.status = "play-started";
		}
		else
		{
			worldEditor.SwitchToEditMode();
			response.status = "play-stopped";
		}
		response.accepted = true;
		return response;
	}
}
#endif
