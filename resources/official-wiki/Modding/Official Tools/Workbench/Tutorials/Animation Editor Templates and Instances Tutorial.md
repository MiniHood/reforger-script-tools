# [Animation Editor: Templates and Instances Tutorial](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Templates_and_Instances_Tutorial)

In this article I will be explaining how you will utilise the animation template and animation instances in your workspaces.

The essence of Templates and Instances is that the **Animation** **Template** holds the **generic lines** in which the **Animation Instances** fills in.

## Templates and Instances

When you first create your workspace, you will have this visible in your Workspace tab:

Right clicking on **Anim Template** will allow you to create one, which will be the basis of your animation system. Name this **player\_template**. You will need a template before you can create an instance, doing so in the same manner, and name it **player\_unarmed**.

## Preview Model

In order for you to preview your animations, you will need to right click on **Preview Models** and add your player model, otherwise you might get an error message, or worse.

## Filling it in

Once you have created the template and at least one instance, you will need to add one of your animations to the **Animation Sets** window. Open the **File Browser** and look for the animation you wish to add, select it and then click the **F->L** (File Browser -> Line) button.

This will create the first line of your animation template, which can now be referenced in the graph nodes. However, this does not allow you to utilise the full functionality of the graph in the end. By right clicking on the line you can **Create Line**, which allows you to have a generic name for, i.e. the player's walk forward, but in all stances. Simply, create the new line and name it **WalkF**.

You will notice that the new line has a red circle because it is missing an animation, whilst the line for standing walk forward does have an animation. Now, right click on the WalkF line and click **Group Animations**, which will create a new group and add that line to that group. Name it **Locomotion.** You can delete the old line with the animation now, as it is not needed. The WalkF line has been grouped but the group has an **Undefined column,** simply right click on it to rename it, and rename it to something like **Erc** (short for Erected):

After naming it, right click on the **Locomotion** group (not the column) to create another column, this time naming it **Cro** (Crouched). Do the same thing for **Pne** (Prone). This is where you should be at this point:

However, the columns needs to have an assigned animation to them, so simply click on the red dot under the **Cro Column,** and then select the **Crouched WalkF** animation in the file browser and press **Set Anim:**

This will make the dot turn green, signalling that it now has an animation. Repeat this for **Erected** and **Prone WalkF**. You can add as many columns as you would like. For the DayZ player, we have **Cro, CroRas, Erc, ErcRas, Pne, PneRas** for stances and raised weapon stances, and using the Group Select makes getting the right animations from one group with multiple columns a breeze.

ⓘ

Read more on the group select node in the [Animation Editor: Nodes](/wiki/Arma_Reforger:Animation_Editor:_Nodes "Arma Reforger:Animation Editor: Nodes") article.

## The power of Instances

However, none of this explains why one template can have multiple instances.

You have multiple guns (**instances**), that have multiple stances (**columns**), sharing the same lines (**template**) and nodes in the graph.

They would all share the same lines and columns, **but they don't have the same animations.** By creating more instances, you can one **Reload Line** in your **Animation Template,** but switch what animations are slotted into the **Template's Columns** inside of each of your **Animation Instances.** Simply put, the Animation Template contain generic and immutable lines which then are dynamically filled by Animation Instances.

«

« One Template with one graph, multiple instances with multiple animations. »
