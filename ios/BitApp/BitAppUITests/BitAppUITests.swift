import XCTest

/// Drives the real app through the simulator via Xcode's test runner
/// (not host-level window automation, which doesn't see Simulator's
/// window as a standard accessibility element — AppleScript/System
/// Events was tried and rejected for exactly this reason).
final class BitAppUITests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    /// A fresh, isolated save file per launch — see Engine.swift's
    /// `UITEST_SAVE_TOKEN` handling. Every test in this file needs a
    /// blank-slate app (Round 0 Demonstration, nothing discovered yet);
    /// without this, persistence working correctly would make tests leak
    /// state into each other.
    private func launchFreshApp(token: String = UUID().uuidString) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["UITEST_SAVE_TOKEN"] = token
        app.launch()
        return app
    }

    /// Puzzle of the Day is deliberately independent of discovery progress
    /// (it draws from every practicable concept, not the discovered-only
    /// `gamePool`) — so unlike endless Play, the card should be reachable
    /// from the very first launch. Covers the answering path (the exact
    /// correct answer is seeded and unknowable client-side, same reasoning
    /// as `testPlayEndlessGameMode`'s wrong-answer-only coverage) and that
    /// re-opening after answering shows the result directly, not the
    /// question again.
    func testDailyChallengeOfTheDayIsReachableAndTracksResult() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Play"].tap()

        let card = app.buttons["daily-challenge-card"]
        XCTAssertTrue(card.waitForExistence(timeout: 5), "Puzzle of the Day should be reachable before anything is discovered")
        card.tap()

        XCTAssertTrue(app.navigationBars["Puzzle of the Day"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'Day #'")).firstMatch.waitForExistence(timeout: 3))
        let field = app.textFields["daily-answer-field"]
        XCTAssertTrue(field.waitForExistence(timeout: 3))

        field.tap()
        field.typeText("definitely-not-a-valid-answer")
        app.buttons["daily-check-answer"].tap()

        XCTAssertTrue(app.staticTexts["Not quite"].waitForExistence(timeout: 3))
        let streakReadout = app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'day streak'")).firstMatch
        XCTAssertTrue(streakReadout.waitForExistence(timeout: 3))
        XCTAssertTrue(app.buttons["daily-share-button"].waitForExistence(timeout: 3))
        attachScreenshot(app, named: "daily-challenge-result")

        app.buttons["Close"].tap()
        XCTAssertTrue(card.waitForExistence(timeout: 3))
        card.tap()

        // Re-opening the same day should show the already-answered result
        // directly, never the question again.
        XCTAssertTrue(app.staticTexts["Not quite"].waitForExistence(timeout: 3))
        XCTAssertFalse(app.textFields["daily-answer-field"].exists, "an already-answered daily puzzle shouldn't be re-askable the same day")
    }

    /// Swiping the Daily Challenge card left collapses it into a small
    /// bubble (chat-heads style) rather than dismissing it outright —
    /// still one tap away, just out of the way for the rest of the
    /// session.
    func testDailyChallengeCardCollapsesIntoABubbleOnSwipe() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Play"].tap()

        let card = app.buttons["daily-challenge-card"]
        XCTAssertTrue(card.waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["daily-challenge-bubble"].exists)

        card.swipeLeft()

        let bubble = app.buttons["daily-challenge-bubble"]
        XCTAssertTrue(bubble.waitForExistence(timeout: 3), "swiping left should collapse the card into a bubble")
        XCTAssertFalse(card.exists, "the full card should be gone once collapsed")

        bubble.tap()
        XCTAssertTrue(app.navigationBars["Puzzle of the Day"].waitForExistence(timeout: 3), "the bubble should still open the same daily puzzle sheet")
    }

    /// Play is now pure puzzle mode: empty until something's discovered,
    /// then an endless stream of challenges drawn only from the discovered
    /// pool, with a plain running-total number on top (no picker, no
    /// interactive demos — those moved entirely to Explore).
    func testPlayEndlessGameMode() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Play"].tap()

        XCTAssertTrue(app.staticTexts["Nothing to play yet"].waitForExistence(timeout: 5))
        XCTAssertEqual(app.staticTexts["game-level-number"].label, "0", "level number should start at 0")

        // The empty state names a concrete starting concept ("Bits" has
        // no prerequisites, so it's always first) rather than a generic
        // "go learn something" — tapping it jumps straight to that
        // concept's Explore page, proving cross-tab deep-linking, not
        // just a tab switch.
        XCTAssertTrue(app.buttons["play-start-first-concept"].waitForExistence(timeout: 3))
        app.buttons["play-start-first-concept"].tap()
        XCTAssertTrue(app.navigationBars["Bits"].waitForExistence(timeout: 3)) // records discovery

        // "Bits" itself has no puzzle generator (see
        // GeneratorKind::default_for_concept), so Play is still empty —
        // but the suggestion should have moved on to "Binary", the next
        // concept whose only prerequisite ("bits") is now discovered.
        app.tabBars.buttons["Play"].tap()
        XCTAssertTrue(app.staticTexts["Nothing to play yet"].waitForExistence(timeout: 3))
        let startBinary = app.buttons["play-start-first-concept"]
        XCTAssertTrue(startBinary.waitForExistence(timeout: 3))
        XCTAssertEqual(startBinary.label, "Start with Binary")
        startBinary.tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3)) // records discovery

        app.tabBars.buttons["Play"].tap()
        XCTAssertFalse(app.staticTexts["Nothing to play yet"].exists, "discovering Binary should populate the pool")
        XCTAssertTrue(app.staticTexts["Binary"].waitForExistence(timeout: 5), "the puzzle's concept badge should show which concept it's drawn from")
        XCTAssertTrue(app.textFields["play-answer-field"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.buttons["play-check-answer"].exists)

        let puzzleAttachment = XCTAttachment(screenshot: app.screenshot())
        puzzleAttachment.lifetime = .keepAlways
        puzzleAttachment.name = "play-active-puzzle"
        add(puzzleAttachment)

        // The exact expected answer is seeded randomly client-side (by
        // design — bit-ffi never exposes it, see FfiChallenge's doc
        // comment), so a UI test can't predict a *correct* one. What's
        // verifiable end-to-end here is the wrong-answer path; the
        // correct-answer -> level-increment path is proven at the Rust
        // level instead (bit-ffi's game_progress_counts_and_suggests_a_
        // new_topic_after_threshold test).
        let levelNumber = app.staticTexts["game-level-number"]
        let field = app.textFields["play-answer-field"]
        field.tap()
        field.typeText("definitely-not-a-valid-answer")
        app.buttons["play-check-answer"].tap()
        // Every built-in generator's `validate_exact_match` always
        // populates an explanation on a miss ("Expected <value>") — the
        // exact value is unpredictable (seeded randomly), so match the
        // prefix rather than the whole string.
        let feedback = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'Expected '")).firstMatch
        XCTAssertTrue(feedback.waitForExistence(timeout: 3))
        XCTAssertEqual(levelNumber.label, "0", "a wrong answer should not move the level number")
        XCTAssertFalse(app.staticTexts["play-combo-multiplier"].exists, "a miss should never leave a combo badge showing")

        // A miss is a teaching moment: one tap on "Explain this" should
        // land on the concept's own Explore page, not just re-show the
        // same puzzle.
        XCTAssertTrue(app.buttons["play-explain-this"].waitForExistence(timeout: 3))
        app.buttons["play-explain-this"].tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3), "should land on Binary's real concept page, not a blank one")
        app.tabBars.buttons["Play"].tap()

        // Tab navigation — every tab loads real content, not a blank screen.
        app.tabBars.buttons["Build"].tap()
        XCTAssertTrue(app.navigationBars["Build"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary Counter"].waitForExistence(timeout: 3))
        attachScreenshot(app, named: "build-list")

        app.tabBars.buttons["Lab"].tap()
        XCTAssertTrue(app.navigationBars["Lab"].waitForExistence(timeout: 3))
        attachScreenshot(app, named: "lab-binary-playground")

        app.tabBars.buttons["Profile"].tap()
        XCTAssertTrue(app.navigationBars["Profile"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Concepts discovered"].waitForExistence(timeout: 3))
        attachScreenshot(app, named: "profile-overview")
    }

    private func attachScreenshot(_ app: XCUIApplication, named name: String) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.lifetime = .keepAlways
        attachment.name = name
        add(attachment)
    }

    func testExploreDrillDownAndSearch() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()

        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 5))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Bits & Binary"].waitForExistence(timeout: 3))

        app.staticTexts["Bits & Binary"].tap()
        XCTAssertTrue(app.navigationBars["Bits & Binary"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary"].waitForExistence(timeout: 3))

        // .firstMatch: the destination ConceptDetailView reuses "Binary"
        // as its own title, and under load its accessibility elements can
        // materialize before the push visually completes, making a bare
        // `staticTexts["Binary"]` query ambiguous between the list row
        // and the not-yet-visible destination.
        app.staticTexts["Binary"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3), "concept detail should render prose sections")

        // The concept page's interactive demo is live, not decorative —
        // toggle a switch inside it and confirm the value responds,
        // exactly like Play's copy of the same widget.
        XCTAssertTrue(app.buttons["switch-7"].waitForExistence(timeout: 3), "Binary's concept page should render its interactive_demo")
        app.buttons["switch-7"].tap()
        XCTAssertTrue(app.staticTexts["1"].waitForExistence(timeout: 3))

        let conceptAttachment = XCTAttachment(screenshot: app.screenshot())
        conceptAttachment.lifetime = .keepAlways
        conceptAttachment.name = "explore-concept-detail-binary"
        add(conceptAttachment)

        // Back out all three pushes (Binary -> Bits & Binary -> Foundations
        // -> root) — the search field only lives on the root of the
        // NavigationStack that `.searchable` is attached to.
        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()

        let searchField = app.searchFields.firstMatch
        XCTAssertTrue(searchField.waitForExistence(timeout: 3))
        searchField.tap()
        searchField.typeText("hexadecimal")
        XCTAssertTrue(app.staticTexts["hexadecimal"].waitForExistence(timeout: 3))

        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.lifetime = .keepAlways
        attachment.name = "explore-search-hexadecimal"
        add(attachment)
    }

    /// Explore is purely graph-driven — no Swift code changed to add the
    /// Data Representation domain, only `/content`. This confirms that
    /// held: new domains/concepts are reachable with zero app changes.
    func testNewDataRepresentationDomainIsReachableWithNoAppChanges() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()

        XCTAssertTrue(app.staticTexts["Data Representation"].waitForExistence(timeout: 5))
        app.staticTexts["Data Representation"].tap()
        XCTAssertTrue(app.navigationBars["Data Representation"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Integers"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Text Encoding"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Data Integrity"].waitForExistence(timeout: 3))

        app.staticTexts["Integers"].tap()
        XCTAssertTrue(app.navigationBars["Integers"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Two's Complement"].waitForExistence(timeout: 3))

        app.staticTexts["Two's Complement"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Two's Complement"].waitForExistence(timeout: 3))
        // Learning objectives render with a bullet prefix (see
        // ConceptDetailView's `Text("• \($0)")`).
        XCTAssertTrue(app.staticTexts["• Negate a value by flipping every bit and adding one"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Prerequisites"].waitForExistence(timeout: 3))
        // Inside ConceptDetailView's plain VStack (not a List), SwiftUI
        // exposes a NavigationLink's label as a Button's accessibility
        // label, not a separately-queryable staticText — unlike List
        // rows (e.g. DomainListView), which expose both.
        XCTAssertTrue(
            app.buttons["Signed Integers"].waitForExistence(timeout: 3),
            "prerequisite link should be real, not a placeholder"
        )

        // Two's Complement's demo shows the signed reading too, not just
        // the plain unsigned value — toggling the sign bit (switch 0)
        // should show a negative number.
        XCTAssertTrue(app.buttons["switch-0"].waitForExistence(timeout: 3))
        app.buttons["switch-0"].tap()
        XCTAssertTrue(app.staticTexts["signed: -128"].waitForExistence(timeout: 3), "sign bit alone should read as -128 in two's complement")

        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.lifetime = .keepAlways
        attachment.name = "explore-twos-complement-detail"
        add(attachment)
    }

    /// AND/OR/XOR/NOT all share one gate demo widget with a picker — this
    /// exercises the real FFI-backed gate session, not just that the
    /// picker renders.
    func testGateDemoComputesRealOutput() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()
        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 5))

        // A nav-bar wait after every push (not just a content-element
        // wait) before the next tap — under full-suite load, a content
        // element can exist transiently before its screen is actually
        // interactive, and the following tap lands on nothing.
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Logic"].waitForExistence(timeout: 3))

        app.staticTexts["Logic"].tap()
        XCTAssertTrue(app.navigationBars["Logic"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["AND"].waitForExistence(timeout: 3))

        app.staticTexts["AND"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["AND"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.buttons["gate-input-0"].waitForExistence(timeout: 3))
        // Unlike the input buttons, the output swatch isn't interactive
        // (nothing to tap), so SwiftUI surfaces it as `.otherElements`,
        // not `.buttons` — see its `.accessibilityElement(children:
        // .ignore)` in GateDemoView.
        let output = app.otherElements["gate-output"]
        XCTAssertTrue(output.waitForExistence(timeout: 3))
        XCTAssertEqual(output.label, "output, off", "AND(0, 0) should start off")

        app.buttons["gate-input-0"].tap()
        app.buttons["gate-input-1"].tap()
        // Poll briefly for the label to update rather than asserting
        // immediately after the taps.
        let predicate = NSPredicate(format: "label == 'output, on'")
        expectation(for: predicate, evaluatedWith: output, handler: nil)
        waitForExpectations(timeout: 3)
    }

    /// The "first build completed" milestone should fire exactly once,
    /// ever — not once per build. Complete Binary Counter (target 10 =
    /// 8 + 2, bit_width 8, so switches 4 and 6), see the celebration,
    /// dismiss it, then complete it a second time and confirm nothing
    /// re-fires.
    func testFirstBuildCompletionCelebratesExactlyOnce() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Build"].tap()
        XCTAssertTrue(app.staticTexts["Binary Counter"].waitForExistence(timeout: 5))
        app.staticTexts["Binary Counter"].tap()
        XCTAssertTrue(app.navigationBars["Binary Counter"].waitForExistence(timeout: 3))

        app.buttons["switch-4"].tap() // +8
        app.buttons["switch-6"].tap() // +2

        XCTAssertTrue(app.staticTexts["First Build Complete!"].waitForExistence(timeout: 3))
        attachScreenshot(app, named: "milestone-first-build")
        let dismiss = app.buttons["milestone-dismiss"]
        XCTAssertTrue(dismiss.waitForExistence(timeout: 3))
        dismiss.tap()
        XCTAssertFalse(app.staticTexts["First Build Complete!"].exists)

        // Leave and re-enter the same build, complete it again.
        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertTrue(app.staticTexts["Binary Counter"].waitForExistence(timeout: 3))
        app.staticTexts["Binary Counter"].tap()
        XCTAssertTrue(app.navigationBars["Binary Counter"].waitForExistence(timeout: 3))
        app.buttons["switch-4"].tap()
        app.buttons["switch-6"].tap()

        // Give the objective banner a moment to update, then confirm the
        // milestone sheet does not reappear for an already-celebrated id.
        XCTAssertTrue(app.staticTexts["Objective met!"].waitForExistence(timeout: 3))
        XCTAssertFalse(app.staticTexts["First Build Complete!"].waitForExistence(timeout: 2))
    }

    /// Bookmarking a concept in Explore should show up in Profile's
    /// Bookmarks section, and tapping it there should navigate straight
    /// A note typed into a concept's own editor should survive
    /// navigating away and back — proving the save-on-blur/onDisappear
    /// path actually persists, not just that the field accepts input.
    func testPersonalNoteSurvivesNavigatingAwayAndBack() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()
        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 5))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 3))
        app.staticTexts["Bits & Binary"].tap()
        XCTAssertTrue(app.navigationBars["Bits & Binary"].waitForExistence(timeout: 3))
        app.staticTexts["Binary"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3))

        let editor = app.textViews["concept-notes-editor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 3))
        editor.tap()
        editor.typeText("remember: MSB is index 0")

        // Back out (triggers the onDisappear save fallback), then
        // navigate straight back in.
        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertTrue(app.navigationBars["Bits & Binary"].waitForExistence(timeout: 3))
        app.staticTexts["Binary"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3))

        let editorAgain = app.textViews["concept-notes-editor"]
        XCTAssertTrue(editorAgain.waitForExistence(timeout: 3))
        XCTAssertEqual(editorAgain.value as? String, "remember: MSB is index 0")
    }

    /// back to that concept — proving the round trip, not just that the
    /// button toggles its own icon.
    func testBookmarkingAConceptSurfacesItInProfile() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()
        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 5))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 3))
        app.staticTexts["Bits & Binary"].tap()
        XCTAssertTrue(app.navigationBars["Bits & Binary"].waitForExistence(timeout: 3))
        app.staticTexts["Binary"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3))

        let bookmarkButton = app.buttons["bookmark-toggle"]
        XCTAssertTrue(bookmarkButton.waitForExistence(timeout: 3))
        XCTAssertEqual(bookmarkButton.label, "Add bookmark")
        bookmarkButton.tap()
        XCTAssertEqual(bookmarkButton.label, "Remove bookmark")

        app.tabBars.buttons["Profile"].tap()
        XCTAssertTrue(app.navigationBars["Profile"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Bookmarks"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.switches["streak-reminders-toggle"].waitForExistence(timeout: 3), "reminders toggle should be a real, working control")
        // The new Reminders section (streak notifications) pushed
        // Achievements further down a now-longer list — same lazy-List
        // materialization consideration as Build's Array Builder (§23).
        app.swipeUp()
        XCTAssertTrue(app.staticTexts["Achievements"].waitForExistence(timeout: 3), "locked achievements should list even with none unlocked yet")
        XCTAssertTrue(app.staticTexts["First Concept Mastered"].waitForExistence(timeout: 3))

        app.buttons["Binary"].tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3), "tapping the bookmark should land back on Binary's own page")

        // Unbookmarking round-trips too: back to Profile, it should be gone.
        XCTAssertTrue(app.buttons["bookmark-toggle"].waitForExistence(timeout: 3))
        app.buttons["bookmark-toggle"].tap()
        app.tabBars.buttons["Profile"].tap()
        XCTAssertFalse(app.staticTexts["Bookmarks"].waitForExistence(timeout: 2))
    }

    /// Networking (a new top-level domain) and Foundations.Arithmetic
    /// (a new subdomain) — both concepts reuse or extend existing
    /// puzzle mechanics, but the domains/concepts themselves are new
    /// content that should be reachable end to end with zero placeholder
    /// screens. Also covers the Subtraction/Multiplication/Division
    /// trio added alongside Binary Addition, including drilling into
    /// Binary Subtraction to prove its negative-result (two's
    /// complement) framing actually renders, not just parses.
    func testNetworkingAndArithmeticContentIsReachable() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()
        XCTAssertTrue(app.staticTexts["Networking"].waitForExistence(timeout: 5))

        app.staticTexts["Networking"].tap()
        XCTAssertTrue(app.navigationBars["Networking"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["IP Addresses"].waitForExistence(timeout: 3))
        app.staticTexts["IP Addresses"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["IP Addresses"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3))

        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 3))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Arithmetic"].waitForExistence(timeout: 3))

        app.staticTexts["Arithmetic"].tap()
        XCTAssertTrue(app.navigationBars["Arithmetic"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary Addition"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary Subtraction"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary Multiplication"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary Division"].waitForExistence(timeout: 3))

        app.staticTexts["Binary Addition"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary Addition"].waitForExistence(timeout: 3))
        XCTAssertTrue(
            app.buttons["Binary"].waitForExistence(timeout: 3),
            "prerequisite link should be real, not a placeholder"
        )

        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertTrue(app.staticTexts["Binary Subtraction"].waitForExistence(timeout: 3))
        app.staticTexts["Binary Subtraction"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary Subtraction"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3))
        XCTAssertTrue(
            app.buttons["Binary Addition"].waitForExistence(timeout: 3),
            "prerequisite link should be real, not a placeholder"
        )
        attachScreenshot(app, named: "explore-binary-subtraction-detail")
    }

    /// (proving `Domain.description` actually renders, not just parses),
    /// and both new Linear/Binary Search concepts.
    func testAlgorithmsDomainAndDomainDescriptionsAreReachable() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()

        XCTAssertTrue(app.staticTexts["Every concept in BIT, organized by topic — tap a folder to start exploring."].waitForExistence(timeout: 5), "root domain description should render")
        XCTAssertTrue(app.staticTexts["Algorithms"].waitForExistence(timeout: 3))

        app.staticTexts["Algorithms"].tap()
        XCTAssertTrue(app.navigationBars["Algorithms"].waitForExistence(timeout: 3))
        XCTAssertTrue(
            app.staticTexts["Step-by-step procedures for working with data — starting with the most fundamental question: how do you find something?"]
                .waitForExistence(timeout: 3),
            "Algorithms' own domain description should render at the top of its page"
        )
        XCTAssertTrue(app.staticTexts["Searching"].waitForExistence(timeout: 3))

        app.staticTexts["Searching"].tap()
        XCTAssertTrue(app.navigationBars["Searching"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Linear Search"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Binary Search"].waitForExistence(timeout: 3))

        app.staticTexts["Binary Search"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary Search"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3))
        XCTAssertTrue(
            app.buttons["Linear Search"].waitForExistence(timeout: 3),
            "Linear Search prerequisite link should be real, not a placeholder"
        )
    }

    /// Linked List (Data Structures.Linear) and Selection Sort
    /// (Algorithms.Sorting) — a concept-aware generator reuse and a
    /// genuinely new puzzle mechanic, respectively — both reachable
    /// end to end with real content, not placeholders.
    func testLinkedListAndSelectionSortAreReachable() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()

        XCTAssertTrue(app.staticTexts["Data Structures"].waitForExistence(timeout: 5))
        app.staticTexts["Data Structures"].tap()
        XCTAssertTrue(app.navigationBars["Data Structures"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Linear Structures"].waitForExistence(timeout: 3))

        app.staticTexts["Linear Structures"].tap()
        XCTAssertTrue(app.navigationBars["Linear Structures"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Linked Lists"].waitForExistence(timeout: 3))
        app.staticTexts["Linked Lists"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Linked Lists"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3))
        XCTAssertTrue(
            app.buttons["Arrays"].waitForExistence(timeout: 3),
            "prerequisite link should be real, not a placeholder"
        )
        attachScreenshot(app, named: "explore-linked-list-detail")

        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertTrue(app.staticTexts["Algorithms"].waitForExistence(timeout: 3))
        app.staticTexts["Algorithms"].tap()
        XCTAssertTrue(app.navigationBars["Algorithms"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Sorting"].waitForExistence(timeout: 3))

        app.staticTexts["Sorting"].tap()
        XCTAssertTrue(app.navigationBars["Sorting"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Selection Sort"].waitForExistence(timeout: 3))
        app.staticTexts["Selection Sort"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Selection Sort"].waitForExistence(timeout: 3))
        XCTAssertTrue(
            app.buttons["Bubble Sort"].waitForExistence(timeout: 3),
            "Bubble Sort prerequisite link should be real, not a placeholder"
        )
        attachScreenshot(app, named: "explore-selection-sort-detail")
    }

    /// FFI-backed Arrays demo you can actually set a value in, and the
    /// new Array Builder build listed in the Build tab.
    func testDataStructuresAndBitManipulationAreReachable() throws {
        let app = launchFreshApp()
        app.tabBars.buttons["Explore"].tap()

        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 5))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Bit Manipulation"].waitForExistence(timeout: 3))

        app.staticTexts["Bit Manipulation"].tap()
        XCTAssertTrue(app.navigationBars["Bit Manipulation"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Bitwise Masks"].waitForExistence(timeout: 3))

        app.staticTexts["Bitwise Masks"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Bitwise Masks"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3))

        // Back to Explore root (Bitwise Masks -> Bit Manipulation ->
        // Foundations -> root, three pushes deep), then into the new
        // top-level Data Structures domain.
        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        app.navigationBars.buttons.element(boundBy: 0).tap()
        XCTAssertTrue(app.staticTexts["Data Structures"].waitForExistence(timeout: 3))

        app.staticTexts["Data Structures"].tap()
        XCTAssertTrue(app.navigationBars["Data Structures"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Linear Structures"].waitForExistence(timeout: 3))

        app.staticTexts["Linear Structures"].tap()
        XCTAssertTrue(app.navigationBars["Linear Structures"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Arrays"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Stacks"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Queues"].waitForExistence(timeout: 3))

        app.staticTexts["Arrays"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Arrays"].waitForExistence(timeout: 3))

        // The array demo is live, not decorative: tap slot 0, type a
        // value, set it, and confirm the slot's own accessibility label
        // reflects the real FFI round trip.
        let slot0 = app.buttons["array-slot-0"]
        XCTAssertTrue(slot0.waitForExistence(timeout: 3), "Arrays' concept page should render its interactive_demo")
        slot0.tap()

        let valueInput = app.textFields["array-value-input"]
        XCTAssertTrue(valueInput.waitForExistence(timeout: 3))
        valueInput.tap()
        valueInput.typeText("42")
        app.buttons["array-set-button"].tap()

        let predicate = NSPredicate(format: "label == 'slot 0, value 42'")
        expectation(for: predicate, evaluatedWith: slot0, handler: nil)
        waitForExpectations(timeout: 3)

        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.lifetime = .keepAlways
        attachment.name = "explore-arrays-demo-set-value"
        add(attachment)

        // The new Array Builder build (riding the same ArraySim) is
        // listed in Build, not just reachable from Explore. It's a Core
        // build sitting a few sections down a now-longer, tier-grouped
        // list (intro banner + Beginner + Core), so it isn't guaranteed
        // to already be materialized in the accessibility tree without
        // scrolling — same lazy-List consideration as Profile's
        // Bookmarks section (§22).
        app.tabBars.buttons["Build"].tap()
        XCTAssertTrue(app.navigationBars["Build"].waitForExistence(timeout: 3))
        app.swipeUp()
        XCTAssertTrue(app.staticTexts["Array Builder"].waitForExistence(timeout: 3))
    }

    /// The single highest-priority fix from this session: progress used
    /// to reset on every relaunch because nothing ever called
    /// save/restore. Discover a concept, force-terminate the app (not
    /// just background it), relaunch, and confirm the discovery survived.
    func testProgressPersistsAcrossRelaunch() throws {
        // Both launches share one token, unlike every other test's fresh
        // random token per launch — that's what lets this test prove
        // state survives a relaunch while staying isolated from every
        // other test method.
        let token = UUID().uuidString
        let app = launchFreshApp(token: token)

        app.tabBars.buttons["Explore"].tap()
        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 5))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.navigationBars["Foundations"].waitForExistence(timeout: 3))
        app.staticTexts["Bits & Binary"].tap()
        XCTAssertTrue(app.navigationBars["Bits & Binary"].waitForExistence(timeout: 3))
        app.staticTexts["Binary"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary"].waitForExistence(timeout: 3)) // records discovery

        app.terminate()

        // A fresh process, same save file. Go straight to Play — without
        // ever revisiting Explore in this second launch, so if Binary
        // shows up in the pool, that can only be because discovery
        // actually persisted to disk, not because something in this
        // launch re-triggered it.
        let relaunched = launchFreshApp(token: token)
        relaunched.tabBars.buttons["Play"].tap()
        // A positive check (the puzzle's concept badge) rather than
        // waiting out a negative one (absence of the empty state) — the
        // two are mutually exclusive, and waitForExistence can only
        // confirm absence by exhausting its full timeout.
        XCTAssertTrue(
            relaunched.staticTexts["Binary"].waitForExistence(timeout: 5),
            "Binary's discovery should have survived the relaunch"
        )
    }

    /// Not a regression test — a dedicated capture pass for App Store
    /// listing screenshots. Deliberately avoids two things the other
    /// tests can't help: `checkAnswer()` producing a wrong-answer result
    /// (the correct answer is unpredictable, seeded randomly — see
    /// `testPlayEndlessGameMode`'s doc comment), and a screenshot taken
    /// immediately after a tap, which can catch a mid-flight touch-
    /// highlight artifact. A short settle delay before each capture here
    /// avoids that.
    func testAppStoreScreenshots() throws {
        let app = launchFreshApp()

        app.tabBars.buttons["Play"].tap()
        XCTAssertTrue(app.buttons["daily-challenge-card"].waitForExistence(timeout: 5))
        app.buttons["daily-challenge-card"].tap()
        XCTAssertTrue(app.textFields["daily-answer-field"].waitForExistence(timeout: 3))
        Thread.sleep(forTimeInterval: 2.0)
        attachScreenshot(app, named: "store-daily-puzzle")
        app.buttons["Close"].tap()

        app.tabBars.buttons["Explore"].tap()
        XCTAssertTrue(app.staticTexts["Foundations"].waitForExistence(timeout: 3))
        app.staticTexts["Foundations"].tap()
        XCTAssertTrue(app.staticTexts["Arithmetic"].waitForExistence(timeout: 3))
        app.staticTexts["Arithmetic"].tap()
        XCTAssertTrue(app.staticTexts["Binary Subtraction"].waitForExistence(timeout: 3))
        app.staticTexts["Binary Subtraction"].firstMatch.tap()
        XCTAssertTrue(app.navigationBars["Binary Subtraction"].waitForExistence(timeout: 3))
        XCTAssertTrue(app.staticTexts["Real world"].waitForExistence(timeout: 3))
        Thread.sleep(forTimeInterval: 2.0)
        attachScreenshot(app, named: "store-explore-concept")
    }
}
