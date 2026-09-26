## ADDED Requirements

### Requirement: The view opens on the Stoa list

When the view is started, the main-area screen it renders MUST be the Stoa list,
and it MUST NOT render another main-area screen in the list's place before the
user has acted. Which screen the view opens on MUST NOT depend on what the core
answers to the calls the list makes on being shown. That includes an answer in
the error shape, and the absence of a bridge to the core.

This requirement constrains which screen is rendered and nothing the list renders
on it. The list's listing, key and failure states are `stoa-navigation-view`'s.

#### Scenario: The view opens on the Stoa list

- **WHEN** the view is started and the user has taken no action
- **THEN** the Stoa list is rendered
- **AND** no other main-area screen is rendered

#### Scenario: What the core answers does not change the opening screen

- **WHEN** the view is started while the listing call answers with no items and
  the master-key query answers that no key is held, again while the listing call
  answers with the error shape, again while the master-key query answers with
  the error shape, and again with no bridge to the core
- **THEN** in each case the Stoa list is rendered
- **AND** no other main-area screen is rendered

### Requirement: A paste refused as not a Stoa reference leaves the list rendered

When the user acts on the paste field with input that `stoa-navigation-view`
refuses as not a Stoa reference, the view MUST keep rendering the Stoa list. It
MUST NOT render the preview, or any other main-area screen, in the list's place.

This requirement constrains only the transition, which is not taken. The refusal
itself, and the rule that no call is made for the input, belong to
`stoa-navigation-view`'s scenario "Input that is not a Stoa reference is refused
before any call". Which input counts as a Stoa reference is fixed by its
requirement "The reference encoding is a compatibility surface and is fixed
here".

#### Scenario: Text that is not a Stoa reference does not leave the list

- **WHEN** the Stoa list is rendered, the paste field is given text that does not
  carry both an address and a genesis record, and the user acts on the paste
  action
- **THEN** the Stoa list is still rendered
- **AND** the preview is not rendered

#### Scenario: A reference missing its record half does not leave the list

- **WHEN** the Stoa list is rendered, the paste field is given a JSON object that
  carries `stoa` as a string and no `genesis`, and the user acts on the paste
  action
- **THEN** the Stoa list is still rendered
- **AND** the preview is not rendered
