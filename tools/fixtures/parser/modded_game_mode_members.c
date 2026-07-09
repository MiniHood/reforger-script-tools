// Fixture truth: game-data-derived composition from SCR_BaseGameMode.c and real modded-class syntax shapes; not Workbench-confirmed in this repo.
void SCR_BaseGameMode_PlayerIdAndEntity(int playerId, IEntity player);
typedef func SCR_BaseGameMode_PlayerIdAndEntity;

typedef ScriptInvokerBase<SCR_BaseGameMode_PlayerIdAndEntity> SCR_PlayerEntityInvoker;

modded class SCR_BaseGameMode
{
	const static string WB_GAME_MODE_CATEGORY = "Game Mode";

	protected ref ScriptInvoker Event_OnGameStart = new ScriptInvoker();
	protected ref ScriptInvokerBase<SCR_BaseGameMode_PlayerIdAndEntity> m_OnPlayerSpawned = new ScriptInvokerBase<SCR_BaseGameMode_PlayerIdAndEntity>();
	protected ref array<ref SCR_BaseGameModeComponent> m_aComponents = {};
	protected ref map<typename, ref array<string>> m_mDebugTags = new map<typename, ref array<string>>();

	[Attribute("1", uiwidget: UIWidgets.CheckBox, "When true, allows players to freely swap their faction after initial assignment.", category: WB_GAME_MODE_CATEGORY)]
	protected bool m_bAllowFactionChange;

	override void EOnInit(IEntity owner)
	{
		super.EOnInit(owner);
	}

	array<SCR_BaseGameModeComponent> GetComponentsByType(typename componentType, out int foundCount)
	{
		array<SCR_BaseGameModeComponent> components = {};
		foundCount = 0;
		return components;
	}
};
