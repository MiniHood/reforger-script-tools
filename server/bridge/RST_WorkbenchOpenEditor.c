#ifdef WORKBENCH
class RST_WorkbenchOpenEditorRequest : JsonApiStruct
{
	string editorId;

	void RST_WorkbenchOpenEditorRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenEditorResponse : JsonApiStruct
{
	string editorId;
	bool opened;
	string status;

	void RST_WorkbenchOpenEditorResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenEditor : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchOpenEditorRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchOpenEditorRequest typedRequest = RST_WorkbenchOpenEditorRequest.Cast(request);
		RST_WorkbenchOpenEditorResponse response = new RST_WorkbenchOpenEditorResponse();
		if (!typedRequest || typedRequest.editorId == string.Empty)
		{
			response.status = "editor-id-required";
			return response;
		}
		response.editorId = typedRequest.editorId;
		switch (typedRequest.editorId)
		{
			case "world":     response.opened = Workbench.OpenModule(WorldEditor);
			break;
			case "animation":     response.opened = Workbench.OpenModule(AnimEditor);
			break;
			case "audio":     response.opened = Workbench.OpenModule(AudioEditor);
			break;
			case "behavior":     response.opened = Workbench.OpenModule(BehaviorEditor);
			break;
			case "localization":     response.opened = Workbench.OpenModule(LocalizationEditor);
			break;
			case "particle":     response.opened = Workbench.OpenModule(ParticleEditor);
			break;
			case "procedural-animation":     response.opened = Workbench.OpenModule(ProcAnimEditor);
			break;
			case "script":     response.opened = Workbench.OpenModule(ScriptEditor);
			break;
			default:     response.status = "unknown-editor";
			PrintFormat("RST open editor rejected: %1", typedRequest.editorId);
			return response;
		}
		if (response.opened)
			response.status = "opened";
		else
			response.status = "open-failed";
		PrintFormat("RST open editor: id=%1 opened=%2 status=%3", response.editorId, response.opened, response.status);
		return response;
	}
}
#endif
