# [Scripting: Operators](https://community.bistudio.com/wiki/Arma_Reforger:Scripting:_Operators)

**Operators** are elements used to generate operations between two values.

**Contents**

* 1 [Precedence](#Precedence)
* 2 [Types](#Types)
  + 2.1 [Assignment](#Assignment)
  + 2.2 [Arithmetic](#Arithmetic)
  + 2.3 [Relational](#Relational)
  + 2.4 [Logical](#Logical)
  + 2.5 [Bitwise](#Bitwise)
  + 2.6 [String](#String)
  + 2.7 [Indexing](#Indexing)

## Precedence

Operators precedence (one having execution priority over another) in Enforce Script is identical to the C language:

Show Enfusion Operator Precedence table

| Precedence | Operator | Description | Associativity |
| --- | --- | --- | --- |
| 1 | ENFORCECODEMARKER   ``` ++ ``` | Postfix increment | Left-to-right |
| ENFORCECODEMARKER   ``` -- ``` | Postfix decrement |
| ENFORCECODEMARKER   ``` () ``` | Function call |
| ENFORCECODEMARKER   ``` [] ``` | Array subscripting |
| ENFORCECODEMARKER   ``` . ``` | Element selection by reference |
| 2 | ENFORCECODEMARKER   ``` ++ ``` | Prefix increment | Right-to-left |
| ENFORCECODEMARKER   ``` -- ``` | Prefix decrement |
| ENFORCECODEMARKER   ``` + ``` | Unary plus |
| ENFORCECODEMARKER   ``` - ``` | Unary minus |
| ENFORCECODEMARKER   ``` ! ``` | Logical NOT |
| ENFORCECODEMARKER   ``` ~ ``` | Bitwise NOT |
| ENFORCECODEMARKER   ``` new ``` | Dynamic memory allocation |
| ENFORCECODEMARKER   ``` delete ``` | Dynamic memory deallocation |
| 3 | ENFORCECODEMARKER   ``` * ``` | Multiplication | Left-to-right |
| ENFORCECODEMARKER   ``` / ``` | Division |
| ENFORCECODEMARKER   ``` % ``` | [Modulo](https://en.wikipedia.org/wiki/Modulo_operation) (remainder) |
| 4 | ENFORCECODEMARKER   ``` + ``` | Addition | Left-to-right |
| ENFORCECODEMARKER   ``` - ``` | Subtraction |
| 5 | ENFORCECODEMARKER   ``` << ``` | [Bitwise](https://en.wikipedia.org/wiki/Bitwise_operation) left shift | Left-to-right |
| ENFORCECODEMARKER   ``` >> ``` | [Bitwise](https://en.wikipedia.org/wiki/Bitwise_operation) right shift |
| 6 | ENFORCECODEMARKER   ``` < ``` | Less than | Left-to-right |
| ENFORCECODEMARKER   ``` <= ``` | Less than or equal to |
| ENFORCECODEMARKER   ``` > ``` | Greater than |
| ENFORCECODEMARKER   ``` >= ``` | Greater than or equal to |
| 7 | ENFORCECODEMARKER   ``` == ``` | Equal to | Left-to-right |
| ENFORCECODEMARKER   ``` != ``` | Not equal to |
| 11 | ENFORCECODEMARKER   ``` & ``` | Bitwise AND | Left-to-right |
| 12 | ENFORCECODEMARKER   ``` ^ ``` | Bitwise XOR (e**x**clusive **or**) | Left-to-right |
| 13 | ENFORCECODEMARKER   ``` | ``` | Bitwise OR (inclusive or) | Left-to-right |
| 14 | ENFORCECODEMARKER   ``` && ``` | Logical AND | Left-to-right |
| 15 | ENFORCECODEMARKER   ``` || ``` | Logical OR | Left-to-right |
| 8 | ENFORCECODEMARKER   ``` = ``` | Direct assignment | Right-to-left |
| ENFORCECODEMARKER   ``` += ``` | Assignment by sum |
| ENFORCECODEMARKER   ``` -= ``` | Assignment by difference |
| ENFORCECODEMARKER   ``` *= ``` | Assignment by product |
| ENFORCECODEMARKER   ``` /= ``` | Assignment by quotient |
| ENFORCECODEMARKER   ``` %= ``` | Assignment by remainder |
| ENFORCECODEMARKER   ``` <<= ``` | Assignment by bitwise left shift |
| ENFORCECODEMARKER   ``` >>= ``` | Assignment by bitwise right shift |
| ENFORCECODEMARKER   ``` &= ``` | Assignment by bitwise AND |
| ENFORCECODEMARKER   ``` ^= ``` | Assignment by bitwise XOR |
| ENFORCECODEMARKER   ``` |= ``` | Assignment by bitwise OR |

[↑ Back to spoiler's top](#bikisp6a6366226a6af)

or see the [Wikipedia article section](https://en.wikipedia.org/wiki/Operators_in_C_and_C%2B%2B#Operator_precedence).

## Types

### Assignment

#### =

The "assign" operator sets the righthand value to the lefthand variable.

```enforce
int five = 5;
```

⚠

This operator is **NOT** an equality check. For this, see [==](#==).

#### ++

The increment (or post-increment) operator adds 1 to the provided numerical variable. It only applies to int and float values.

```enforce
int result = 5;
result++; // result is 6
```

This operator can be used *before* the value as well (it is then called *pre-increment* operator):

```enforce
int result = 5;
++result; // result is 6
```

The difference is that the *returned* value is incremented too:

|  |  |
| --- | --- |
| ENFORCECODEMARKER   ``` int result = 5; Print(result++); // outputs 5, result is 6 ``` | ENFORCECODEMARKER   ``` int result = 5; Print(++result); // outputs 6, result is 6 ``` |

#### --

The decrement (or post-decrement) operator removes 1 to the provided numerical variable. It only applies to int and float values.

```enforce
int result = 5;
result--; // result is 4
```

Same as [++](#++), this operator can be used *before* the value (then called *pre-decrement* operator):

```enforce
int result = 5;
--result; // result is 4
```

|  |  |
| --- | --- |
| ENFORCECODEMARKER   ``` int result = 5; Print(result--); // outputs 5, result is 4 ``` | ENFORCECODEMARKER   ``` int result = 5; Print(--result); // outputs 4, result is 4 ``` |

#### +=

The "add to" operator adds the righthand value to the lefthand variable.

```enforce
int result = 5;
result += 3; // result is 8
```

#### -=

The "subtract from" operator subtract the righthand value to the lefthand variable.

```enforce
int result = 5;
result -= 3; // result is 2
```

#### \*=

The "multiply with" operator multiplies the lefthand variable with the righthand value.

```enforce
int result = 5;
result *= 3; // result is 15
```

#### /=

The "divide by" operator multiplies the lefthand variable with the righthand value.

```enforce
int result = 5;
result /= 3; // result is 1 (floored int value)
```

#### <<=

The "left-offset by" operator offsets the lefthand variable to the left by the righthand value.

```enforce
int result = 0x0000FF32;
result <<= 8; // result is 0x00FF3200
```

#### >>=

The "right-offset by" operator offsets the lefthand variable to the right by the righthand value.

```enforce
int result = 0x00FF3200;
result >>= 8; // result is 0x0000FF32
```

⚠

Beware of [sign extension](https://en.wikipedia.org/wiki/Sign_extension):

```enforce
int result = 0x80000000;
result >>= 8; // result is 0xFF800000
```

### Arithmetic

#### +

he "adds" operator does an addition between two numeric values, float and/or integer.

```enforce
5 + 3; // result is 8 (int)
5 + 2.5; // result is 7.5 (float)
3.25 + 1.75; // result is 5.0 (float)
```

#### -

The "subtract from" operator does a subtraction between two numerical values, float and/or integer.

```enforce
5 - 3; // result is 2 (int)
5 - 2.5; // result is 2.5 (float)
3.25 - 1.25; // result is 2.0 (float)
```

#### \*

The multiplication sign does a multiplication between two numerical values, float and/or integer.

```enforce
5 * 3; // result is 15
5 * 2.5; // result is 12.5 (float)
1.5 * 3.25; // result is 4.875 (float)
```

#### /

The "divided by" operator divides the left argument by the right one.  
It can be applied to int, float or a mix of those.
If both values are int, the return value will be a **floored** integer as well.

```enforce
5 / 3; // result is 1 (int): 5/3 = 1.666(...), but integer floors the value
5 / 2.5; // result is 2.0 (float)
3.5 / 0.5; // result is 7.0 (float)
```

#### %

Modulo is an operator that gives the remainder of a division.  
It only applies to integers.

ⓘ

See the c[Math](enfusion://ScriptEditor/scripts/Core/generated/Math/Math.c;12).Repeat() method for a float modulo approximation.

```enforce
5 % 3; // result is 2: 5 is one time 3, remaining 2
10 % 2; // result is 0: 10 is five times 2, remaining 0
```

### Relational

#### >

The "greater than" operator returns **true** if the left argument is strictly greater than the right argument, false otherwise.  
It can be applied to int, float or a mix of those.

```enforce
10 > 11; // false
10 > 10; // false
10 >  9; // true

10 > 5; // true
10 > 5.5; // true
10.5 > 5; // true
```

#### <

The "lesser than" operator returns **true** if the left argument is strictly smaller than the right argument, false otherwise.  
It can be applied to int, float or a mix of those.

```enforce
10 < 11; // true
10 < 10; // false
10 <  9; // false

10 < 5; // false
10 < 5.5; // false
10.5 < 5; // false
```

#### >=

The "greater than or equal" operator returns **true** if the left argument is greater than or equal to the right argument, false otherwise.  
It can be applied to int, float or a mix of those.

```enforce
10 >= 11; // false
10 >= 10; // true
10 >=  9; // true

10 >= 5; // true
10 >= 5.5; // true
10.5 >= 5; // true
```

#### <=

The "lesser than or equal" operator returns **true** if the left argument is smaller than or equal to the right argument, false otherwise.  
It can be applied to int, float or a mix of those.

```enforce
10 <= 11; // true
10 <= 10; // true
10 <=  9; // false

10 <= 5; // false
10 <= 5.5; // false
10.5 <= 5; // false
```

#### ==

The "equals" operator returns **true** if the left argument is strictly equal to the right argument, false otherwise.

Applies to:

* bool
* int
* float
* int / float, float / int
* string (case-sensitive)
* vector (value)
* array (reference)
* object (reference)

```enforce
true == true; // true
true == false; // false

10 == 11; // false
10 == 10; // true
10 ==  9; // false

10 == 10.0; // true
10 == 10.1; // false
MyClass object1 = new MyClass();
MyClass object2 = object1;
object1 == object2; // true - same type, same instance
MyClass object1 = new MyClass();
MyClass object2 = new MyClass();
object1 == object2; // false - same type, different instance
```

#### !=

The "different" operator returns **true** if the left argument is different from the right argument, false otherwise.

Applies to:

* bool
* int
* float
* int / float, float / int
* string (case-sensitive)
* vector (value)
* array (reference)
* object (reference)

```enforce
10 != 11; // true
10 != 10; // false
10 !=  9; // true

10 != 10.0; // false
10 != 10.1; // true
MyClass object1 = new MyClass();
MyClass object2 = object1;
object1 != object2; // false - same instance
MyClass object1 = new MyClass();
MyClass object2 = new MyClass();
object1 != object2; // true - different instance
```

### Logical

#### &&

The "and" operator returns **true** if both left and right conditions are true.

```enforce
bool a = false;
bool b = false;
bool result = a && b; // result is false
bool a = true;
bool b = false;
bool result = a && b; // result is false
bool a = true;
bool b = true;
bool result = a && b; // result is true
```

#### ||

The "or" logical operator returns **true** if left or right condition is true (or both).

```enforce
bool a = false;
bool b = false;
bool result = a || b; // result is false
bool a = true;
bool b = false;
bool result = a || b; // result is true
bool a = true;
bool b = true;
bool result = a || b; // result is true
```

#### !

The "not" logical operator inverts a bool value: true becomes false and false becomes true.  
It can also be used on:

* int or float (0 comparison)
* string (empty string comparison - usage of the string.IsEmpty() method is preferred)
* object (null comparison)

```enforce
bool a = true;
bool result = !a; // result is false
bool a = false;
bool result = !a; // result is true
bool isAlive = player.IsAlive();
bool isDead = !isAlive;
int health = 0;
if (!health) // same as if (health == 0) - not recommended
Print("dead");
float fHealth = 0.0;
if (!fHealth) // same as if (fHealth == 0) - not recommended
Print("dead");
string data = "";
if (!data) // same as if (data.IsEmpty()) and if (data == "") - not recommended
Print("empty string");
MyClass instance = null;
if (!instance) // same as if (instance == null)
Print("instance is null!");
// note that if (value) to check an object is -not- null works too
if (instance) // same as if (instance != null)
Print("instance is NOT null!");
```

### Bitwise

#### &

The "bitwise AND" operator applies a AND operation between values bits - both bits must be 1 to return 1, otherwise 0 is returned.

```enforce
int a = 13; // 00001101 in binary
int b = 11; // 00001011 in binary
int result = a & b; // 00001001 in binary
// result is 9
```

#### |

The "bitwise OR" operator applies a OR operation between values bits - one of the bits must be 1 to return 1, otherwise 0 is returned.

```enforce
int a = 13; // 00001101 in binary
int b = 11; // 00001011 in binary
int result = a | b; // 00001111 in binary
// result is 15
```

#### ~

The "bitwise NOT" operator reverts all bits of the provided value: 1 becomes 0 and vice-versa.

```enforce
int a = 7; // 0000 0000 0000 0000 0000 0000 0000 0111 in binary (integer is 32 bits)
int result = ~a; // 1111 1111 1111 1111 1111 1111 1111 1000 in binary
// result is -8
```

#### <<

The left shift operator moves all bits of the provided value to the left.

```enforce
int a = 1; // 00000001 in binary
int result = a << 2; // shift 'a' bits 2 steps to the left
// leftmost bits are pushed "out"
// rightmost bits are filled with zeros
// result is 00000100
// result is 4
```

#### >>

The right shift operator moves bits of the provided value to the right.

```enforce
int a = 6; // 00000110 in binary
int result = a >> 2; // shift 'a' bits 2 steps to the left
// leftmost bits are filled with zeros
// rightmost bits are pushed "out"
// result is 00000001
// result is 1
```

### String

#### +

The "adds" operator used with strings concatenates them.
When it is used with string and another type, it will stringify (calling its .ToString() method) the other value.
The left argument **must** be of string type for it to happen.

```enforce
string a = "Hello";
string b = "there";
string result = a + b; // result is "Hellothere"
string result = a + " " + b; // result is "Hello there"
string stringifiedBool = "string " + true; // result is "string true"
string stringifiedInt = "string " + 42; // result is "string 42"
string stringifiedFloat = "string " + 4.0; // result is "string 4"
string stringifiedFloat = "string " + 4.2; // result is "string 4.2"
string stringifiedArray = "string " + myArray; // result is e.g "string 0x000002820B75EA68 {0,1,2,3,4}"
string stringifiedVector = "string " + myVector; // result is e.g "string <0.000000, 0.500000, 1.000000>"
string stringifiedEnum = "string " + MyEnum.B; // result is e.g "string 1"
string stringifiedObject = "string " + myObject; // result is e.g "string MyClass<0x00000282360ECFD0>"
string stringifiedTypename = "string " + MyClass; // result is e.g "string MyClass"
string ok1 = "test" + 3; // will return "test3"
string ok2 = "test" + 3 + true + false + 4; // will return "test3104"
string error = 3 + "test"; // will throw a parsing error
```

### Indexing

#### []

The "get index" operator is used on arrays to get an element by its **zero-based** index (e.g 0 is first element, 1 is second, etc). It is **very** efficient on **static** arrays.
It can also be used on strings, as strings are arrays of characters.

```enforce
int staticNumbers[5] = { 1, 2, 3, 4, 5 };
int thirdStaticNumber = staticNumbers[2]; // result is 3, lightning fast on static arrays
array<int> numbers = { 1, 2, 3, 4, 5 };
int thirdNumber = letters[2]; // result is 3
array<string> letters = { "A", "B", "C", "D", "E" };
string thirdLetter = letters[2]; // result is "C"
string text = "ABCDE";
string thirdLetter = text[2]; // result is "C"
```

⚠

The [i] operator is equivalent to a c.Get(i) method call for anything else than static arrays;

in order to save unrequired method calls, either use [foreach](/wiki/Arma_Reforger:Scripting:_Keywords#foreach "Arma Reforger:Scripting: Keywords") or cache the call result whenever possible.

```enforce
// bad
for (int i, count = myArray.Count(); i < count; i++)
{
	if (myArray[i].IsEmpty()) // .Get(i) equivalent
	Print("#" + i + " is empty");
	else
	Print("trimmed '" + myArray[i] + "' is '" + myArray[i].Trim() + "'"); // .Get(i) equivalent, twice
}
// good
for (int i, count = myArray.Count(); i < count; i++)
{
	string value = myArray[i]; // .Get(i) equivalent
	if (value.IsEmpty())
	Print("#" + i + " is empty");
	else
	Print("trimmed '" + value + "' is '" + value.Trim() + "'");
}
// best
foreach (int i, string value : myArray)
{
	if (value.IsEmpty())
	Print("#" + i + " is empty");
	else
	Print("trimmed '" + value + "' is '" + value.Trim() + "'");
}
```
