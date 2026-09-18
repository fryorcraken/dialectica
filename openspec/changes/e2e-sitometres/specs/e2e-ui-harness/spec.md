## Purpose

Fixes what an end-to-end run must establish before its result may be believed, and what counts as a pass. Every other UI gate in this repo runs with the host absent, so a green from them says nothing about the assembled application; this capability contracts the obligations that make a green from the assembled application mean something — that the thing driven was the thing intended, that the run executed the whole specification rather than stopping early, and that a run which mutates state cannot damage anything a person owns.

The boundary with the view capabilities is the load-bearing part of this contract. `stoa-navigation-view`, `view-identity-onboarding`, `composer-view` and `feed-view` own **what the screens must render**; they are contracts on the application and this capability re-specifies none of them. What this capability owns is the contract on the **gate**: an obligation each of those creates and none of them can discharge, because a requirement about a screen cannot also say whether the harness that observed the screen was pointed at the right binary.

## ADDED Requirements

### Requirement: A run is adjudicated from the machine-readable report, not from the exit code alone

The harness MUST write a machine-readable report for every run, and the gate MUST decide pass or fail by reading that report. The harness's exit code MUST NOT be the only thing the gate consults.

The report is the authority because the terminal summary is not one: it is styled for a human reader and carries no stable field a gate can match on, whereas the report is written on every exit path.

#### Scenario: A report is produced and adjudicated

- **WHEN** a run completes, by any exit path
- **THEN** a machine-readable report exists for that run
- **AND** the gate's verdict for the run is derived from the contents of that report

#### Scenario: A run killed before it could report is a failure, named as one

- **WHEN** a run is terminated before it writes its report, such as by a job timeout
- **THEN** the gate MUST fail
- **AND** the gate MUST report that no report was written and therefore nothing was proved, rather than failing with an error about a missing file

### Requirement: A pass requires the verdict, every step, and the step count to agree

The gate MUST treat a run as passing only when all three of the following hold:

1. the report's overall verdict is a pass;
2. every step present in the report is a pass;
3. the number of steps in the report equals the number of steps in the specification that was run.

Failing any one of the three MUST fail the gate. The third condition is the one that carries the weight: without it, a run that started the application, executed nothing further, and reported an empty sheet satisfies both of the first two conditions and reads as a pass.

#### Scenario: All three conditions hold

- **WHEN** a report's verdict is a pass, every step in it is a pass, and its step count equals the specification's step count
- **THEN** the gate passes

#### Scenario: The run stopped early with nothing failing

- **WHEN** a report's verdict is a pass and no step in it failed, but the report carries fewer steps than the specification declares
- **THEN** the gate MUST fail
- **AND** the gate MUST report that the run did not execute the whole specification

#### Scenario: A step failed inside an otherwise passing report

- **WHEN** a report's overall verdict is a pass but at least one step in it is not a pass
- **THEN** the gate MUST fail
- **AND** the gate MUST name the steps that did not pass

#### Scenario: Every problem is reported, not just the first

- **WHEN** a report fails more than one of the three conditions
- **THEN** the gate MUST report each condition that failed before exiting

### Requirement: An inconclusive run is a failure, never a pass

A run that proved nothing MUST NOT be reported as a run that proved something. The gate MUST be configured so that an inconclusive outcome fails it.

This is stated as a requirement rather than left to a default because the default is the opposite: the harness exits with a success code on an inconclusive outcome by design, so a gate that reads only the exit code reports a run that established nothing as green.

#### Scenario: An inconclusive outcome fails the gate

- **WHEN** a run's outcome is inconclusive
- **THEN** the gate MUST fail

#### Scenario: The strict behaviour is requested explicitly

- **WHEN** the harness is invoked by the gate
- **THEN** it MUST be invoked in the mode that treats an inconclusive outcome as a failure, rather than relying on the default

### Requirement: The driven application is proved to carry the inspector before a run is believed

The harness drives the application through a QML inspector that is compiled into the host binary rather than enabled at runtime. The gate MUST verify that the host binary it is about to drive actually contains that inspector, and MUST fail with a message naming that cause when it does not.

The verification MUST be made against the binary the gate will drive. A host build that silently ships without the inspector MUST NOT be reported as a passing run and MUST NOT be reported only as a downstream timeout.

#### Scenario: The inspector is present

- **WHEN** the gate checks the host binary it is about to drive and the inspector is present
- **THEN** the gate proceeds to the run

#### Scenario: The inspector is absent

- **WHEN** the gate checks the host binary it is about to drive and the inspector is absent
- **THEN** the gate MUST fail before attempting the run
- **AND** the failure MUST state that the binary carries no QML inspector and therefore cannot be driven

#### Scenario: The check discriminates rather than matching everything

- **WHEN** the same check is applied to a host build known to be built without the inspector
- **THEN** the check MUST report the inspector absent

### Requirement: The module build and the host build are a matched pair

A host build accepts modules built for the variant that matches it, and rejects modules built for another. The gate MUST build the modules it installs and the host it drives as a matched pair.

A mismatch MUST NOT be allowed to present only as a timeout on the run's first step. The failure mode this requirement exists to prevent is that the host declines to load the module with a single warning, the interface then renders nothing, and the run fails with no indication of the cause in the harness's own output.

#### Scenario: A matched pair runs

- **WHEN** the modules are built for the variant the host build accepts
- **THEN** the host loads the modules and the run proceeds

#### Scenario: A mismatch is reported as a mismatch

- **WHEN** the modules are built for a variant the host build does not accept
- **THEN** the gate MUST fail
- **AND** the diagnosis MUST be available from the run's retained output rather than requiring the failure to be reproduced by hand

### Requirement: The pinned host revision is derived from the single place it is written

The repository already records the host revision the project builds against, in its scaffold configuration. The gate MUST derive the revision it builds from that record, and MUST NOT carry a second copy of the value.

A second copy fails silently rather than loudly: the build succeeds against whichever revision was written, and the only symptom of the two disagreeing is that cached artefacts key on a revision nothing else uses, so every run is slow and nothing reports an error.

#### Scenario: The revision comes from the configuration

- **WHEN** the gate builds the host
- **THEN** the revision it builds is the one recorded in the scaffold configuration

#### Scenario: Changing the recorded pin changes what the gate builds

- **WHEN** the recorded host revision in the scaffold configuration is changed
- **THEN** the gate builds the newly recorded revision, with no other file requiring an edit

### Requirement: A specification that mutates state runs against a disposable profile

A run that only reads MAY run against any profile. A specification that causes the application to write state MUST run against a profile created for that run and discarded after it, and MUST NOT run against a profile belonging to a person.

The reason is that the writes are not reversible by the harness: a specification that publishes or joins on every run accumulates state with no counterpart that removes it, so pointing it at a real profile degrades that profile a little on every run.

#### Scenario: A mutating specification is isolated

- **WHEN** a specification that causes a write is run
- **THEN** the profile it runs against MUST have been created for that run

#### Scenario: A run leaves no state behind outside its own profile

- **WHEN** a run that mutates state completes, whether it passed or failed
- **THEN** no state outside the profile created for that run has been modified

### Requirement: Specifications are validated against the harness schema without building the application

A malformed specification MUST be detectable without building a host or a module. The project MUST provide a validation gate that parses every specification against the harness's schema, reports the step count it parsed for each, and fails when any specification does not parse.

This is separated from the run because the two have different costs and different reasons to fail. A schema mistake found in seconds on every change is worth having; the same mistake found after a host build is the same information arriving much later.

#### Scenario: Every specification parses

- **WHEN** the validation gate runs over the specification directory and every file parses against the schema
- **THEN** the gate passes
- **AND** it reports, per file, the number of steps it parsed

#### Scenario: A malformed specification fails validation

- **WHEN** the specification directory contains a file that does not parse against the schema
- **THEN** the validation gate MUST fail and MUST name the file that did not parse

#### Scenario: Validation needs no host and no module

- **WHEN** the validation gate runs
- **THEN** it completes without building the host application or any module

### Requirement: Evidence from a failed run is retained

When a run fails, the gate MUST retain the material needed to diagnose it — at minimum the run's report and the host application's own log — and MUST retain it in a form reachable without reproducing the run.

A failed end-to-end run is the case where reproduction is most expensive, so the evidence has to survive the run that produced it.

#### Scenario: A failed run retains its evidence

- **WHEN** a run fails
- **THEN** the run's report and the host application's log are retained for inspection

#### Scenario: A passing run need not retain it

- **WHEN** a run passes
- **THEN** the gate MAY discard that material

### Requirement: Only screens the assembled application can reach are covered by a run

A specification MUST drive the application only through the screens the assembled application actually mounts, and MUST NOT assert against a component that the running application never presents.

This is the property that distinguishes an end-to-end run from the component suite, and it is what the run is for. The component suite instantiates a component directly, so it reports on a component that the assembled application may never mount; a specification asserting against such a component would inherit exactly that blindness while appearing to have escaped it.

#### Scenario: A covered screen is reached by navigating

- **WHEN** a specification asserts against a screen
- **THEN** it MUST have reached that screen by driving the application from its starting state
- **AND** it MUST NOT have instantiated the screen's component directly

#### Scenario: An unmounted component is out of scope rather than asserted against

- **WHEN** a component is declared by the view but is not mounted by the assembled application on any path
- **THEN** no specification in the suite asserts against it

### Requirement: Every element a specification drives carries a stable name

An element a specification clicks or types into MUST carry a stable name of its own, and the specification MUST select it by that name rather than by its displayed text, its position among siblings, or its type alone.

The name MUST sit on the element that receives the interaction, not on an ancestor of it. An identifier that is private to the component defining it does not satisfy this requirement, because nothing outside that component can select by it.

Selecting by displayed text is forbidden because it makes every rewording of the interface a test failure, and because two controls on different screens may carry identical text while both are present in the object tree — the view mounts all its screens at once and hides the inactive ones, so a control on a hidden screen is still in the tree to be matched.

The consequence to accept rather than work around: where a control a covered flow needs has no such name, the view MUST gain one. A specification written against a brittle selector instead is a specification that will fail for the wrong reason.

#### Scenario: A driven element is selected by its own name

- **WHEN** a specification clicks or types into an element
- **THEN** it selects that element by a stable name carried on the element itself

#### Scenario: A renaming of user-facing text breaks no specification

- **WHEN** the displayed text of a control a specification drives is changed, and nothing else is
- **THEN** every specification that drives that control still selects it

#### Scenario: A control needed by a covered flow is named before it is driven

- **WHEN** a flow covered by the suite requires interacting with a control that carries no stable name
- **THEN** a stable name is added to that control
- **AND** the name is placed on the element that receives the interaction

### Requirement: Failure text supplied by the core is asserted by presence, not by wording

Where the view renders a failure message it received from the core, a specification MUST assert that the failure is presented — that the panel carrying it is shown and its text is non-empty — and MUST NOT assert the wording of that message.

The wording of a core-supplied message is not part of any contract the view holds. Pinning it would make a test fail on a reworded diagnostic while passing on a view that showed the wrong message entirely, which is the weaker of the two checks in both directions.

#### Scenario: A failure is asserted by presence

- **WHEN** a specification covers a path on which the core returns a failure
- **THEN** it asserts that the failure is presented to the user and that the presented text is not empty

#### Scenario: Core wording is not pinned

- **WHEN** the core changes the wording of a failure message without changing whether it fails
- **THEN** no specification in the suite fails

### Requirement: What the harness cannot observe is declared out of scope rather than approximated

Two properties of the running application are outside what this harness can establish, and the suite MUST NOT contain specifications that claim to cover them:

1. **Anything requiring a real display surface.** The harness runs the application against an offscreen platform, so window geometry cannot be set by a specification and the system clipboard does not function. Behaviour that depends on either MUST be covered at the component layer, where a fixture owns the geometry, or left uncovered and said to be uncovered.
2. **Whether a given core call was made.** The view reaches the core over a transport whose calls the harness does not observe, so a specification MUST assert the effect a call produced rather than that the call happened.

Declaring these is the requirement. A specification that appears to cover one of them while actually asserting something weaker is worse than an absent specification, because it closes the question.

#### Scenario: A display-dependent property is not claimed

- **WHEN** a behaviour depends on the window's size or on the system clipboard
- **THEN** no specification in the suite asserts it
- **AND** the gap is recorded where someone choosing coverage will read it

#### Scenario: An effect is asserted instead of a call

- **WHEN** a specification covers a flow whose point is that the view reaches the core
- **THEN** it asserts the observable effect of that call on the interface, not that the call occurred

### Requirement: A specification that is not run by the gate is not part of the suite

Every specification file present in the suite directory MUST be executed by the gate. The set of specifications the gate runs MUST be derived from the directory rather than maintained by hand alongside it, or the gate MUST fail when the two differ.

A hand-maintained list beside a directory goes stale silently: a specification added to the tree and not to the list runs nowhere, passes nothing, and is indistinguishable from a specification that passes.

#### Scenario: A newly added specification runs

- **WHEN** a specification file is added to the suite directory and nothing else is changed
- **THEN** the gate executes it

#### Scenario: A specification present but unexecuted fails the gate

- **WHEN** the suite directory contains a specification that the gate would not execute
- **THEN** the gate MUST fail and MUST name that specification

### Requirement: One specification's outcome does not conceal another's

The gate MUST run every specification and report the outcome of each, rather than stopping at the first failure.

When two specifications break, both are worth seeing: one failing for an environmental reason would otherwise hide a genuine failure in another, and the run would have to be repeated to learn what it already knew.

#### Scenario: Every specification reports its own outcome

- **WHEN** more than one specification in the suite fails in a single run
- **THEN** the gate reports the outcome of each specification that ran
- **AND** the gate fails
