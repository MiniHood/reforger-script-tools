# [Behavior Editor: Nodes](https://community.bistudio.com/wiki/Arma_Reforger:Behavior_Editor:_Nodes)

Flow nodes define base rules how and with which conditions, are behavior tree branches executed.

### Flow

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Root | The starting point of a behavior tree. |  | Cannot be removed or inserted from the nodes palette. |
| Selector | Executes children until one of them returns Success. Returns:   * Running: A child node still runs * Success: A child node returned success * Fail: All children returned fail |  |  |
| Sequence | Executes children from left to right as long as they return success. Returns success only if all children returned success. Returns:   * Running: A child node still runs * Success: All children returned success * Fail: Any child returned fail |  |  |
| Parallel | Executes all children regardless what they return. Returns:   * Running: If no child is assigned for getting its result * Success: If the index of a child is assigned and the assigned child returns success * Fail: If the index of a child is assigned and the assigned child returns fail | Parameters:  * Use Child Result: Select index of a child node, which result will be the result of the parallel node. |  |
| Repeater | Sequence node that is executed multiple times. Returns:   * Running: A child node still runs * Success: All children returned success * Fail: Any child returned fail | Parameters:  * Repeat Times: Specifies how many times you want to execute. Default is once. |  |
| Run BT | Runs selected Behavior Tree on the agent. Returns:   * Running: If the subtree returns running * Success: If the result of the subtree is success * Fail: If the result of subtree is fail, or the subtree could not be executed | Parameters:  * Behavior Tree: Specify behavior tree to run from the list. * Run Repeatedly:   + True (Checked): Initialise the subtree once, then run it repeatedly without reinitialization   + False (Unchecked): Reinitialise and run the subtree every time.  * Variables in Array: New, unfinished feature   Input:   * InBehaviorTree: Behavior Tree to run |  |
| Run Once | Executes all of its children only once. Returns:   * Success: Always |  | Intended to be used for one-time-only tasks, such as initialization. |
| Run On Entity | Executes children on a different entity than the owner of the behavior tree. Returns:   * Success: If the children were executed on entity successfully and returned success or running * Fail: If one of the children returns fail or the entity is not found | Input:  * Entity | Should be used only with nodes and tasks that are not responding with Running result (reason: currently BT does not switch to scope of entity on which you wish to run those nodes, stays in scope of main BT) |
| Switch | Picks children depending on a variable value or randomly. Returns:   * Running: A child node still runs * Success: A child node returned success * Fail: It is aborted because of the variable change, the child returned fail, or could not find a proper child to run | Parameters:  * Values Array: For each children maximum value of in variable * Random Range: For each children probability of execution. Specifies the range. * Abort Variable Changed:   + True (Checked): Abort if the input variable is changed. The default is checked.   + False (Unchecked): Do not abort if the input variable is changed   Input:   * InVariable: The variable that may be tracked for aborting (if abort variable changed is checked) and/or for picking the child node to run from the values array |  |
| For Each Child | Iterates through all group children. Returns:   * Running: A child node still runs * Fail: If no entity, no AI group, no agent in the AI group, or a child returns fail * Success: Otherwise | Parameters:  * Index from: Which should be starting index in an array. (Default: 0) * Index to: Which should be max index to select. (-1 by default, to go through whole array) * Return controlled entity: Return controlled entity, if false, return AI Agent.   Output:   * Entity |  |

### Decorators

Decorators are test checking different states and variables, and determine if child or child branch should be executed.

Decorators

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Decorator | Node which tests specific condition. If condition result is true, decorator will execute its child and return true. Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | **Common decorator parameters**:  * Negative result: Decorator will return negative value of the test. * Use child result: Decorator will execute the child and return its result. * Always true: Decorator will execute child node and then return true. * Abort type:   + None: Do not abort anything   + Abort children branch: Abort all children under this node.   + Abort parent node with children: Abort self and all children under this node.   + Abort parent node further children: Abort parent node and its children, but proceed with children nodes. | Useful for tests through its parameters – use child result, aborting, etc. Combination of parameters can be used: When you use always true and negative result, decorator will also execute its child and return false. It's useful in case when you want your branch to return false, but execute child node in any case. |
| Test Variable | Decorator checking provided variable for assignment, comparation against value, or check for value change. Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | Parameters:  * Test value: Specific value to test. * Test type: Check if value is:   + Assigned   + Variable equals value   + Variable bigger than value   + Variable lower than value   + Variable was changed   Inputs:   * Variable |  |
| Test Entity | Various tests on entity. Decorator checks validity of entity, either connected through variable, or according to its name. Can test other conditions like life state. Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | Parameters:  * Entity name * TEST: Space for selecting new kinds of tests, not working at the moment.   Input:   * Entity |  |
| Test Distance to Entity | Checks distance of entity with treshold value. Returns true if distance is smaller than treshold by default. Check for ditance larger than treshold can be done with Use Negative Value parameter.  Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | Parameters:  * Entity name: Search entity by name. * Distance treshold: Until what distance result will return true   Inputs:   * Entity * Position * Distance |  |
| Danger Event | Checks agent if he has any danger events. Test can be performed against any or specified danger event. Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | Parameters:  * Has Any: check for any type of danger event * Danger event type: list of available danger events to check * Distance treshold: Limit search for danger events to certain radius. Default 0 = range not limited. | * Is there possibility to test several possible danger events in one decorator? (projectile hit + damage taken)   Danger event is type of event about threat to entity, like someone fired, someone was killed, etc. They all have specified position in world. |
| Test Order | Tests if agent has order, or has specific order according to parameters. Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | Parameters:  * Has Any * Order Type |  |
| Aiming | Tests aiming Returns:   * Success: The child node returned success or when negating the result is selected and the child returns fail * Fail: The child node returned fail or when negating the result is selected and the child returns success | Parameters:  * Test type: What test to run   + Is aimed   + Is aiming in limits * Aim Threshold: Value for testing |  |

### Scripted Decorators

Decorators created in script, inherited from Scripted Decorator.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Scripted Decorator | Scripted base class of decorator, used for inheritence of other scripted decorators. | **Common decorator params**:  * Negative result: Decorator will return negative value of the test. * Use child result: Decorator will execute the child and return its result. * Always true: Decorator will execute child node and then return true. * Abort type:   + None: Do not abort anything   + Abort children branch: Abort all children under this node.   + Abort parent node with children: Abort self and all children under this node.   + Abort parent node further children: Abort parent node and its children, but proceed with children nodes. | Does not contain any new functionality than basic decorator. |

### Tasks

Leaf nodes tasked to perform specific operation. They can return true, false and running.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Get Controlled Entity | Finds character entity controlled by agent running Behavior tree. Return:   * True: Entity was found * False: Entity was not found | Parameters:  * Character Only: If true, accepts only character as controlled entity, not vehicle for example.   Output:   * Entity | AI is composed of two entities, character and AI Agent. This is common way how to access character from Agent in Behavior tree. |
| Create Position | Creates position according to specifie vector or position of entity. It's possible to randomise the position from its given center. Return:   * True: Position was created. * False: Failed to create position. | Parameters:  * Entity name: Calculate vector according to entity * Offset Vector: * Random Offset Multiplier:   Input:   * Root vector * Offset vector * Entity   Output:   * Position | Position output has to be saved as variable, it cannot be used directly as input node. |
| Follow Path | Follow given path Return:   * True: Move was completed * False: Path couldn't be found or move is invalid * Running: Movement on the path in progress | Parameters:  * Path points: Custom array of points to follow, optional   Input   * Target Location |  |
| Move to Entity | Requests move to position of entity. This position is regularly updated, it should be used to move to target which can move. Return:   * True: Move was completed. * False: Failed to add action on entity. * Running: Movement to entity in progress. | Parameters:  * Precision XZ: Precision of ground position. * Target object name: Name of the entity to find and follow. * Entity update precision: How far away needs the entity to be to update the move request.   Input:   * Entity |  |
| Move | Request move to specific position. Use to move to a static target. Return:   * True: Move was completed. * False: Failed to add action on entity. * Running: Movement in progress. | Parameters:  * Target location: In world location to go to * Precision XZ: Precision of ground position. * Precision Y: Height precision. * Target rotation: Rotation of entity during action. * Target rotation to: Rotation related to specified entity (sun for example). * Rotation precision: Angle in degrees * Time to complete: Desired time to move completion (default: -1: Unlimited time) * Enable avoidance: Avoid obstacles during the action.   Input:   * Target location * Target rotation |  |
| Aim | Requests aiming on entity, position or specified direction. Returns:   * True – task succesfully finished * Running – task is ongoing * False – request failed | Parameters:  * Precision XZ * Precision Y   Input:   * Direction * Aim at position * Entity |  |
| Get in Vehicle | Moves character into vehicle. Return:   * True: Entity can get into vehicle * False: Unit is in vehicle already or cannot board | Parameters:  * Position:   + Pilot   + Gunner   + Turret   + Cargo * Vehicle Name: Select vehicle by name   Input:   * Vehicle |  |
| Land | Realizes movement over landing position and descend into position. Return:   * True: Land was completed. * False: Agent failed landing or cannot move. * Running: Landing in progress. | Parameters:  * Target location * Precision XZ: Precision of ground position. * Precision Y: Height precision. * Target rotation: Rotation of entity during action. * Target rotation to: Rotation related to specified entity (sun for example). * Rotation precision * Time to complete: Desired time to move completion (default: -1: Unlimited time) * Enable avoidance: Avoid obstacles during the action.   Input:   * Target location * Target rotation |  |
| Stop | Cancel movement and/or aiming of entity. Return:   * True: Entity can be stopped * False: Stopping failed | Parameters:  * Stop flags:   + Movements   + Aiming |  |
| Get Out Vehicle | Request disembarking from vehicle. Return:   * True: Unit is in vehicle * False: Unit is outside vehicle |  |  |
| Set Movement Speed | Request change of movement speed of controlled entity. Return:   * True: Movement speed set. * False: Failed to set movement speed. | Parameters:  * Speed: Wanted movement speed (m/s) |  |
| Orient | Requests change of orientation of controlled entity. Return:   * True: Request completed. * False: Request failed. * Running: During orientation change. | Parameters:  * Rotation precision * Target rotation to: Rotation related to specified entity (sun for example).   Input:   * Target location |  |
| Enable Obstacle Avoidance | Allows or disables obstacle avoidance for controlled character. Return:   * True: Change of obstacle avoidance succeeded. * False: Change of obstacle avoidance failed. | Parameters:  * Obstacle avoidance   Input:   * Obstacle avoidance |  |
| Get Aiming Position | Calculate the position of the target. It's possible to predict target's position in future using parameter Lead movement time. This node will not calculate any ballistics, just tracks the position of the target and interpolate the result. Return:   * True: Position found. * False: Failed to find position. | Parameters:  * Lead movement time - Predict position in future, using current velocity   Input:   * Entity   Output:   * Position |  |
| Send Order | Creates order and add it to order list. Can be used on any entity, controlled entity is selected by default. Any entity can be also selected as sender. Return:   * True: Order was added to target entity. * False: Order failed to add. | Parameters:  * Order type   + None   + Hold   + Move   + Follow   + Rearm   + Defend   + Get In * Highest priority - If true, order will be first one in queue. If false will be queued.   Input:   * Sender * Receiver * Position * Entity * Order type |  |
| Idle | Wait for given seconds. Waiting time can be randomised. Return:   * True: Timer is finished * Running: Timer is active | Parameters:  * Period - For how long to idle * Period random - Random dispersion   Input:   * Period key |  |
| Current Order | Get data about current order. Return:   * True: Current order data received. * False: Failed to receive current order data. | Output:  * Order object * Order position * Order type |  |
| Finish Order | Finishes current order. Return:   * True – if there was order and it was finished * False – test failed or order was not finished |  |  |

### Utility Tasks

Tasks handling very basic functions as operations with variables.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Set/Clear Variable | Allows for setting value or clearing variable. Return:   * True: Value set or cleared successfully. * False: Failed to set/clear variable. | Parameters:  * Set value - What value should variable have * Clear variable - If value should be cleared (deassigned) |  |
| Find Entity | Find entity by name or class name. Return:   * True: Entity found. * False: Entity not found. | Parameters:  * Entity name - Search by entity name * Entity class name - Search by class name * Distance test - if bigger than 0, returns entity only if closer than this distance   Output:   * Entity |  |

### Scripted Tasks

Scripted variants of Task, inherited from scriptedTask.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Find Target | Example scripted node for finding target in certain radius. For now enemy is any character entity other than BT agent or player character. Return:   * True: Target found. * False: Target not found. | Input:  * Radius: In what distance should target be searched.   Output:   * Target: Target character. Outputs null if entity not found. |  |

### Character Tasks

Tasks that can be used only on characters. They will not work on vehicles, turrets, etc.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Fire | Request fire action on controlled character. Return:   * True: Action was requested * False: Requester is not a character |  |  |
| Change Stance | Request stance change on character. Return:   * True: Action request succeeded * False: Action request failed | Parameters:  * Stance - Wanted Stance   + Prone   + Crouch   + Stand   Input:   * Stance |  |
| Throw Grenade | Request throw grenade action on controlled character. Return:   * + True: Action was requested   + False: Requester is not a character |  | This node might be obsoleted in future, depends on implementation of grenade throwing. |
| Character Raise Weapon | Request action to raise or lower weapon. Return:   * + True: Action was requested   + False: Requester is not a character | Parameters:  * Raise   + True   + False |  |
| Character Set Movement Speed | Request change of movement type on controlled entity. Return:   * + True: Action was requested   + False: Requester is not a character | Parameters:  * Movement type - Wanted movement speed   + Idle   + Walk   + Run   + Sprint |  |
| Play Gesture | Attempts to find a gesture by name and request it. Returns:   * True: If succeeded * False: Gesture not found | Parameters:  * Gesture: Pick gesture type from the list   + Greeting   + Point finger   + Thumb up   + Thumb down   + Silence   + Middle finger   + Timeout   + Heart   + Facepalm   + Watching   + Hold   + Listening   + Point at myself   + Look at me * Play: Play gesture if TRUE; Stop gesture if FALSE |  |
| Perform Object Action | Finds and performs action on object, for example open door. Returns:   * True: Action found and attempted * False: no object, no character or no action | Input  * Action Entity |  |

### Waypoints

Tasks related for handling waypoints.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Get Waypoint | Finds and returns waypoint with selected index. | Parameters:  * Index - From 0 up, index of waypoint in current queue   Input:   * Index   Output:   * Waypoint * Behavior * Contact behavior * Danger behavior |  |

### Group

Tasks related to group management.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Get Group children | Iterates through array of group children, and output child of current index as variable. It can be specified which indexes should be iterated through. Return:   * True: Entity found * False: Invalid or empty group | Parameters:  * Index from: Which should be starting index in an array. (Default: 0) * Index to: Which should be max index to select. (-1 by default, to go through whole array) * Return controlled entity: Return controlled entity, if false, return AI Agent.   Output:   * Entity | Example: Behavior tree that will cause all children of the group stop. This BT has to be called multiple times, as Get Group children only stores correct index of array, but cannot call anything multiple times.  Note: This handling of "forEach" might change in the future. |
| Create Group |  |  | This seems to be WIP node. |

### Perception

Tasks related to AI Agent perception, which is basic source for finding threats and targets.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Pick Target | Picks one target from known target list, and output it as variable. Return:   * True: Target found and assigned * False: No target found | Output:  * Entity |  |

### Action Tasks

Task nodes helping with performing actions.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Request Action | Attempt to request specified action on controlled entity. Return:   * True: Action requested succesfully * False: Invalid action requested, or unable to complete request. | Parameters:  * Action name - as defined by input configuration * Action value   Input:   * Action name * Action value |  |

### Smart Actions

Tasks related to smart actions. Smart actions are objects in world, with which AI can interact. Bench can have smart action for sitting.

| Name | Description | Parameters/Input/Output | Notes |
| --- | --- | --- | --- |
| Perform Smart Action | Makes controlled entity to perform smart action Return:   * True: Smart action assigned to entity. * False: Smart action assignment failed. | Input:  * Smart action |  |
| Find Smart Action | Allows finding smart object in the world which support smart action and save it into variable. If multiple smart actions found, random is selected. Return:   * True: Smart action found. * False: Smart action not found. | Parameters  * Radius: The radius to search for actions within. * Tags: Tags of smart action * Tags test: Way of testing tags   Output:   * Smart action |  |
| Get Current Smart Action | Get smart action object currently realised by Agent. Return:   * True: Always. | Output:  * Smart action |  |
| Get Smart Action Pos | Get position of given smart action. Return:   * True: Always. | Input:  * Smart action   Output:   * Action pos |  |
| Get Smart Action BT | Returns behavior tree for given smart action. Return:   * True: Always. | Input:  * Smart action   Output:   * Behavior |  |

### Experimental / Work in Progress

These nodes might not be finished or are used in some experimental or unfinished systems. These nodes are changed often, so it's not recommended to use them.

| Name | Description | Parameters | Notes |
| --- | --- | --- | --- |
| Flocking | Implements features | Parameters:  * Separation distance * Neighborhood radius * Accumulation type   + Ground   + Aerial |  |
| Get Random Point |  |  |  |
| Scripted Target | Allows for overriding in scripts |  |  |
