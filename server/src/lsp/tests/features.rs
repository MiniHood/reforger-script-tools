#[test]
fn full_shared_executor_evicts_or_drops_rich_before_semantic() {
    let (sender, receiver) = mpsc::channel();
    // Deliberately do not start a worker: this exercises admission and
    // eviction deterministically, without a dispatch race.
    let scheduler = RuntimeWorkExecutor {
        state: Arc::new((Mutex::new(BTreeMap::new()), Condvar::new())),
        sender,
        test_before_execute: None,
    };
    let now = Instant::now();
    let mut runtime = AnalysisRuntime::new(AdmissionLimits::new(128, 128));
    {
        let (lock, _) = &*scheduler.state;
        let mut pending = lock.lock().unwrap();
        for index in 0..MAX_PENDING_DOCUMENT_ANALYSIS_JOBS - 1 {
            let uri = format!("file:///semantic-{index}.c");
            pending.insert(
                (TaskClass::Semantic, uri.clone()),
                RuntimeWorkJob::Semantic(semantic_analysis_job(&mut runtime, &uri, 1, now)),
            );
        }
        let rich_uri = "file:///rich.c";
        pending.insert(
            (TaskClass::Rich, rich_uri.to_string()),
            RuntimeWorkJob::Rich(rich_semantic_tokens_job(&mut runtime, rich_uri, 1, now)),
        );
    }

    let incoming_semantic_uri = "file:///semantic-incoming.c";
    scheduler.schedule(semantic_analysis_job(
        &mut runtime,
        incoming_semantic_uri,
        1,
        now,
    ));
    {
        let (lock, _) = &*scheduler.state;
        let pending = lock.lock().unwrap();
        assert_eq!(pending.len(), MAX_PENDING_DOCUMENT_ANALYSIS_JOBS);
        assert!(pending
            .keys()
            .all(|(class, _)| *class == TaskClass::Semantic));
    }

    let incoming_rich_uri = "file:///rich-incoming.c";
    scheduler.schedule_rich(rich_semantic_tokens_job(
        &mut runtime,
        incoming_rich_uri,
        1,
        now,
    ));
    let (lock, _) = &*scheduler.state;
    let pending = lock.lock().unwrap();
    assert_eq!(pending.len(), MAX_PENDING_DOCUMENT_ANALYSIS_JOBS);
    assert!(pending
        .keys()
        .all(|(class, _)| *class == TaskClass::Semantic));
    drop(pending);
    // The evicted and dropped rich jobs are both completed through the
    // normal event channel, preserving cancellation/publication handling.
    assert!(matches!(
        receiver.recv().unwrap(),
        ServerEvent::RichSemanticTokensSkipped { .. }
    ));
    assert!(matches!(
        receiver.recv().unwrap(),
        ServerEvent::RichSemanticTokensSkipped { .. }
    ));
}

#[test]
fn semantic_token_refresh_coalesces_until_the_client_acknowledges_it() {
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());

    let mut effects = Vec::new();
    server
        .document_runtime
        .request_semantic_tokens_refresh_effect(&mut effects);
    server
        .document_runtime
        .request_semantic_tokens_refresh_effect(&mut effects);
    for effect in effects {
        server.deliver_effect(effect).unwrap();
    }
    assert_eq!(
        String::from_utf8_lossy(&server.writer)
            .matches("workspace/semanticTokens/refresh")
            .count(),
        1
    );

    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": "server-1", "result": null }),
            None,
            0,
            0,
        )
        .unwrap();

    assert_eq!(
        String::from_utf8_lossy(&server.writer)
            .matches("workspace/semanticTokens/refresh")
            .count(),
        2
    );
}

#[test]
fn positions_are_zero_based_utf16() {
    let source = "class A\n{\n\tstring Name;\n}\n";
    let symbols = document_symbols_for_source(source);

    assert_eq!(
        symbols[0].range.start,
        LspPosition {
            line: 0,
            character: 0
        }
    );
    assert_eq!(
        symbols[0].selection_range.start,
        LspPosition {
            line: 0,
            character: 6
        }
    );
}

#[test]
fn semantic_tokens_classify_lexer_and_symbol_facts() {
    let source = r#"// docs
[Attribute()]
class Base
{
}

class Example
	: Base
{
	static const int COUNT = 4;
	void Example(int initialValue)
	{
	}
	void ~Example()
	{
	}
	void Run(int value)
	{
		string name = "x";
		Example other;
		other.Run(value);
	}
}
#ifdef DEBUG
#define GAME_MODE_DEBUG
#endif
"#;

    let report = fast_semantic_tokens_report_for_source(source);

    assert_eq!(report.parse_diagnostics, 0);
    assert!(!report.tokens.data.is_empty());
    assert_eq!(report.tokens.data.len() % 5, 0);
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "Example" && token.token_type == "class"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "Run" && token.token_type == "method"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "Example" && token.token_type == "class"));
    assert!(
        !report
            .decoded
            .iter()
            .any(|token| token.text == "Example" && token.token_type == "method"),
        "{:?}",
        report.decoded
    );
    assert!(
        report
            .decoded
            .iter()
            .filter(|token| token.text == "Run" && token.token_type == "method")
            .count()
            >= 2
    );
    assert!(
        report
            .decoded
            .iter()
            .filter(|token| token.text == "Example" && token.token_type == "class")
            .count()
            >= 3
    );
    assert!(
        report
            .decoded
            .iter()
            .filter(|token| token.text == "Base" && token.token_type == "class")
            .count()
            >= 2,
        "{:?}",
        report.decoded
    );
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "COUNT" && token.token_type == "field"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "value" && token.token_type == "parameter"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "name" && token.token_type == "variable"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "Attribute" && token.token_type == "class"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "void" && token.token_type == "keyword"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "int" && token.token_type == "keyword"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "string" && token.token_type == "class"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "\"x\"" && token.token_type == "string"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "4" && token.token_type == "number"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "// docs" && token.token_type == "comment"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "ifdef" && token.token_type == "preprocessor"));
    assert!(report
        .decoded
        .iter()
        .any(|token| token.text == "define" && token.token_type == "preprocessor"));
    assert_semantic_token(&report, "DEBUG", "variable", Some("#cfcfcf"));
    assert_semantic_token(&report, "GAME_MODE_DEBUG", "variable", Some("#cfcfcf"));
}

#[test]
fn semantic_tokens_color_external_enum_member_references() {
    let root = temp_test_dir("semantic_tokens_external_enum");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("EWeaponType.c"),
        "enum EWeaponType\n{\n\tWT_NONE,\n\tWT_FRAGGRENADE,\n}\n",
    )
    .unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            &root,
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = r#"class Example
{
	void Run()
	{
		EWeaponType value = EWeaponType.WT_FRAGGRENADE;
	}
}
"#;

    let report = semantic_tokens_report_for_source_with_external(source, Some(&external));

    assert!(
        report
            .decoded
            .iter()
            .any(|token| token.text == "WT_FRAGGRENADE" && token.token_type == "enumMember"),
        "{:?}",
        report.decoded
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn semantic_tokens_color_primitive_keywords_and_external_class_types_separately() {
    let root = temp_test_dir("semantic_tokens_external_class_type");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("KickCauseCode.c"),
        "class KickCauseCode : handle64\n{\n\tstatic KickCauseCode NONE;\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("SCR_InstigatorContextData.c"),
        "class SCR_InstigatorContextData {}\n",
    )
    .unwrap();
    fs::write(root.join("IEntity.c"), "class IEntity {}\n").unwrap();
    fs::write(root.join("array.c"), "class array {}\n").unwrap();
    fs::write(
        root.join("EResourceType.c"),
        "enum EResourceType\n{\n\tSUPPLIES,\n}\n",
    )
    .unwrap();
    fs::write(
        root.join("ScriptInvokerBase.c"),
        "class ScriptInvokerBase {}\n",
    )
    .unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            &root,
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = "\
void SCR_BaseGameMode_OnPlayerDisconnected(int playerId, KickCauseCode cause = KickCauseCode.NONE, int timeout = -1);
void SCR_BaseGameMode_OnControllableDestroyed(notnull SCR_InstigatorContextData instigatorContextData);
void SCR_BaseGameMode_PlayerIdAndEntity(int playerId, IEntity player);
void SCR_BaseGameMode_OnResourceEnabledChanged(array<EResourceType> disabledResourceTypes);
typedef ScriptInvokerBase<OnPreloadFinished> OnPreloadFinishedInvoker;
class Example { protected ref ScriptInvoker m_OnGameEnd = new ScriptInvoker(); }
";

    let report = semantic_tokens_report_for_source_with_external(source, Some(&external));

    assert!(
        report
            .decoded
            .iter()
            .any(|token| token.text == "void" && token.token_type == "keyword"),
        "{:?}",
        report.decoded
    );
    assert!(
        report
            .decoded
            .iter()
            .filter(|token| token.text == "int" && token.token_type == "keyword")
            .count()
            >= 2,
        "{:?}",
        report.decoded
    );
    assert!(
        report
            .decoded
            .iter()
            .filter(|token| token.text == "KickCauseCode" && token.token_type == "class")
            .count()
            >= 2,
        "{:?}",
        report.decoded
    );
    assert_semantic_token(&report, "SCR_InstigatorContextData", "class", None);
    assert_semantic_token(&report, "IEntity", "class", None);
    assert_semantic_token(&report, "array", "class", None);
    assert_semantic_token(&report, "EResourceType", "enum", None);
    assert_semantic_token(&report, "ScriptInvokerBase", "class", None);
    assert!(
        report
            .decoded
            .iter()
            .any(|token| token.text == "NONE" && token.token_type == "enumMember"),
        "{:?}",
        report.decoded
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn semantic_tokens_color_source_backed_type_spans_before_external_index_is_ready() {
    let source = "\
void SCR_BaseGameMode_OnPlayerDisconnected(int playerId, KickCauseCode cause = KickCauseCode.NONE, int timeout = -1);
void SCR_BaseGameMode_OnControllableDestroyed(notnull SCR_InstigatorContextData instigatorContextData);
void SCR_BaseGameMode_PlayerIdAndEntity(int playerId, IEntity player);
void SCR_BaseGameMode_OnResourceEnabledChanged(array<EResourceType> disabledResourceTypes);
typedef ScriptInvokerBase<OnPreloadFinished> OnPreloadFinishedInvoker;
class Example { protected ref ScriptInvoker m_OnGameEnd = new ScriptInvoker(); }
";

    let report = fast_semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "void", "keyword", Some("#59A6E9"));
    assert_semantic_token(&report, "int", "keyword", Some("#59A6E9"));
    assert_semantic_token(&report, "KickCauseCode", "class", Some("#40b5ac"));
    assert_semantic_token(
        &report,
        "SCR_InstigatorContextData",
        "class",
        Some("#40b5ac"),
    );
    assert_semantic_token(&report, "IEntity", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "array", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "EResourceType", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "ScriptInvokerBase", "class", Some("#40b5ac"));
    assert_semantic_token_count_at_least(&report, "ScriptInvoker", "class", 2);
}

#[test]
fn semantic_tokens_apply_type_keyword_color_policy_in_declarations() {
    let source = r#"class SCR_Class {}
enum SCR_EEnum { VALUE, }
class ResourceName {}
class LocalizedString {}
class Curve {}
class Color {}
class array<Class T> {}
class map<Class TKey, Class TValue> {}
class set<Class T> {}

class Example
{
	bool m_bValue;
	int m_iValue;
	float m_fValue;
	string m_sValue;
	SCR_EEnum m_eValue;
	vector m_vValue;
	array<SCR_Class> m_aValue;
	map<string, SCR_Class> m_mValue;
	ResourceName m_sResourceName;
	LocalizedString m_sLocalisedString;
	Curve m_aCurve;
	SCR_Class m_ClassInstance;
	typename m_ClassTypename;
	set<SCR_Class> m_Set;
	Color m_Color;
	void Run()
	{
		bool b = true;
		bool c = false;
	}
}
"#;

    let report = semantic_tokens_report_for_source(source);

    for text in ["bool", "int", "float", "typename", "true", "false"] {
        assert_semantic_token(&report, text, "keyword", Some("#59A6E9"));
    }
    for text in [
        "string",
        "SCR_EEnum",
        "vector",
        "array",
        "map",
        "ResourceName",
        "LocalizedString",
        "Curve",
        "SCR_Class",
        "set",
        "Color",
    ] {
        assert_semantic_type_family_token_count_at_least(&report, text, 1);
    }
}

#[test]
fn semantic_tokens_color_enum_static_member_values_as_variables() {
    let source = r#"enum EHealthState
{
	INJURED,
}

class Example
{
	void Run()
	{
		EHealthState state = EHealthState.INJURED;
	}
}
"#;

    let report = semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "EHealthState", "enum", Some("#40b5ac"));
    assert_semantic_token(&report, "INJURED", "enumMember", Some("#cfcfcf"));
}

#[test]
fn semantic_tokens_keep_generic_callback_type_arguments_type_colored() {
    let source = "\
void SCR_BaseGameMode_PlayerId(int playerId);
typedef func SCR_BaseGameMode_PlayerId;
class Example
{
\tprotected ref ScriptInvokerBase<SCR_BaseGameMode_PlayerId> m_OnPlayerAuditSuccess = new ScriptInvokerBase<SCR_BaseGameMode_PlayerId>();
}
";

    let report = semantic_tokens_report_for_source(source);

    assert_semantic_type_family_token_count_at_least(&report, "SCR_BaseGameMode_PlayerId", 2);
    assert_semantic_token_count_at_least(&report, "ScriptInvokerBase", "class", 2);
    assert_eq!(
        report
            .decoded
            .iter()
            .filter(|token| {
                token.text == "SCR_BaseGameMode_PlayerId" && token.token_type == "function"
            })
            .count(),
        1,
        "{:?}",
        report.decoded
    );
}

#[test]
fn semantic_tokens_resolve_attribute_argument_expressions() {
    let root = temp_test_dir("semantic_tokens_attribute_arguments");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("Attribute.c"),
        r#"class Attribute {}
class UIWidgets
{
	static const string Flags = "flags";
}
class ParamEnumArray
{
	static ParamEnumArray FromEnum(typename value);
}
enum EGameFlags
{
	TEST,
}
"#,
    )
    .unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            &root,
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = r#"class Example
{
	static const string WB_GAME_MODE_CATEGORY = "Game";
	[Attribute("0", uiwidget: UIWidgets.Flags, "Test Game Flags for when you run mission via WE.", "", ParamEnumArray.FromEnum(EGameFlags), WB_GAME_MODE_CATEGORY)]
	protected EGameFlags m_eTestGameFlags;
}
"#;

    let report = semantic_tokens_report_for_source_with_external(source, Some(&external));

    assert_semantic_token(&report, "Attribute", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "uiwidget", "variable", Some("#cfcfcf"));
    assert_semantic_token(&report, "UIWidgets", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "Flags", "enumMember", Some("#cfcfcf"));
    assert_semantic_token(&report, "ParamEnumArray", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "FromEnum", "method", Some("#f3ad58"));
    assert_semantic_token(&report, "EGameFlags", "enum", Some("#40b5ac"));
    assert_semantic_token(&report, "WB_GAME_MODE_CATEGORY", "field", Some("#cfcfcf"));
    assert!(
        !report.decoded.iter().any(|token| {
            matches!(
                token.text.as_str(),
                "Attribute"
                    | "uiwidget"
                    | "UIWidgets"
                    | "Flags"
                    | "ParamEnumArray"
                    | "FromEnum"
                    | "EGameFlags"
                    | "WB_GAME_MODE_CATEGORY"
            ) && token.token_type == "decorator"
        }),
        "{:?}",
        report.decoded
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn semantic_tokens_refine_unqualified_attribute_arguments_with_external_facts() {
    let root = temp_test_dir("semantic_tokens_attribute_argument_enum");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Attribute.c"), "class Attribute {}\n").unwrap();
    fs::write(root.join("EGameFlags.c"), "enum EGameFlags { Test, }\n").unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            root.clone(),
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = r#"class Example
{
	[Attribute(ParamEnumArray.FromEnum(EGameFlags))]
	void Run();
}
"#;

    let fast_report = semantic_tokens_report_for_source(source);
    assert_semantic_token(&fast_report, "EGameFlags", "class", Some("#40b5ac"));

    let rich_report = semantic_tokens_report_for_source_with_external(source, Some(&external));
    assert_semantic_token(&rich_report, "EGameFlags", "enum", Some("#40b5ac"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn semantic_tokens_color_attribute_expression_shape_before_external_index_is_ready() {
    let source = r#"class Example
{
	[Attribute("0", uiwidget: UIWidgets.Flags, "Test", "", ParamEnumArray.FromEnum(EGameFlags), WB_GAME_MODE_CATEGORY)]
	protected EGameFlags m_eTestGameFlags;
}
"#;

    let report = semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "Attribute", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "uiwidget", "variable", Some("#cfcfcf"));
    assert_semantic_token(&report, "UIWidgets", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "Flags", "enumMember", Some("#cfcfcf"));
    assert_semantic_token(&report, "ParamEnumArray", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "FromEnum", "method", Some("#f3ad58"));
    assert_semantic_token(&report, "EGameFlags", "class", Some("#40b5ac"));
    assert_semantic_token(
        &report,
        "WB_GAME_MODE_CATEGORY",
        "variable",
        Some("#cfcfcf"),
    );
    assert!(
        report
            .decoded
            .iter()
            .filter(|token| matches!(
                token.text.as_str(),
                "Attribute"
                    | "uiwidget"
                    | "UIWidgets"
                    | "Flags"
                    | "ParamEnumArray"
                    | "FromEnum"
                    | "WB_GAME_MODE_CATEGORY"
            ))
            .all(|token| token.token_type != "decorator"),
        "{:?}",
        report.decoded
    );
}

#[test]
fn semantic_tokens_keep_attribute_shape_after_invalid_previous_line() {
    let source = r#"class Example
{
	this

	[RplRpc(RplChannel.Reliable, RplRcver.Server)]
	protected void RpcDo();
}
"#;

    let report = semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "RplRpc", "class", Some("#40b5ac"));
    assert!(
        !report
            .decoded
            .iter()
            .any(|token| token.text == "RplRpc" && token.token_type == "function"),
        "{:?}",
        report.decoded
    );
}

#[test]
fn semantic_tokens_color_call_shapes_before_rich_resolution() {
    let source = r#"class Example
{
	void Run()
	{
		RunTimer();
		stateComponent.GetDuration();
	}
}
"#;

    let report = semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "RunTimer", "function", Some("#f3ad58"));
    assert_semantic_token(&report, "GetDuration", "method", Some("#f3ad58"));
}

#[test]
fn semantic_tokens_color_static_member_shapes_before_rich_resolution() {
    let source = r#"class Example
{
	void Run()
	{
		SCR_BaseGameModeStateComponent stateComponent = GetStateComponent(SCR_EGameModeState.GAME);
		EHealthState.INJURED;
		int testnnn = GRAY_TEST2.testnum;
		stateComponent.GetDuration();
	}
}
"#;

    let report = semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "SCR_EGameModeState", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "GAME", "enumMember", Some("#cfcfcf"));
    assert_semantic_token(&report, "EHealthState", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "INJURED", "enumMember", Some("#cfcfcf"));
    assert_semantic_token(&report, "GRAY_TEST2", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "testnum", "enumMember", Some("#cfcfcf"));
    assert!(
        !report
            .decoded
            .iter()
            .any(|token| token.text == "stateComponent" && token.token_type == "class"),
        "{:?}",
        report.decoded
    );
}

#[test]
fn semantic_tokens_color_scope_references_before_rich_resolution() {
    let source = r#"class OwnerType
{
	void Run()
	{
	}
}

class Example
{
	OwnerType GetOwner();
	void Run(OwnerType owner)
	{
		OwnerType localOwner = GetOwner();
		if (owner == GetOwner())
			return owner;
		if (localOwner == owner)
		{
			OwnerType owner = localOwner;
			owner.Run();
		}
		int testnnn = GRAY_TEST2.testnum;
	}
}
"#;

    let report = fast_semantic_tokens_report_for_source(source);

    assert_semantic_token(&report, "owner", "parameter", Some("#cfcfcf"));
    assert_semantic_token(&report, "localOwner", "variable", Some("#cfcfcf"));
    assert_semantic_token_count_at_least(&report, "owner", "variable", 2);
    assert_semantic_token(&report, "GRAY_TEST2", "class", Some("#40b5ac"));
    assert_semantic_token(&report, "testnum", "enumMember", Some("#cfcfcf"));
}

#[test]
fn semantic_tokens_keep_comment_contents_comment_colored() {
    let source = r#"class Example
{
	//! \param[in] enable{} Set() true to enable supplies, set false to disable
	/*!
		\return[] // True{} <> if the game is hosted by a player (i.e., not dedicated server)
	*/
	int testnnn = 1; /* testnnn {} Set() */
	void Run();
}
"#;

    let report = semantic_tokens_report_for_source(source);

    assert!(
        report.decoded.iter().any(|token| {
            token.token_type == "comment"
                && token.text.contains("\\return[]")
                && token.text.contains("True{} <> if")
        }),
        "{:?}",
        report.decoded
    );
    assert!(
        !report.decoded.iter().any(|token| {
            matches!(
                token.text.as_str(),
                "[" | "]" | "{" | "}" | "(" | ")" | "<" | ">" | "if" | "Set"
            ) && token.range.start.line >= 2
                && token.range.end.line <= 5
                && token.token_type != "comment"
        }),
        "{:?}",
        report.decoded
    );
    assert!(
        report.decoded.iter().any(|token| {
            token.token_type == "comment"
                && token.text == "/* testnnn {} Set() */"
                && token.range.start.line == 6
        }),
        "{:?}",
        report.decoded
    );
}

#[test]
fn semantic_token_cache_is_keyed_by_external_generation() {
    let mut cache = open_documents::SemanticTokenCache::default();
    let lexical_baseline = LspSemanticTokenProjection {
        tokens: LspSemanticTokens {
            data: vec![1, 2, 3],
        },
        token_count: 1,
        parse_diagnostics: 0,
        timings: LspSemanticTokenTimings::default(),
    };
    let selection = cache.select_or_insert_lexical(7, 1, || lexical_baseline.clone());
    assert_eq!(selection.kind, TokenProjectionKind::LexicalBaseline);
    assert_eq!(selection.result_id, "reforger:7:lexical");
    assert_eq!(selection.disposition, TokenResultDisposition::Full);

    cache.set_rich(7, 1, lexical_baseline);

    assert!(cache
        .rich_for_revision_and_external_generation(7, 1)
        .is_some());
    assert!(cache
        .rich_for_revision_and_external_generation(7, 2)
        .is_none());
    assert!(cache
        .rich_for_revision_and_external_generation(8, 1)
        .is_none());

    let matching_generation = cache.select_or_insert_lexical(7, 1, || unreachable!());
    assert_eq!(matching_generation.kind, TokenProjectionKind::RichOverlay);
    assert_eq!(matching_generation.result_id, "reforger:7:rich:1");

    let stale_generation = cache.select_or_insert_lexical(7, 2, || unreachable!());
    assert_eq!(stale_generation.kind, TokenProjectionKind::LexicalBaseline);
    assert_eq!(stale_generation.result_id, "reforger:7:lexical");
    assert!(cache
        .rich_for_revision_and_external_generation(7, 1)
        .is_none());
}

#[test]
fn hover_selects_class_method_field_parameter_typedef_enum_member_and_global() {
    let source = r#"//! Global typedef docs
typedef string FactionKey;

Game g_Game;

[EnumBitFlag()]
enum ExampleFlags
{
	None = 0,
	Enabled = 1
}

//! Class docs.
class Example : Base
{
	[Attribute("0")]
	protected int m_Value;
	void Run(string name)
	{
		int localValue = 5;
		localValue = localValue + 1;
		Print(name);
		m_Value = localValue;
		foreach (int index, auto item : m_aItems)
		{
			string itemName = item.ToString();
		}
		for (int i = 0, count = 4; i < count; i++)
		{
		}
		FactionKey key;
		g_Game = null;
	}
}
"#;

    assert_hover(
        source,
        "Example : Base",
        "Example",
        SymbolKind::Class,
        "Example",
    );
    assert_hover(source, "m_Value", "m_Value", SymbolKind::Field, "m_Value");
    assert_hover(source, "Run(string", "Run", SymbolKind::Method, "Run");
    assert_hover(source, "string name", "name", SymbolKind::Parameter, "name");
    assert_hover(
        source,
        "localValue = 5",
        "localValue",
        SymbolKind::LocalVariable,
        "localValue",
    );
    assert_hover(
        source,
        "localValue + 1",
        "localValue",
        SymbolKind::LocalVariable,
        "localValue",
    );
    assert_hover(source, "Print(name)", "name", SymbolKind::Parameter, "name");
    assert_hover(
        source,
        "m_Value = localValue",
        "m_Value",
        SymbolKind::Field,
        "m_Value",
    );
    assert_hover(
        source,
        "int index, auto item",
        "index",
        SymbolKind::LocalVariable,
        "index",
    );
    assert_hover(
        source,
        "auto item :",
        "item",
        SymbolKind::LocalVariable,
        "item",
    );
    assert_hover(source, "int i = 0", "i =", SymbolKind::LocalVariable, "i");
    assert_hover(
        source,
        "count = 4",
        "count",
        SymbolKind::LocalVariable,
        "count",
    );
    assert_hover(source, "i++)", "i++", SymbolKind::LocalVariable, "i");
    assert_hover(
        source,
        "typedef string FactionKey",
        "FactionKey",
        SymbolKind::Typedef,
        "FactionKey",
    );
    assert_hover(
        source,
        "FactionKey key",
        "FactionKey",
        SymbolKind::Typedef,
        "FactionKey",
    );
    assert_hover(
        source,
        "Enabled = 1",
        "Enabled",
        SymbolKind::EnumMember,
        "Enabled",
    );
    assert_hover(
        source,
        "Game g_Game",
        "g_Game",
        SymbolKind::GlobalField,
        "g_Game",
    );
    assert_hover(
        source,
        "g_Game = null",
        "g_Game",
        SymbolKind::GlobalField,
        "g_Game",
    );
}

#[test]
fn hover_uses_cursor_token_range_for_file_local_identifier() {
    let source = r#"class Example
{
	void Run()
	{
		string label = "é"; int localValue = 0; localValue = 1;
	}
}
"#;
    let position = position_for_needle(source, "localValue = 1", "localValue");

    let report = hover_report_for_source_position(source, position);
    let hover = report.hover.expect("local identifier should have hover");

    assert_eq!(position.character, 42, "position uses UTF-16 code units");
    assert_eq!(
        hover.range,
        Some(LspRange {
            start: position,
            end: LspPosition {
                line: position.line,
                character: position.character + 10,
            },
        })
    );
}

#[test]
fn hover_uses_cursor_token_range_for_crlf_source() {
    let source = "class Example\r\n{\r\n\tvoid Run()\r\n\t{\r\n\t\tstring label = \"é\"; int localValue = 0; localValue = 1;\r\n\t}\r\n}\r\n";
    let position = position_for_needle(source, "localValue = 1", "localValue");

    let report = hover_report_for_source_position(source, position);
    let hover = report.hover.expect("local identifier should have hover");

    assert_eq!(position.line, 4, "CRLF advances one LSP line per break");
    assert_eq!(position.character, 42, "position uses UTF-16 code units");
    assert_eq!(
        hover.range,
        Some(LspRange {
            start: position,
            end: LspPosition {
                line: position.line,
                character: position.character + 10,
            },
        })
    );
}

#[test]
fn hover_type_position_selects_class_instead_of_constructor() {
    let source = r#"class Example
{
	void Example();
	static Example Make()
	{
		Example value = new Example();
		return value;
	}
}
"#;

    let return_type = hover_at(source, "static Example Make", "Example");
    let local_type = hover_at(source, "Example value", "Example");
    let constructor_call = hover_at(source, "new Example()", "Example");

    assert_eq!(return_type.selected_kind, Some(SymbolKind::Class));
    assert_eq!(return_type.selected_label.as_deref(), Some("Example"));
    assert_eq!(
        return_type.identifier_context,
        Some(IdentifierContext::TypePosition)
    );

    assert_eq!(local_type.selected_kind, Some(SymbolKind::Class));
    assert_eq!(
        local_type.identifier_context,
        Some(IdentifierContext::TypePosition)
    );

    assert_eq!(
        constructor_call.selected_kind,
        Some(SymbolKind::Constructor)
    );
    assert_eq!(
        constructor_call.identifier_context,
        Some(IdentifierContext::ValueOrCallable)
    );
}

#[test]
fn hover_resolves_member_access_through_receiver_type() {
    let source = r#"class Entity
{
	vector GetOrigin();
}

class Example
{
	void Run(Entity ent)
	{
		ent.GetOrigin();
	}
}
"#;

    let report = hover_at(source, "ent.GetOrigin", "GetOrigin");

    assert_eq!(report.selected_kind, Some(SymbolKind::Method));
    assert_eq!(report.selected_label.as_deref(), Some("GetOrigin"));
    assert_eq!(
        report.identifier_context,
        Some(IdentifierContext::MemberAccess)
    );
    assert_eq!(
        report.resolver_reason,
        Some(ResolutionReason::ReceiverMember)
    );
    assert_eq!(report.resolver_candidate_count, 1);
    assert_eq!(
        report
            .receiver_resolution
            .as_ref()
            .and_then(|receiver| receiver.owner_type.as_deref()),
        Some("Entity")
    );
}

#[test]
fn hover_uses_external_index_for_type_position_symbols() {
    let source = r#"class Example
{
	void Run()
	{
		Widget widget;
	}
}
"#;
    let external = file_index_for_source("class Widget {}").index;
    let position = position_for_needle(source, "Widget widget", "Widget");
    let report = hover_report_for_source_position_with_external(source, position, Some(&external));

    assert!(report.is_hit());
    assert_eq!(report.selected_kind, Some(SymbolKind::Class));
    assert_eq!(report.selected_label.as_deref(), Some("Widget"));
    assert_eq!(report.selected_source, Some(CandidateSource::External));
    assert_eq!(
        report.hover.as_ref().and_then(|hover| hover.range),
        Some(LspRange {
            start: position,
            end: LspPosition {
                line: position.line,
                character: position.character + 6,
            },
        })
    );
    assert_eq!(
        report.identifier_context,
        Some(IdentifierContext::TypePosition)
    );
}

#[test]
fn hover_and_definition_resolve_type_references_after_an_incomplete_for_statement() {
    let source = r#"class Example
{
	void Run()
	{
		for (t)

		GRAY_TEST2 test44;
		test44.TestNumFun();
	}
}

class GRAY_TEST2
{
	void TestNumFun();
}
"#;
    let hover = hover_at(source, "GRAY_TEST2 test44", "GRAY_TEST2");
    assert!(hover.is_hit(), "{hover:?}");
    assert_eq!(hover.selected_kind, Some(SymbolKind::Class));
    assert_eq!(hover.selected_label.as_deref(), Some("GRAY_TEST2"));

    let definition = definition_at(source, "GRAY_TEST2 test44", "GRAY_TEST2");
    assert!(definition.is_hit(), "{definition:?}");
    assert_eq!(definition.selected_kind, Some(SymbolKind::Class));
    assert_eq!(definition.selected_label.as_deref(), Some("GRAY_TEST2"));

    let method_hover = hover_at(source, "test44.TestNumFun", "TestNumFun");
    assert!(method_hover.is_hit(), "{method_hover:?}");
    assert_eq!(method_hover.selected_kind, Some(SymbolKind::Method));
    assert_eq!(method_hover.selected_label.as_deref(), Some("TestNumFun"));

    let method_definition = definition_at(source, "test44.TestNumFun", "TestNumFun");
    assert!(method_definition.is_hit(), "{method_definition:?}");
    assert_eq!(method_definition.selected_kind, Some(SymbolKind::Method));
    assert_eq!(
        method_definition.selected_label.as_deref(),
        Some("TestNumFun")
    );
}

#[test]
fn hover_type_usage_renders_same_class_display_as_class_declaration() {
    let source = r#"class Example
{
	void Run()
	{
		SCR_BaseGameModeStateComponent stateComponent;
	}
}
"#;
    let external = file_index_for_source(
        r#"//! Base component for handling game mode states.
class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent
{
	bool GetAllowControls();
	float GetDuration();
}

class SCR_BaseGameModeComponent
{
	void InheritedRun();
}
"#,
    )
    .index;

    let report = hover_report_for_source_position_with_external(
        source,
        position_for_needle(
            source,
            "SCR_BaseGameModeStateComponent stateComponent",
            "SCR_BaseGameModeStateComponent",
        ),
        Some(&external),
    );
    let markdown = report.hover.as_ref().unwrap().contents.value.as_str();

    assert_eq!(report.selected_kind, Some(SymbolKind::Class));
    assert_eq!(
        report.selected_label.as_deref(),
        Some("SCR_BaseGameModeStateComponent")
    );
    assert_eq!(report.selected_source, Some(CandidateSource::External));
    assert_eq!(
        report.identifier_context,
        Some(IdentifierContext::TypePosition)
    );
    assert!(markdown.contains("<span style=\"color:#59A6E9;\">Class</span>"));
    assert!(markdown.contains(
        "data-code=\"class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent\""
    ));
    assert!(markdown.contains("Base component for handling game mode states."));
    assert!(markdown.contains("### Functions"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetAllowControls</span>"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetDuration</span>"));
    assert!(!markdown.contains("### Inherited members"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">InheritedRun</span>"));
    assert!(!markdown.contains("inherited from"));
}

#[test]
fn hover_class_declaration_uses_external_overlay_for_inherited_member_summary() {
    let source = r#"//! Base component for handling game mode states.
class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent
{
	bool GetAllowControls();
	float GetDuration();
}
"#;
    let external = file_index_for_source(
        r#"//! Base component for handling game mode states.
class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent
{
	bool GetAllowControls();
	float GetDuration();
}

class SCR_BaseGameModeComponent
{
	void InheritedRun();
}
"#,
    )
    .index;

    let report = hover_report_for_source_position_with_external(
        source,
        position_for_needle(
            source,
            "SCR_BaseGameModeStateComponent",
            "SCR_BaseGameModeStateComponent",
        ),
        Some(&external),
    );
    let markdown = report.hover.as_ref().unwrap().contents.value.as_str();

    assert_eq!(report.selected_source, Some(CandidateSource::FileLocal));
    assert_eq!(
        report.selected_label.as_deref(),
        Some("SCR_BaseGameModeStateComponent")
    );
    assert!(markdown.contains(
        "data-code=\"class SCR_BaseGameModeStateComponent : SCR_BaseGameModeComponent\""
    ));
    assert!(markdown.contains("### Functions"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetAllowControls</span>"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">GetDuration</span>"));
    assert!(!markdown.contains("### Inherited members"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">InheritedRun</span>"));
    assert!(!markdown.contains("inherited from"));
}

#[test]
fn file_local_symbols_beat_external_symbols() {
    let source = r#"class Widget {}
class Example
{
	void Run()
	{
		Widget widget;
	}
}
"#;
    let external = file_index_for_source("class Widget {}").index;
    let report = hover_report_for_source_position_with_external(
        source,
        position_for_needle(source, "Widget widget", "Widget"),
        Some(&external),
    );

    assert!(report.is_hit());
    assert_eq!(report.selected_kind, Some(SymbolKind::Class));
    assert_eq!(report.selected_label.as_deref(), Some("Widget"));
    assert_eq!(report.selected_source, Some(CandidateSource::FileLocal));
}

#[test]
fn completion_returns_members_for_receiver_and_replaces_prefix() {
    let source = r#"class Example
{
	void Run()
	{
		Widget widget;
		widget.Set
	}
}
"#;
    let external = file_index_for_source(
        r#"class Widget
{
	void SetVisible(bool visible);
	void SetText(string text);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "widget.Set"),
        Some(&external),
    );

    assert_eq!(report.receiver_text.as_deref(), Some("widget"));
    assert_eq!(report.owner_type.as_deref(), Some("Widget"));
    assert_eq!(report.prefix, "Set");
    assert!(report.candidate_count >= 2);
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "SetVisible"
            && item.kind == 2
            && item.text_edit.new_text == "SetVisible(${1:visible})"
            && item.insert_text_format == Some(2)
            && item
                .label_details
                .as_ref()
                .and_then(|details| details.detail.as_deref())
                == Some("(bool visible)")
            && item
                .label_details
                .as_ref()
                .and_then(|details| details.description.as_deref())
                == Some("-> void")
            && item.text_edit.range.start.character == 9
            && item.text_edit.range.end.character == 12));
}

#[test]
fn completion_follows_external_game_api_return_type_chain() {
    let game_source = r#"class Example
{
	void Run()
	{
		GetGame().
	}
}
"#;
    let external = file_index_for_source(
        r#"class Game {}

class ChimeraGame : Game
{
	proto external PlayerController GetPlayerController();
}

class ArmaReforgerScripted : ChimeraGame {}

ArmaReforgerScripted GetGame();

class PlayerController
{
	proto external IEntity GetControlledEntity();
}
"#,
    )
    .index;
    assert_eq!(
        external
            .methods_by_owner_name("ChimeraGame", "GetPlayerController")
            .len(),
        1
    );
    assert_eq!(
        crate::expression_type::member_lookup_owners(&external, "ArmaReforgerScripted"),
        vec!["ArmaReforgerScripted", "ChimeraGame"]
    );

    let game_report = completion_report_for_source_position_with_external(
        game_source,
        position_after_needle(game_source, "GetGame()."),
        Some(&external),
    );
    assert_eq!(
        game_report.owner_type.as_deref(),
        Some("ArmaReforgerScripted")
    );
    assert!(game_report
        .list
        .items
        .iter()
        .any(|item| item.label == "GetPlayerController"));

    let controller_source = r#"class Example
{
	void Run()
	{
		GetGame().GetPlayerController().
	}
}
"#;
    let controller_report = completion_report_for_source_position_with_external(
        controller_source,
        position_after_needle(controller_source, "GetGame().GetPlayerController()."),
        Some(&external),
    );
    assert_eq!(
        controller_report.owner_type.as_deref(),
        Some("PlayerController")
    );
    assert!(controller_report
        .list
        .items
        .iter()
        .any(|item| item.label == "GetControlledEntity"));
}

#[test]
fn completion_hides_restricted_members_for_external_receivers() {
    let source = r#"class GRAY_TEST2
{
	protected void proTestnum();
	private void proPrivate();
	void proPublic();
}

class Other
{
	void Run()
	{
		GRAY_TEST2 test33;
		test33.pro
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "test33.pro"),
        None,
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"proPublic"));
    assert!(!labels.contains(&"proTestnum"));
    assert!(!labels.contains(&"proPrivate"));
}

#[test]
fn completion_keeps_restricted_members_for_self_receivers() {
    let source = r#"class GRAY_TEST2
{
	protected void proTestnum();
	private void proPrivate();
	void Run()
	{
		this.pro
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "this.pro"),
        None,
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"proTestnum"));
    assert!(labels.contains(&"proPrivate"));
}

#[test]
fn completion_returns_type_candidates_in_type_position() {
    let source = "class Example { void Run(SCR_ value) {} }";
    let external = file_index_for_source(
        r#"class SCR_Widget {}
enum SCR_Mode {}
typedef int SCR_Alias;
void SCR_Function();
int SCR_Global;
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "SCR_"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "type");
    assert_eq!(report.prefix, "SCR_");
    assert_eq!(
        report
            .list
            .items
            .iter()
            .map(|item| (item.label.as_str(), item.kind))
            .collect::<Vec<_>>(),
        vec![("SCR_Mode", 13), ("SCR_Alias", 25), ("SCR_Widget", 7)]
    );
    assert!(report
        .list
        .items
        .iter()
        .all(|item| item.text_edit.range.start.character == 25
            && item.text_edit.range.end.character == 29));
}

#[test]
fn completion_uses_identifier_prefix_inside_existing_token() {
    let source = "class Example { void Run(SCR_Widget value) { GetGame(); } }";
    let external = file_index_for_source(
        r#"class SCR_Widget {}
class SCR_Other {}
void GetGame();
void GetGameMode();
"#,
    )
    .index;

    let type_report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "SCR_"),
        Some(&external),
    );
    let value_report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "GetG"),
        Some(&external),
    );

    assert_eq!(type_report.completion_context, "type");
    assert_eq!(type_report.prefix, "SCR_");
    assert!(type_report
        .list
        .items
        .iter()
        .any(|item| item.label == "SCR_Widget"));
    assert_eq!(value_report.completion_context, "top-level");
    assert_eq!(value_report.prefix, "GetG");
    assert!(value_report
        .list
        .items
        .iter()
        .any(|item| item.label == "GetGame"));
}

#[test]
fn completion_returns_type_candidates_in_generic_type_argument() {
    let source = "class Example { void Run() { array<SCR_> values; } }";
    let external = file_index_for_source(
        r#"class SCR_Widget {}
void SCR_Function();
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "SCR_"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "type");
    assert_eq!(
        report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["SCR_Widget"]
    );
}

#[test]
fn completion_expands_builtin_collections_with_type_slots() {
    let source = "class Example { void Run(arr value) {} }";
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "arr"),
        None,
    );

    assert_eq!(report.completion_context, "type");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "array")
        .expect("expected array collection completion");
    assert_eq!(item.text_edit.new_text, "array<${1}>");
    assert_eq!(item.insert_text_format, Some(2));
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholder")
    );
}

#[test]
fn completion_expands_collection_at_an_incomplete_member_declaration_start() {
    let source = "class Example { arr }";
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "arr"),
        None,
    );
    let array = report
        .list
        .items
        .iter()
        .find(|item| item.label == "array")
        .expect("array completion");
    assert_eq!(array.text_edit.new_text, "array<${1}>");
    assert_eq!(array.insert_text_format, Some(2));
}

#[test]
fn completion_expands_map_and_ref_type_slots() {
    let map_source = "class Example { void Run(m value) {} }";
    let map_report = completion_report_for_source_position_with_external(
        map_source,
        position_after_needle(map_source, "Run(m"),
        None,
    );
    let map = map_report
        .list
        .items
        .iter()
        .find(|item| item.label == "map")
        .unwrap();
    assert_eq!(map.text_edit.new_text, "map<${1}, ${2}>");
    assert_eq!(
        map.command
            .as_ref()
            .and_then(|command| command.arguments.as_ref()),
        Some(&vec![json!(""), json!("")])
    );

    let ref_source = "class Example { void Run(r value) {} }";
    let ref_report = completion_report_for_source_position_with_external(
        ref_source,
        position_after_needle(ref_source, "Run(r"),
        None,
    );
    let reference = ref_report
        .list
        .items
        .iter()
        .find(|item| item.label == "ref")
        .unwrap();
    assert_eq!(reference.text_edit.new_text, "ref ${1}");
}

#[test]
fn completion_offers_collection_snippets_in_every_supported_type_position() {
    let samples = [
        "class Example { arr value; }",
        "class Example { void Run() { arr value; } }",
        "class Example { void Run(arr value) {} }",
        "class Example { arr Run() {} }",
        "class Base {} class Example : arr {}",
        "class Example { void Run() { new arr; } }",
    ];

    for source in samples {
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, "arr"),
            None,
        );
        let collection = report
            .list
            .items
            .iter()
            .find(|item| item.label == "array")
            .unwrap_or_else(|| panic!("missing array completion for {source:?}: {report:?}"));
        assert_eq!(collection.text_edit.new_text, "array<${1}>");
    }
}

#[test]
fn generic_collection_type_completion_excludes_void_and_ranks_builtin_types_first() {
    let source = "class Example { void Run(array<Type value) {} }";
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "array<Type"),
        None,
    );
    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();

    assert!(!labels.contains(&"void"));
    let int = labels.iter().position(|label| *label == "int");
    let reference = labels.iter().position(|label| *label == "ref");
    let array = labels.iter().position(|label| *label == "array");
    assert!(int < reference, "{labels:?}");
    assert!(reference < array, "{labels:?}");
}

#[test]
fn empty_collection_type_slots_open_ranked_type_completion() {
    for (source, needle) in [
        ("class Example { void Run(array<> value) {} }", "array<"),
        (
            "class Example { void Run(map<int, > value) {} }",
            "map<int, ",
        ),
    ] {
        let report = completion_report_for_source_position_with_external(
            source,
            position_after_needle(source, needle),
            None,
        );
        let labels = report
            .list
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(report.completion_context, "type");
        assert!(
            labels.starts_with(&["int", "auto", "bool", "float"]),
            "{labels:?}"
        );
        assert!(!labels.contains(&"void"));
        assert!(
            labels.iter().position(|label| *label == "ref").unwrap()
                < labels.iter().position(|label| *label == "array").unwrap()
        );
    }
}

#[test]
fn precise_indexed_type_match_beats_weaker_builtin_prefix() {
    let source = "class Example { void Run(fl value) {} }";
    let external = file_index_for_source("class FL {}").index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "Run(fl"),
        Some(&external),
    );
    assert_eq!(
        report.list.items.first().map(|item| item.label.as_str()),
        Some("FL")
    );
}

#[test]
fn completion_offers_collection_declaration_tail_choices_after_space() {
    let source = "class Example\n{\n\tarray<int> values \n}";
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "values "),
        None,
    );

    assert_eq!(report.completion_context, "collection-declaration-tail");
    let custom = report
        .list
        .items
        .iter()
        .find(|item| item.text_edit.new_text == " = ${1:Expression}")
        .expect("custom initializer item");
    assert_eq!(custom.label, "Custom initializer\u{2026}");
    assert_eq!(custom.insert_text_format, Some(2));
    assert_eq!(
        custom
            .command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholder")
    );
    /* assert_eq!(
        report
            .list
            .items
            .iter()
            .map(|item| (item.label.as_str(), item.text_edit.new_text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("Initialize with empty literal", " = {};"),
            ("Initialize with new array", " = new array<int>;"),
            ("Declare without initializer", ";"),
            ("Custom initializer…", " = ${1:Expression}"),
        ]
    ); */
    assert_eq!(
        report
            .list
            .items
            .iter()
            .map(|item| item.text_edit.new_text.as_str())
            .collect::<Vec<_>>(),
        vec![" = {};", " = new array<int>;", ";", " = ${1:Expression}"]
    );
}

#[test]
fn completion_returns_attribute_classes_in_attribute_name_position() {
    let source = r#"class Example
{
	[Attribu]
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"class Attribute
{
	void Attribute(string defvalue = "");
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "Attribu"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "Attribu");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "Attribute"
            && item.kind == 7
            && item.text_edit.new_text == "Attribute($0)"
            && item.insert_text_format == Some(2)
            && item.optional_parameter_count == 1
            && item
                .command
                .as_ref()
                .map(|command| command.command.as_str())
                == Some("editor.action.triggerParameterHints")));
}

#[test]
fn completion_wraps_attribute_shorthand_at_declaration_boundary() {
    let source = r#"class Example
{
	attribut
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "");
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "attribut"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "type");
    assert_eq!(report.prefix, "attribut");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "Attribute"
            && item.kind == 7
            && item.text_edit.new_text == "[Attribute($0)]"
            && item.insert_text_format == Some(2)
            && item.optional_parameter_count == 1));
}

#[test]
fn completion_wraps_indirect_unique_attribute_shorthand() {
    let source = r#"class Example
{
	custom
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"class UniqueAttribute {}
class SharedAttributeBase : UniqueAttribute {}
class CustomFlag : SharedAttributeBase
{
	void CustomFlag(string value = "");
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "custom"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "type");
    assert_eq!(report.prefix, "custom");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "CustomFlag"
            && item.kind == 7
            && item.text_edit.new_text == "[CustomFlag($0)]"
            && item.insert_text_format == Some(2)
            && item.optional_parameter_count == 1));
}

#[test]
fn completion_returns_optional_parameter_labels_inside_attribute_args() {
    let source = r#"class Example
{
	[Attribute(defv)]
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "", string uiwidget = "auto", string desc = "");
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "defv"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "defv");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "defvalue")
        .expect("expected defvalue parameter-label completion");
    assert_eq!(item.kind, 10);
    assert_eq!(item.text_edit.new_text, "defvalue");
    assert_eq!(item.insert_text_format, Some(2));
    assert_eq!(item.optional_parameter_count, 1);
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("editor.action.triggerParameterHints")
    );
}

#[test]
fn completion_returns_parameter_labels_inside_function_calls() {
    let source = r#"void SendToEveryone(ENotification notificationID, int param1 = 0, string label = "ok");

class Example
{
	void Run()
	{
		SendToEveryone(notif)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "SendToEveryone(notif"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "notif");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "notificationID")
        .expect("expected function parameter-label completion");
    assert_eq!(item.text_edit.new_text, "notificationID");
    assert_eq!(item.required_parameter_count, 1);
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("editor.action.triggerParameterHints")
    );
}

#[test]
fn completion_prefers_positional_value_when_prefix_matches_active_parameter_name() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(input)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "TestNumFun2(input"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "input");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "input")
        .expect("expected positional input value completion");
    assert_eq!(item.text_edit.new_text, "input");
    assert!(!report
        .list
        .items
        .iter()
        .any(|item| item.text_edit.new_text == "input: $0"));
}

#[test]
fn completion_does_not_offer_active_parameter_label_for_positional_slot_prefix() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(inp)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "TestNumFun2(inp"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "inp");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "input" && item.text_edit.new_text == "input"));
    assert!(!report
        .list
        .items
        .iter()
        .any(|item| item.text_edit.new_text == "input: $0"));
}

#[test]
fn completion_keeps_value_candidates_after_parameter_labels() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num, string testValue)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(input, tes)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "TestNumFun2(input, tes"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "tes");
    let first = report
        .list
        .items
        .first()
        .expect("expected parameter label completion");
    assert_eq!(first.label, "test");
    assert_eq!(first.text_edit.new_text, "test: $0");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "testValue" && item.text_edit.new_text == "testValue"));
}

#[test]
fn completion_uses_active_parameter_when_no_matching_value_exists() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(inpu, num,)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "TestNumFun2(inpu"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "inpu");
    let first = report
        .list
        .items
        .first()
        .expect("expected active parameter completion");
    assert_eq!(first.label, "input");
    assert_eq!(first.text_edit.new_text, "input");
}

#[test]
fn completion_parameter_labels_default_enum_arguments_to_enum_owner() {
    let source = r#"enum ENotification
{
	PLAYER_JOINED
}

void SendToEveryone(ENotification notificationID, int param1 = 0);

class Example
{
	void Run()
	{
		SendToEveryone(notif)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "SendToEveryone(notif"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "notif");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "notificationID")
        .expect("expected enum-backed function parameter-label completion");
    assert_eq!(item.text_edit.new_text, "${0:ENotification.}");
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("editor.action.triggerParameterHints")
    );
    assert_eq!(item.required_parameter_count, 1);
}

#[test]
fn completion_uses_named_parameter_when_parameter_is_out_of_order() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(num, inp)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "TestNumFun2(num, inp"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "inp");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "input")
        .expect("expected out-of-order input named-parameter completion");
    assert_eq!(item.text_edit.new_text, "input: $0");
}

#[test]
fn completion_offers_active_parameter_for_empty_trailing_argument_slot() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run(int input, float num)
	{
		GRAY_TEST2 test44;
		test44.TestNumFun2(input, num,)
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "TestNumFun2(input, num,"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "");
    let first = report
        .list
        .items
        .first()
        .expect("expected parameter completions for trailing argument slot");
    assert_eq!(first.label, "test");
    assert_eq!(first.text_edit.new_text, "test");
}

#[test]
fn completion_attribute_shorthand_defaults_required_enum_parameters_to_enum_owners() {
    let source = r#"class Example
{
	rplr int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Server
	Owner
}
enum RplCondition
{
	None
}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver, RplCondition condition = RplCondition.None, string customConditionName = "");
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "rplr"),
        Some(&external),
    );

    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "RplRpc")
        .expect("expected RplRpc attribute shorthand completion");
    assert_eq!(
        item.text_edit.new_text,
        "[RplRpc(${1:RplChannel.Reliable}, ${2:RplRcver.Server})]"
    );
    let wire_item = serde_json::to_value(item).unwrap();
    assert_eq!(
        wire_item["textEdit"]["newText"],
        "[RplRpc(${1:RplChannel.Reliable}, ${2:RplRcver.Server})]"
    );
    assert!(wire_item["textEdit"].get("range").is_some());
    assert!(wire_item["textEdit"].get("insert").is_none());
    assert!(wire_item["textEdit"].get("replace").is_none());
    assert_eq!(wire_item["insertTextFormat"], 2);
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("reforger-sript-tools.completion.triggerSuggestAtSnippetPlaceholder")
    );
    assert_eq!(
        item.command
            .as_ref()
            .and_then(|command| command.arguments.as_ref())
            .cloned(),
        Some(vec![
            serde_json::json!("RplChannel.Reliable"),
            serde_json::json!("RplRcver.Server")
        ])
    );
    assert_eq!(item.required_parameter_count, 2);
    assert_eq!(item.optional_parameter_count, 2);
}

#[test]
fn completion_enum_member_advances_attribute_snippet_to_next_parameter() {
    let source = r#"class Example
{
	[RplRpc(RplChannel.)]
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Server
	Owner
}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver);
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "RplChannel."),
        Some(&external),
    );

    assert_eq!(report.completion_context, "member");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "RplChannel.Reliable")
        .expect("expected enum member completion");
    assert_eq!(item.text_edit.new_text, "RplChannel.Reliable");
    assert_eq!(item.insert_text_format, None);
    assert_eq!(item.filter_text.as_deref(), Some("RplChannel."));
    assert!(item.command.is_none());
    let wire_item = serde_json::to_value(item).unwrap();
    assert_eq!(
        wire_item["textEdit"]["range"],
        serde_json::to_value(item.text_edit.range).unwrap()
    );
    assert!(wire_item["textEdit"].get("insert").is_none());
    assert_eq!(item.text_edit.range.start.character, 9);
    assert_eq!(item.text_edit.range.end.character, 20);

    let source = r#"class Example
{
	[RplRpc(RplChannel.Reliable, RplRcver.)]
	int m_Value;
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "RplRcver."),
        Some(&external),
    );
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "RplRcver.Server")
        .expect("expected final enum member completion");
    assert_eq!(item.text_edit.new_text, "RplRcver.Server");
    assert!(item.command.is_none());
}

#[test]
fn completion_falls_back_to_value_candidates_when_argument_label_prefix_has_no_match() {
    let source = r#"int testChannel;

class Example
{
	[RplRpc(tes, RplRcver.Server)]
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"enum RplChannel
{
	Reliable
}
enum RplRcver
{
	Server
}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver);
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "RplRpc(tes"),
        Some(&external),
    );

    assert_eq!(report.prefix, "tes");
    assert_ne!(report.completion_context, "argument-label");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "testChannel"));
}

#[test]
fn completion_returns_parameter_labels_inside_constructor_calls() {
    let source = r#"class Widget
{
	void Widget(string name = "", int value = 0);
}

class Example
{
	void Run()
	{
		Widget widget = new Widget(na);
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "new Widget(na"),
        None,
    );

    assert_eq!(report.completion_context, "argument-label");
    assert_eq!(report.prefix, "na");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "name")
        .expect("expected constructor parameter-label completion");
    assert_eq!(item.text_edit.new_text, "name");
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("editor.action.triggerParameterHints")
    );
    assert_eq!(item.optional_parameter_count, 1);
}

#[test]
fn callable_completion_triggers_signature_help_after_insert() {
    let source = r#"class GRAY_TEST2
{
	int TestNumFun2(int input, float num, string test = "eeeeee");
}

class Example
{
	void Run()
	{
		GRAY_TEST2 test44;
		test44.TestNum
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "test44.TestNum"),
        None,
    );

    assert_eq!(report.completion_context, "member");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "TestNumFun2")
        .expect("expected callable member completion");
    assert_eq!(item.text_edit.new_text, "TestNumFun2(${1:input}, ${2:num})");
    assert_eq!(item.required_parameter_count, 2);
    assert_eq!(item.optional_parameter_count, 1);
    assert_eq!(
        item.command
            .as_ref()
            .map(|command| command.command.as_str()),
        Some("editor.action.triggerParameterHints")
    );
}

#[test]
fn callable_enum_placeholders_cover_methods_constructors_and_attributes() {
    let enum_declarations = r#"enum FirstChoice { First }
enum SecondChoice { Second }
"#;

    let method_source = format!(
        r#"{enum_declarations}
class Example
{{
	void UseChoices(FirstChoice first, SecondChoice second, int count);
	void Run()
	{{
		UseCho
	}}
}}
"#,
    );
    let report = completion_report_for_source_position_with_external(
        &method_source,
        position_after_needle(&method_source, "UseCho"),
        None,
    );
    let method = report
        .list
        .items
        .iter()
        .find(|item| item.label == "UseChoices")
        .unwrap();
    assert_eq!(
        method.text_edit.new_text,
        "UseChoices(${1:FirstChoice.}, ${2:SecondChoice.}, ${3:count})"
    );
    assert_eq!(
        method
            .command
            .as_ref()
            .and_then(|command| command.arguments.as_ref()),
        Some(&vec![
            serde_json::json!("FirstChoice."),
            serde_json::json!("SecondChoice.")
        ])
    );

    let constructor_source = format!(
        r#"{enum_declarations}
class Widget
{{
	void Widget(FirstChoice first, SecondChoice second, int count);
}}
class Example
{{
	void Run()
	{{
		Widget widget = new Widg
	}}
}}
"#,
    );
    let report = completion_report_for_source_position_with_external(
        &constructor_source,
        position_after_needle(&constructor_source, "new Widg"),
        None,
    );
    let constructor = report
        .list
        .items
        .iter()
        .find(|item| item.label == "Widget")
        .unwrap();
    assert_eq!(
        constructor.text_edit.new_text,
        "Widget(${1:FirstChoice.}, ${2:SecondChoice.}, ${3:count})"
    );
    assert_eq!(
        constructor
            .command
            .as_ref()
            .and_then(|command| command.arguments.as_ref()),
        Some(&vec![
            serde_json::json!("FirstChoice."),
            serde_json::json!("SecondChoice.")
        ])
    );

    let attribute_source = format!(
        r#"{enum_declarations}
class UniqueAttribute {{}}
class ChoiceAttribute : UniqueAttribute
{{
	void ChoiceAttribute(FirstChoice first, SecondChoice second, int count);
}}
class Example
{{
	cho int m_Value;
}}
"#,
    );
    let report = completion_report_for_source_position_with_external(
        &attribute_source,
        position_after_needle(&attribute_source, "cho"),
        None,
    );
    let attribute = report
        .list
        .items
        .iter()
        .find(|item| item.label == "ChoiceAttribute")
        .unwrap();
    assert_eq!(
        attribute.text_edit.new_text,
        "[ChoiceAttribute(${1:FirstChoice.}, ${2:SecondChoice.}, ${3:count})]"
    );
    assert_eq!(
        attribute
            .command
            .as_ref()
            .and_then(|command| command.arguments.as_ref()),
        Some(&vec![
            serde_json::json!("FirstChoice."),
            serde_json::json!("SecondChoice.")
        ])
    );
}

#[test]
fn completion_hides_already_supplied_parameter_labels() {
    let source = r#"class Example
{
	[Attribute(DEFVALUE: "", defv)]
	int m_Value;
}
"#;
    let external = file_index_for_source(
        r#"class UniqueAttribute {}
class Attribute : UniqueAttribute
{
	void Attribute(string defvalue = "", string uiwidget = "auto", string desc = "");
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "DEFVALUE: \"\", defv"),
        Some(&external),
    );

    assert!(!report
        .list
        .items
        .iter()
        .any(|item| item.label == "defvalue"));
}

#[test]
fn completion_returns_top_level_value_candidates_for_prefix() {
    let source = "class Example { void Run() { SCR_ } }";
    let external = file_index_for_source(
        r#"class SCR_Widget {}
enum SCR_Mode
{
	SCR_Value
}
typedef int SCR_Alias;
void SCR_Function();
int SCR_Global;
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "SCR_"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "SCR_");
    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"SCR_Function"));
    assert!(labels.contains(&"SCR_Global"));
    assert!(!labels.contains(&"SCR_Value"));
}

#[test]
fn completion_caps_broad_top_level_prefixes() {
    let source = "class Example { void Run() { s } }";
    let mut external_source = String::new();
    for index in 0..400 {
        external_source.push_str(&format!("class sGenerated{index} {{}}\n"));
    }
    let external = file_index_for_source(&external_source).index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "{ s"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "s");
    assert_eq!(report.list.items.len(), 250);
    assert_eq!(report.candidate_count, 250);
    assert!(report.list.is_incomplete);
}

#[test]
fn completion_returns_visible_locals_for_unqualified_value_prefix() {
    let source = r#"class Example
{
	void Run(IEntity owner)
	{
		ow
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "ow"),
        None,
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "ow");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "owner" && item.kind == 6 && item.text_edit.new_text == "owner"));
}

#[test]
fn completion_returns_current_class_members_for_unqualified_value_prefix() {
    let source = r#"class Example
{
	IEntity GetOwner();
	void Run()
	{
		GetO
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "GetO"),
        None,
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "GetO");
    assert!(report.list.items.iter().any(|item| item.label == "GetOwner"
        && item.kind == 2
        && item.text_edit.new_text == "GetOwner()"));
}

#[test]
fn completion_matches_unqualified_prefix_case_insensitively() {
    let source = r#"class Example
{
	IEntity GetOwner();
	void Run(IEntity owner)
	{
		if (owner == get)
		{
		}
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "get"),
        None,
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "get");
    assert!(report.list.items.iter().any(|item| item.label == "GetOwner"
        && item.kind == 2
        && item.text_edit.new_text == "GetOwner()"));
}

#[test]
fn completion_returns_cross_layer_inherited_members_for_unqualified_prefix() {
    let source = r#"class Example : ScriptComponent
{
	void Run(IEntity owner)
	{
		if (owner == getow)
		{
		}
	}
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	IEntity GetOwner();
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "getow"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "getow");
    assert!(report.list.items.iter().any(|item| item.label == "GetOwner"
        && item.kind == 2
        && item.text_edit.new_text == "GetOwner()"));
}

#[test]
fn completion_keeps_value_context_for_incomplete_statement_before_declaration() {
    let source = r#"class Game
{
}

Game GetGame();

class Example
{
	void Run()
	{
		getgam

		int testnum = 44;

		GetGame().GetPlayerController().GetControlledEntity();
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "getgam"),
        None,
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "getgam");
    assert!(report.list.items.iter().any(|item| item.label == "GetGame"
        && item.kind == 3
        && item.text_edit.new_text == "GetGame()"));
}

#[test]
fn completion_returns_language_keywords_for_value_prefixes() {
    let source = r#"class Example
{
	void Run()
	{
		retur
	}
}
"#;
    let external = file_index_for_source(
        r#"enum EOrder
{
	RETURN_FIRE,
	RETURN_TO_PREVIOUS_STATE
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "retur"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "retur");
    let first = report.list.items.first().unwrap();
    assert_eq!(first.label, "return");
    assert_eq!(first.kind, 14);
    assert_eq!(first.text_edit.new_text, "return");
    assert!(!report
        .list
        .items
        .iter()
        .any(|item| item.label == "RETURN_FIRE"));
}

#[test]
fn completion_ranks_closest_keyword_before_matching_source_symbols() {
    let source = r#"class Example
{
	void Run()
	{
		stati
	}
}
"#;
    let external = file_index_for_source(
        r#"enum EStaticKind
{
	STATIC,
	STATIC_ARLAND_AIRBASE
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "stati"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "stati");
    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels.first().copied(), Some("static"));
    assert!(!labels.contains(&"STATIC"));
    assert!(!labels.contains(&"Static"));
}

#[test]
fn completion_ranks_primitive_type_keyword_before_modifier_prefix() {
    let source = r#"class Example
{
	void Run()
	{
		in
	}
}
"#;
    let external = file_index_for_source(
        r#"class int
{
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "\t\tin"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "in");
    let labels = report
        .list
        .items
        .iter()
        .map(|item| (item.label.as_str(), item.kind))
        .collect::<Vec<_>>();
    assert_eq!(labels.first().copied(), Some(("int", 14)));
    assert!(labels.contains(&("inout", 14)));
    assert_eq!(
        labels.iter().filter(|(label, _)| *label == "int").count(),
        1
    );
}

#[test]
fn completion_keeps_declaration_keywords_out_of_expression_contexts() {
    let source = r#"class Example
{
	void Run(bool enabled)
	{
		if (enabled == stati)
		{
		}
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "stati"),
        None,
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(!labels.contains(&"static"));
}

#[test]
fn completion_returns_declaration_keywords_at_declaration_boundaries() {
    let source = r#"class Example
{
	sta
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "sta"),
        None,
    );

    let first = report.list.items.first().unwrap();
    assert_eq!(first.label, "static");
    assert_eq!(first.kind, 14);
}

#[test]
fn completion_returns_modifier_keywords_when_prefix_is_type_context() {
    let boundary_source = r#"class Example
{
	overr
}
"#;
    let boundary_report = completion_report_for_source_position_with_external(
        boundary_source,
        position_after_needle(boundary_source, "overr"),
        None,
    );
    let first = boundary_report.list.items.first().unwrap();
    assert_eq!(first.label, "override");
    assert_eq!(first.kind, 14);
    assert_eq!(first.text_edit.new_text, "override");

    let type_context_source = r#"class Example
{
	override overr
}
"#;
    let report = completion_report_for_source_position_with_external(
        type_context_source,
        position_after_needle(type_context_source, "override overr"),
        None,
    );
    let first = report.list.items.first().unwrap();
    assert_eq!(first.label, "override");
    assert_eq!(first.kind, 14);
    assert_eq!(first.text_edit.new_text, "override");
}

#[test]
fn completion_returns_inherited_override_method_skeletons() {
    let source = r#"class Child : Parent
{
	OnPostIn
}
"#;
    let external = file_index_for_source(
        r#"class Parent
{
	protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "OnPostIn"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    assert_eq!(report.prefix, "OnPostIn");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "OnPostInit")
        .expect("expected inherited override completion");
    assert_eq!(item.kind, 2);
    assert_eq!(item.detail.as_deref(), Some("override protected void"));
    assert_eq!(item.insert_text_format, Some(2));
    assert_eq!(
        item.text_edit.new_text,
        "override protected void OnPostInit(IEntity owner)\n{\n\t$0\n}"
    );
}

#[test]
fn completion_returns_override_skeletons_for_event_base_methods() {
    let source = r#"class Child : ScriptComponent
{
	onpostin
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "onpostin"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "OnPostInit")
        .expect("expected event base method override completion");
    assert_eq!(
        item.text_edit.new_text,
        "override protected void OnPostInit(IEntity owner)\n{\n\t$0\n}"
    );
}

#[test]
fn completion_ranks_short_override_prefix_before_fuzzy_global_symbols() {
    let source = r#"class Child : ScriptComponent
{
	on
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}

class SCR_OrientToSeaNormalContextAction {}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "\ton"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    assert_eq!(report.prefix, "on");
    assert_eq!(
        report.list.items.first().map(|item| item.label.as_str()),
        Some("OnPostInit")
    );
}

#[test]
fn completion_keeps_override_keyword_when_override_skeletons_are_available() {
    let source = r#"class Child : ScriptComponent
{
	o
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "\to"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels.first().copied(), Some("override"));
    assert!(labels.contains(&"OnPostInit"));
}

#[test]
fn completion_keeps_source_symbols_when_override_skeletons_are_available() {
    let source = r#"class Child : Parent
{
	rp
}
"#;
    let external = file_index_for_source(
        r#"class UniqueAttribute {}
class RplProp : UniqueAttribute
{
}

class Parent
{
	protected bool RplLoad(ScriptBitReader reader);
	protected bool RplSave(ScriptBitWriter writer);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "rp"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    assert_eq!(report.prefix, "rp");
    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert!(labels.contains(&"RplLoad"));
    assert!(labels.contains(&"RplSave"));
    assert!(labels.contains(&"RplProp"));
}

#[test]
fn completion_ranks_closest_source_symbol_before_capping() {
    let source = r#"class Example
{
	rp
}
"#;
    let mut external_source = String::new();
    for index in 0..400 {
        external_source.push_str(&format!("class RplGenerated{index} {{}}\n"));
    }
    external_source.push_str("class RplProp {}\n");
    let external = file_index_for_source(&external_source).index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "rp"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "rp");
    assert_eq!(report.list.items.len(), 250);
    assert!(report.list.is_incomplete);
    assert_eq!(report.list.items.first().unwrap().label, "RplProp");
    assert!(report.list.items.iter().any(|item| item.label == "RplProp"));
}

#[test]
fn completion_match_quality_beats_source_rank_for_top_level_symbols() {
    let source = r#"typedef func SCR_BaseGameMode_PlayerId;
typedef func SCR_BaseGameMode_PlayerIdAndEntity;
typedef func SCR_BaseGameMode_OnPlayerRoleChanged;

class Example
{
	rplr
}
"#;
    let external = file_index_for_source(
        r#"enum RplRcver {}
class UniqueAttribute {}
class RplRpc : UniqueAttribute
{
	void RplRpc(RplChannel channel, RplRcver rcver, RplCondition condition = RplCondition.None);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "rplr"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "top-level");
    assert_eq!(report.prefix, "rplr");
    let labels = report
        .list
        .items
        .iter()
        .take(5)
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels.first().copied(), Some("RplRpc"));
    assert!(labels.contains(&"RplRcver"));
    assert!(
        labels.iter().position(|label| *label == "RplRpc").unwrap()
            < labels
                .iter()
                .position(|label| *label == "SCR_BaseGameMode_PlayerId")
                .unwrap_or(usize::MAX)
    );
}

#[test]
fn completion_override_skeleton_omits_already_typed_modifiers() {
    let source = r#"class Child : ScriptComponent
{
	override protected onpostin
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "onpostin"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    let item = report
        .list
        .items
        .iter()
        .find(|item| item.label == "OnPostInit")
        .expect("expected inherited override completion");
    assert_eq!(item.detail.as_deref(), Some("void"));
    assert_eq!(
        item.text_edit.new_text,
        "void OnPostInit(IEntity owner)\n{\n\t$0\n}"
    );
}

#[test]
fn completion_returns_override_skeletons_before_inline_comment_at_class_scope() {
    let source = r#"class GRAY_TEST : ScriptComponent
{
	int testnnn;
	onpostin//Nothing appearing

	override protected void OnPostInit(IEntity owner)
	{
	}
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "onpostin"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "OnPostInit"));
}

#[test]
fn completion_returns_override_skeletons_before_following_method_without_comment() {
    let source = r#"class GRAY_TEST : ScriptComponent
{
	int testnnn;
	onpostin

	override protected void OnPostInit(IEntity owner)
	{
	}
}
"#;
    let external = file_index_for_source(
        r#"class ScriptComponent
{
	event protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "onpostin"),
        Some(&external),
    );

    assert_eq!(report.completion_context, "override");
    assert!(report
        .list
        .items
        .iter()
        .any(|item| item.label == "OnPostInit"));
}

#[test]
fn completion_excludes_private_and_static_methods_from_override_skeletons() {
    let source = r#"class Child : Parent
{
	On
}
"#;
    let external = file_index_for_source(
        r#"class Parent
{
	private void OnPrivate();
	static void OnStatic();
	protected void OnAllowed();
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "On"),
        Some(&external),
    );
    let labels = report
        .list
        .items
        .iter()
        .filter(|item| item.text_edit.new_text.starts_with("override "))
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();

    assert!(labels.contains(&"OnAllowed"));
    assert!(!labels.contains(&"OnPrivate"));
    assert!(!labels.contains(&"OnStatic"));
    assert!(!report
        .list
        .items
        .iter()
        .any(|item| item.text_edit.new_text.starts_with("override static")));
}

#[test]
fn completion_does_not_return_override_skeletons_inside_method_bodies() {
    let source = r#"class Child : Parent
{
	void Run()
	{
		OnPostIn
	}
}
"#;
    let external = file_index_for_source(
        r#"class Parent
{
	protected void OnPostInit(IEntity owner);
}
"#,
    )
    .index;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "OnPostIn"),
        Some(&external),
    );

    assert_ne!(report.completion_context, "override");
    assert!(!report
        .list
        .items
        .iter()
        .any(|item| item.text_edit.new_text.contains("override protected void")));
}

#[test]
fn completion_returns_empty_inside_comments() {
    let source = r#"class Example
{
	void Run()
	{
		// get
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "get"),
        None,
    );

    assert_eq!(report.completion_context, "none");
    assert!(report.list.items.is_empty());
}

#[test]
fn completion_returns_empty_inside_block_comments_after_code() {
    let source = r#"class Example
{
	void Run()
	{
		int testnnn = 1; /* testnnn */
	}
}
"#;
    let report = completion_report_for_source_position_with_external(
        source,
        position_for_needle(source, "/* testnnn", "test"),
        None,
    );

    assert_eq!(report.completion_context, "none");
    assert!(report.list.items.is_empty());
}

#[test]
fn completion_returns_enum_members_for_static_enum_owner() {
    let source = r#"enum LogLevel
{
	DEBUG,
	NORMAL
}

class Example
{
	void Run()
	{
		LogLevel.
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "LogLevel."),
        None,
    );

    assert_eq!(report.receiver_text.as_deref(), Some("LogLevel"));
    assert_eq!(report.owner_type.as_deref(), Some("LogLevel"));
    assert_eq!(
        report
            .list
            .items
            .iter()
            .take(2)
            .map(|item| (item.label.as_str(), item.kind))
            .collect::<Vec<_>>(),
        vec![("LogLevel.DEBUG", 20), ("LogLevel.NORMAL", 20)]
    );
    assert!(report.list.items.len() > 2);
    assert!(
        report
            .list
            .items
            .iter()
            .take(2)
            .all(|item| item.command.is_none()),
        "enum members themselves do not carry callable commands; ranked value fallbacks may"
    );
    let enum_item = &report.list.items[0];
    let wire_item = serde_json::to_value(enum_item).unwrap();
    assert_eq!(
        wire_item["textEdit"]["range"],
        serde_json::to_value(enum_item.text_edit.range).unwrap()
    );
    assert!(wire_item["textEdit"].get("insert").is_none());
    assert_eq!(enum_item.text_edit.range.start.character, 2);
    assert_eq!(enum_item.text_edit.range.end.character, 11);
}

#[test]
fn completion_returns_static_class_members_for_static_class_owner() {
    let source = r#"class Example
{
	static int s_Value;
	static void StaticRun();
	void InstanceRun();
	int m_Value;
}

class User
{
	void Run()
	{
		Example.
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "Example."),
        None,
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(report.receiver_text.as_deref(), Some("Example"));
    assert_eq!(labels, vec!["s_Value", "StaticRun"]);
}

#[test]
fn completion_returns_engine_class_cast_for_static_class_owner() {
    let source = r#"class Example
{
}

class User
{
	void Run()
	{
		Example.
	}
}
"#;
    let external = file_index_for_source(
        r#"class Class
{
	static Class Cast(Class from);
}

class Example
{
}
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "Example."),
        Some(&external),
    );

    assert_eq!(
        report
            .list
            .items
            .iter()
            .map(|item| (item.label.as_str(), item.kind))
            .collect::<Vec<_>>(),
        vec![("Cast", 2)]
    );
}

#[test]
fn completion_expands_typedef_receiver_owner() {
    let source = r#"class Example
{
	void Run(TIntArray values)
	{
		values.
	}
}
"#;
    let external = file_index_for_source(
        r#"class array<Class T>
{
	void Insert(T value);
	void Remove(T value);
}

typedef array<int> TIntArray;
"#,
    )
    .index;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "values."),
        Some(&external),
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(report.owner_type.as_deref(), Some("TIntArray"));
    assert_eq!(labels, vec!["Insert", "Remove"]);
}

#[test]
fn completion_infers_direct_new_expression_receiver() {
    let source = r#"class SCR_AIAnimateBehavior
{
	array<string> GetPortNames();
}

class Example
{
	void Run()
	{
		(new SCR_AIAnimateBehavior()).
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "())."),
        None,
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        report.receiver_text.as_deref(),
        Some("(new SCR_AIAnimateBehavior())")
    );
    assert_eq!(report.owner_type.as_deref(), Some("SCR_AIAnimateBehavior"));
    assert_eq!(labels, vec!["GetPortNames"]);
}

#[test]
fn completion_uses_full_receiver_chain_before_dot() {
    let source = r#"class AIWaypoint
{
	string ToString();
}

class SCR_BTParam<Class T>
{
	T m_Value;
}

class SCR_AIDefendBehavior
{
	ref SCR_BTParam<AIWaypoint> m_RelatedWaypoint;

	void Run()
	{
		m_RelatedWaypoint.m_Value.
	}
}
"#;

    let report = completion_report_for_source_position_with_external(
        source,
        position_after_needle(source, "m_Value."),
        None,
    );

    let labels = report
        .list
        .items
        .iter()
        .map(|item| item.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        report.receiver_text.as_deref(),
        Some("m_RelatedWaypoint.m_Value")
    );
    assert_eq!(report.owner_type.as_deref(), Some("AIWaypoint"));
    assert_eq!(labels, vec!["ToString"]);
}

#[test]
fn completion_returns_empty_for_non_member_positions_and_unresolved_receivers() {
    let non_member = "class Example {}";
    let non_member_report = completion_report_for_source_position_with_external(
        non_member,
        LspPosition {
            line: 0,
            character: 5,
        },
        None,
    );
    assert!(non_member_report.list.items.is_empty());

    let unresolved = "class Example { void Run() { missing. } }";
    let unresolved_report = completion_report_for_source_position_with_external(
        unresolved,
        position_after_needle(unresolved, "missing."),
        None,
    );
    assert_eq!(unresolved_report.receiver_text.as_deref(), Some("missing"));
    assert_eq!(unresolved_report.owner_type, None);
    assert!(unresolved_report.list.items.is_empty());
    assert_eq!(
        unresolved_report.failure_reason.as_deref(),
        Some("receiver type was not inferred")
    );
}

#[test]
fn hover_returns_none_for_whitespace_outside_symbols() {
    let source = "\nclass Example {}\n";

    let report = hover_report_for_source_position(
        source,
        LspPosition {
            line: 0,
            character: 0,
        },
    );

    assert!(!report.is_hit());
    assert_eq!(report.parse_diagnostics, 0);
    assert_eq!(report.selection_source, HoverSelectionSource::None);
    assert_eq!(report.resolver_reason, None);
    assert_eq!(report.resolver_candidate_count, 0);
}

#[test]
fn hover_uses_resolver_syntax_span_for_non_identifier_inside_symbol_span() {
    let source = r#"class Example
{
	void Run(int value);
}
"#;

    let report = hover_at(source, "void Run", "void");

    assert!(report.is_hit());
    assert_eq!(
        report.selection_source,
        HoverSelectionSource::ResolverSyntaxSpan
    );
    assert_eq!(report.resolver_reason, Some(ResolutionReason::SyntaxSpan));
    assert!(report.resolver_candidate_count > 0);
    assert_eq!(report.selected_kind, Some(SymbolKind::Method));
    assert_eq!(report.selected_label.as_deref(), Some("Run"));
    assert_eq!(
        report.hover.as_ref().and_then(|hover| hover.range),
        Some(LspRange {
            start: LspPosition {
                line: 2,
                character: 6,
            },
            end: LspPosition {
                line: 2,
                character: 9,
            },
        })
    );
}

#[test]
fn hover_does_not_use_broad_class_span_for_modifiers() {
    let source = r#"class Example : Base
{
	protected RplComponent m_RplComponent;
	private static const int COUNT = 4;
}
"#;

    for (needle, cursor) in [
        ("protected RplComponent", "protected"),
        ("private static", "private"),
        ("static const", "static"),
        ("const int", "const"),
    ] {
        let report = hover_at(source, needle, cursor);
        assert!(
            !report.is_hit(),
            "modifier `{cursor}` should not select enclosing symbol: {report:?}"
        );
    }
}

#[test]
fn hover_returns_none_for_comments_inside_symbol_span() {
    let source = r#"class ExampleClass
{
	/*
		ExampleClass comment text should not select the class.
	*/
}
"#;

    let report = hover_at(source, "ExampleClass comment", "ExampleClass");

    assert!(!report.is_hit());
    assert_eq!(report.selection_source, HoverSelectionSource::None);
    assert_eq!(report.resolver_reason, None);
    assert_eq!(report.resolver_candidate_count, 0);
}

#[test]
fn debug_hover_does_not_select_symbol_for_comments_inside_symbol_span() {
    let source = r#"class ExampleClass
{
	/*
		ExampleClass comment text should not select the class.
	*/
}
"#;
    let position = position_for_needle(source, "ExampleClass comment", "ExampleClass");

    let report = debug_hover_report_for_source_position(source, position);

    assert!(report.contains("- Selected Symbol: no"));
    assert!(report.contains("Cursor is not on an identifier token"));
    assert!(report.contains("No symbol matched the cursor position."));
    assert!(!report.contains("| 1 | syntax-span | `Class` | `ExampleClass`"));
}

#[test]
fn hover_returns_none_for_unresolved_identifier_without_syntax_span_selection() {
    let source = r#"class Example
{
	void Run()
	{
		MissingThing();
	}
}
"#;

    let report = hover_at(source, "MissingThing();", "MissingThing");

    assert!(!report.is_hit());
    assert_eq!(report.selection_source, HoverSelectionSource::None);
    assert_eq!(report.resolver_reason, Some(ResolutionReason::Unresolved));
    assert_eq!(report.resolver_candidate_count, 0);
}

#[test]
fn definition_selects_declarations_and_usages() {
    let source = r#"typedef string FactionKey;
Game g_Game;
enum ExampleFlags
{
	Enabled = 1
}
class Example
{
	protected int m_Value;
	void Run(string name)
	{
		int localValue = 5;
		localValue = localValue + 1;
		Print(name);
		m_Value = localValue;
		FactionKey key;
		ExampleFlags flag = ExampleFlags.Enabled;
		g_Game = null;
	}
}
"#;
    let uri = "file:///Scripts/Definition.c";

    assert_definition(
        source,
        uri,
        "class Example",
        "Example",
        SymbolKind::Class,
        "Example",
        "file:///Scripts/Definition.c",
    );
    assert_definition(
        source,
        uri,
        "localValue + 1",
        "localValue",
        SymbolKind::LocalVariable,
        "localValue",
        "file:///Scripts/Definition.c",
    );
    assert_definition(
        source,
        uri,
        "Print(name)",
        "name",
        SymbolKind::Parameter,
        "name",
        "file:///Scripts/Definition.c",
    );
    assert_definition(
        source,
        uri,
        "m_Value = localValue",
        "m_Value",
        SymbolKind::Field,
        "m_Value",
        "file:///Scripts/Definition.c",
    );
    assert_definition(
        source,
        uri,
        "FactionKey key",
        "FactionKey",
        SymbolKind::Typedef,
        "FactionKey",
        "file:///Scripts/Definition.c",
    );
    assert_definition(
        source,
        uri,
        "ExampleFlags.Enabled",
        "Enabled",
        SymbolKind::EnumMember,
        "Enabled",
        "file:///Scripts/Definition.c",
    );
    assert_definition(
        source,
        uri,
        "g_Game = null",
        "g_Game",
        SymbolKind::GlobalField,
        "g_Game",
        "file:///Scripts/Definition.c",
    );
}

#[test]
fn definition_returns_null_for_non_targets() {
    let source = r#"class LogLevel {}
class Example
{
	void Run()
	{
		Print("hello", level: LogLevel);
		MissingThing();
	}
}
"#;
    let whitespace = definition_report_for_source_position(
        source,
        "file:///Scripts/Definition.c",
        LspPosition {
            line: 0,
            character: 0,
        },
    );
    assert!(!whitespace.is_hit());
    assert_eq!(whitespace.resolver_reason, None);

    let named_arg = definition_at(source, "level: LogLevel", "level");
    assert!(!named_arg.is_hit());
    assert_eq!(
        named_arg.resolver_reason,
        Some(ResolutionReason::NamedArgumentLabel)
    );

    let unresolved = definition_at(source, "MissingThing();", "MissingThing");
    assert!(!unresolved.is_hit());
    assert_eq!(
        unresolved.resolver_reason,
        Some(ResolutionReason::Unresolved)
    );
}

#[test]
fn definition_resolves_preprocessor_macro_references_when_defined() {
    let source = r#"#define ENABLE_DIAG
#ifdef ENABLE_DIAG
#define GAME_MODE_DEBUG
#endif
"#;

    let report = definition_report_for_source_position(
        source,
        "file:///Scripts/Preprocessor.c",
        position_for_needle(source, "#ifdef ENABLE_DIAG", "ENABLE_DIAG"),
    );

    assert!(report.is_hit(), "{report:?}");
    assert_eq!(report.selected_kind, Some(SymbolKind::PreprocessorMacro));
    assert_eq!(report.selected_label.as_deref(), Some("ENABLE_DIAG"));
    assert_eq!(
        report.resolver_reason,
        Some(ResolutionReason::PreprocessorMacro)
    );
    assert_eq!(
        report.locations[0].range.start,
        LspPosition {
            line: 0,
            character: 8
        }
    );

    let missing = definition_report_for_source_position(
        "#ifdef MISSING_DIAG\n#endif\n",
        "file:///Scripts/Preprocessor.c",
        LspPosition {
            line: 0,
            character: 8,
        },
    );
    assert!(!missing.is_hit());
    assert_eq!(
        missing.resolver_reason,
        Some(ResolutionReason::PreprocessorMacro)
    );
}

#[test]
fn definition_uses_external_file_uri_when_available() {
    let root = temp_test_dir("external_definition");
    fs::create_dir_all(&root).unwrap();
    let external_path = root.join("External Type.c");
    fs::write(&external_path, "class ExternalType\n{\n\tvoid Run();\n}\n").unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            &root,
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = r#"class Example
{
	void Run()
	{
		ExternalType value;
	}
}
"#;

    let report = definition_report_for_source_position_with_external(
        source,
        "file:///Scripts/Definition.c",
        position_for_needle(source, "ExternalType value", "ExternalType"),
        Some(&external),
    );

    assert!(report.is_hit());
    assert_eq!(report.selected_source, Some(CandidateSource::External));
    assert_eq!(report.selected_kind, Some(SymbolKind::Class));
    assert_eq!(report.selected_label.as_deref(), Some("ExternalType"));
    assert_eq!(report.locations.len(), 1);
    assert!(report.locations[0].uri.ends_with("/External%20Type.c"));
    assert_eq!(
        report.locations[0].range.start,
        LspPosition {
            line: 0,
            character: 6
        }
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_resolves_keyword_type_positions_to_external_generated_types() {
    let root = temp_test_dir("external_keyword_type_definition");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("string.c"), "sealed class string\n{\n}\n").unwrap();
    fs::write(root.join("vector.c"), "sealed class vector\n{\n}\n").unwrap();
    fs::write(root.join("bool.c"), "sealed class bool\n{\n}\n").unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            &root,
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = r#"class Example
{
	string m_sValue;
	vector m_vValue;
	bool m_bValue;
	void Run()
	{
		bool value = true;
	}
}
"#;

    for (needle, cursor, expected) in [
        ("string m_sValue", "string", "string.c"),
        ("vector m_vValue", "vector", "vector.c"),
        ("bool m_bValue", "bool", "bool.c"),
    ] {
        let report = definition_report_for_source_position_with_external(
            source,
            "file:///Scripts/KeywordTypes.c",
            position_for_needle(source, needle, cursor),
            Some(&external),
        );
        assert!(report.is_hit(), "{cursor}: {report:?}");
        assert_eq!(report.selected_source, Some(CandidateSource::External));
        assert_eq!(report.selected_kind, Some(SymbolKind::Class));
        assert!(
            report.locations[0].uri.ends_with(expected),
            "{:?}",
            report.locations
        );
    }

    let literal = definition_report_for_source_position_with_external(
        source,
        "file:///Scripts/KeywordTypes.c",
        position_for_needle(source, "true;", "true"),
        Some(&external),
    );
    assert!(!literal.is_hit());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn definition_resolves_receiver_member_with_external_index() {
    let root = temp_test_dir("receiver_definition");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("Entity.c"),
        "class IEntity\n{\n\tvector GetOrigin();\n}\n",
    )
    .unwrap();
    let external = crate::index_build::build_index(&crate::index_build::IndexBuildConfig {
        roots: vec![crate::index_build::IndexSourceRoot::new(
            &root,
            crate::model::SourceKind::GameData,
            crate::model::SOURCE_PRIORITY_GAME_DATA,
        )],
    })
    .unwrap()
    .index;
    let source = r#"class Example
{
	void Run(IEntity ent)
	{
		ent.GetOrigin();
	}
}
"#;

    let report = definition_report_for_source_position_with_external(
        source,
        "file:///Scripts/Definition.c",
        position_for_needle(source, "ent.GetOrigin", "GetOrigin"),
        Some(&external),
    );

    assert!(report.is_hit());
    assert_eq!(report.selected_source, Some(CandidateSource::External));
    assert_eq!(report.selected_kind, Some(SymbolKind::Method));
    assert_eq!(report.selected_label.as_deref(), Some("GetOrigin"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_uri_for_path_encodes_windows_style_paths_and_spaces() {
    assert_eq!(file_uri_for_path(Path::new("relative/File.c")), None);
    if cfg!(windows) {
        let uri = file_uri_for_path(Path::new("C:\\Game Data\\Scripts\\File Name.c")).unwrap();
        assert_eq!(uri, "file:///C:/Game%20Data/Scripts/File%20Name.c");
    } else {
        let uri = file_uri_for_path(Path::new("/tmp/Game Data/File Name.c")).unwrap();
        assert_eq!(uri, "file:///tmp/Game%20Data/File%20Name.c");
    }
}

#[test]
fn file_uri_for_path_encodes_windows_unc_authority() {
    if cfg!(windows) {
        assert_eq!(
            file_uri_for_path(Path::new(r"\\server\share\File Name.c")).unwrap(),
            "file://server/share/File%20Name.c"
        );
        assert_eq!(
            file_uri_for_path(Path::new(r"\\?\UNC\server\share\File.c")).unwrap(),
            "file://server/share/File.c"
        );
    }
}

#[test]
fn hover_markdown_uses_signature_detail_docs_modifiers_and_attributes() {
    let source = r#"//! Runs the example.
class Example
{
	//! Runs the example.
	[Attribute("0")]
	protected void Run(int value = 4);
}
"#;

    let report = hover_at(source, "Run(int", "Run");
    let hover = report.hover.unwrap();
    let markdown = hover.contents.value;

    assert!(markdown.contains("data-code=\"protected void Run(int value = 4)\""));
    assert!(markdown.contains("<span style=\"color:#59A6E9;\">protected</span>"));
    assert!(markdown.contains("<span style=\"color:#59A6E9;\">void</span>"));
    assert!(markdown.contains("<span style=\"color:#f3ad58;\">Run</span>"));
    assert!(markdown.contains("Runs the example."));
    assert!(!markdown.contains("Modifiers: protected"));
    assert!(!markdown.contains("Attributes: Attribute"));
}

#[test]
fn position_index_preserves_utf16_and_crlf_boundaries() {
    let source = "ab😀\r\nclass Marker {}";
    let index = LspPositionIndex::new(source);

    assert_eq!(
        index.position_for_offset(source.find('😀').expect("emoji")),
        LspPosition {
            line: 0,
            character: 2
        }
    );
    assert_eq!(
        index.position_for_offset(source.find("class").expect("second line")),
        LspPosition {
            line: 1,
            character: 0
        }
    );
}

#[test]
fn only_single_full_text_changes_are_coalescible() {
    let full_text = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": "file:///Scripts/Full.c", "version": 2 },
            "contentChanges": [{ "text": "class Full {}" }]
        }
    });
    let ranged = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": "file:///Scripts/Range.c", "version": 2 },
            "contentChanges": [{
                "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } },
                "text": "x"
            }]
        }
    });
    let multiple = json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": { "uri": "file:///Scripts/Multiple.c", "version": 2 },
            "contentChanges": [{ "text": "class A {}" }, { "text": "class B {}" }]
        }
    });

    assert_eq!(
        coalescible_full_sync_did_change(&full_text).map(|change| (change.uri, change.version)),
        Some(("file:///Scripts/Full.c".to_string(), 2))
    );
    assert!(coalescible_full_sync_did_change(&ranged).is_none());
    assert!(coalescible_full_sync_did_change(&multiple).is_none());
}

#[test]
fn semantic_tokens_keep_existing_rich_display_until_current_rich_result() {
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
    let uri = "file:///Scripts/StableTokens.c";
    for (method, params) in [(
        "textDocument/didOpen",
        json!({ "textDocument": {
            "uri": uri, "languageId": "enforce", "version": 1,
            "text": "class SCR_GameModeEndData {}"
        }}),
    )] {
        server
            .handle_message(
                json!({ "jsonrpc": "2.0", "method": method, "params": params }),
                None,
                0,
                0,
            )
            .unwrap();
    }
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 1, "method": "textDocument/semanticTokens/full", "params": { "textDocument": { "uri": uri } } }),
            None, 0, 0,
        )
        .unwrap();
    assert!(
        server
            .document_runtime
            .test_document_state(uri)
            .unwrap()
            .rich_semantic_tokens
    );

    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": "// edit\nclass SCR_GameModeEndData {}" }]
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 2, "method": "textDocument/semanticTokens/full", "params": { "textDocument": { "uri": uri } } }),
            None, 0, 0,
        )
        .unwrap();

    let output = String::from_utf8_lossy(&server.writer);
    assert!(output.contains("\"id\":2"));
    assert!(
        output.contains("\"resultId\":\"reforger:2:rich:"),
        "{output}"
    );
    assert!(!output.contains("\"resultId\":\"reforger:2:lexical\""));
    assert!(!output.contains("workspace/semanticTokens/refresh"));
}

#[test]
fn block_comment_pair_returns_a_current_revision_multiline_edit_and_selection() {
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
    let uri = "file:///Scripts/BlockCommentPair.c";
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": "\t/**/"
                }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 1, "method": BLOCK_COMMENT_PAIR_METHOD, "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 },
                "version": 1,
                "options": { "tabSize": 4, "insertSpaces": false }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    let output = String::from_utf8_lossy(&server.writer);
    assert!(
        output.contains("\"newText\":\"/*\\n\\t\\t\\n\\t*/\""),
        "{output}"
    );
    assert!(
        output.contains("\"selection\":{\"character\":2,\"line\":1}"),
        "{output}"
    );

    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 2, "method": BLOCK_COMMENT_PAIR_METHOD, "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 3 },
                "version": 2,
                "options": { "tabSize": 4, "insertSpaces": false }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    assert!(String::from_utf8_lossy(&server.writer).contains("\"edits\":[]"));
}

#[test]
fn range_formatting_projects_current_comment_only_edits() {
    let mut server = LspServer::new(Vec::new(), LspServerOptions::default());
    let uri = "file:///Scripts/CommentFormatting.c";
    let source = "\t//! First\n  //! \\param value Input\n\tint value;\n";
    let comment_end = source.find("\tint").unwrap();
    let range = LspRange {
        start: LspPosition {
            line: 0,
            character: 0,
        },
        end: position_for_offset(source, comment_end),
    };
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "enforce",
                    "version": 1,
                    "text": source
                }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 1, "method": RANGE_FORMATTING_METHOD, "params": {
                "textDocument": { "uri": uri },
                "range": range,
                "options": { "tabSize": 4, "insertSpaces": false }
            }}),
            None,
            0,
            0,
        )
        .unwrap();

    let output = String::from_utf8_lossy(&server.writer);
    assert!(output.contains("\"newText\":\"\\t\""), "{output}");
    assert!(output.contains("\"character\":2,\"line\":1"), "{output}");

    server.writer.clear();
    server
        .handle_message(
            json!({ "jsonrpc": "2.0", "id": 2, "method": RANGE_FORMATTING_METHOD, "params": {
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 2, "character": 1 }
                },
                "options": { "tabSize": 4, "insertSpaces": false }
            }}),
            None,
            0,
            0,
        )
        .unwrap();
    assert!(String::from_utf8_lossy(&server.writer).contains("\"result\":[]"));
}

#[test]
fn parser_diagnostic_projection_adds_stable_source_and_code() {
    let source = "class Broken\n{\n\tvoid Run(\n}\n";
    let parse = parse_source(source);

    let diagnostics = parser_diagnostics_for_source(source, &parse.diagnostics);

    assert!(!diagnostics.is_empty());
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.source == diagnostics::PARSER_DIAGNOSTIC_SOURCE));
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.code == diagnostics::PARSER_DIAGNOSTIC_CODE));
    assert!(diagnostics
        .iter()
        .all(|diagnostic| diagnostic.severity == 1));
}

#[test]
fn parser_diagnostic_projection_expands_zero_width_ranges() {
    let source = "class Broken\n";
    let diagnostics = parser_diagnostics_for_source(
        source,
        &[ParseDiagnostic {
            message: "Expected declaration".to_string(),
            span: TextSpan::new(source.len(), source.len()),
        }],
    );

    let range = diagnostics[0].range;
    assert_ne!(
        range.start, range.end,
        "zero-width parser diagnostics should project to a visible editor range"
    );
}

#[test]
fn debug_hover_report_includes_language_engine_context() {
    let source = "class Smoke\n{\n\tvoid Run(int value);\n}\n";
    let hover_position = position_for_needle(source, "Run(int", "Run");

    let report = debug_hover_report_for_source_position(source, hover_position);

    assert!(report.contains("# Reforger Hover Debug"));
    assert!(report.contains("## Resolver Resolution"));
    assert!(report.contains("## Tokens Around Cursor"));
    assert!(report.contains("## Semantic Token Coloring Context"));
    assert!(report.contains("## Candidate Symbols At Cursor"));
    assert!(report.contains("- Selected Symbol: yes"));
    assert!(report.contains("- Label: `Run`"));
    assert!(report.contains("Smoke.Run(int value) -> void"));
    assert!(report.contains("`Method`"));
    assert!(report.contains("`method`"));
    assert!(report.contains("#f3ad58"));
}

#[test]
fn diagnostic_log_is_structured_and_does_not_add_unprovided_source_content() {
    let path = std::env::temp_dir().join(format!(
        "reforger_lsp_diagnostic_{}_{}.jsonl",
        std::process::id(),
        timestamp_millis()
    ));
    let logger = LspLogger::new(None, Some(path.clone()));
    logger.diagnostic(
        "rpc.completed",
        json!({"method": "textDocument/hover", "elapsedMs": 4}),
    );
    logger.flush_diagnostics();
    let record = fs::read_to_string(&path).unwrap();
    assert!(record.contains("\"component\":\"languageServer\""));
    assert!(record.contains("\"method\":\"textDocument/hover\""));
    assert!(!record.contains("class Secret"));
    let _ = fs::remove_file(path);
}
