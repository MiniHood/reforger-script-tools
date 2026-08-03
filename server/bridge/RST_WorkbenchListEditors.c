#ifdef WORKBENCH
class RST_WorkbenchListEditorsResponse : JsonApiStruct
{
	string editors;

	void RST_WorkbenchListEditorsResponse()
	{
		RegAll();
	}
}

class RST_WorkbenchListEditors : NetApiHandler
{
	override JsonApiStruct GetResponse(JsonApiStruct request)
	{
		RST_WorkbenchListEditorsResponse response = new RST_WorkbenchListEditorsResponse();
		response.editors = "world|World Editor;animation|Animation Editor;audio|Audio Editor;behavior|Behavior Editor;localization|String Editor;particle|Particle Editor;procedural-animation|Procedural Animation Editor;script|Script Editor";
		return response;
	}
}
#endif
