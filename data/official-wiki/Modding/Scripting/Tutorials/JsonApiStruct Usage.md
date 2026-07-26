# [JsonApiStruct Usage](https://community.bistudio.com/wiki/Arma_Reforger:JsonApiStruct_Usage)

**[JsonApiStruct](enfusion://ScriptEditor/scripts/GameLib/generated/online/JsonApiStruct.c;15)** is scripted object with support to:

* Encode script object/variable to JSON format
* Decode data from JSON format onto script object/ variable
* Import/export from/to file
* Import/export from/to string

It can also be used as callback object when handling responses from Backend API (script) where incoming data are automatically expanded.

ⓘ

[JsonApiStruct](enfusion://ScriptEditor/scripts/GameLib/generated/online/JsonApiStruct.c;15) is primarily meant for automatic data "conversion" from JSON to script - but it is also possible to assemble JSON using object's API:

1. by using the variable registration system - based upon additional variable registering *via* the cRegV() (Register Variable) method
2. as general stream assembly - by calling methods to add values, object or starting arrays

## How To

* Create your own class by inheriting [JsonApiStruct](enfusion://ScriptEditor/scripts/GameLib/generated/online/JsonApiStruct.c;15)
* Register all variables you require in constructor of object
* Create hierarchy from objects if you need more complex structures

### Simple Declaration

```enforce
// Example object
class MyObject : JsonApiStruct
{
	string name;
	string year;
	void MyObject()
	{
		// these variables will be converted to JSON or filled from JSON
		RegV("name");
		RegV("year");
	}
}
```

### Hierarchy Declaration

```enforce
// Child 1
class MyName : JsonApiStruct
{
	string name;
	void MyName()
	{
		RegV("name");
	}
}
// Child 2
class MyFloats : JsonApiStruct
{
	float float1;
	float float2;
	void MyFloats()
	{
		RegV("float1");
		RegV("float2");
	}
}
// Parent structure
class MyParent : JsonApiStruct
{
	MyName name;
	MyFloats floats;
	void MyParent()
	{
		RegV("name");
		RegV("floats");
	}
}
```

ⓘ

cRegV() adds the provided variable name to the defined object's list of values to be processed.

* Registered variable names are case-sensitive!
* Non-registered variables are simply ignored.

## File Operations

You can store your object into file (ie. invoke Pack + Save to file) or you can just save already packed JSON (Save to File).

### File Import

```enforce
// unpack from RAW data - either put data there manually (JSON!) or its callback result
void UnpackFromRAW(string data)
{
	Dummy dummy = new Dummy(); // Dummy extends JsonApiStruct
	dummy.ExpandFromRAW(data);
	// pack + create file at once
	dummy.SaveToFile("dummy_test.json");
}
// call OnPack() and save result into file
void PackAndSaveLocally()
{
	dummy.PackToFile("dummy_test.json");
}
```

### File Export

```enforce
void LoadJSON()
{
	Dummy dummy = new Dummy(); // Dummy extends JsonApiStruct
	dummy.LoadFromFile("dummy_test.json"); // now dummy object is hydrated with json data
	// note that Expand is automatically called upon object
	// you can also use its data as string
	Print(dummy.AsString());
}
```

## Packing

Packing is fairly easy, when object is either asked by thread/callback or invoked manually to pack its data, the cOnPack() event method is called.

ⓘ

See the [JSON Validation](#JSON_Validation) chapter below for a simple way to check JSON structure.

### Automated Packing/Expanding

To make things easier, it is possible to work with a JSON object just by registering variables and object into [JsonApiStruct](enfusion://ScriptEditor/scripts/GameLib/generated/online/JsonApiStruct.c;15).

| What Works | What Does not |
| --- | --- |
| * If JSON is expanded onto given object or hierarchy and variable name and format match, variables are filled with appropriate data * If JSON is assembled from given object or hierarchy, variable content is used to create JSON structures * float, int, bool, string, array, object and array of objects are supported * automatic allocation of object * getting JSON as string | * Even though the JSON format does support it, multi-type arrays (like combined strings and integers) are not supported by Enforce Script * Declaration of variables or objects - if there is no adequate variable present on during expand, it is ignored - if an object type is missing, it cannot be expanded |

```enforce
// Child
class MyChild : JsonApiStruct
{
	// object example
}
// Parent
class MyParent : JsonApiStruct
{
	string name;
	string year;
	MyChild obj1;
	void MyParent()
	{
		obj1 = new MyChild();
		// this register variables for auto use
		RegV("name");
		RegV("year");
		RegV("obj1");
	}
}
```

```enforce
// Create JSON and print to console
void PackExample()
{
	MyParent dummy = new MyParent();
	// variables will be default unless set to something reasonable
	dummy.name = "King Henry VIII";
	dummy.year = 1509;
	// create JSON
	dummy.Pack();
	// print result data to console as string
	Print(dummy.AsString());
}
// Load from existing file in JSON format
void LoadAndExpandExample()
{
	MyParent dummy = new MyParent();
	dummy.LoadFromFile("dummy_test.json");
	// now json is upon dummy object
	// note that Expand is called automatically upon object and all variables registered!
	// no additional scripting required unless you want to handle something specific or debug perhaps
	// you can also use its data as string
	Print(dummy.AsString());
}
```

### Variables

```enforce
// assuming you have variables declared upon JsonApiStruct itself!
float m_fMyFloat;
// add simple variable
void OnPack()
{
	StoreFloat("MyFloat", m_fMyFloat);
}
```

### Objects and Variables

```enforce
// assuming variables are declared upon JsonApiStruct itself
float m_fMyFloat;
int m_iMyInt;
protected ref AvatarStruct m_Avatar; // AvatarStruct extends JsonApiStruct
// add object and several variables
void OnPack()
{
	StoreFloat("MyFloat", m_fMyFloat);
	StoreInt("MyInt", m_iMyInt);
	StoreObject("avatar", m_Avatar);
}
```

If the cOnPack() event method is declared upon children objects, it is called upon them hierarchically as well.

```enforce
// assuming the array is declared upon JsonStruct itself
protected ref array<string> m_aItems = {};
// add array and its items
void OnPack()
{
	StartArray("m_aItems");
	foreach (string item : m_aItems)
	{
		ItemString(item);
	}
	EndArray();
}
```

## Error Handling

If you use object as a callback and an error happen during JSON processing, it generates events upon that very object:

```enforce
class Dummy : JsonApiStruct
{
	void OnExpand()
	{
		// Event when expand (unpack) process starts
		// if you want to handle something before process starts (init/clear variables for example)
	}
	void OnBufferReady()
	{
		// this is called after successfull JSON pakc process
		// if you want the buffer's finalised string for any purpose
	}
	void OnSuccess(int errorCode)
	{
		// errorCode is EJsonApiError
		// Event called when pending store operation is finished - callback when all went as expected
	}
	void OnError(int errorCode)
	{
		// errorCode is EJsonApiError
		// Event called when pending store operation is finished - callback when error happened
	}
}
```

ⓘ

Error codes are processed as such due to the asynchronous processing of REST requests in order not to block the main thread and stop the game from processing.

### Error Codes

As defined in [EJsonApiError](enfusion://ScriptEditor/scripts/GameLib/generated/online/EJsonApiError.c;15):

| Code Enum | Description |
| --- | --- |
| ETJSON\_UNKNOWN | General error - not implemented |
| ETJSON\_OK | This is generated upon sucessfull processing at onSuccess() event |
| ETJSON\_COMMSEND | Sending of object failed (callback got error code) |
| ETJSON\_COMMRECV | Receiving of object failed (callback got error code) |
| ETJSON\_PARSERERROR | Parsing process failed - invalid data or corrupted format? |
| ETJSON\_PACKNOSTART | Packing process cannot start - invalid state or other problems |
| ETJSON\_TIMEOUT | Failed to send data due to timeout (when JsonStruct data is sent via RestApi or BackendApi |
| ETJSON\_NOBUFFERS | Too many objects processed at once! |
| ETJSON\_FAILFILELOAD | Could not load file with JSON data |
| ETJSON\_FAILFILESAVE | Could not save file with JSON data |

## JSON Validation

JSON validation allows to test the input/output values.

Result structure can be used as input as well and it should match in the end, similar process can be used for verifying already existing structures.

```enforce
// root structure example
class DummyRoot : JsonApiStruct
{
	// content + pack/expand methods
}
DummyRoot dummy = new DummyRoot();

dummy.Pack(); // pack content
string data1 = dummy.AsString(); // get packed JSON #1

dummy.ExpandFromRAW(data1); // load structure once more from data!
// repeat process :-)
dummy.Pack(); // pack content
string data2 = dummy.AsString(); // get packed JSON #2
// now both can be visualised
```

Packing object will produce a long, non-formatted string like:

```
{"m_bool":false,"m_int":1024,"m_double":1.2345678806304932,"m_string":"Hello I am your new string!","m_objects":{"obj1":{"name":"Van Ceulen","year":"1706","val":3.1415927410125734},"obj2":{"name":"Bernoulli","year":"1683","val":0.5772156715393066},"obj3":{"name":"Feidius","year":"430BC","val":1.6180340051651}}}
```

Use suitable online/other tool for readable structure, result can be stored to textfile and compared by binary/text compare tool if necessary.

ⓘ

* [JSONFormatter.org](https://jsonformatter.org/) offers a JSON beautifier (default tabulation = 2 spaces)
* [Notepad++](https://notepad-plus-plus.org/) has the feature built-in (default tabulation = 1 tab); Plugins > JSON Viewer > Format JSON (`Ctrl` + `Alt` + `⇧ Shift` + `M`): Show Result

  ```
  {
  	"m_bool": false,
  	"m_int": 1024,
  	"m_double": 1.2345678806304932,
  	"m_string": "Hello I am your new string!",
  	"m_objects": {
  		"obj1": {
  			"name": "Van Ceulen",
  			"year": "1706",
  			"val": 3.1415927410125734
  		},
  		"obj2": {
  			"name": "Bernoulli",
  			"year": "1683",
  			"val": 0.5772156715393066
  		},
  		"obj3": {
  			"name": "Feidius",
  			"year": "430BC",
  			"val": 1.6180340051651
  		}
  	}
  }
  ```
