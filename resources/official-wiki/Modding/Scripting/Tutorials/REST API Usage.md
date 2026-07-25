# [REST API Usage](https://community.bistudio.com/wiki/Arma_Reforger:REST_API_Usage)

The **REST API** ([RestApi](enfusion://ScriptEditor/scripts/GameLib/generated/online/RestApi.c;12)) is a simplified way to call REST requests and handle results/data, allowing to communicate with a web API.
This page explains the terminology and how to use the scripting API.

ⓘ

See also the [Representational state transfer](https://en.wikipedia.org/wiki/Representational_state_transfer) Wikipedia article.

⚠

The REST API is only meant to provide basic REST functionality (GET and POST methods - see [HTTP request methods](https://en.wikipedia.org/wiki/Hypertext_Transfer_Protocol#Request_methods)), *not* full server functionality (like e.g [curl](https://curl.se/)).

* Data size accepted by the API has to be **below 1 MB**
* There is no support for custom headers
* There is limited printout

## Definitions

### Context

The **context** is simply the website URL with which transactions will happen, e.g <https://httpbin.org/>.

A [RestContext](enfusion://ScriptEditor/scripts/GameLib/generated/online/RestContext.c;13) context can be created with the following:

```enforce
RestContext ctx = GetGame().GetRestApi().GetContext("https://httpbin.org/");
```

### Callback

A **callback** is a script class handling a request's success, error or timeout.

Once the request happens (asynchronously) the callback methods are called to process the received result.

The scripter is responsible for the callback object's lifetime. Inherit from [RestCallback](enfusion://ScriptEditor/scripts/GameLib/generated/online/RestCallback.c;71) and implement the desired methods to create a custom callback:

```enforce
class RestCallbackExample : RestCallback
{
}
```

## Example

### Declaration

```enforce
class RestCallbackExample : RestCallback
{
	//------------------------------------------------------------------------------------------------
	override void OnError(int errorCode)
	{
		PrintFormat("OnError(%1)", errorCode);
	}
	//------------------------------------------------------------------------------------------------
	override void OnTimeout()
	{
		Print("OnTimeout()");
	}
	//------------------------------------------------------------------------------------------------
	override void OnSuccess(string data, int dataSize)
	{
		PrintFormat("OnSuccess() - data size = %1 bytes", dataSize);
		if (dataSize > 0)
		Print(data); // note that Print() will not output strings longer than 1024b to console, check the dataSize!
	}
}
class HttpElement
{
	protected ref RestCallbackExample m_CallbackExample;
	//------------------------------------------------------------------------------------------------
	void ExecuteRequest()
	{
		if (!m_CallbackExample)
		m_CallbackExample = new RestCallbackExample();
		string contextURL = "https://httpbin.org/";
		RestContext context = GetGame().GetRestApi().GetContext(contextURL);
		// executed request is "get" in "https://httpbin.org/" context (i.e "https://httpbin.org/get")
		// using the HTTP's GET method
		context.GET(m_CallbackExample, "get");
		// it is possible to assemble a more complex request using arguments, like this:
		// ctx.GET(m_CallbackExample, "get?x=10&y=5");
	}
}
```

### Call

```enforce
class HolderClass
{
	protected ref HttpElement m_HttpElement;
	void PerformGETRequest()
	{
		if (!m_HttpElement)
		m_HttpElement = new HttpElement();
		m_HttpElement.ExecuteRequest();
	}
}
```
