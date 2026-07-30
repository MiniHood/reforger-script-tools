#ifdef WORKBENCH
class RST_WorkbenchCapabilitiesRequest : JsonApiStruct
{
	void RST_WorkbenchCapabilitiesRequest()
	{
		RegAll();
	}
}
class RST_WorkbenchCapabilitiesResponse : JsonApiStruct
{
	string bridgeVersion;
	int protocolVersion;
	string capabilities;
	void RST_WorkbenchCapabilitiesResponse()
	{
		RegAll();
	}
}
class RST_WorkbenchCapabilities : NetApiHandler
{
	override JsonApiStruct GetRequest()
	{
		return new RST_WorkbenchCapabilitiesRequest();
	}
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchCapabilitiesResponse response = new RST_WorkbenchCapabilitiesResponse();
		response.bridgeVersion = "1.51.0";
		response.protocolVersion = 1;
		response.capabilities = "state;editors;open-resource;play-session;project-context;inspect-resource;world-selection;entity-hierarchy;list-resources;list-entities;layer-state;inspect-entity;set-selection;clear-selection;entity-position;entity-details;create-entity;rename-entity;delete-entity;move-entity;rotate-entity;reparent-entity;duplicate-entity;entity-properties;components;component-properties;reload-action";
		return response;
	}
}
#endif
