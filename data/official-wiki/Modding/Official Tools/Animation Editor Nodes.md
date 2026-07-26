# [Animation Editor: Nodes](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Nodes)

An **animation node** is a fundamental block of the animation engine. Each node has a single (and simple) purpose. By connecting these nodes to one big hierarchy, complex behaviour can be achieved.
Usually the hierarchy is organised top to bottom, connecting parent nodes to child nodes.

Each node is displayed as a small rectangle, first line tells us the node type, second line contains node name.
The **name has to be unique**, which is used as a reference when linking to it from other nodes. Naming is **case sensitive**.

Slots, which can be found at the bottom of some node types, serve to connect a node to any child node.

## Animation Graph Evaluation

Imagine the evaluation as a process happening in two phases.

**DOWN**. Animator and character programmer agree on "master" (main, topmost) node. The animation graph evaluation starts at this node and continues to child nodes.
Logic in animation nodes affects which child paths are evaluated and which are ignored. This way the evaluation traverses from topmost node to bottom of the graph.

**UP**. Once there are no valid child nodes where to continue the search, the evaluation returns back, carrying an animation pose as a result.
This pose can be blended with other poses returned from different child paths or for example.

(**SIMPLIFIED**. Logic is evaluated from top to bottom, left to right. Results (animation poses) are passed to parent node until they reach the topmost node.)

**Order of evaluation is important, because it affects values used in conditions and other expressions**.
For example, when using tags, remaining time or events in an expression, it must be checked after all child nodes are evaluated, otherwise these values are still unknown.

## Time and Timing

One of the inputs of an animation node is a *time*. Time is for advancing animations, essentially - *playing* the animations.
The node usually passes this *time* to linked child nodes, however, some node hold their own clock.
There are two ways to measure time:

* Real time. Time as we know it, it advances forward.
* Normal time. Time used to synchronise animations. It scales between synchronization points, so multiple animations can be blended nicely.

ⓘ

Read more about Syncing Animation: [Animation Editor: Sync Tutorial](/wiki/Arma_Reforger:Animation_Editor:_Sync_Tutorial "Arma Reforger:Animation Editor: Sync Tutorial").

## Node Types

### Common Properties

| Property | Description |
| --- | --- |
| Node Name | Unique node name, case sensitive. |
| Tags | User defined tags. Tags are returned together with resulting pose. Game code can read the returned tags and react to them. |
| Child | Link to child node/multiple children |

### Attachment

#### Attachment

Attachment node provides dynamic link to a different graph using different animation instance.
The attachment is provided by the game and attachment node picks it by the binding name.
Attachments can be inserted for debugging in the animation editor through Attachments Debug window.
Attachment node translates the inputs (graph variables, commands...) into form accepted by attached graph which may influence the performance.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Binding | Binding name of the attachment. |

### Blend

#### Blend

Used to blend between two child nodes. Blend node supports both "normal" and "additive" blending and required no further setup for that manner, since every animation track already contains information whether it is additive. Blending is immediate - based on a value or variables. Blending weight can change rapidly between frames, no time-based blending is done.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BlendWeight | Blending weight (0 - 1). |
| Child0 | First child, having 100% influence when blending weight is 0. |
| Child1 | Second child, having 100% influence when blending weight is 1. |
| BlendFn | Blend weight shaping (linear, s-curve, etc.). |
| MotionVectors | Option specifying where to accept root motion from. 'Both' blends root motion vectors from both children. |
| IkTargets | Behavior of IK targets blending |
| Optimization | Optimization may turn off evaluation of child nodes which have 0% influence. |
| SelectMainPath | When enabled, only the path with weight over 0.5 is marked as a main path. |

#### Blend N

Blend node N allows mixing results from multiple child nodes, each child node corresponds to one threshold value. Can be useful for example when blending between different directions of walking animations (0-360). Blending is immediate - based on a value or variables. Blending weight can change rapidly between frames, no time-based blending is done.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BlendWeight | Value which controls the blending between two surrounding thresholds. |
| BlendFn | Blend weight shaping (linear, s-curve, etc.) |
| Thresholds | Each child node corresponds to one threshold, thresholds must go from lower to higher values. |
| Children | Links to child nodes. |
| IsCyclic | Enable cyclic behavior (first threshold is identical to last, even with different values). Tick and add another Threshold at the end of Thresholds that corresponds to the first Input Link (for example, threshold would be -180,-90,0,90,180 when doing directional animations) |
| SelectMainPath | When enabled, only the path with weight over 0.5 is marked as a main path. |

#### Blend T

Provides blending between two child nodes over time. When a condition is met, Blend T starts blending towards the other child node over time.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BlendTime | Full blending time in seconds. |
| BlendFn | Blend weight shaping (linear, s-curve, etc.). |
| TriggerOn | Condition to start blending into secondary branch (if filled in, 'Condition' is ignored). |
| TriggerOff | Condition to start blending back into primary branch (if filled in, 'Condition' is ignored). |
| Condition | Condition to switch between primary and secondary branch. |
| Child0 | Primary child node |
| Child1 | Secondary child node |
| Optimization | Select when to evaluate child nodes. |
| SelectMainPath | When enabled, only the path with weight over 0.5 is marked as a main path. |
| PostEval | If post-eval is checked, triggers or condition are evaluated after child nodes. |
| IkTargets | Behavior of IK targets blending |

#### Blend T Add

Similarly to Blend T, this node provides blending over time, primary path always keeps playing as a main path. Useful for additive blending.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BlendTime | Full blending time in seconds. |
| BlendFn | Blend weight shaping (linear, s-curve, etc.). |
| TriggerOn | Condition to start blending into secondary branch (if filled in, 'Condition' is ignored). |
| TriggerOff | Condition to start blending out of secondary branch (if filled in, 'Condition' is ignored). |
| Condition | Condition to switch between primary and secondary branch. |
| Child0 | Primary child node |
| Child1 | Secondary child node |
| AdditiveAlways | When enabled, secondary child node is always evaluated regarless its current weight. |

#### Blend TW

Time dependent blend between two child nodes. Unlike BlendT which utilizes thresholds, Blend TW blends towards target value.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| TargetWeight | Target blending value, reached over time. |
| BlendTime | Time to reach target weight = 1 all the way from 0. |
| BlendFn | Blend weight shaping (linear, s-curve, etc.). |
| BlendTimeFn | Shaping of difference between target and current weight |
| Child0 | First child, having 100% influence when blending weight is 0. |
| Child1 | Second child, having 100% influence when blending weight is 1. |
| Optimization | Optimization may turn off evaluation of child nodes which have 0% influence. |
| SelectMainPath | When enabled, only the path with weight over 0.5 is marked as a main path. |
| IkTargets | Behavior of IK targets blending |
| OptimizeMin | When set to true and blending weight is zero, second child node is not evaluated |
| OptimizeMax | When set to true and blending weight is one, first child node is not evaluated |

#### Queue

Supports enqueuing and playing secondary child nodes and blending them with primary path.
This can be used for both additive and full body animations, where the full body will override the main link, and the additive animation will play on top of the main link.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Primary child node, always evaluated regardless queued actions. |
| QueueItems | List of queue items which can be played as secondary actions.  * **AnimSrcNodeQueueItem**   | Property | Description | | --- | --- | | Child | Child node of queue item. | | StartExpr | Condition to start playing this item | | InterruptExpr | Optional condition to interrupt this item | | StartTime | Optional starting time | | BlendInTime | Time to fully blend in | | BlendOutTime | Time to fully blend out | | EnqueueMethod | Behavior of item start - select if it is allowed to interrupt other items or self. | | TagMainPath | When this item is playing, this tag is set on the primary path of the queue node. | | CacheOnEnqueue | List of cached variable values and commands, frozen during enqueuing and passed unchanged to this playing item.  * **AnimSrcVarCmdTempStorage**   | Property | Description | | --- | --- | | Variables | *No description* | | Commands | *No description* | | |
| IkTargets | Behavior of IK targets blending |

#### Switch

Plays child nodes and switches to next one randomly, once they finish playing.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BlendTime | Transition time between child nodes. |
| FirstProbabilities | List of numbers separated by commas or spaces, representing probability per node. |
| SwitchItems | List of child nodes and their probability tables.  * **AnimSrcNodeSwitchItem**   | Property | Description | | --- | --- | | Child | Link to child node | | NextProbabilities | List of numbers separated by commas or spaces, representing probability per node. | |

### Buffer

#### Buffer Save

Saves contents of current pose buffer in the node to named temporary storage.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| BufferName | Storage name |
| BoneMask | Inclusive mask restricting saved bones. If no mask is selected, all bones are saved. |

#### Buffer Use

Restore previously saved pose buffer from temporary storage.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BufferName | Storage name |
| BoneMask | Inclusive mask restricting restored bones. If no mask is selected, all bones are used. |

#### Filter

Filter current pose buffer by bone mask.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| BoneMask | Inclusive mask restricting bones kept in the pose buffer. If no mask is selected, all bones are kept. |
| Condition | Expression enabling this filter |

### Ctx

#### Ctx Begin

Ctx Begin and Ctx End surround branched block of the graph, protecting subgraph below Ctx End against losing its state during branch switches the state of subgraph is bypassed and safely stored in Ctx Begin. Traversing through Ctx End is allowed only for the first branch which reaches it, other branches inside the block receive cached result.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node. First node in the branched block. |

#### Ctx End

Ctx Begin and Ctx End surround branched block of the graph, protecting subgraph below Ctx End against losing its state during branch switches the state of subgraph is bypassed and safely stored in Ctx Begin. Traversing through Ctx End is allowed only for the first branch which reaches it, other branches inside the block receive cached result.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node. Subgraph below the branched block of the graph. |

### Event

#### Event

Fires animation events when the node is activated or condition is fulfilled.
This is useful when we need to signal other parts of engine or game code that a branch in animation graph became active (for example, audio engine can react to stance changes).

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node. |
| Events | List of events and their conditions  * AnimSrcEvent   | Property | Description | | --- | --- | | Name | Event identifier | | Comment | Any explaining messages belong here. Ignored in runtime. | | MainPathOnly | When enabled, event is sampled only on main path | | InitOnly | Event is sampled just on node init (Event Node only) | | Condition | When filled in, event is sampled only if this condition results true. (Event Node only) | | Once | When enabled, this events triggers only on first frame the condition is valid. Re-trigger is possible only after a frame the condition evaluates to false (Event Node only). | | Frame | Frame number of the event start (Animation only) | | FrameCount | Duration of the event (Animation only) |   * **AnimSrcEventGeneric** : AnimSrcEvent   | Property | Description | | --- | --- | | UserString | Additional string attribute | | UserInt | Additional integer attribute |   * **AnimSrcEventAudio** : AnimSrcEvent |

### Function

#### Function Begin

Function Begin and Function end surround graph block which can be called (reused) on multiple other places.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node. First node inside the function block. |
| EndsCount | Number of function end points (FunctionEnd nodes) |

#### Function Call

Redirects evaluation into a function - a block of nodes starting with Function Begin. After the evaluation inside the block reaches any Function End, evaluation resumes at this place.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Method | Function name (Function Begin node name) |
| Child0 | *No description* |
| Child1 | *No description* |
| Child2 | *No description* |
| Child3 | *No description* |
| Child4 | *No description* |
| Child5 | *No description* |
| Child6 | *No description* |
| Child7 | *No description* |

#### Function End

Function Begin and Function end surround graph block which can be called (reused) on multiple other places.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| EndIndex | Index of a following child node at call site. |

### Group Select

#### Group Select

Selects an active column in anim set group. Animation and constants identifiers may then omit column name because it is selected by this node.
Example: Group select node selects column "Erc" in group "Locomotion".
Source node below refers to animation "Locomotion.Walk" but active column is filled in, resulting in unique animation identifier "Locomotion.Erc.Walk".

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Group | Anim set group containing the column |
| Column | Anim set column within the group to be selected |

### IK

#### IK2

Node which applies inverse kinematics on a chain of bones to reach a specific target.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Weight | Weight (0-1). When zero, no IK is applied, when one, IK is fully applied. |
| Chains | List of chains and their IK targets.  * **AnimSrcIkBinding**   | Property | Description | | --- | --- | | IkTarget | IK target name | | IkChain | IK chain name | |
| Solver | Select an instance of IK solver to run.  * **AnimSrcNodeIK2Solver** * **AnimSrcNodeIK2FabrikSolver** : AnimSrcNodeIK2Solver Iterative solver with basic rotation limits.   | Property | Description | | --- | --- | | SnapRotation | Snap rotation of end effector to align with IK target. | | RotationLimitPerBone | Maximum rotation [degrees] applied per bone in chain | | TightenToTarget | Percentage of chain where rotation limit tightens to zero | | TwistPropagation | Percentage of chain receiving twist from the snapped rotation | | MaxIterations | Maximum number of iterations when the solver stops even if the target is not reached |   * **AnimSrcNodeIK2TwoBoneSolver** : AnimSrcNodeIK2Solver Analytic two-bone solution. If the chain contains more than two bones, it is split into two segments by its mid-joint.   | Property | Description | | --- | --- | | SnapRotation | Snap rotation of end effector to align with IK target. | | AllowStretching | Allow chain stretching when the IK target is out of reach. | | MidPivotTwist | Additional twist of mid pivot plane in radians. |   * **AnimSrcNodeIK2LookAtSolver** : AnimSrcNodeIK2Solver Solver which rotates the chain that its end effector is facing the target.   | Property | Description | | --- | --- | | Axis | Axis to aim at target. |   * **AnimSrcNodeIK2LookInDirSolver** : AnimSrcNodeIK2Solver Solver which rotates the chain that its end effector is aligned with selected axis of the target.   | Property | Description | | --- | --- | | Axis | Axis to aim in target's direction. | | TargetAxis | Target axis chosen as its direction. |   * **AnimSrcNodeIK2PoleSolver** : AnimSrcNodeIK2Solver Solver rotating around twist axis, aligning mid-joint with the target. |

#### IK2 Plane

Computes inverse kinematics on a chain to reach target plane. Target planes are set by application (character/gameplay programmers).

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Weight | Weight (0-1). When zero, no IK is applied, when one, IK is fully applied. |
| Chains | List of chains and their IK plane targets.  * AnimSrcIkBinding   | Property | Description | | --- | --- | | IkTarget | IK target name | | IkChain | IK chain name | |
| ActiveDistance | Thresholding distance [m]. If the end effector is further, no IK is applied even when weight is one. In meters. |
| ThresholdSmoothness | Smoothing region [m] to turn on IK continuously. |
| SnapRotation | Snap the rotation of end effector to be the same as ik target rotation. |
| CustomNormal | When zero, IK2 Plane pushes the end effector along the normal of plane. In some situations it may be feasible to move it in a custom direction, for example pushing up when correcting feet by IK. |

#### IK2 Target

Creates an ik target from model-space position of a bone or end effector of a chain.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Chains | List of ik targets to be created and assigned transform from specified chains. Additional offsets might be applied.  * **AnimSrcIkTargetBinding**   | Property | Description | | --- | --- | | IkTarget | IK target name | | IkChain | Chain name | | LocalSpaceOffset | Offset applied to chain in local space (affected by parent transforms) |   * **AnimSrcTransformOffset**   | Property | Description | | --- | --- | | Position | Position offset | | Rotation | Rotation offset | | Pose | Source id of an animation pose. This pose is used to set the local offset from chain's end effector. Serves probably better than guessing transform numbers. Note: column/group selects apply here same way as in source nodes. Example: when we were constraining left hand to first person camera, we exported a pose with two bones in world space - head and left hand. Then these two bones describe the offset where to place an IK target. | | FromBone | Offset is measured from bone having this name... | | ToBone | Offset is measured to bone having this name. | | FromFrame | Offset is measured from bone sampled on this frame | | ToFrame | Offset is measured to bone sampled on this frame. | | BonesAreInModelSpace | When checked, the pose is in model space and does not need to accumulate bones from local space. | |
| ModelSpaceOffset | Offset applied to chain in model space (unaffected by parent transforms)  * **AnimSrcTransformOffset**   | Property | Description | | --- | --- | | Position | Position offset | | Rotation | Rotation offset | | Pose | Source id of an animation pose. This pose is used to set the local offset from chain's end effector. Serves probably better than guessing transform numbers. Note: column/group selects apply here same way as in source nodes. Example: when we were constraining left hand to first person camera, we exported a pose with two bones in world space - head and left hand. Then these two bones describe the offset where to place an IK target. | | FromBone | Offset is measured from bone having this name... | | ToBone | Offset is measured to bone having this name. | | FromFrame | Offset is measured from bone sampled on this frame | | ToFrame | Offset is measured to bone sampled on this frame. | | BonesAreInModelSpace | When checked, the pose is in model space and does not need to accumulate bones from local space. | |
| Bones | List of ik targets to be be created and assigned transform from specified model-space transforms of bones.  * **AnimSrcIkTargetBinding**   | Property | Description | | --- | --- | | IkTarget | IK target name | | Bone | Measured offset from bone of this name... | |

#### IK Lock

Animation node locking the ik target in place and applying two-bone IK solver.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| IsLocked | Expression enabling the lock. |
| BlendinTime | Blend time for IK to fully take over. |
| BlendoutTime | Blend time for IK to be fully released. |
| SnapRotation | Snap the rotation of end effector to be the same as ik target rotation. |
| World | Name of world transform delta passed by the game. This can be used to lock targets onto moving ground, for example. |
| Chains | List of IK targets and affected chains  * **AnimSrcIkBinding**   | Property | Description | | --- | --- | | IkTarget | IK target name | | IkChain | IK chain name | |

#### IK Rotation

Animation node applying inverse kinematics to reach target rotation, local translations are untouched.
When enabled, rotation of end-effector is identical to IK target, parent bones receive a fraction of the rotation.
Useful for spine inverse kinematics instead of counter-animating hip rotations.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Weight | Weight (0 - 1). When zero, no rotation changes are applied, when one, rotations are fully applied. |
| Chains | List of IK targets and affected chains  * **AnimSrcIkBinding**   | Property | Description | | --- | --- | | IkTarget | IK target name | | IkChain | IK chain name | |

#### RBF

*No description*

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | *No description* |
| Rbf | *No description* |

#### Weapon IK

Specialized node positioning arms according to aim direction and applying 2-joint ik on arms to snap to weapon.
Algorithm: Compares current weapon direction with desired direction. Rotate weapon to correct direction. Snaps primary and secondary chain (arms) onto weapon.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| IkconfigName | Name of the Weapon IK configuration |
| Ikconfig | Configuration. Exposes IK pose channels to solver. Defines primary and secondary chain and their orientation. |
| BlendInTime | Blending time to fully enable 'Aim On', 'Prim On' and 'Sec On'. |
| BlendOutTime | Blending time to fully disable 'Aim On', 'Prim On' and 'Sec On'. |
| WeightAim | Enable aiming correction. |
| WeightPrimaryChain | Enable IK on primary chain (right arm). |
| WeightSecondaryChain | Enable IK on secondary chain (left arm). |
| WeaponDirLr | Horizontal weapon aiming angle, used when Aim On=1 |
| WeaponDirUd | Vertical weapon aiming angle, used when Aim On=1 |
| WeaponRoll | Weapon roll, used when Aim On=1 |
| WeaponTransX | Additional weapon translation in model-space X direction |
| WeaponTransY | Additional weapon translation in model-space Y direction |
| WeaponTransZ | Additional weapon translation in model-space Z direction |

### Memory

#### Memory

Allows to save several bones into the local memory and fill them in on later frames if they are not present in pose buffer anymore.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BoneMask | Bone mask with a list of remembered bones |
| Child | Link to child node |

### Procedural

#### Constraint

*No description*

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Weight | Weight of the applied constraint |
| Constraint | Select a constraint to be applied  * **AnimSrcConstraint**   | Property | Description | | --- | --- | | AffectedBone | Name of bone affected by the constraint. If filled in, affected IK target must remain empty. | | AffectedIk | Name of IK target affected by the constraint. If filled in, affected bone must remain empty. | | TargetBone | Target bone for the constraint solver. If filled in, target IK target must remain empty. | | TargetIk | Target IK target for the constraint solver. If filled in, target bone must remain empty. | | InitPose | Constraint can be initialized from current or bind pose. | | CustomSpace | If filled in, constraint is computed in custom space  * **AnimSrcConstraintCustomSpace**   | Property | Description | | --- | --- | | AffectedSpace | *No description* | | AffectedPivotBone | *No description* | | AffectedPivotIk | *No description* | | TargetSpace | *No description* | | TargetPivotBone | *No description* | | TargetPivotIk | *No description* | |   * **AnimSrcConstraintPosition** : AnimSrcConstraint Constraint which can fixate translation offset between two bones (or ik targets). The offset can also decay over time.   | Property | Description | | --- | --- | | OffsetSoftLimit | Threshold which where the limit distance starts to be applied softly. Must be less than hard limit. | | OffsetHardLimit | Maximum distance travelled from original position. | | OffsetDampening | Rate of decay from offset hard limit to offset soft limit. Zero means immediate dampening, one means no dampening. | | Components | Influence per component X, Y, Z. |   * **AnimSrcConstraintParent** : AnimSrcConstraint Constraint which can fixate translation and rotation offset between two bones (or ik targets).   | Property | Description | | --- | --- | | RotateFirst | Rotate affected around target first before resolving position. | |

#### Procedural

AnimNodeProcTransform applies procedural transform on a set of joints or IK targets through entered expressions.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| Expression | Applied amount multiplier. |
| Bones | List of operations on bones  * AnimSrcNodeProcTrBoneItem   | Property | Description | | --- | --- | | Bone | Name of affected bone | | Axis | Translation/rotation axis | | Space | Proc transform can be done in model-space, local-space or pivot-space. | | Op | Operation to perform (translation, rotation, ...) | | Amount | Applied amount, multiplied by shared Expression in the node | | PivotBone | Pivot bone, used when operating in pivot-space | | PivotIk | Pivot IK target, used when operating in pivot-space | |
| IkTargets | List of operations on IK targets  * **AnimSrcNodeProcTrIkTargetItem**   | Property | Description | | --- | --- | | IkTarget | Name of affected IK target | | Axis | Translation/rotation axis | | Space | Proc transform can be done in model-space, local-space or pivot-space. | | Op | Operation to perform (translation, rotation, ...) | | Amount | Applied amount, multiplied by shared Expression in the node | | PivotBone | Pivot bone, used when operating in pivot-space. | | PivotIk | Pivot IK target, used when operating in pivot-space | |

### Sleep

#### Sleep

Sleep node can turn off evaluation of all its child nodes and return empty result instead, drastically saving performance.
The node goes to sleep after its awake condition is not fulfilled during entire timeout period. In an ideal case, it should be used as a starting node - when sleeping, many processes inside graph evaluation are skipped early.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| AwakeExpr | When false, node goes to sleep mode after a timeout, effectively turning off child nodes. |
| Timeout | Timeout before going to sleep mode. Awake expression resets the timeout. |

### Source

#### Bind Pose

Sample bind pose of the animated mesh.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| BoneMask | Inclusive bone mask restricts which bones are sampled. If no mask is specified, all bones are sampled. |

#### Pose

Sample a pose from an animation.
Unlike Source node, Pose node does not play the animation, sampled frame is computed from an expression. Pose node blends between two closest frames when the variable value falls between them.
Example usage: steering in a vehicle, having all steering poses in one animation and using a variable to select a pose to be played

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Source | Animation source id in format Group.Column.Animation or Group.Animation when relying on Group Select nodes. |
| Time | Normalized sampling time (0 - 1) |

#### Pose 2

Samples a pose from an animation based on control expressions and 2D table.
Similar to plain Pose node, but it works in two-dimensional space.
Example usage: Aim Spaces and Look Animations - mapping horizontal and vertical direction to a pose.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Source | Animation source id in format Group.Column.Animation or Group.Animation when relying on Group Select nodes. |
| ValueX | Expression for the X coordinate |
| ValueY | Expression for the Y coordinate |
| Table | Table remapping X and Y coord to frames |

#### Source

Plays an animation from anim set. Playback can be looped.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Source | Animation source id in format Group.Column.Animation or Group.Animation when relying on Group Select nodes. |
| Looptype | Enables looped playback. |
| Interpolate | When enabled, keyframes are interpolated during playback. |
| Predictions | List of sampled predictions (future bone transforms in the animation)  * AnimSrcNodePrediction   | Property | Description | | --- | --- | | Name | Prediction name | | Bone | Name of channel to be sampled. | | Event | Bind the time of this prediction to an event with matching name. | | PercentTime | Normalized time of the prediction [0 - 1]. | | Translation | Select when translation should be sampled. | | Rotation | Select when rotation should be sampled. | | MainPath | Select when the prediction should be sampled only on graph's main path. | |
| BonesInterpolatedInModelSpace | List of bones interpolated in model-space, which be required to prevent stutters caused by inheriting all interpolations in hierarchy. |
| ReportDistance | Using root motion channel, report sample distance travelled until end of cycle (when looped) or animation (when unlooped). |

#### Source InLoopOut

Plays an animation from anim set. The animation is divided by events into three segments: intro, loop, outro. Playback leaves the looped region only a condition is fulfilled.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Source | Animation source id in format Group.Column.Animation or Group.Animation when relying on Group Select nodes. |
| InEvent | Name of event marking the end of intro and start of looped region. |
| OutEvent | Name of event marking the end of looped region and start of outro. |
| EndExpression | If the expression results in true, further looping is disabled, allowing playback to reach the end. |

#### Source Sync

Plays an animation from anim set using normalized time. Playback can be looped.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Source | Animation source id in format Group.Column.Animation or Group.Animation when relying on Group Select nodes. |
| Looptype | Enables looped playback. |
| Predictions | List of sampled predictions (future bone transforms in the animation)  * **AnimSrcNodePrediction**   | Property | Description | | --- | --- | | Name | Prediction name | | Bone | Name of channel to be sampled. | | Event | Bind the time of this prediction to an event with matching name. | | PercentTime | Normalized time of the prediction [0 - 1]. | | Translation | Select when translation should be sampled. | | Rotation | Select when rotation should be sampled. | | MainPath | Select when the prediction should be sampled only on graph's main path. | |
| SyncLine | Name of sync line which maps normalized time to events in the animation. |
| ReportDistance | Using root motion channel, report sample distance travelled until end of cycle (when looped) or animation (when unlooped). |

### State Machine

#### State

Represents one state inside state machine node, serves as a bridge from state machine to other nodes. State node cannot exist on its own.

| Property | Description |
| --- | --- |
| Child | Link to child node |
| StartCondition | Evaluated on state machine initialization, first state returning true becomes active |
| TimeStorage | By default, time is 'inherited' from parent nodes. Change if you want to measure time locally. |
| IsExit | Enable to pass remaining time from child node to parent of state machine. Disable to mask out the remaining time. |
| PassThrough | State machine will try to leave this state even if it was just entered. Child nodes are still evaluated. Pass-through is ignored in post-eval. |

#### State Machine

State machines contain multiple state nodes and transitions between them.
One of the states is marked 'active'. On every frame, state machine evaluates transitions leading from the active state and if their condition is fulfilled, target state of the transition becomes the new 'active' state.
Previous state keeps playing for a short 'blend-out' time, if specified by transition.

| Property | Description |
| --- | --- |
| States | List of state nodes inside state machine.  * **State** Represents one state inside state machine node, serves as a bridge from state machine to other nodes. State node cannot exist on its own.   | Property | Description | | --- | --- | | Child | Link to child node | | StartCondition | Evaluated on state machine initialization, first state returning true becomes active | | TimeStorage | By default, time is 'inherited' from parent nodes. Change if you want to measure time locally. | | IsExit | Enable to pass remaining time from child node to parent of state machine. Disable to mask out the remaining time. | | PassThrough | State machine will try to leave this state even if it was just entered. Child nodes are still evaluated. Pass-through is ignored in post-eval. | |
| Transitions | List of transitions inside state machine.  * **AnimSrcNodeTransition**   | Property | Description | | --- | --- | | FromState | Source state | | ToState | Target state | | Duration | Transition blending time | | StartTime | Start time to be set on the target state once transition triggers | | Condition | Once true, state machine will set target state as its new active state. | | BlendFn | Blend weight shaping (linear, s-curve, etc.). | | PostEval | Post-eval transition is evaluated in bottom-up phase of this part of animation graph. | | Reentering | *No description* | | Priority | Set the order of evaluating transition conditions. Lower value is evaluated earlier. | | MotionVecBlend | Root motion blending options. | | Events | Fired event once this transition is triggered  * **AnimSrcEvent**   | Property | Description | | --- | --- | | Name | Event identifier | | Comment | Any explaining messages belong here. Ignored in runtime. | | MainPathOnly | When enabled, event is sampled only on main path | | InitOnly | Event is sampled just on node init (Event Node only) | | Condition | When filled in, event is sampled only if this condition results true. (Event Node only) | | Once | When enabled, this events triggers only on first frame the condition is valid. Re-trigger is possible only after a frame the condition evaluates to false (Event Node only). | | Frame | Frame number of the event start (Animation only) | | FrameCount | Duration of the event (Animation only) |   * **AnimSrcEventGeneric** : AnimSrcEvent   | Property | Description | | --- | --- | | UserString | Additional string attribute | | UserInt | Additional integer attribute |   * **AnimSrcEventAudio** : AnimSrcEvent | | Tags | Tags set on the output during this transition. | |

### Tag

#### Tag

Adds a tag to animation output. The node can also temporarily add tags just for child nodes.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| ChildTags | List of tags passed to the child nodes but not to the parent nodes. |

### Time

#### Time Save

Anim graph node saving time into stash.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| TimeName | Storage name |

#### Time Scale

AnimNodeTimeScale scales the input time, effectively slowing down or speeding up animations in the subtree below.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| TimeExpr | Multiplier remapping time. For example, 2.0 will cause the subtree to run twice as fast. |
| TimeStorage | By default, time is 'inherited' from parent nodes. Change if you want to measure time locally. |
| TimeMappingTable | When filled in, result from TimeExpr is remapped through this table to new actual value of the multiplier. |

#### Time Use

Anim graph node restoring time from stash.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| TimeName | Storage name |
| TimeType | Specify type of restored time - either real or normalized. |

### Variables

#### Var Reset

Anim Node Var Set - sets variable values when defined conditions are met

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| VarReset | List of variables to be reset to original game input  * **AnimSrcNodeVarResetItem**   | Property | Description | | --- | --- | | VariableName | *No description* | | SetOnInit | When enabled, variable is set on init (first frame) | | SetOnMainPath | When enabled, variable is set every frame on main path | | SetOnBlendOut | When enabled, variable is set every frame on secondary path (when blending out) | |

#### Var Set

Sets new value to variables when conditions are met. Changes apply only to child nodes.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| VarSet | List of variables to be set to new value. Value is restored once returning to parent node.  * **AnimSrcNodeVarSetItem**   | Property | Description | | --- | --- | | VariableName | Variable name | | SetOnInit | When enabled, variable is set on init (first frame) | | SetOnMainPath | When enabled, variable is set every frame on main path | | SetOnBlendOut | When enabled, variable is set every frame on secondary path (when blending out) | | PostEval | When enabled, value is updated after child nodes, stored and applied for the subtree on next frame. |   * **AnimSrcNodeVarSetBoolItem** : AnimSrcNodeVarSetItem Change value of bool variable. Changes apply only to child nodes.   | Property | Description | | --- | --- | | Value | New bool value |   * **AnimSrcNodeVarSetIntItem** : AnimSrcNodeVarSetItem Change value of int variable. Changes apply only to child nodes.   | Property | Description | | --- | --- | | Value | New int value |   * **AnimSrcNodeVarSetFloatItem** : AnimSrcNodeVarSetItem Change value of float variable. Changes apply only to child nodes.   | Property | Description | | --- | --- | | Value | New float value |   * **AnimSrcNodeVarSetStringItem** : AnimSrcNodeVarSetItem Change value of string variable. Changes apply only to child nodes.   | Property | Description | | --- | --- | | Value | New string value | |

#### Var Update

Limit control variable updates by conditions, limit change rate. Changes apply only to child nodes.
Example Usage: Freeze movement direction variable when blending out from a branch.

| Property | Description |
| --- | --- |
| Tags | Tags set to animation output. |
| NodeGroup | Name of parent node group, visual and logical aid. |
| Child | Link to child node |
| VarUpdate | List of variables with limited update rate. Limits apply only to child nodes.  * **AnimSrcNodeVarUpdateItem**   | Property | Description | | --- | --- | | VariableName | Identifier | | UpdateOnInit | When enabled, variable is updated on init (first frame) | | UpdateOnMainPath | When enabled, variable is updated every frame on main path | | UpdateOnBlendOut | When enabled, variable is updated every frame on secondary path (when blending out) | | PostEval | When enabled, value is updated after child nodes, stored and applied for the subtree on next frame. | | Condition | When filled in, variable is updated only when this condition is fulfilled. | | MaxDifferencePerSecond | Maximum difference of value per second. Variable value will not change faster below this node. Float variables only. | | IsCyclic | When enabled, variable might blend towards target 'the other way around' its range. Useful for example for motion direction angle. | |

## See Also

* [Animation Editor](/wiki/Arma_Reforger:Animation_Editor "Arma Reforger:Animation Editor")
* [Animation Editor: State Machine](/wiki/Arma_Reforger:Animation_Editor:_State_Machine "Arma Reforger:Animation Editor: State Machine")
* [Animation Editor: Sync Tutorial](/wiki/Arma_Reforger:Animation_Editor:_Sync_Tutorial "Arma Reforger:Animation Editor: Sync Tutorial")
