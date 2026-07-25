# Third-party add-on symbol metadata and interoperability

Research date: 2026-07-25. This note records a narrow licensing and technical
assessment for an editor feature that supplies completion and validation for a
user's own Reforger add-on when it depends on a third-party Workshop add-on.
It is not legal advice; the answer can turn on the selected add-on license and
the jurisdiction.

## Conclusion

Bohemia expressly supports the *compatibility result*: a project may declare a
Workshop mod as a dependency and call its exposed script API without copying
the implementation. Bohemia's example says the dependency reference and the
usual class/method identifiers are technical interoperability facts, normally
not protected expression. This is strong support for using an author-published
public API contract (names, kinds, signatures, bases, and deliberately public
documentation) to provide editor assistance.

The binding documents do **not** expressly grant an editor permission to
derive a symbol table from another add-on's package, loaded code, Workbench, or
network API. Workshop use remains conditional on the uploader's selected
license, and the Tools EULA prohibits reverse engineering the Tools, their
files/data, and network services. A documented API is therefore not itself a
license override.

Keeping the result only in a process-local, in-memory index materially reduces
the risks of redistribution, publication, and reuse of the information. It
does not by itself make the acquisition lawful: EU law treats temporary
reproduction, including reproduction necessitated by loading, as an act that
normally needs authorisation. It may nevertheless be relevant that a lawful
user may observe, study, or test a program while carrying out an entitled act
of loading/running, and that interfaces/underlying ideas are not protected
program expression. Whether a particular automated inspection stays within
those limits is fact- and jurisdiction-specific.

## Binding Bohemia terms

The [Workshop Terms of Use](https://reforger.armaplatform.com/workshop-terms)
allow Workshop users to download, use, alter, and further develop content, but
make each downloader's use subject to both the Terms and the license chosen by
the uploader (sections 2--4). A Workshop item can use a custom license. Thus a
feature must evaluate the particular item's license; a license that forbids
inspection, development use, or automated API harvesting is not displaced by
the general Workshop permission.

The [Tools EULA](https://store.steampowered.com/eula/1874910_eula_1) permits
non-commercial development/testing/production of game content for Bohemia
games, which covers the stated add-on-development purpose. It also prohibits
hacking, modification, or reverse engineering of the software or its files,
data, or network services. It does not specifically name third-party symbol
metadata, so it is not a direct answer to the proposed index; it does mean that
an undocumented extraction technique cannot be called authorised merely
because it runs through installed Workbench.

The [EULA FAQ](https://reforger.armaplatform.com/news/eula-faq) confirms that
the Tools EULA governs tool-created add-ons, the Workshop Terms govern Workshop
distribution, and the Workshop Terms do not negate the Tools EULA. It also
states that the EULA does not grant a general right to take, modify, or own
other authors' mods without their permission.

## Bohemia's directly relevant interoperability guidance

Bohemia's [Workshop Licenses and IP FAQ](https://reforger.armaplatform.com/news/workshop-licenses-and-ip-faq)
is expressly informational rather than a binding extension to the Workshop
Terms. Its APL-ND example nevertheless addresses this exact development model:
the third-party mod is a project dependency and the new mod calls an **exposed
script API**. Bohemia says that, in the example, the dependency reference and
usual class/method identifiers are technical requirements rather than
copyrightable creative expression, and that interfaces/class and method names
needed for interoperability are generally not protected program expression.
It contrasts that with copying substantial source, which it says creates
adapted material and breaches APL-ND. It further advises authors who do not
want such use to make classes sealed and members private/protected.

The inference for this extension is limited but useful: an index of only the
*public, exposed interface facts* needed to call a dependency is aligned with
Bohemia's stated interoperability rationale. The FAQ does not say that a tool
may discover those facts by unpacking or inspecting any third-party add-on;
neither should that permission be inferred from the FAQ.

## EU interoperability boundary

[Directive 2009/24/EC](https://eur-lex.europa.eu/eli/dir/2009/24/oj/eng),
Article 1(2), protects the expression of a computer program, not the ideas and
principles underlying it, including interfaces. Article 4(1)(a) makes permanent
or temporary reproduction subject to the right-holder where loading, display,
running, transmission, or storage requires it. Article 5(3) permits a person
entitled to use a copy to observe, study, or test its functioning while doing
those entitled acts, to determine underlying ideas/principles.

Article 6 supplies a narrower decompilation exception only when code
reproduction/translation is indispensable to achieve interoperability of an
independently created program, the information is not readily available, and
the work is confined to necessary parts. Information obtained that way may be
used only for interoperability, not passed on except as necessary for it, nor
used to make a substantially similar program. Article 8 preserves other law,
including contract and trade-secret law. The Directive is not a blanket right
to extract a mod's symbol data, and national implementation/application matters.

## Workbench NET API: no established symbol-query route

The official [Workbench NET API reference](https://community.bistudio.com/wikidata/external-data/arma-reforger/EnfusionScriptAPIPublic/Page_NetApi.html)
lists the built-in endpoints as resource opening, module focus, Workbench/world
status, and `ValidateScripts`; it does not document a built-in endpoint for
enumerating add-on script symbols. It supports custom endpoints only where
Enforce Script code implementing `NetApiHandler` is running in the Workbench
instance. Consequently, NET API is a transport for documented built-ins or for
a handler deliberately supplied by its author/project; it is not established
evidence of permission or capability to inspect arbitrary third-party add-on
code. No official source reviewed here establishes a Workbench LSP operation
that publishes third-party add-on symbols.

## Safe product boundary

1. Treat author-published API documentation/manifests and explicit author
   consent as the authoritative source of third-party symbol facts.
2. Limit records to an intentional public interface: public names, symbol
   kind, call signatures/types, inheritance facts needed for type-checking, and
   author-provided documentation. Exclude bodies, comments, literals,
   non-public members, assets, and any source reconstruction.
3. Keep the index process-local and discard it at shutdown; do not export,
   cache, publish, or use it for cross-mod search. This is a helpful minimising
   control, not a substitute for authority to acquire the facts.
4. Respect each add-on's license and an author opt-out. When no public
   contract exists or the license is unclear/restrictive, require author
   permission rather than deriving symbols from the package.
5. If Bohemia publishes a documented public-symbol endpoint, use that narrow
   endpoint. Before implementing package- or runtime-derived metadata, obtain
   written clarification from Bohemia and, for restrictive third-party items,
   their authors.
