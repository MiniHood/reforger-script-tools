# [Server Management](https://community.bistudio.com/wiki/Arma_Reforger:Server_Management)

ⓘ

See also: [Server Hosting](/wiki/Arma_Reforger:Server_Hosting "Arma Reforger:Server Hosting").

## Administrator Roles

An in-game server administrator can be:

* the player hosting the server (PC only)
* a logged-in admin, using `#login passwordAdmin` (see [login](#login) and [passwordAdmin](/wiki/Arma_Reforger:Server_Config#passwordAdmin "Arma Reforger:Server Config") config)
* a voted-in admin

## Permissions

Standard

| Right | Player Role | | | RCON Permission | |
| --- | --- | --- | --- | --- | --- |
| Logged Admin | Voted Admin | Player | RCON Admin | RCON Monitor |
| #login | Unchecked | Unchecked | Checked | Unchecked | Unchecked |
| #logout | Checked | Checked | Unchecked | Unchecked | Unchecked |
| #roles | Checked | Checked | Checked | Unchecked | Unchecked |
| #restart | Checked | Unchecked | Unchecked | Checked | Unchecked |
| #shutdown | Checked | Unchecked | Unchecked | Checked | Unchecked |
| #kick | Checked | Unchecked | Unchecked | Checked | Unchecked |
| #ban | Checked | Unchecked | Unchecked | Checked | Unchecked |
| #id | Checked | Checked | Checked | Unchecked | Unchecked |
| #players | Checked | Checked | Checked | Checked | Checked |

Custom

| Right | Player Role | | | RCON Permission | |
| --- | --- | --- | --- | --- | --- |
| Logged Admin | Voted Admin | Player | RCON Admin | RCON Monitor |
| [armareforger-symbol black.png](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)") @logout | Unchecked | Unchecked | Unchecked | Checked | Checked |

## Commands

### login

Login as server administrator with the password set in server config's field [passwordAdmin](/wiki/Arma_Reforger:Server_Hosting#passwordAdmin "Arma Reforger:Server Hosting").

ⓘ

Admins (in server config) or Owner (in ownerToken.bin) can use the command without password.

Syntax

```
#login
#login <password>
```

Example

```
#login
#login myServersPassword
```

### logout

Example

```
#logout
```

### roles

Print the list of server roles the player possesses.

Example

```
#roles
```

### restart

Restart the actually running scenario. The server keeps the clients connected.

Example

```
#restart
```

### shutdown

Shut down the server, disconnecting all the clients.

Example

```
#shutdown
```

### kick

Kick (eject) the player out of the server through their playerId (obtained by [players](#players)). Said player can still rejoin the server.

Syntax

```
#kick <playerId>
```

Example

```
#kick 123456789
```

### ban

This command prefixes ban-related commands: "create", "remove", "list"
Ban (eject and banish forever or for a certain duration in seconds) the player out of the server through their playerId (obtained by [players](#players)). Said player cannot rejoin until ban expiration.

⚠

This command can only be executed on a dedicated server.

#### create

Create a ban entry for the player related to the provided id.

Syntax

```
#ban create <playerId or identityId or ![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/22px-armareforger-symbol_black.png) 1.2.0 playerName> <durationInSeconds> <reason (optional)>
```

if using playerId, the player must actually be connected to the server; use identityId otherwise.

Example

Ban for an hour

```
#ban create 123456789 3600
```

Ban forever

```
#ban create 123456789 0
```

Ban with a reason

```
#ban create 123456789 86400 teamkilling
```

#### remove

Remove a ban entry linked to the provided identity, allowing the player to rejoin.

Syntax

```
#ban remove <identityId>
```

Example

```
#ban remove 123456789
```

#### list

List current bans in a format BanID ; Player UID ; Duration.

⚠

* If the *page* parameter is not provided then the first page is shown.
* RCON can show 25 bans per page while game only 10.

Syntax

```
#ban list <page (optional)>
```

Example

```
#ban list
#ban list 2
```

### id

Gives the local player's id.

Example

```
#id
```

### players

Lists session's players and their playerId.

Example

```
#players
```

## Custom RCON Commands

ⓘ

Custom RCON commands are BI-specific and are not part of RCON standards. They start with @ to differenciate them from standard commands.

[![armareforger-symbol black.png](/wikidata/images/thumb/6/69/armareforger-symbol_black.png/30px-armareforger-symbol_black.png)](/wiki/Category:Arma_Reforger/Version_1.2.1 "Category:Arma Reforger/Version 1.2.1") [1.2.1](/wiki?title=Category:Arma_Reforger/Version_1.2.1&action=edit&redlink=1 "Category:Arma Reforger/Version 1.2.1 (page does not exist)")

### logout

De-authenticate RCON client on server server side which immediately frees the connection for new RCON client.
If the RCON client is killed without issuing this command, it is de-authenticated automatically after 45s due to connection timeout.
