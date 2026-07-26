# [Modding/Guidelines/Systems Category](https://community.bistudio.com/wiki/Category:Arma_Reforger/Modding/Guidelines/Systems)

A **System** is a standalone piece of code or process that takes care of one specific operation.

For instance, if entities are capable of shooting and reloading, one system should handle the shooting logic, while the other should handle the reloading logic.

The idea behind splitting the logic into multiple systems is as follows:

* **Clear Operations Order** - with one glance at where the systems are registered, it is possible to see right away how and when they are run
* **Logic Separation** - rather than doing everything each system is only responsible for one specific feature/operation
* **Parallelisation** - grouping allows for potential easy parallelisation engine-side
  + depending on what the other places in the code do, thanks to separating the logic it is vastly easier to manage parallel code execution
  + rather than processing things in the ABC ABC ABC ABC order, it is easier to change order and process all As before moving to Bs and Cs (AAAA BBBB CCCC)
* **Performance** - because the engine needs to search through the hierarchy when updating components/entities, whereas Systems prevent doing so.

ⓘ

Systems should be used when something needs to be updated periodically.

## Template

ⓘ

See [Doxygen World Systems documentation](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/Page_WorldSystems.html#WorldSystemDocs_SystemConfiguration).

## Current Limitations

* **No replication** - there is no replication involved in Systems for now. If RPC or replicated properties are needed, use entities/components
* **Script only** - a scripted system can only reference another scripted system, not an engine one
* **No cycle setting** - there is currently no way to define a system's period (e.g tick only every x seconds instead of each frame)

## Pages in category "Arma Reforger/Modding/Guidelines/Systems"

This category contains only the following page.

### 

* [Arma Reforger:Persistence System](/wiki/Arma_Reforger:Persistence_System "Arma Reforger:Persistence System")
