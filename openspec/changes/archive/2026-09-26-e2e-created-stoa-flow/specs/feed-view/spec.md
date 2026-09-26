## ADDED Requirements

### Requirement: A feed that holds nothing and a feed that could not be read are different screens

The feed MUST distinguish three outcomes of reading a Stoa's feed, and MUST NOT
render any two of them alike: the read succeeded and returned rows, which are
rendered; the read succeeded and returned none; the read failed.

`feed-read`'s requirement "A feed read that fails is the error shape, and never
an empty feed" puts the two answers on the wire as different replies. This
requirement constrains how the screen renders them, and nothing about the read.

A read that succeeds with no items MUST put the screen in its read-succeeded
state, and the screen MUST render no failure.

A read that fails MUST put the screen in its failed state. The screen MUST render
no rows in that state, including rows an earlier successful read of the same
Stoa returned. It MUST render the reason the core gave, unreworded. A failure
reached through the view's one call path without a core reason, such as an
unreachable core, MUST be rendered with the reason that path reports.

A reply that is neither a page of rows nor the error shape — a success carrying
no array of items, or an `items` field that is not an array — MUST be treated as
a failed read, with a reason of the view's own naming what was wrong with the
reply, and MUST NOT be rendered as an empty feed.

#### Scenario: A read with no items is the success state, not a failure

- **WHEN** the feed's read succeeds and carries no items
- **THEN** the screen is in its read-succeeded state
- **AND** it renders no failure
- **AND** it renders no rows

#### Scenario: A read that fails is a failure, not an empty feed

- **WHEN** the feed's read answers with the error shape
- **THEN** the screen is in its failed state
- **AND** it renders no rows
- **AND** the core's message is present in what is rendered

#### Scenario: The empty state and the failed state are not the same state

- **WHEN** one feed is driven with a successful read carrying no items, and
  another with the error shape
- **THEN** the two screens are in different states
- **AND** the text they render differs

#### Scenario: A failed read does not leave an earlier read's rows on screen

- **WHEN** the feed's read succeeds with rows, and a later read of the same Stoa
  answers with the error shape
- **THEN** the screen is in its failed state
- **AND** none of the earlier read's rows is rendered

#### Scenario: A success with no items array is a failure rather than an empty feed

- **WHEN** the feed's read answers with an object carrying no `items` array, and
  again with an `items` field that is not an array
- **THEN** the screen is in its failed state in both cases
- **AND** it renders a failure of its own naming what was wrong with the reply
