# [Animation Editor: Character Action Commands](https://community.bistudio.com/wiki/Arma_Reforger:Animation_Editor:_Character_Action_Commands)

## Commands

### CMD\_Weapon\_Reload

* 0 - none
* 1 - rack bolt
* 2 - attach the magazine, without racking the bolt
* 3 - attach the magazine, with racking the bolt
* 4 - detach + attach the magazine, without racking the bolt
* 5 - detach + attach the magazine, with racking the bolt
* 6 - detach mag (no attachment will follow)
* 7 - insert a single projectile (rn for UGLs)
* 8 - remove a single projectile (rn for UGLs)
* 9 - remove all projectiles and then reinsert them (on UGLs)
* 10 - skip all animations and reload

| Int | Float | Action |
| --- | --- | --- |
| 1 | 0.0 | Racks the bolt on the weapon |
| 2 | N/A | Inserts a mag into the gun |
| 3 | 0.0 | Inserts a mag into the gun, and then racks the bolt |
| 4 | N/A | Removes the current magazine, and inserts a new one |
| 5 | 0.0 | Removes the current magazine, and inserts a new one, then racks the bolt |
| 1 | 1.0 | Releases the bolt on the weapon |
| 3 | 1.0 | Inserts a mag into the gun, and then releases the bolt |
| 5 | 1.0 | Removes the current magazine, and inserts a new one, then releases the bolt |
