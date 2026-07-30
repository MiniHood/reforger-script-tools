#ifdef WORKBENCH
class RST_WorkbenchOpenResourceRequest : JsonApiStruct
{
	string resourcePath;

	void RST_WorkbenchOpenResourceRequest()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenResourceResponse : JsonApiStruct
{
	string resourcePath;
	bool opened;
	string status;

	void RST_WorkbenchOpenResourceResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchOpenResource : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchOpenResourceRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchOpenResourceRequest typedRequest = RST_WorkbenchOpenResourceRequest.Cast(request);
		RST_WorkbenchOpenResourceResponse response = new RST_WorkbenchOpenResourceResponse();
		if (!typedRequest || typedRequest.resourcePath == string.Empty)
		{
			response.status = "resource-path-required";
			return response;
		}
		response.resourcePath = typedRequest.resourcePath;
		string workbenchPath = typedRequest.resourcePath;
		if (workbenchPath.IndexOf("{") == 0)
		{
			int guidEnd = workbenchPath.IndexOf("}");
			if (guidEnd >= 0)
				workbenchPath = workbenchPath.Substring(guidEnd + 1, workbenchPath.Length() - guidEnd - 1);
		}
		if (workbenchPath.Length() > 3 && workbenchPath.Substring(workbenchPath.Length() - 3, 3) == ".st")
		{
			Workbench.OpenModule(LocalizationEditor);
			LocalizationEditor stringEditor = Workbench.GetModule(LocalizationEditor);
			if (stringEditor)
				response.opened = stringEditor.SetOpenedResource(typedRequest.resourcePath) && stringEditor.GetTable() != null;
		}
		else
		{
			response.opened = Workbench.OpenResource(workbenchPath);
		}
		if (response.opened)
			response.status = "opened";
		else
			response.status = "open-failed";
		PrintFormat("RST open resource: path=%1 opened=%2 status=%3", response.resourcePath, response.opened, response.status);
		return response;
	}
}
#endif
