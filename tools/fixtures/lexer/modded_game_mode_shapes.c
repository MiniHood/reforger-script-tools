// Fixture truth: game-data-derived composition from SCR_BaseGameMode.c and modded-class shapes; not copied as a real Workbench-confirmed modded class.
void SCR_BaseGameMode_PlayerIdAndEntity(int playerId, IEntity player);
typedef func SCR_BaseGameMode_PlayerIdAndEntity;

void SCR_BaseGameMode_OnPlayerDisconnected(int playerId, KickCauseCode cause = KickCauseCode.NONE, int timeout = -1);
typedef func SCR_BaseGameMode_OnPlayerDisconnected;

typedef ScriptInvokerBase<SCR_BaseGameMode_PlayerIdAndEntity> SCR_PlayerEntityInvoker;

modded class SCR_BaseGameMode
{
	#ifdef ENABLE_DIAG
	#define GAME_MODE_DEBUG
	#endif

	const static string WB_GAME_MODE_CATEGORY = "Game Mode";

	protected ref ScriptInvoker Event_OnGameStart = new ScriptInvoker();
	protected ref ScriptInvokerBase<SCR_BaseGameMode_PlayerIdAndEntity> m_OnPlayerSpawned = new ScriptInvokerBase<SCR_BaseGameMode_PlayerIdAndEntity>();
	protected ref ScriptInvokerBase<SCR_BaseGameMode_OnPlayerDisconnected> m_OnPlayerDisconnected = new ScriptInvokerBase<SCR_BaseGameMode_OnPlayerDisconnected>();
	protected ref array<ref SCR_BaseGameModeComponent> m_aComponents = {};
	protected ref map<typename, ref array<string>> m_mDebugTags = new map<typename, ref array<string>>();

	[Attribute("1", uiwidget: UIWidgets.CheckBox, "When true, allows players to freely swap their faction after initial assignment.", category: WB_GAME_MODE_CATEGORY)]
	protected bool m_bAllowFactionChange;

	[Attribute("30", UIWidgets.Slider, params: "-1 600 1", desc: "Time in seconds after which the mission is reloaded upon completion or -1 to disable it.", category: WB_GAME_MODE_CATEGORY)]
	private float m_fAutoReloadTime;

	[RplProp(onRplName: "OnGameStateChanged")]
	private SCR_EGameModeState m_eGameState = SCR_EGameModeState.PREGAME;

	[RplProp(condition: RplCondition.NoOwner)]
	protected float m_fTimeElapsed;

	override void EOnInit(IEntity owner)
	{
		super.EOnInit(owner);

		foreach (SCR_BaseGameModeComponent component : m_aComponents)
		{
			if (!component)
				continue;

			component.OnGameModeStart();
		}
	}

	protected void RegisterPlayer(int playerId, IEntity player)
	{
		if (playerId < 0 || !player)
			return;

		m_OnPlayerSpawned.Invoke(playerId, player);
	}

	array<SCR_BaseGameModeComponent> GetComponentsByType(typename componentType, out int foundCount)
	{
		array<SCR_BaseGameModeComponent> components = {};
		foundCount = 0;

		foreach (SCR_BaseGameModeComponent component : m_aComponents)
		{
			if (!component || !component.Type().IsInherited(componentType))
				continue;

			components.Insert(component);
			foundCount++;
		}

		return components;
	}

	static SCR_BaseGameMode GetInstance()
	{
		auto gameMode = SCR_BaseGameMode.Cast(GetGame().GetGameMode());
		return gameMode;
	}
};
