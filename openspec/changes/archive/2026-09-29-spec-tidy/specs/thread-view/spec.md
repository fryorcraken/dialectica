## REMOVED Requirements

### Requirement: The thread screen is reached from the feed and can be left again

**Reason**: `view-navigation` is the single home for how each screen is reached
and left, by owner decision. This requirement duplicated `view-navigation`'s "A
thread is opened from a feed row and can be left" and restated its general
rule, so two capabilities held one transition.

**Migration**: Every obligation it carried now lives in `view-navigation`'s "A
thread is opened from a feed row and can be left", as amended by this change.
The return surviving every state including a refused read, the prohibition on
the screen carrying its own thread identifier, Stoa address or genesis record,
and the absence of a thread screen and a thread read before a thread is chosen
are added there, each with the scenario that stated it here. That the thread
read names the acted-on row's thread identifier and Stoa is added to its
existing scenario. The return to the feed without a restart, and that no
record is invented, were already there.
