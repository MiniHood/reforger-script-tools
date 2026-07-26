# [Multiplayer Scripting](https://community.bistudio.com/wiki/Arma_Reforger:Multiplayer_Scripting)

**Multiplayer Scripting** in Enfusion is based on **Replication** - it determines which machine does what in a situation.

When a Prefab entity is created on the server (and has a replication node set to broadcast), it is *replicated* on other network machines for them to see a representation of this object.
This created representation is called a *proxy* – more information below.

## Network

The Arma Reforger network architecture is a classical one with one server to which clients connect. Clients do not communicate between each others, they communicate with the server which redistributes data.

ⓘ

In **single player**, the local machine is considered a **player-hosted server**.
A properly designed multiplayer code should run flawlessly in single player, and ideally *vice versa*.

### Server

The **Server** is the core network machine: it is the central system that receives and redistributes network information.
There is only **one** server per multiplayer game, and the multiplayer game is destroyed if the server loses connection (server role does not get transferred).

ⓘ

A server can be:

* **dedicated**, as in it is a game instance running by itself, without a player behind
* [![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_0.9.7 "Category:Arma Reforger/Version 0.9.7") [0.9.7](/wiki?title=Category:Arma_Reforger/Version_0.9.7&action=edit&redlink=1 "Category:Arma Reforger/Version 0.9.7 (page does not exist)") **player-hosted** (also known as **listen server**), meaning that a player's game instance is also hosting a game.

Distributed server (multiple servers) is **not** a thing in Arma Reforger.

⚠

The following Components and others inheriting from them are **not** instanciated on a Dedicated Server:

* [CameraHandlerComponent](enfusion://ScriptEditor/scripts/Game/generated/Camera/CameraHandlerComponent.c;12)
* [DebugShootComponent](enfusion://ScriptEditor/scripts/Game/generated/Character/DebugShootComponent.c;16)
* [MotorExhaustEffectComponent](enfusion://ScriptEditor/scripts/Game/generated/Vehicle/MotorExhaustEffectComponent.c;16)
* [BaseSoundComponent](enfusion://ScriptEditor/scripts/GameLib/generated/Components/BaseSoundComponent.c;16)

### Client

A **Client** is a machine that is connected to the server. A client is everything else that is **not** the server.
A client can join after a game has started (even leave and join again) depending on the scenario. Such client is known as **JIP**, standing for Join in Progress - see the [Join In Progress](#Join_In_Progress) section below.

ⓘ

A client can be

* a **player client**, the most common understanding of a client: a machine connecting to the server with a player behind it to play the game and control a character

Arma 3's [Headless Client](/wiki/Arma_3:_Headless_Client "Arma 3: Headless Client") system is **not** a thing in Arma Reforger.

## Replication

**Replication** (shortened as **RPL**) is the name of the system used to *replicate* an entity or effect on all the concerned machines. For example, when a grenade explodes the damage is calculated on the server, whereas the visual effect is broadcast (replicated) to all the clients.

**Remote Procedure Call** (shortened as **RPC**) is the system allowing to queue network messages on other machines.

ⓘ

An entity that is to be replicated should **always** have an **RplComponent** script component.

### Preamble

The network structure is based on server-client relationship; Scripting and RPC Replication methods, on the other hand, should be oriented towards **Roles** - code should be written on **authority role** principle, server-client principle should be avoided:

* is the entity present on this machine the **Authority**?
* if not, the local entity is a **Proxy**
  + if so, is it an **Owner** entity?

See the next chapters for further explanations.

### Entity Role

When an entity gets created on a network machine, this entity is the **Authority** and this role is set in stone; it can never get changed or transferred.

If the network machine is the server, the entity's existence is broadcast to other network machines; the authority (server's entity) is represented on each client by locally-created entities named **Proxies** (local representation of the authority's reference).

* For an entity created on the **server:**
  + the server hosts the authority entity
  + the entity is replicated as proxy on other machines
  + ownership can be given and taken by the authority

* For an entity created on a **client:**
  + the client hosts the authority entity
  + the server being unaware of it, the entity is **not** replicated on any network machine

⚠

An entity that has been created on a client is not known by the server and its methods cannot therefore be broadcast to everyone.

#### Authority

The **Authority** is the reference entity.

All mission objects belong to the **server**; the server has almost if not all the authority entities.

ⓘ

A client can host an authority on its machine by having it created locally;
by doing so the entity will be local to this client, no other machine including the server will be aware of it and therefore no replication can ever happen on it.

#### Proxy

A **Proxy** is an entity that *represents* an authority on another machine; it only **receives** entity updates from the authority and cannot send network updates about it - unless the machine has been designated **owner** (see below).

##### Owner

An **Owner** is a special entity role that comes in addition to an authority or a proxy. An owner entity is the entity that has minor rights on the entity such as owner-specific methods.
Only **one** machine can own an entity. The **server** can be an **owner**, and ownership can be transferred by the authority.

A client owner is only a machine with an "elevated" proxy access, being able to use RPC methods about this entity towards the authority (and only towards the authority).

Ownership behaviour example:

* a machinegun car exists on the server; the server is **hosting** the **authority** and is the **owner** of it
* a player enters the car as the driver; the host (the server) gives car's ownership to the player's machine, which becomes the **owner** of the car
* another player enters the car's machinegun; the host (still and always the server) gives machinegun's ownership to this player's machine, which becomes the **owner** of the car's machinegun entity
* the server keeps authority of this car and its machinegun - it is the one that overrides all clients' positions/states
* when said players leave their respective entities, the server takes ownership back

⚠

Giving ownership is **not** giving authority and is unrelated to it!  
Ownership can be changed, authority location is set in stone.

### Replication State

There are two main replication game states which replication distinguishes and which affect some characteristics of replicated items: loadtime state and runtime state. Items inserted into replication while game is loading (ie. at loadtime) are treated as loadtime items, while those inserted once the game is running (ie. at runtime) are treated as runtime items when inserted on server, or local items when inserted on client (as they only exist locally).

ⓘ

Previously, loadtime items were referred to as *static*, while runtime items were referred to as *dynamic*.
However, for several reasons (mainly naming consistency and collision with other systems using the same terminology), they were renamed to hopefully better reflect their nature.

for simplicity, this page will use "items" to refer to both entities, components and in general anything else that might be replicated.

#### Loadtime

**Loadtime** items are generally entity instances placed in world. These are objects that shouldn't be moved around too much (though some degree of movement is allowed) and usually need to be visible from greater distances. Typical examples are buildings or street signs.
Main things to keep in mind:

* They do **not** require prefab for spawning.
* Their insertion **must** be deterministic on the server and clients. The server relies on clients to have the same initial world state after the map has been loaded so it can replicate changes from this initial state as they become relevant for a given client, reducing overall traffic (by sending only changes) and spreading the load over time. Clients will still be able to see things in the distance, even though their state has not been perfectly synchronised.
  + Replication validates that the initial world state matches. That is, RplId and type information of each loadtime item is the same on client as it initially was on the server. When a mismatch is detected, "inconsistent item table" error will appear in log and client will be disconnected with JIP\_ERROR.
* They may be out-of-sync with the server **for a long time** (possibly through the whole game session). The replication scheduler (running server-side) decides when to stream their current state to each client. When this decision is based on proximity, clients will only get the current state streamed in when they get "close enough" for these changes to be relevant. Because of the world size, this may take time or never happen at all.
  + The only exception is complete removal of these items on the server. In that case, removal will be replicated to clients unconditionally.
* As long as the authority exists on the server, the proxy exists on the clients.
  + Streaming in synchronizes state of proxy with authority. Once streamed in, proxy starts receiving state updates and it can send and receive RPCs.  

    ⚠

    Streaming out while the authority exists on the server is currently considered undefined behaviour and must be avoided!

#### Runtime

**Runtime** items generally come from a game system that creates them during the session (A game mode, [Game Master](/wiki/Arma_Reforger:Game_Master "Arma Reforger:Game Master"), etc).
They can move around the map freely and they are usually not visible from far away. Typical examples are vehicles, player characters, collectible items.
Main things to keep in mind:

* They require prefab for spawning.
* They may only be inserted on the server. This is the authority.
* Their proxy may or may not exist on a client.

##### Local Runtime

Local items are items created on client during the session.
They can be used for locally predicted effects of player actions, such as firing from rocket launcher immediately creating a flying rocket on client who fired it, instead of waiting for server-side rocket to be streamed to this client.
Main things to keep in mind:

* They do not require prefab for spawning.
* They may only be inserted **on a client**. This is the authority.
* There are no proxies on the server or other clients.

#### Replication State Override

Replication state override (a.k.a "Rpl State Override") is a [RplComponent](enfusion://ScriptEditor/scripts/Game/generated/Components/RplComponent.c;17) property that allows modifying the behaviour of spawning and insertion process to behave as-if insertion of node hierarchy happened in the specified state.
The currently supported values are:

* None: no override. The state for this node and its descendants at the time of insertion is inherited from its parent node (whether it is Loadtime or Runtime).
* Runtime: The state for this node and its descendants at the time of insertion is overridden to be Runtime.

⚠

With entities and components, child entities spawned during initialisation must be attached to parent as part of spawning process.

A common mistake is to spawn an entity first and then attach it as child of entity that spawned it. However, this results in spawned entity being inserted separately from parent.

If this happens at loadtime and the parent entity had its state overridden to be runtime, it will revert state override back to loadtime, which is currently considered **undefined behaviour** and must be avoided!

### RPC Call

An RPC call is an entity method's call through the network, e.g from the authority to its proxies (on other machines). Specific targets can be defined, such as "all proxies" or "owner only".

### Join In Progress

**Join In Progress** is the fact of having a client that connects to the server while a multiplayer game is already running.

### Streaming

**Streaming** is the system creating/updating/deleting entities that are **relevant** to the local machine.
An entity is **not** always present on all machines. Streaming is the way of saving network and CPU process by only sending **relevant** entities' information.

Consider a server hosting a game with multiple clients, with "player" designating the local player on the local client machine.
A car parked 5 km away from the player is not important to them, so only the server and players near the car have this entity's information - it actually does not exist on the player's machine.
If someone damages that car, the server is informed and this information is transferred to whomever it is relevant - nearby players, not distant players.

If the player gets close to it, the server decides this car becomes relevant to that client and streams it the proxy's information along with its current state (damage, position, etc).

* Streaming in creates a proxy
* Streaming out destroys this proxy
* While the proxy exists, it receives state updates and can send and receive RPCs.

ⓘ

The streaming system is also what is at work during **Join In Progress**.

#### Relevance

Relevance is a value decided authority-side by the engine that sets whether or not a proxy should be streamed to a network machine or not, in order to save network messages and CPU cycles.

A simple example would be a car that is five kilometres away from a player is not **relevant** to that player;
so the proxy on this player's machine is deleted by the replication system automatically, and re-created/synchronised again when it becomes relevant (e.g close-by) again.

#### Special Cases

The authority **always** exists, as it is the reference entity for replication.

The owner entity is **always** present on its machine; it is never unloaded/reloaded depending on relevance unless ownership is taken away.

#### Broadcast

An RPC call with broadcast as a target will send the message on machines that have said entity present on their system; as seen above, a proxy owner entity is guaranteed to exist on

⚠

This is a very important aspect of Replication: if the proxy is not streamed on a machine, the broadcast message will **not** reach this machine, as there is no entity target!

#### Operations Order

⚠

As of v0.9.5: loading order between RplLoad and member variable updates is **not** guaranteed and may be **subject to change**.

**Current** loading order:

| Order | Operation |
| --- | --- |
| 1 | entity hierarchy creation |
| 2 | RplLoad operations |
| 3 | member variable values in ***unguaranteed*** order triggering their respective onRplName methods |

## Code

Entity methods get executed with the Rpc method using their RplRpc attribute to determine the targets.

### Rpc Method

The Rpc method (present in both [GenericEntity](enfusion://ScriptEditor/scripts/GameLib/generated/Entities/GenericEntity.c;15) and [GenericComponent](enfusion://ScriptEditor/scripts/GameLib/generated/Components/GenericComponent.c;12)) is the starting point to trigger a network-friendly code call.

```enforce
Rpc(Rpc_MyMethod, parameter1, parameter2);
```

The engine will make it so that this method will call the provided one where it needs to be, depending on the RplRpc target method attribute.

By convention, methods that can be called through replication are prefixed with the Rpc\_ prefix: e.g Rpc\_MyMethod().

the RpcAsk\_ prefix is used by the owner to ask the authority to do an action (sender = owner, receiver = authority).

the RpcDo\_ prefix is used by the authority to do an action (sender = authority, receiver = owner proxy or all proxies).

#### Usage

```enforce
void Rpc(func method, void p0 = NULL, void p1 = NULL, void p2 = NULL, void p3 = NULL, void p4 = NULL, void p5 = NULL, void p6 = NULL, void p7 = NULL);
```

Network-wise, a method RPC is added to a list of "to do tasks"; it means that they might be sent separately, or all in one network packet. The order of the calls between two machines is however guaranteed.

ⓘ

Calling an RPC method from EOnInit is **not** supported for initialisation order reasons.

### RplRpc Attribute

The RplRpc attribute is to decorate an entity's **method**, allowing the engine to run said method on targeted machines through the Rpc method.

```enforce
[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
```

#### Usage

Here is RplRpc attribute's signature:

```enforce
void RplRpc(RplChannel channel, RplRcver rcver, RplCondition condition = RplCondition.None, string customConditionName = "")
```

| Parameter | Usage |
| --- | --- |
| channel | RplChannel enum, can be one of:  * **Reliable**: the packet is guaranteed to reach its destination * **Unreliable**: the packet is sent but may be overridden depending on the order of arrival   ⚠  Using **Reliable** is more expensive and should be used carefully. **Unreliable** should be used for non-important or very frequent calls (e.g updating positions). |
| rcver (receiver) | RplRcver enum, can be one of:  * ***Server**:* run the method on the authority entity * **Owner**: run the method on the owner entity * **Broadcast**: run the method on proxies only (**not** on the authority)   + only machines that have the proxy streamed on their machine will receive the broadcast, **not** all the machines.   + an immediate **broadcast** may *not* reach the owner entity right after transferring ownership; however an **owner** RPC call is guaranteed to reach it.  | Permissions Table | | | | | | --- | --- | --- | --- | --- | | Checked = RPL call is processed over the network  Unchecked = RPL call is abandoned | | | | | | Action | Authority | | Proxy | | | Server | Client (local entity) | Owner | Client | | RplRcver.Server Send to the authority | Checked (code is run locally) | Checked (code is run locally) | Checked | Unchecked | | RplRcver.Owner Send to the owner | Checked (code is run locally if authority is owner) | Checked (code is run locally) | Checked (code is run locally) | Unchecked | | RplRcver.Broadcast Send to all but authority | Checked (code is run on all proxies and *not* locally) | Unchecked (other machines have no proxy) | Unchecked (code is run locally) | Unchecked (code is run locally) | |
| condition | RplCondition enum, can be one of:  * **None**: run the method in all cases * **OwnerOnly**: run the method on the owner machine only * **NoOwner**: run the method on all the other machines that are not the owner * **Custom**: uses the customConditionName field to use the named function as custom execution condition |
| customConditionName | if condition has been set to Custom, customConditionName must be set to the name of the method to be checked. This method must return a bool. Example:  ENFORCECODEMARKER   ``` class RplClass { 	[RplProp(onRplName: "OnBroadcastValueUpdated", condition: RplCondition.Custom, customConditionName: "RpcConditionMethod")] 	protected int m_iBroadcastValue; 	void OnBroadcastValueUpdated() 	{ 		// this method will only run on proxies if authority's RpcConditionMethod returns true 	} 	[RplRpc(RplChannel.Reliable, RplRcver.Server)] 	void RpcAsk_Method() 	{ 		m_iBroadcastValue = 33; 		Replication.BumpMe(); 	} 	bool RpcConditionMethod() 	{ 		return m_iBroadcastValue < 50; 	} } ``` |

### RplProp Attribute

The RplProp attribute is to decorate an object's **property**, allowing the engine to synchronise values on network machines.

Only a change on the authority **and** notifying the replication system with Replication.BumpMe() method will trigger a broadcast to other machines (see the [BumpMe](#BumpMe) section).

ⓘ

RplProp-decorated properties are automatically synchronised by the [Streaming](#Streaming) system.

```enforce
[RplProp()]
```

#### Usage

Here is RplProp attribute's signature:

```enforce
void RplProp(RplGroup group = RplGroup.Mandatory, string onRplName = "", ScriptCtx ctx = NULL, RplCondition condition = RplCondition.None, string customConditionName = "")
```

| Parameter | Usage |
| --- | --- |
| group | RplGroup enum, can be one of:  * Mandatory * Group\_1 * Group\_2 * Group\_3 * Group\_4 * Group\_5 * Group\_6   ⓘ  These groups are meant to update parts of said entity in the event of a big data structure - more details *later* |
| onRplName | The method name to be called on proxies when the property's value is changed on the authority (and synchronised with Replication.BumpMe() method). ⚠  * This method will **not** be called on the authority * This method **will** be called on proxies' streaming (synchronisation) * only machines that have the proxy streamed on their machine will receive the broadcast, **not** all the machines. |
| ctx | Script context - engine only, unused script-side |
| condition | RplCondition, can be one of:  * **None**: update the property in all cases * **OwnerOnly**: update the property on the owner machine only * **NoOwner**: update the property on all the machines that are not the owner * **Custom**: uses the customConditionName field as custom execution condition |
| customConditionName | if condition has been set to Custom, customConditionName must be set to the name of the method to be checked. This method must return a bool. Example:  ENFORCECODEMARKER   ``` class RplClass { 	[RplProp(onRplName: "OnBroadcastValueUpdated", condition: RplCondition.Custom, customConditionName: "RpcConditionMethod")] 	protected int m_iBroadcastValue; 	void OnBroadcastValueUpdated() 	{ 		// this method will only run on proxies if authority's RpcConditionMethod returns true 	} 	[RplRpc(RplChannel.Reliable, RplRcver.Server)] 	void RpcAsk_Method() 	{ 		m_iBroadcastValue = 33; 		Replication.BumpMe(); 	} 	bool RpcConditionMethod() 	{ 		return m_iBroadcastValue < 50; 	} } ``` |

### RplSave/RplLoad

#### RplSave

**RplSave** is the method called on the authority's side to write custom synchronisation data.

ⓘ

RplSave writes data to a provided ScriptBitWriter instance. Different data types have different sizes, see Values - Primitive Types for their default size.  

Helper methods such as WriteBool or WriteVector exist to save some trouble as syntactical sugar.

Note that using the default size is not always the best/economical option; see below example.

#### RplLoad

**RplLoad** is the method called on the streamed entity after the entity, its components and the whole entities hierarchy to which it belongs has been loaded, if any.

It is called immediately after initialisation but before anything else (EOnFrame, etc).

⚠

RplLoad reads data from a provided ScriptBitReader instance.
Reading from it **must** be done in the same order and with the same number of bits than the writing that happened in RplSave, otherwise data discrepancy may happen and behaviour is not guaranteed.

### Replication Class

The Replication class is a very useful Replication tool, holding the following methods (only the most used ones are listed here).

#### FindId

Return the provided entity's id. *should be an RplId*

If the entity does not have one, it returns Replication.INVALID\_ID.

ⓘ

This method uses a table lookup and therefore should not be used in a tight loop - once found, the id should instead be stored in a variable.

#### FindItem

Return the provided id's entity.

If no entities have the provided id, it returns null.

#### FindOwner

Return the owner machine's network id.

If the entity has no owner id, it returns Replication.INVALID\_IDENTITY.

#### BumpMe

This method tells the Replication system that at least one property has been updated and that the change(s) should be broadcast.

⚠

This method should be used whenever an RplProp property has been changed and is **not** to be randomly called hoping a property has been changed.

```enforce
class RplClass
{
	[RplProp()]
	protected int m_iValue;
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void RpcAsk_Authority_Method(int newValue)
	{
		if (m_iValue != newValue) // not necessary for small changes
		{
			m_iValue = newValue;
			Replication.BumpMe();
		}
	}
}
```

## Network Modding

Defining codec methods is required when a scripted class is marked with c[[RplProp](enfusion://ScriptEditor/scripts/Core/proto/EnNetwork.c;56)()].

### Definitions

Codec
:   A **Cod**er-**Dec**oder - an element that is in charge of encoding/decoding data

Snapshot
:   An "image" of an object's replicated properties - used by the engine to compare with the current status' snapshot and detect a change

Packet
:   (naming may change) An object's transferrable data - unrelated to network packet

### Methods

[EnNetwork.c](enfusion://ScriptEditor/scripts/Core/proto/EnNetwork.c) provides an example class with declarable methods overriding the default behaviour. These methods can be written/overridden in any Enforce Script classes.

ⓘ

See e.g [SCR\_MapMarkerBase](enfusion://ScriptEditor/scripts/Game/Map/Markers/Objects/SCR_MapMarkerBase.c;3) for a real implementation.

```enforce
class CustomClass
{
	//! From snapshot to packet.
	// Takes snapshot and compresses it into packet. Opposite of Decode()
	static void Encode(SSnapSerializerBase snapshot, ScriptCtx ctx, ScriptBitSerializer packet);
	//! From packet to snapshot.
	// Takes packet and decompresses it into snapshot. Opposite of Encode()
	static bool Decode(ScriptBitSerializer packet, ScriptCtx ctx, SSnapSerializerBase snapshot);
	//! Snapshot to snapshot comparison.
	// Compares two snapshots to see whether they are the same or not
	static bool SnapCompare(SSnapSerializerBase lhs, SSnapSerializerBase rhs, ScriptCtx ctx);
	//! Property mem to snapshot comparison.
	// Compares instance and a snapshot to see if any property has changed enough to require a new snapshot
	static bool PropCompare(CustomClass prop, SSnapSerializerBase snapshot, ScriptCtx ctx);
	//! Property mem to snapshot extraction.
	// Extracts relevant properties from an instance of type T into snapshot. Opposite of Inject()
	static bool Extract(CustomClass prop, ScriptCtx ctx, SSnapSerializerBase snapshot);
	//! Snapshot to property memory injection.
	// Injects relevant properties from snapshot into an instance of type T . Opposite of Extract()
	static bool Inject(SSnapSerializerBase snapshot, ScriptCtx ctx, RplTestPropType prop);
}
```

See a full codec implementation

```enforce
class ComplexType
{
	bool m_bValue;
	int m_iValue;
	float m_fValue;
	vector m_vValue;
	// ## Extract/Inject
	// Extracting data from instance into snapshot, and injecting data from snapshot to instance.
	// Snapshot is meant to be fast to work with, so values are left uncompressed to avoid extra work when accessing these values.
	// ## Encode/Decode
	// Encoding snapshot into a packet and decoding snapshot from a packet.
	// Packets need to be as small as possible, so this process tries to reduce the size as much as it can.
	// Knowing what range of values can certain variable have and encoding that range in minimum number of bits required is key.
	// If it is to assume the full range of values is needed, helpers that already implement those for different types can be used.
	static bool Extract(ComplexType instance, ScriptCtx ctx, SSnapSerializerBase snapshot)
	{
		// Fill a snapshot with values from an instance.
		snapshot.SerializeBool(instance.m_bValue);
		snapshot.SerializeInt(instance.m_iValue);
		snapshot.SerializeFloat(instance.m_fValue);
		snapshot.SerializeVector(instance.m_vValue);
		return true;
	}
	static bool Inject(SSnapSerializerBase snapshot, ScriptCtx ctx, ComplexType instance)
	{
		// Fill an instance with values from snapshot.
		snapshot.SerializeBool(instance.m_bValue);
		snapshot.SerializeInt(instance.m_iValue);
		snapshot.SerializeFloat(instance.m_fValue);
		snapshot.SerializeVector(instance.m_vValue);
		return true;
	}
	static void Encode(SSnapSerializerBase snapshot, ScriptCtx ctx, ScriptBitSerializer packet)
	{
		// Read values from snapshot, encode them into smaller representation, then
		// write them into packet.
		snapshot.EncodeBool(packet); // m_bValue
		snapshot.EncodeInt(packet); // m_iValue
		snapshot.EncodeFloat(packet); // m_fValue
		snapshot.EncodeVector(packet); // m_vValue
	}
	static bool Decode(ScriptBitSerializer packet, ScriptCtx ctx, SSnapSerializerBase snapshot)
	{
		// Read values from packet, decode them into their original representation,
		// then write them into snapshot.
		snapshot.DecodeBool(packet); // m_bValue
		snapshot.DecodeInt(packet); // m_iValue
		snapshot.DecodeFloat(packet); // m_fValue
		snapshot.DecodeVector(packet); // m_vValue
		return true;
	}
	static bool SnapCompare(SSnapSerializerBase lhs, SSnapSerializerBase rhs, ScriptCtx ctx)
	{
		// Compare two snapshots and determine whether they are the same.
		return lhs.CompareSnapshots(rhs, 4 + 4 + 4 + 12);
	}
	static bool PropCompare(ComplexType instance, SSnapSerializerBase snapshot, ScriptCtx ctx)
	{
		// Determine whether current values in instance are sufficiently different from
		// an existing snapshot that it's worth creating new one.
		// For float or vector values, it is possible to use some threshold to avoid creating too
		// many snapshots due to tiny changes in these values.
		return snapshot.CompareBool(instance.m_bValue)
		&& snapshot.CompareInt(instance.m_iValue)
		&& snapshot.CompareFloat(instance.m_fValue)
		&& snapshot.CompareVector(instance.m_vValue);
	}
}
```

[↑ Back to spoiler's top](#bikisp6a62f3cfeb679)

### Codec Steps

| On Server | On Client |
| --- | --- |
| * Replication.BumpMe() is used to signal that properties of an item have changed and they need to be replicated to clients * Replication compares replicated properties against the most recent snapshot it has using the PropCompare() codec function. If the codec says snapshot is the same as the current state, the process ends * Replication creates a new snapshot and uses the Extract() codec function to copy values from instance to snapshot * That created snapshot is transmitted to clients as needed.   ⓘ  This process tracks multiple snapshots per item per connected client, join-in-progress, streaming, relevancy, etc. It often uses the SnapCompare() codec function to determine whether two snapshots are the same. When a snapshot is finally being prepared for transmission over network, the Encode() codec function will be used to convert snapshot into compressed form (using as few bits as possible for each value) suitable for network packet. | * When a new packet with the compressed snapshot arrives, it is decompressed using codec function Decode() * Snapshots are compared using the codec function SnapCompare() to determine whether or not changes have occurred * Replication updates properties of the replicated item using the codec function Inject(). |

## Examples

For the sake of the examples, let's assume that:

* the server hosts the ComputerEntity authority entity
* a client hosts the owner entity

### RplRpc

In the following code:

* the protected RpcAsk\_Authority\_Method method is set to be RPC-called on the authority
* the protected RpcDo\_Owner\_Method method is set to be RPC-called on the owner
* the protected RpcDo\_Broadcast\_Method method is set to be RPC-called on every proxy

We assume the public TurnOn method has been triggered **owner-side**, making the following events happen:

* the TurnOn method calls the RpcAsk\_Authority\_Method through RPC; as stated earlier, this method executes on the authority's machine - the server in this case
* the RpcAsk\_Authority\_Method method (running on the authority):
  + prints "authority-side code" in the authority machine's logs
  + calls the PlayMusic method on the authority
  + calls the RpcDo\_Broadcast\_Method method on all the proxies (see below)
  + calls the RpcDo\_Owner\_Method method on the owner's proxy (see below)
* the RpcDo\_Broadcast\_Method method (running on every proxy, including owner's):
  + prints "proxy-side code" in proxy machines' logs
  + calls the PlayMusic method on the proxy
* the RpcDo\_Owner\_Method method (running on the owner's proxy):
  + prints "owner-side code"

The end result:

* all the machines' instances:
  + called the PlayMusic method and played the "OSWelcome" music
* the authority's machine has
  + "authority-side code" in the logs - had the server been the owner too, "owner-side code" would be present as well
* the owner's machine has
  + "proxy-side code" **then** "owner-side code" in the logs (the order is guaranteed)
* all other machines have
  + "proxy-side code" in the logs

```enforce
class ComputerEntity : IEntity
{
	protected bool m_bIsTurnedOn; // this value is edited only on authority's side
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void RpcAsk_Authority_Method(bool turningOn)
	{
		Print("authority-side code");
		if (turningOn == m_bIsTurnedOn) // the authority has authority
		return; // prevent useless network messages
		m_bIsTurnedOn = turningOn;
		PlayMusic(turnOn); // play music on authority
		Rpc(RpcDo_Broadcast_Method, turningOn); // send the music broadcast request
		Rpc(RpcDo_Owner_Method); // run specific code on the owner's entity (that may or may not be the authority)
	}
	[RplRpc(RplChannel.Reliable, RplRcver.Owner)]
	protected void RpcDo_Owner_Method()
	{
		Print("owner-side code");
	}
	[RplRpc(RplChannel.Reliable, RplRcver.Broadcast)]
	protected void RpcDo_Broadcast_Method(bool turningOn)
	{
		Print("proxy-side code");
		PlayMusic(turningOn);
	}
	protected void PlayMusic(bool turningOn)
	{
		if (turningOn)
		SomeSoundClass.PlayMusic(this, "OSWelcome");
		else
		SomeSoundClass.PlayMusic(this, "OSGoodbye");
	}
	// public methods
	void TurnOn()
	{
		Rpc(RpcAsk_Authority_Method, true);
	}
	void TurnOff()
	{
		Rpc(RpcAsk_Authority_Method, false);
	}
}
```

### RplProp

In the following code:

* the m\_bIsTurnedOn property is set to update to all the proxies on authority value's modification
* the m\_bIsTurnedOn property is set to execute the OnTurnedOnUpdated method on proxies when its value is updated by Replication (JIP included)

We assume the public TurnOn method has been triggered **owner-side**, making the following events happen:

* the TurnOn method calls the RpcAsk\_Authority\_Method through RPC; as stated earlier, this method executes on the authority's machine - the server in this case
* the RpcAsk\_Authority\_Method method (running on the server):
  + sets LED colour on the server
  + sets the m\_bIsTurnedOn property to "true" on the authority; as the m\_bIsTurnedOn property is set to broadcast its changes thanks to the RplProp attribute, the change is broadcast to every proxy machine
* the OnTurnedOn method is triggered on proxies and "The proxy has been updated" is printed

The end result:

* all the machines' instances:
  + have the m\_bIsTurnedOn property set to "true"
  + have their light set to green
* the authority:
  + printed "authority-side code"
* all the proxies:
  + printed "proxy-side code"

```enforce
class ComputerEntity : IEntity
{
	[RplProp(onRplName: "OnTurnedOnUpdated")]
	protected bool m_bIsTurnedOn = false; // this value is to be updated by the authority, not set locally by proxies (even owner)
	// if it is set locally, the change will not broadcast and there will be a difference between the proxy and the authority
	// this state discrepancy will last until authority's next update broadcast
	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void RpcAsk_Authority_Method(bool turningOn)
	{
		if (turningOn == m_bIsTurnedOn)
		return; // prevent useless network messages
		Print("authority-side code");
		m_bIsTurnedOn = turningOn; // m_bIsTurnedOn is changed only in an authority-targeting method
		// it will broadcast over the network automatically due to Replication setting (line 3)
		SetLedLightColour(); // SetLedLightColour is not automatically called on the authority
		Replication.BumpMe(); // tell the Replication system this entity has changes to be broadcast
		// the Replication system will update the member variable AFTER RpcAsk_Authority_Method is done
	}
	protected void OnTurnedOnUpdated()
	{
		Print("proxy-side code"); // this method is called on proxies when m_bIsTurnedOn is updated through Replication
		// it is NOT called on the authority
		SetLedLightColour();
	}
	protected void SetLedLightColour()
	{
		if (m_bIsTurnedOn)
		SomeLightClass.SetLedLightColour(this, Color.Green);
		else
		SomeLightClass.SetLedLightColour(this, Color.Red);
	}
	// public methods
	void TurnOn()
	{
		if (m_bIsTurnedOn) // m_bIsTurnedOn can be read from any entity
		return;
		Rpc(RpcAsk_Authority_Method, true);
	}
	void TurnOff()
	{
		if (!m_bIsTurnedOn)
		return;
		Rpc(RpcAsk_Authority_Method, false);
	}
}
```

### RplSave/RplLoad

Without RplSave/RplLoad override, the following entity would be created with m\_iSoldierId, m\_iHealth and m\_bHadLunch set to their default value, as these are not decorated with RplProp.

```enforce
class SCR_RplTestSoldier : IEntity
{
	[RplProp()]
	protected string m_sName = "Player 1"; // joining in progress automatically synchronises RplProp variables
	protected int m_iSoldierId = 12345678;
	protected int m_iHealth = 100; // range 0..100
	protected bool m_bHadLunch = false;
	protected string m_sSoldierDogTag = "PID 1234";
	// Called on the authority when an entity gets streamed
	override bool RplSave(ScriptBitWriter writer)
	{
		// m_sName is automatically synchronised, no need to do it manually
		writer.Write(m_iSoldierId, 32); // write 32 bits of soldier ID	- int is 32 bits in size
		writer.Write(m_iHealth,		7); // write  7 bits of health		- 7 bits are enough if the value cannot go over 100 (7 bits range is 0..127)
		writer.WriteBool(m_bHadLunch); // write  1 bit only
		writer.WriteString(m_sSoldierDogTag); // write the string - the size varies with the string and characters used
		return true;
	}
	// Called on the streamed proxy
	override bool RplLoad(ScriptBitReader reader)
	{
		// m_sName is automatically synchronised, no need to do it manually
		if (!reader.Read(m_iSoldierId, 32)) // read 32 bits of data - the authority wrote soldier ID first, so it needs to be read first
		return false;
		if (!reader.Read(m_iHealth,		7)) // read  7 bits of data - the authority wrote health second, so it needs to be read second
		return false;
		if (!reader.ReadBool(m_bHadLunch)) // read  1 bit only
		return false;
		if (!reader.ReadString(m_sSoldierDogTag)) // read the string - size is managed automatically
		return false;
		return true;
	}
}
```
