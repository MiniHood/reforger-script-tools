# [Task System Usage](https://community.bistudio.com/wiki/Arma_Reforger:Task_System_Usage)

As a rule, the task system is used with the Scenario Framework. In this article, you will learn how to use it separately.

## Setup

The c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5) game system is a tool for controlling tasks. The system is automatically registered, and you do not need to add anything to it.

### World Editor

You can directly place prefabs in the world. They will be automatically inserted into the task system on init.

There are two basic task prefabs:

* [BaseTask.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Tasks/BaseTask.et)
* [ExtendedTask.et](enfusion://ResourceManager/~ArmaReforger:Prefabs/Tasks/ExtendedTask.et)

Scenario Framework tasks folder: *Prefabs/Systems/ScenarioFramework/Tasks/*

[![Task System Workbench UI.png](/wikidata/images/9/9a/Task_System_Workbench_UI.png)](/wiki/File:Task_System_Workbench_UI.png)

\*The featured fields exist only for the [extended tasks](#Tasks_and_Extended_Tasks).

Base properties:

* **Task ID:** ID to distinguish between different tasks.
* **Task UI Info:** data to be shown in the game UI: a name, a description, an icon, and an icon set name.
* **Task State:** initial state of the task.
* **Task Ownership:** [ownership level](#Ownership_and_Visibility).
* **Task Visibility:** [visibility level](#Ownership_and_Visibility).
* **Task UI Visibility:** UI visibility level to determine where the task is visible for players (a task list, a map, or both).
* **Owner Faction Keys:** key of factions that are owners or viewers of the task.
* **Owner Group IDs:** group ID of groups that are owners of viewers of the task.
* **Owner Executors:** [task executor](#Task_Executor) that is owner or viewer of the task.
* **Assignees:** array that allows you to pre-assign players/groups/entities on init.

Extended properties:

* **Link Data To Related Tasks:** enable or disable sharing data between parent/child tasks if you work with [nested tasks](#Task_Nesting).
* **Progress:** the initial progress of the task on init.

### Game User Interface

Press and hold `J` to open or close the task list. Two tabs open:

[![Task System Game UI 1.png](/wikidata/images/thumb/0/04/Task_System_Game_UI_1.png/300px-Task_System_Game_UI_1.png)](/wiki/File:Task_System_Game_UI_1.png)

The left tab shows available objectives, the right one shows completed and failed tasks.

When clicking the task, a window with the details opens:

[![Task System Game UI 2.png](/wikidata/images/thumb/a/a8/Task_System_Game_UI_2.png/300px-Task_System_Game_UI_2.png)](/wiki/File:Task_System_Game_UI_2.png)

You can see the task icon, name, description, the **Show on map** button, the **Assign** button, and the list of assignees.

If you enabled progress for extended tasks, the progress bar appears in the task entry.

If the task has a child task, you can click a dropdown icon to open a child task. If you click the parent task entry, the info panel will show child tasks required for completion. It does not allow to assign.

If you enabled visibility on map, you can select tasks right there by clicking their icons.

[![Task System Game Assignment.png](/wikidata/images/thumb/b/ba/Task_System_Game_Assignment.png/500px-Task_System_Game_Assignment.png)](/wiki/File:Task_System_Game_Assignment.png)

To assign someone to a task, hover over the task icon on the map, and click the panel next to it.

### Debug Menu

The debug menu allows manipulating asks at runtime for debugging and monitoring. To enable it:

1. Press `⊞ Win` + `Alt`
2. Go to **Systems → Task System**

[![Debug system task system.png](/wikidata/images/thumb/7/7e/Debug_system_task_system.png/600px-Debug_system_task_system.png)](/wiki/File:Debug_system_task_system.png)

**Task List** shows all the tasks that exist in the game world. You can expand the selected task to open its properties. You can open a context menu for each property that provides tools for the further configuration.

At the moment, it is not possible to input number or string attributes for editable entities via the Game Master - the game mode itself. However, support for this feature is being considered for future updates.

## Tasks and Extended Tasks

There are two classes: c[SCR\_Task](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_Task.c;78) and c[SCR\_ExtendedTask](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_ExtendedTask.c;9).

c[SCR\_Task](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_Task.c;78) features the following:

* basic info, such as a unique ID, a name, a description, and location
* array of assignee executors
* task state, levels of [ownership and visibility](#Ownership_and_Visibility), and a type of UI visibility
* FactionKey/GroupID/Executor array to restrict tasks in [visibility and ownership](#Ownership_and_Visibility)

c[SCR\_ExtendedTask](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_ExtendedTask.c;9) has all the same with a couple of actions in addition:

* creation of [nested tasks](#Task_Nesting). You can parent this class to another extended task or create child tasks. It allows you to create complex missions with multiple moving parts.
* monitoring progression. You can set progress in increments (0..100%).

  ⓘ

  Use progression for tasks that imply collecting multiple items.

All the data for both types is stored in c[SCR\_TaskData](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskData.c;1) and c[SCR\_ExtendedTaskData](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_ExtendedTaskData.c;1) accordingly. Both classes provide methods for copying data or merging two instances of data. All the data is replicated as one package. Storing important information in its own separate class helps keep the codebase clean and easier to work with.

## Task System API

| Method | Parameters | Description |
| --- | --- | --- |
| ENFORCECODEMARKER   ``` void GetAssigneesForTask ``` | * SCR\_Task **task** * out array<ref SCR\_TaskExecutor> **assignees** * bool **notify** | Outputs an array of assignee executors for the task. ⓘ  Set notify to false not to receive a warning message when the assignee list is empty. |
| ENFORCECODEMARKER   ``` bool IsTaskVisibleFor ``` | * SCR\_Task task * SCR\_TaskExecutor executor | Returns **true** if the task is visible to the executor. |
| ENFORCECODEMARKER   ``` bool CanTaskBeAssignedTo ``` | * SCR\_Task **task** * SCR\_TaskExecutor **executor** | Returns **true** if the task is assignable to the executor. The method looks at the [task ownership](#Ownership_and_Visibility) for comparison. |
| ENFORCECODEMARKER   ``` void GetTasksByState ``` | * out array<SCR\_Task> **outTasks** * SCR\_ETaskState **state** * FactionKey **filterFaction** * int **filterGroup** | Outputs an array of tasks that match the state. You can further filter tasks by factions and groups. |
| ENFORCECODEMARKER   ``` void RegisterTask ``` | * SCR\_Task **task** | Registers the task into the task system. |
| ENFORCECODEMARKER   ``` void UnregisterTask ``` | * SCR\_Task **task** | Unregisters the task from the task system. |
| ENFORCECODEMARKER   ``` void AssignTask ``` | * SCR\_Task **task** * SCR\_TaskExecutor **executor** | Tries to assign the task to the executor. |
| ENFORCECODEMARKER   ``` void UnassignTask ``` | * SCR\_Task **task** * SCR\_TaskExecutor **executor** | Tries to unassign the task from the executor. |
| ENFORCECODEMARKER   ``` float GetTaskProgress ``` | * SCR\_Task **task** | Returns progress of the task as float. The value ranges from 0 to 100. |
| ENFORCECODEMARKER   ``` void AddTaskProgress ``` | * SCR\_Task **task** * float **percentage** | Tries to add progress to the task. |
| ENFORCECODEMARKER   ``` void RemoveTaskProgress ``` | * SCR\_Task **task** * float **percentage** | Tries to remove progress from the task. |
| ENFORCECODEMARKER   ``` SCR_ETaskState GetTaskState ``` | * SCR\_Task **task** | Returns the state of the task. |
| ENFORCECODEMARKER   ``` void SetTaskState ``` | * SCR\_Task **task** * SCR\_ETaskState **state** | Changes the state of the task. |
| ENFORCECODEMARKER   ``` SCR_ETaskOwnership GetTaskOwnership ``` | * SCR\_Task **task** | Returns ownership of the task. |
| ENFORCECODEMARKER   ``` void SetTaskOwnership ``` | * SCR\_Task **task** * SCR\_ETaskOwnership **ownership** | Tries to change ownership of the task. |
| ENFORCECODEMARKER   ``` SCR_ETaskVisibility GetTaskVisibility ``` | * SCR\_Task **task** | Returns visibility of the task. |
| ENFORCECODEMARKER   ``` void SetTaskVisibility ``` | * SCR\_Task **task** * SCR\_ETaskVisibility **visibility** | Tries to change visibility of the task. |
| ENFORCECODEMARKER   ``` SCR_ETaskUIVisibility GetTaskUIVisibility ``` | * SCR\_Task **task** | Returns visibility of the task in the UI (e.g. on the map). |
| ENFORCECODEMARKER   ``` void SetTaskUIVisibility ``` | * SCR\_Task **task** * SCR\_ETaskUIVisibility **visibility** | Tries to change visibility of the task in the UI (e.g. on the map). |
| ENFORCECODEMARKER   ``` array<string> GetTaskFactions ``` | * SCR\_Task **task** | Returns owner factions of the task. |
| ENFORCECODEMARKER   ``` void AddTaskFaction ``` | * SCR\_Task **task** * FactionKey **factionKey** | Adds the owner faction to the task. |
| ENFORCECODEMARKER   ``` void RemoveTaskFaction ``` | * SCR\_Task **task** * FactionKey **factionKey** | Removes the owner faction from the task. |
| ENFORCECODEMARKER   ``` array<int> GetTaskGroups ``` | * SCR\_Task **task** | Returns owner group IDs of the task. |
| ENFORCECODEMARKER   ``` void AddTaskGroup ``` | * SCR\_Task **task** * int **groupID** | Adds the owner group ID to the task. |
| ENFORCECODEMARKER   ``` void RemoveTaskGroup ``` | * SCR\_Task **task** * int **groupID** | Removes the owner group ID from the task. |
| ENFORCECODEMARKER   ``` array<ref SCR_TaskExecutor> GetTaskExecutors ``` | * SCR\_Task **task** | Returns owner executors of the task. |
| ENFORCECODEMARKER   ``` void AddTaskExecutor ``` | * SCR\_Task **task** * SCR\_TaskExecutor executor | Adds the owner [task executor](#Task_Executor) to the task. |
| ENFORCECODEMARKER   ``` void RemoveTaskExecutor ``` | * SCR\_Task **task** * SCR\_TaskExecutor executor | Removes the owner task executor from the task. |
| ENFORCECODEMARKER   ``` vector GetTaskLocation ``` | * SCR\_Task **task** | Returns the location of the task as a vector. |
| ENFORCECODEMARKER   ``` void MoveTask ``` | * SCR\_Task **task** * vector **destination** | Sets the location of the task as a new one. |
| ENFORCECODEMARKER   ``` void GetChildTasksFor ``` | * SCR\_Task **task** * out array<SCR\_Task> **childTasks** | Outputs an array of tasks that are parented to a particular task. |
| ENFORCECODEMARKER   ``` void AddChildTaskTo ``` | * SCR\_Task **task** * SCR\_Task **childTask** | Tries to parent a **childTask** to the task. ⓘ  Tasks that are already parented to a particular task cannot have their own child tasks as the maximum allowed depth is set as 1. |
| ENFORCECODEMARKER   ``` void RemoveChildTaskFrom ``` | * SCR\_Task **task** * SCR\_Task **childTask** | Tries to unparent a **childTask** from the task |
| ENFORCECODEMARKER   ``` SCR_Task CreateTask ``` | * ResourceName **taskResourceName** * string **taskID** * string **name** * string **desc** * vector **position** | Creates a new task and registers it into c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5). |
| ENFORCECODEMARKER   ``` void DeleteTask ``` | * SCR\_Task **task** | Deletes the task from c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5). |
| ENFORCECODEMARKER   ``` SCR_Task GetTaskFromTaskID ``` | * string **taskID** * bool **notify** | Finds the task by the **taskID** and returns it. ⓘ  Set notify to false not to receive a warning message if the task could not be found. |

## Task System Control Flow

The task system performs operations in strict order. Rules ensure that no sensitive data gets manipulated and potentially corrupted by external scripts.

[![Control Flow.png](/wikidata/images/thumb/4/46/Control_Flow.png/600px-Control_Flow.png)](/wiki/File:Control_Flow.png)

where each element of the system only performs certain actions:

| c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5) | c[SCR\_Task](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_Task.c;78) | c[SCR\_TaskData](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskData.c;1) |
| --- | --- | --- |
| * exposes an API for the UI and external scripts * checks if operations are allowed/valid * provides feedback for almost all operations (whether it is successful of failed, and the reason of fail) | * performs operations directly * handles replication * reads and writes data | * stores data |

When handling the task system, follow the rules below:

* When managing tasks on a high level (assigning to players, changing task states, etc.), only use the API provided by c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5).
* You can retrieve information directly from c[SCR\_Task](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_Task.c;78) using its public getters. However, it is not allowed invoking public methods that manipulate task data inside c[SCR\_Task](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_Task.c;78) directly.
* Never modify or read data in c[SCR\_TaskData](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskData.c;1) directly. Use c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5) to modify data and c[SCR\_Task](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_Task.c;78) to read data.

## Task Executor

A task executor serves as data collection for everything related to the player/entity/group assignment.  Whenever the task system needs to handle data related to the assignment or [task ownership](#Ownership_and_Visibility), it uses a task executor for communication of data. The c[SCR\_TaskExecutor](enfusion://ScriptEditor/scripts/Game/Tasks/Executors/SCR_TaskExecutor.c;1) class inherits from c[ScriptAndConfig](enfusion://ScriptEditor/scripts/Core/proto/Types.c;123) and stores the following information:

* Player ID
* Entity RplID
* Group ID

There are currently six ways to create a task executor:

| Method | Parameters | Description |
| --- | --- | --- |
| ENFORCECODEMARKER   ``` SCR_TaskExecutor.FromPlayerID ``` | int **playerID** | Creates and returns a new **player** executor. The task system treats the executor as a player. |
| ENFORCECODEMARKER   ``` SCR_TaskExecutor.FromEntity ``` | IEntity **ent** | Creates and returns a new **entity** executor. The task system treats the executor as an entity. It can be an AI character, a vehicle, etc. |
| ENFORCECODEMARKER   ``` SCR_TaskExecutor.FromGroup ``` | int **groupID** | Creates and returns a new **group** executor. This executor is only used for group assignment. |
| ENFORCECODEMARKER   ``` SCR_TaskSystem.TaskExecutorFromPlayerID ``` | int **playerID** | Creates and returns a new **player** executor. The task system treats the executor as a player. |
| ENFORCECODEMARKER   ``` SCR_TaskSystem.TaskExecutorFromEntity ``` | IEntity **ent** | Creates and returns a new **entity** executor. The task system treats the executor as an entity. It can be an AI character, a vehicle, etc. |
| ENFORCECODEMARKER   ``` SCR_TaskSystem.TaskExecutorFromGroup ``` | int **groupID** | Creates and returns a new **group** executor. This executor is only used for group assignment. |

### Working with Task Executor

How to assign a task:

1. Create a task executor using the methods above.
2. Call c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5).AssignTask and provide the task reference and a newly created task executor.

The task system will directly insert the provided executor into the assignee list of the given task.

How to unassign a task:

1. Create a task executor using the methods above.
2. Call c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5).UnassignTask and provide the task reference and a newly created task executor.

**Deleting a task executor**

When you want to remove an assignee executor from a task, the task system does not care about the exact object in memory.
Instead, it looks at the data inside the new executor you give it (like player ID or group ID).
Then, it compares this data with each executor in the task's list. As soon as it finds a match, it removes that executor from the list.

You do not have to worry about giving the system the same object as before. As long as the data matches, the system will find and remove the right one.

An example in Java: instead of asking if two string objects are the exact same (`==`), you ask if their contents are equal (`.equals()`). The task system always uses the `.equals()` style check.

## Ownership and Visibility

Both ownership and visibility are used to restrict tasks for certain players or group of players.
Ownership determines who or what can assign to a task. There are the following levels of ownership:

* **None:** no one can assign
* **Executor:** only specified executors can assign (it works both for players and entities).

  ⓘ

  Such a task can only be assigned to executors presented in the owner executor array.
* **Group:** all players inside specified groups can assign
* **Faction:** all players inside specified factions can assign
* **Everyone:** everyone can assign

Visibility determines who can see the task in the task list or in the map view. It has all the levels listed above, and one more: **Assignees**.
A task can be set up in a way that players only see the task in their task list once it is assigned to them.

ⓘ

Let's imagine you want to work with the US faction.

If you want the task to be owned by it, set Task Ownership as Faction, then add the US faction key to the array.

If you want the task to appear in the US players' task list or a map, set Task Visibility as Faction, then add the US faction key to the array.

Once ownership and visibility are set, executors/groups/etc. (depending on the set level) need to be added to owner arrays, so that anyone could assign to the task or view it.

## Group Tasks

You can assign a task to a player group. The task needs to meet the following conditions:

* To have the ownership level set as **Executor**.
* To include one or more player groups as executor instances.

If a group is included in the task's owner list, the group leader can assign the task to their group.
When the leader does this, the whole group will be assigned to the task.

[![Group Tasks.png](/wikidata/images/1/1f/Group_Tasks.png)](/wiki/File:Group_Tasks.png)

⚠

Group assignment implies the group as a whole to assign to a task. It does not assign every group member one by one.
If a game event requires checking a player for the assignment, the task system checks whether the player belongs to the whole group.

### Assignment

There are two ways of working with group tasks.

**Assigning group members:**

1. Set ownership as **GROUP**
2. Insert the group ID to the owner group array.

Every member in the group can assign himself to the task.

**Assigning the whole group:**

1. Set ownership as **EXECUTOR**
2. Insert a new group executor to the owner executor array.
3. In the **executor group ID** field, enter the group ID.

A group as a whole can assign to a task.

## Task Progression

The progress bar is part of an extended task that allows it to be completed in increments (0..100%).
It is a useful tool for tasks that require certain amounts of items to be collected. Players can track their progress in the task list.

[![Task Bar.png](/wikidata/images/thumb/6/6d/Task_Bar.png/300px-Task_Bar.png)](/wiki/File:Task_Bar.png)

Use the following c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5) methods to manage the task progress:

```enforce
SCR_TaskSystem.AddTaskProgress
SCR_TaskSystem.RemoveTaskProgress
```

You can enable or disable the progress if you do not want tasks to be progressed in increments, but want to nest them.

The progress behavior changes if both a parent and a child tasks have it enabled. See the [Task Nesting section](#Task_Nesting).

### The State-Progress Behavior

The behavior follows the principles:

* If the progress goes above 0%, the task state switches to **PROGRESSED**.
* When the progress reaches 100%, the task state switches to **COMPLETED**.
* If the progress is enabled, and the task state is manually set as **COMPLETED**, the progress will not updated and show 100%.
* When the progress is added or removed, the task system caches the previous task state in the case if the progress gets reverted.

## Task Nesting

Task nesting means organizing tasks in a parent-child hierarchy, where one task can contain and manage multiple subtasks.

The nested task structure:

[![Nested Task Structure.png](/wikidata/images/thumb/9/92/Nested_Task_Structure.png/700px-Nested_Task_Structure.png)](/wiki/File:Nested_Task_Structure.png)

The nested tasks in the game UI:

[![Nested Task Structure in UI.png](/wikidata/images/thumb/d/d5/Nested_Task_Structure_in_UI.png/500px-Nested_Task_Structure_in_UI.png)](/wiki/File:Nested_Task_Structure_in_UI.png)

### Setting Up Nesting Task

You can set up nested tasks both in Workbench or during the runtime.

#### Workbench

Both a parent and a child tasks must be c[SCR\_ExtendedTask](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_ExtendedTask.c;9).

To create a task hierarchy, drag and drop the child task onto the parent task in the hierarchy, in the [World Editor](/wiki/Arma_Reforger:World_Editor "Arma Reforger:World Editor").

The task system parents the child task to the parent one automatically on init.

[![Workbench nested tasks.png](/wikidata/images/8/86/Workbench_nested_tasks.png)](/wiki/File:Workbench_nested_tasks.png)

#### Runtime

Both a parent and a child tasks must be c[SCR\_ExtendedTask](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_ExtendedTask.c;9).

To manage nested tasks:

* Call c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5).AddChildTaskTo to parent the child task to the parent task.
* Call c[SCR\_TaskSystem](enfusion://ScriptEditor/scripts/Game/Tasks/SCR_TaskSystem.c;5).RemoveChildTaskFrom to unparent it.

### Behavior

Each task can be set to share data with other tasks in the same hierarchy.
This means some settings, like ownership and visibility, will automatically be inherited from the parent task.

#### Assignment

Information about assignees also can be shared with parent or child tasks. The following principles work:

* If the executor is assigned to a child task, it will be assigned to a parent task as well.
* Executors cannot be manually assigned to parent tasks (tasks with child tasks). It works so to avoid confusion about task ownership and responsibility. Only child tasks should have direct assignees.
* Only one task can be assigned to the executor at a time.

#### Ownership and Visibility

Since task assignment depends on ownership settings, there are some corner cases to consider when dealing with nested tasks:

* Parent tasks can have [any level of ownership](#Ownership_and_Visibility).
* Child tasks can only switch between ownership levels that are lower than their parent task's ownership.

Ownership example in nested tasks

If the parent task's ownership is set as FACTION, the child task's ownership can only be set to GROUP, EXECUTOR, or NONE. If the child tasks are set to GROUP ownership, the main task is assigned to the whole faction, and the child tasks are then distributed to individual player groups within that faction

The same principles work for the visibility.

#### Progression

Progress for nested tasks is handled according to the following:

* When progress is made on a child task, that progress is averaged across all sibling tasks and then applied to the parent task. For example, if there are three child tasks and one of them reaches 100% progress, about 33% progress will be reflected on the parent task.
* Parent tasks can still receive progress independently from their child tasks.

ⓘ

A parent task could gain 10% progress for every minute the team holds out, regardless of the status of its child tasks.
After 10 minutes, the parent task would reach 100% progress even if none of the child tasks were completed.
