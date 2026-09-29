## REMOVED Requirements

### Requirement: The screen is reachable, and leaving it returns where the user was

**Reason**: `view-navigation` is the single home for how each screen is reached
and left, by owner decision, and its Purpose says it owns transitions and
nothing a screen renders.

**Migration**: Moved to `view-navigation` as "The moderation screen is
reachable, and leaving it returns where the user was", with its text and its
three scenarios unchanged except that "the screen" names the moderation screen
and the reference to `view-navigation` reads "this capability". One sentence is
added there, leaving what the screen renders to this capability.
