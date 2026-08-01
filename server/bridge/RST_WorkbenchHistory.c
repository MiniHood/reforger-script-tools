#ifdef WORKBENCH
class RST_WorkbenchHistoryResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string operation;
	string status;
	bool historyAvailable;
	bool changed;

	void RST_WorkbenchHistoryResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchHistoryRequest : JsonApiStruct
{
}

class RST_WorkbenchUndo : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchHistoryRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchHistoryResponse response = new RST_WorkbenchHistoryResponse();
		response.bridgeVersion = "1.52.12";
		response.protocolVersion = 1;
		response.operation = "undo";
		response.status = "native-api-unavailable";
		response.historyAvailable = false;
		response.changed = false;
		return response;
	}
}

class RST_WorkbenchRedo : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchHistoryRequest();
	}

	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchHistoryResponse response = new RST_WorkbenchHistoryResponse();
		response.bridgeVersion = "1.52.12";
		response.protocolVersion = 1;
		response.operation = "redo";
		response.status = "native-api-unavailable";
		response.historyAvailable = false;
		response.changed = false;
		return response;
	}
}
#endif
