# [Scripting: Performance](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Performance)

Performance is an important aspect of scripting, as poor performance can tank the whole gaming experience, if not the game itself.

* What is tanking performances
  + Crazy requests
  + Memory saturation
  + Redundant code operations
* How to benchmark/identify what slows down things
  + What is acceptable?
  + What is not?

## Performance Enemies

Many if not all of the usual performance issues can see some of these questions asked:

* is it *needed?*
* is it *needed there?*
* is it *needed that much?*
* is it *needed immediately?*
* is it *needed that frequently?*

and the respective solutions to positive replies to these questions would be:

* don't do it: quite straightforward, can mean to remove the code or to replace it with a simpler code
* don't do it *there*: the concerned class might be dealing with responsibilities that are out of its scope
* don't do it *that much*: the structure can be rethought in order to have fewer operations to be executed
* don't do it *in one frame*: the calculation can be spread over multiple frames and result delayed and cached
* don't do it *that frequently*: the code can be set to run every X seconds, result being delayed and cached until the next iteration

### Not Needed

#### Problem

Partially unneeded calculation is done.

#### Solution

* Remove the unneeded code
* Split the code to only do needed processing in each case

### Misplaced

#### Problem

A calculation/storage is done on every instance whereas calculation could be centralised.

#### Solution

Move the responsibility and the data to a dedicated class (**S** from [SOLID](https://en.wikipedia.org/wiki/SOLID), **S**ingle **R**esponsibility **P**rinciple).

#### Example

| Original | Optimised |
| --- | --- |
| ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref PotentialTarget> m_aBigObjectList = { /* ... */ }; 	protected array<ref PotentialTarget> GetTargets() 	{ 		array<ref PotentialTarget> result = {}; 		foreach (PotentialTarget object : m_aBigObjectList) 		{ 			if (object.IsEnemyTo(this.GetFaction())) 			result.Insert(object); 		} 		return result; 	} } ``` | ENFORCECODEMARKER   ``` class EnemyManager { 	protected ref map<Faction, ref array<ref PotentialTarget>> m_mTargetMap; 	void EnemyManager(array<ref PotentialTarget> potentialTargets) 	{ 		m_mTargetMap = new map<Faction, ref array<ref PotentialTarget>>(); 		array<ref PotentialTarget> factionTargets; 		foreach (Faction faction : Faction.GetAllFactions()) 		{ 			factionTargets = {}; 			foreach (PotentialTarget potentialTarget : potentialTargets) 			{ 				if (potentialTarget.IsEnemyTo(faction)) 				factionTargets.Insert(potentialTarget); 			} 			m_mTargetMap.Set(faction, factionTargets); 		} 	} 	array<ref PotentialTarget> GetTargets(Faction faction) 	{ 		return m_mTargetMap.Get(faction); 	} } class PerformanceExample : IEntity { 	protected ref EnemyManager m_EnemyManager; 	void PerformanceExample(notnull EnemyManager enemyManager) 	{ 		m_EnemyManager = enemyManager; 	} 	protected array<ref PotentialTarget> GetTargets() 	{ 		return m_EnemyManager.GetTargets(this.GetFaction()); 	} } ``` |

### Ill-Conceived

#### Problem

A big array of data is processed at once. Going through all the data is not needed.

#### Solution

Stop as soon as an acceptable result has been obtained.

#### Example

| Original | Optimised |
| --- | --- |
| ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref AliveOrNotObject> m_aBigObjectList = { /* ... */ }; 	protected int GetNumberOfAliveObjects() 	{ 		int result; 		foreach (AliveOrNotObject object : m_aBigObjectList) 		{ 			if (object.IsAlive()) 			result++; 		} 		return result; 	} 	bool IsAnObjectAlive() 	{ 		// heavy: goes through all objects to know if -one- is alive 		return GetNumberOfAliveObjects() > 0; 	} } ``` | ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref AliveOrNotObject> m_aBigObjectList = { /* ... */ }; 	bool IsAnObjectAlive() 	{ 		foreach (AliveOrNotObject object : m_aBigObjectList) 		{ 			if (object.IsAlive()) 			return true; // stops on the first occurence 		} 		return false; // goes through all instances to reach false - less likely 	} } ``` |

### Spread

#### Problem

A big array of data is processed at once. The calculation itself is not the issue, the amount of items is.  
**or**  
A heavy calculation is done, and only one such operation should happen per frame.

#### Solution

Spread the calculation over multiple frames.

#### Example 1

Array size issue:

| Original | Optimised |
| --- | --- |
| ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref AliveOrNotObject> m_aBigObjectList = { /* ... */ }; 	void EOnInit(IEntity owner) 	{ 		GetGame().GetCallqueue().CallLater(PrintResult, 250, true); 	} 	int GetNumberOfAliveObjects() 	{ 		int result = 0; 		foreach (AliveOrNotObject object : m_aBigObjectList) 		{ 			if (object.IsAlive()) 			result++; 		} 		return result; 	} 	void PrintResult() 	{ 		Print("Alive objects = " + GetNumberOfAliveObjects()); 	} } ``` | ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref AliveOrNotObject> m_aBigObjectList = { /* ... */ }; 	protected ref array<ref AliveOrNotObject> m_aObjectsToBeProcessed = {}; 	protected bool m_bIsCalculating; 	protected int m_iTempResult; 	protected int m_iAliveCountBuffer; 	void EOnInit(IEntity owner) 	{ 		Calculate(); 		GetGame().GetCallqueue().CallLater(PrintResult, 250, true); 	} 	void EOnFrame(IEntity owner, float timeSlice) 	{ 		if (m_bIsCalculating) 		Calculate(); 		// other frame things 		// ... 	} 	void Calculate() 	{ 		int objCount = m_aObjectsToBeProcessed.Count(); 		if (objCount < 1) 		{ 			m_bIsCalculating = true; 			m_iTempResult = 0; 			m_aObjectsToBeProcessed.Copy(m_aBigObjectList); 		} 		// max 10k items slice 		for (int i, maxCount = Math.Min(objCount, 10000); i < maxCount; i++) 		{ 			AliveOrNotObject object = m_aObjectsToBeProcessed[0]; // always @ index 0 			// for we remove the first item later 			if (object.IsAlive()) 			m_iTempResult++; 			m_aObjectsToBeProcessed.Remove(0); 			objCount--; 		} 		if (objCount < 1) 		{ 			m_iAliveCountBuffer = m_iTempResult; 			GetGame().GetCallqueue().CallLater(Calculate, 1000); // restart calculation in 1 second 			m_bIsCalculating = false; 		} 	} 	int GetNumberOfAliveObjects() 	{ 		return m_iAliveCountBuffer; 	} 	void PrintResult() 	{ 		Print("Alive objects = " + GetNumberOfAliveObjects()); 	} } ``` |

#### Example 2

Heavy calculation issue:

| Original | Optimised |
| --- | --- |
| ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref Player> m_aPlayers = { /* ... */ }; 	protected ref map<ref Player, bool> m_mCastResult; 	protected BaseWorld m_World; 	void PerformanceExample() 	{ 		m_mCastResult = new map<ref Player, bool>(); 		m_World = GetGame().GetWorld(); 	} 	void CheckLineOfSight() 	{ 		float result; 		TraceParam traceParam; 		foreach (Player player : m_aPlayers) 		{ 			traceParam = new TraceParam(); 			traceParam.Start = this.Position(); 			traceParam.End = player.Position(); 			// a Trace is a very expensive operation 			result = m_World.TraceMove(traceParam, null); 			m_mCastResult.Set(player, result == 1); 		} 	} } ``` | ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref Player> m_aPlayers = { /* ... */ }; 	protected ref map<ref Player, bool> m_mCastResult; 	protected BaseWorld m_World; 	protected int m_iIndex = -1; 	protected bool m_bIsCalculating; 	void PerformanceExample() 	{ 		m_mCastResult = new map<ref Player, bool>(); 		m_World = GetGame().GetWorld(); 	} 	void EOnFrame(IEntity owner, float timeSlice) 	{ 		if (m_bIsCalculating) 		CheckLineOfSight(); 	} 	void CheckLineOfSight() 	{ 		m_iIndex++; 		m_bIsCalculating = m_iIndex < m_aPlayers.Count(); 		if (!m_bIsCalculating) 		{ 			m_iIndex = -1; 			return; 		} 		Player player = m_aPlayers[m_iIndex]; 		TraceParam traceParam = new TraceParam(); 		traceParam.Start = this.Position(); 		traceParam.End = player.Position(); 		// only one Trace per frame 		float result = m_World.TraceMove(traceParam, null); 		m_mCastResult.Set(player, result == 1); 	} } ``` |

### Immediate Calculation

#### Problem

A heavy operation is done frequently.

#### Solution

Do the heavy operation once, store its result in a member variable and accessible through a getter.

#### Example

| Original | Optimised |
| --- | --- |
| ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref AliveOrNotObject> m_aBigObjectList = { /* ... */ }; 	void EOnInit(IEntity owner) 	{ 		GetGame().GetCallqueue().CallLater(PrintResult, 250, true); 	} 	int GetNumberOfAliveObjects() 	{ 		int result = 0; 		foreach (AliveOrNotObject object : m_aBigObjectList) 		{ 			if (object.IsAlive()) 			result++; 		} 		return result; 	} 	void PrintResult() 	{ 		Print("Alive objects = " + GetNumberOfAliveObjects()); 	} } ``` | ENFORCECODEMARKER   ``` class PerformanceExample : IEntity { 	protected ref array<ref AliveOrNotObject> m_aBigObjectList = { /* ... */ }; 	protected int m_iAliveCountBuffer; 	void EOnInit(IEntity owner) 	{ 		GetGame().GetCallqueue().CallLater(Calculate, 1000, true); // calculate only every second 		GetGame().GetCallqueue().CallLater(PrintResult, 250, true); // display result every 1/4 second 	} 	void Calculate() 	{ 		int result = 0; 		foreach (AliveOrNotObject object : m_aBigObjectList) 		{ 			if (object.IsAlive()) 			result++; 		} 		m_iAliveCountBuffer = result; 	} 	int GetNumberOfAliveObjects() 	{ 		return m_iAliveCountBuffer; 	} 	void PrintResult() 	{ 		Print("Alive objects = " + GetNumberOfAliveObjects()); 	} } ``` |

## Specific Issues

### Big Foreach

#### Problem

Going through one big list of items in one operation.

#### Specific Solution

* Use a smaller list by being more precise
* Continue out of the scope if the item is not viable for the operation

### High-Frequency Scripts

#### Problem

OnEachFrame calculations - are they needed?

#### Specific Solution

* Less resource-hungry calls
* Buffering/caching

### High-Performance Request

#### Problem

Is a 10km raycast really needed?

#### Solution

* Reconsider the need
* Find a smarter solution

## Benchmark

### What Should Be Of Concern

* Non time-critical operations that are made in the same frame
* High RAM usage

### What Should Not Be Of Concern

* Normal operations that are required and that one cannot do without (yes, check if the player is alive before performing action, otherwise the feature breaks)
* One-time required big operation (spawning a base = data loading, no can do anything about it)
