// Fixture truth: game-data-derived from common declaration shapes; not Workbench-confirmed.
[BaseContainerProps(configRoot: true, category: "Autotest")]
class SCR_ExampleConfig : Managed
{
	[Attribute(defvalue: "true", UIWidgets.CheckBox)]
	protected bool m_bEnabled;
}

sealed class SCR_ExtendsExample extends SCR_ExampleConfig
{
}

modded class PlayerNameInputController
{
}
