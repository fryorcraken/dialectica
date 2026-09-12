# Dialectica — functional brief for UI design

A decentralized forum. This brief covers **what the interface must do and must
never do**. Look, feel and theme are yours to explore; everything here is a
functional constraint, and most of them come from the fact that this is
peer-to-peer software with no server.

The logo is the Greek delta, **Δ**. The name is from *dialectic* — reasoned
argument between positions.

> **Provenance.** This brief is derived from `docs/PLAN.md` and is kept in step
> with it as changes land — it is a live document, not a snapshot. If something
> here disagrees with PLAN.md, PLAN.md wins and this file has a bug. Last
> reconciled against **PLAN.md §7.2-§7.3** (votes and vouching), **§9.1**
> (the Phase 3 API) and **§5.2.1** (what an identity is called).
>
> **The generated name's vocabulary is settled as of §5.2.1**: four words — two
> adjectives and two nouns — drawn from Greek philosophy and letters, derived
> from the key and never typed. An earlier version of this block called the
> wordlist provisional and the shape "adjectives plus a noun"; both are
> superseded. The *word counts* remain curation work, but the shape, the sizes
> and the source are decided. `git log docs/PLAN.md` answers what has landed;
> this block does not try to.

---

## The one idea everything follows from

The organising concept is a **Stoa**: a sub-forum anyone can create and
moderate. Two properties held in deliberate tension:

- **Permissionless creation.** Making a Stoa needs no approval and no
  registration, because there is no central registry to register with. A Stoa
  *is* a record its creator publishes.
- **Real moderation within a Stoa.** A Stoa's moderators shape it. A forum
  where nothing can be removed is not a forum, it is a firehose.

When a screen seems to need a decision, check it against that pair first.

---

## Five constraints that make this unlike a normal forum app

These are not preferences. They are consequences of the architecture, and a
design that ignores them will be wrong in ways that look fine in a mockup.

### 1. There is no server, so there is no "loading" in the usual sense

Every peer holds its own copy of what it has seen. **Two people can legitimately
see different things**, permanently, and neither is stale or broken. Someone was
offline; someone joined late; a message has not propagated yet.

**Design implication:** an empty feed is ambiguous and must not be presented as
an error or as "nothing here". "You have not received anything for this Stoa
yet" is true; "this Stoa is empty" is a claim you cannot make. Similarly, a
count of *anything* global — members, total posts — is unknowable. Do not show
one.

### 2. Identity is per-Stoa, permanent, and pseudonymous

A person has a **different, unlinkable identity in every Stoa**. That is a
privacy property, by construction — the same human in two Stoas cannot be
correlated. There is no global profile, no avatar service, no display name
registry. No rotation: an identity is permanent within its Stoa.

**Design implication:** no cross-Stoa profile page, no "also active in" links,
no unified inbox that would correlate identities.

**Identities have generated names, and this is new.** An identity renders as
**two adjectives and two nouns drawn from Greek philosophy and letters** —
something like *measured attic thales praxis* or *sober ionic stoic kairos* —
computed from the key itself. Nobody types a name; there is no registry to hold
one and a typed name carried between Stoas would undo the unlinkability above
with a text field.

**The register is deliberate and it is the point: sober, plain, adult.** The
adjectives are geographic and temperamental (`attic`, `ionic`, `doric`,
`measured`, `sober`, `patient`, `laconic`); the nouns pool the vocabulary of
Greek thought (`logos`, `praxis`, `techne`, `aporia`, `kairos`) with thinkers
and writers (`thales`, `hypatia`, `solon`, `sappho`). **If a name reads like a
fantasy handle, something has gone wrong** — an earlier draft drew on science
fiction and produced *vermilion patient sandworm*, which is why this note
exists.

**It is four words, and they all matter.** This is longer than a typical
username and the length is not decorative: it is what makes accidental
collisions rare (see below). **Do not truncate or elide it** — dropping the tail
removes one of the two nouns, which is most of what distinguishes one name from
another.

**An identicon is intended alongside the name, and designing it is part of your
work.** A small visual glyph derived from the same key, giving recognition a
second channel. What is fixed is where it comes from — the key — so it is stable
forever, identical on every peer, and not something anyone can choose or
register. What it looks like is open.

**The mark and the name are computed by two separate hashes of the same key**,
which has one consequence worth designing around: **they vary independently.**
Two identities with similar names have unrelated marks, and two with similar
marks have unrelated names. So the glyph is genuinely a second opinion rather
than a restatement of the first — **design it to be compared at a glance**, since
that independence is the whole of what it buys. **What it cannot do is in
obligation 6, and that half is not negotiable.**

**At onboarding the user picks from a slate of five, and can refresh the slate
as often as they like.** So the name is chosen and carries intent — someone who
refreshed forty times meant the one they kept.

**The thing to get right, because it will otherwise generate support
questions:** what the user is choosing is **the key**. The name is the key's
shadow, and identity is permanent — so the name **can never be changed**. Copy
that says *"pick your username"* promises a settings screen that cannot exist.
*"Pick your identity"* is true.

Two consequences for you:

- **An onboarding screen exists that did not before**: five identities, pick
  one, refresh for more. It is the first thing a new user sees in a Stoa.
- **A name is not unique and not an identifier** — see obligation 6 below.
  Accidental collisions are now rare: in a Stoa of a thousand the chance two
  people share a name is **about 0.003%**, and at five thousand **about 0.07%**.
  (The superseded science-fiction scheme — three words, 2²⁴ — gave 3% at a
  thousand and better than even at five thousand. The fourth word is what bought
  the change.) **Rare is not never**, and none of it touches deliberate
  impersonation — which is the whole of obligation 6.

### 3. Posts are never edited in place

An edit is a **new version** of a post, published and signed by the same author.
History is kept. The UI can and should show that a post was edited — and a
moderator acting on a post is acting on a version they can name.

**Design implication:** "edited" is a first-class state, not a subtle asterisk.
Consider whether previous versions should be reachable.

### 4. Moderation changes what conforming peers *render*, nothing more

A moderator publishes a signed "hide"; every peer verifies it independently and
reaches the same answer. **But nobody can be prevented from publishing**, and
there is no membership to revoke. Moderation is real and it binds — its ceiling
is honest and worth designing to rather than around.

**Design implications:**
- The default feed **omits** hidden posts rather than showing them greyed out.
- A "show hidden" view is explicitly wanted.
- **A hide is currently irreversible** (see the warning below), and that must be
  said at the moment of action.

### 5. Attacker-supplied content is everywhere

Every byte from another peer is attacker-controlled: post bodies, titles,
display names, Stoa addresses embedded in posts. The core validates structure
but **deliberately does not sanitise display text** — normalising it would break
agreement between peers about what a post *is*.

**So sanitising is the interface's job**, and it is listed as an obligation
below.

---

## The screens

Sketched functionally. Composition and hierarchy are yours.

### Stoa list — where a person starts

Stoas the user has joined. **There is no directory to browse**; you join by
pasting an address someone gave you, or by following a link in a post.

Each Stoa shows the title it currently goes by. Note that a Stoa's *founding*
title is fixed forever, while its *current* title can be changed by moderators —
so two Stoas can share a display name and still be entirely different Stoas.
**A title is not an identifier.** The address is.

**Every title you can currently render is the founding one.** Core reports a flag
saying so on each Stoa, because the two are different epistemic states: a founding
title may be years out of date, and nothing yet resolves the moderator-signed
message that would carry the current one. A list that presents a founding title as
current is making a claim core did not. Whether you surface that distinction and how
is yours; silently presenting one as the other is not.

Core also reports whether the user **created** a Stoa or **joined** it. Worth
showing — "did I make this" is a question people ask of their own list, and it is
also the only hint available that they may be its moderator.

### Joining a Stoa — a security surface, not a form

An address is a copyable string that is **self-authenticating**: pasting it is
enough to verify what you joined, because the address is a hash of the Stoa's
founding record.

**Requirements:**
- Show what is being joined **before** joining it.
- An address appearing inside a post is attacker-supplied. Render it as an
  affordance the reader chooses to act on. **Never auto-join.**
- Two Stoas may present the same name. Show something that distinguishes them.

### Feed — a Stoa's posts

Paginated. Hidden posts are omitted by default.

**Orderings, and an honesty problem worth designing around.**

The intended orderings are **new** (most recent first), **active** (threads by
their most recent reply), and **top** (vote-ordered, weighted).

**`top` does not ship in the first UI.** Phase 3 refuses it outright rather
than shipping it provisionally: with no sybil resistance there is no score
worth ordering by. **Design the ordering control so it can carry fewer options
than three**, and so an option can appear later without the layout changing.

**"Most recent" is not available *yet*, and the reason is worth knowing because
it is about to change.** Both `new` and `active` were defined against a
timestamp the transport layer does not deliver, so today both fall back to an
order derived from op ids — *convergent*, in that every peer computes the same
sequence, but carrying **no recency information at all.**

**That is a gap dialectica can close by itself**, by putting an author-asserted
timestamp inside the signed post, and the plan now says so. So treat a genuine
"new" as **coming, not impossible** — design the ordering control as though a
real recency option will arrive, rather than around its permanent absence. What
does not change is the rule below: do not label an ordering "new" **until it
is one**.

**Do not generalise that convergence to the feed as a whole.** It is a property
of *this fallback*, not of Dialectica. Once vote-weighting exists, two readers
will legitimately see different orders, because **vouching is private to each
reader**: Alice vouches for Bob, Carole does not, and they compute different
weights over the identical set of posts. Neither is stale and neither is wrong.
So a label like "the same order for everyone" is true only of the fallback and
must not become the interface's general promise — and nothing in the design
should read as though divergence between two peers' feeds is a fault to be
repaired.

So a feed labelled "new" would currently be ordered by hash. It is temporary
and it resolves when an upstream gap closes, but until then:

- **Do not label an ordering "new", "latest" or "recent"** unless it is one.
- A neutral label is honest and available now. Consider what the control should
  say when the thing it names is not yet true.
- Whatever you design, **the labels must be able to change** when the real
  ordering arrives, without the layout changing around them.

This is the clearest instance of the project's tone problem in miniature: the
convergent order is genuinely useful and genuinely not chronological, and the
interface has to say which without being tedious about it.

### Thread — a post and its replies

A reply is just a post that names a parent, so threads nest naturally. Each reply
carries its parent, so the tree is reconstructable, and the root post comes first.

Each post shows: author identity, body, attachments, whether it was edited, the
up/down control, and a report action.

**The replies are not in chronological order, and there is no way to put them in
one.** This is the feed's ordering problem again, in a place where the assumption
is much easier to make: a thread *looks* like a conversation, so a reader will read
top-to-bottom as "in the order they were said". It is not. Replies come back in the
same convergent-but-recency-free order the feed uses — every peer agrees on it, and
it carries no time information at all.

So **do not label reply order, and do not render anything time-like beside a reply**
— no "3rd reply", no ordinal, no implied sequence. It resolves when the feed's does,
via the same author-asserted timestamp, and a thread view designed around implied
chronology will need rebuilding rather than relabelling.

### Composition

**Gate every posting affordance on whether the user can actually post.** The
core exposes a capability probe that answers `{"canPost": true, "identity": …}`
or `{"canPost": false, "reason": …}`.

Reasons are human-readable and **name a fix** — e.g. "no keystore found; create
one before posting", "keystore permissions are too open (mode 0644); restrict it
to owner-only and replace the key".

**Never gate on a build flag, and never show a compose box that cannot be
submitted** — it loses whatever the user typed. Surface the reason instead.

### Moderation

Moderators see a hide control on any post. **See the irreversibility warning.**

---

## Non-negotiable rendering obligations

These exist because the core's honest answer is *incomplete* without something
the interface does. Each was discovered while building the core, and the gap is
invisible from inside it.

**1. Sanitise display text, because the core deliberately does not — and this
applies to every string, not just titles.**
Bidirectional-override characters (U+202E and friends), zero-width characters,
and homoglyphs — a Cyrillic "А" rendering identically to Latin "A" — all pass
through untouched. The core preserves display text **exactly** and never
normalises, by design: normalising would change what a post *is*, and peers
would stop agreeing about it.

The obvious case is a Stoa title impersonating another. **The larger case is
post bodies**, which are the longest and least constrained strings you will
render. Strip or visibly mark. **Never treat displayed text as an identifier.**

**2. Warn that a hide cannot currently be undone, and do not present the pair
as symmetric.**
For reasons deep in the transport layer, an "unhide" cannot currently reverse a
"hide". A moderator pressing hide is taking an action the core will not tell
them is irreversible. **Say so at the point of action.**

And the subtler half: **an unhide control placed as the mirror of hide asserts
a symmetry that does not exist.** Two buttons side by side, or one toggle, both
claim the operation reverses. It does not, yet. Whatever you design, the
asymmetry should be visible rather than implied away.

Temporary — it resolves when an upstream gap closes, and the warning and the
asymmetry go together when it does.

**2b. A join confirmation showing only a title has shown the forgeable half.**
A Stoa's displayed title comes from a moderator-signed message. It is **not
unique, not verified against anything, and freely chosen** — two unrelated
Stoas can present the same name, and one can be named to impersonate another.
**The address is the identity; the title is decoration.** A confirmation screen
that shows "Join *Agora*?" and nothing else has shown the reader precisely the
part an attacker controls.

**3. Never show anyone's vouching but the viewer's own.**
Vouching — privately deciding whose votes weigh more in your own feed — is
**never published**. Showing counts of it would reconstruct by eye exactly what
the design refuses to publish. So: **no "N people vouch for this author" badge,
no vouch counts, nothing aggregate about other readers' choices.** Vouching
makes nobody more visible to anyone else; it changes one reader's feed.

A related distinction, if vouch weight can also be *earned* rather than only
declared: **the two must be presented differently.** One is something the reader
chose and can revoke in a click; the other is something that happened to them,
which they should be told about and be able to undo.

**4. Prefer a distribution to a net number.**
Note this one has **changed status**: it was written for a two-axis design that
was withdrawn, so it is now a principle rather than a hard rule, and you should
treat it as guidance about tone. A bare "+3" hides which votes produced it, and
a net is what makes disagreement feel like damage. This is a forum named for
dialectic and the interface should not punish disagreement arithmetically.

What remains hard: **a score never goes below zero**, so nothing should render
as a negative number, and downvoting must never look like it removes a post.
Removal is moderation, which is a different and binding thing.

**5. Distinguish an empty result from a failed one.**
A storage failure must never render as an empty feed. An empty feed and "we
could not read the store" look identical and mean opposite things.

**6. A generated name is never unique and never an identifier — the address is.**
This is obligation 2b again, now applying to the thing **every post is
attributed to**, which is a far larger surface than Stoa titles: a feed renders
an attribution on every row.

*(One thing to know about where the name comes from, and an earlier version of
this paragraph had it wrong. The core returns the author as an **address**, not
a name — PLAN §9.1 lists the feed's author field as "the author, as the per-Stoa
address (§5.2) — never a name, because there are no names". But the name is
**not** derived from that address: §5.2.1 derives it from the **public key**, on
purpose, so that a name tracks the key that signs. So a view holding only an
address **cannot** compute the name itself, and core must return the rendered
name alongside the address. Treat both as things you are given.)*

**Uniqueness is not merely unbuilt — it is unavailable.** A uniqueness check
needs agreement about who holds which name, and there is no authority to hold
that: a Stoa has no membership list, peers join and leave without announcing
it, and each peer knows only what has reached it. Two peers can each believe a
name is free. **Whatever the design, uniqueness is not enforceable**, so the
interface must be correct when two identities present the same name.

*(Within one Stoa that is the whole argument. Names are derived per-key and
never cross Stoas, so the privacy property in constraint 2 is not what rules
this out — do not reach for it here.)*

And a bigger name space is not the lever it appears to be: **the space was
enlarged roughly a thousandfold and this paragraph did not change.** It lengthens
the odds of an *accidental* collision, which is worth doing and is all it does;
it costs a deliberate impersonator only a constant factor, because refresh is
unlimited. Treat "two identities can present the same name" as permanent.

Two people in one Stoa can hold the same name by chance, and a name resembling
anyone else's can be obtained **by pressing refresh** — there is no cost to
lean on, and the design deliberately does not pretend otherwise. What an
attacker gets is a lookalike *name* on a different *address*; they cannot forge
the address and cannot forge a signature, so nothing they publish is
attributable to the person they are imitating. **The attack is purely social,
and it is defeated by showing the address.**

So: wherever recognition carries weight — and **above all wherever a moderator
is named**, because that is what converts a button press into apparent
authority — the address must be present, not one click away.

Do **not** disambiguate by numbering. Appending "#2" to the second identical
name requires agreeing which arrived second, and arrival order differs per peer:
two people would number the same pair oppositely, each certain the other was
looking at the impostor.

**Four things help a reader tell two people apart, and only one of them settles
anything.** They are listed in this order on purpose. Read flatly as a list of
four mitigations they would suggest the problem is handled; it is not, because
**three of them are recognition aids and the fourth is the only guarantee.**

**1. The name space.** Makes an accidental collision rare — see constraint 2 for
the numbers. Does nothing else, and nothing at all about impersonation.

**2. The identicon** — intended, and yours to design. A glyph derived from the
same key, shown with the name. It earns its place on *accidental* collisions:
two people who happen to share a name still look different at a glance, because
name and glyph come from separate parts of the hash and vary independently.
**It is nonetheless forgeable in exactly the way the name is.** An attacker
grinds for a key whose name *and* glyph both read close — a two-channel search
instead of a one-channel search, which raises their cost by a factor and changes
nothing about the kind of protection on offer. **A second forgeable channel is
still forgeable**, and the identicon must never be rendered as a verification
mark, a badge, or anything that reads as "checked".

**3. Vouching** (see the vote control section). The one layer an attacker cannot
mint, because **a vouch points at a key** — not at a name, not at a picture. A
lookalike gets the name, gets a near-matching glyph, and does *not* get the
vouch. That is a real distinction. Two limits, and both must stay visible in
whatever you build:

- It is **private and never published**, so it only protects a reader against an
  impersonation of someone **that reader has already vouched for**. It does
  nothing when a reader is meeting either party for the first time — which is
  most impersonation.
- Weight **accrues from upvotes the reader has already cast**, so a new user's
  vouched set is **empty**. New users have no vouches and are the least able to
  spot an impostor: the layer is thinnest exactly where exposure is highest.

**4. The address.** The only unforgeable one, and the only thing that settles
who published something. Its single weakness is not cryptographic — **it works
only if someone looks.** That is the entire reason this obligation demands it be
present rather than one click away.

**The sentence to design against: layers 1–3 make an honest mistake less likely;
only layer 4 makes a dishonest claim false.** A row showing a name and a glyph
and no address has given the reader three recognition aids and zero guarantees.

---

## The vote control — settled, and smaller than it was

**This was the most uncertain part of the brief and it is now decided.** An
earlier version of this document described a live three-way choice between one
axis, two axes, and a one-control/three-gesture shape. **Ignore that if you saw
it.** The design is:

**One axis — up and down, as on Reddit — plus two things that are not votes.**

- **Vouching.** A reader can privately decide that some pseudonym has good
  judgement, which makes that person's votes count for more *in that reader's
  own feed*. It is never published (see obligation 3 below).
- **Report to a moderator.** Spam and abuse leave the ranking system entirely.
  They are not a third vote direction; they are a message to someone with
  binding authority.

So a post carries **one up/down control**, and separately a **report** action
that belongs with the other per-post actions rather than beside the votes.

**Why the richer shapes were dropped**, because the research is the interesting
part and it went against the author's own proposal:

- **Two-axis voting has essentially no published evaluation anywhere.** Two
  large forums have run something like it for years and measured nothing; their
  own design discussion reported the two axes moving together most of the time.
- **The only large quasi-causal study of vote mechanisms** — 155 million
  comments across 55 political subreddits that changed their reaction
  mechanism — found that up-only *and* up+down both associate with **more**
  deliberative discourse. The most demagogic case was **no reaction mechanism at
  all**. Downvotes came out fine.
- **The only randomised removal of downvotes** (a field experiment in a
  3-million-member subreddit) moved the scoreboard and not the behaviour:
  negative scores fell sharply, moderator removals did not change, and newcomers
  became *less* likely to return.
- A "contested" ordering was also dropped. A 50/50 split is precisely where a
  voter network is most polarised, so a contested sort is mechanically a
  *most-polarising-content* sort — which is not the same thing as "good
  arguments I disagree with" — and much of what it surfaces is merely
  off-topic.

**The affordance must still tolerate change, and here is the specific reason.**
The evidenced future direction is **bridging** — the mechanism behind Community
Notes, which ranks something highly when people who usually disagree with each
other *both* rate it well. The important property for you: **bridging is not a
second axis and does not change the control.** It infers the disagreement
dimension by factorising the same single-axis vote matrix, so the buttons stay
exactly as they are and only the ordering behind them changes.

So what has to tolerate change is **what a vote means**, not how many buttons
there are. Do not hard-wire a net score into the layout, and do not build
anything that reads "score = ups minus downs" as a visual primitive.

**One more ordering property worth knowing:** a post's score is **floored at
zero**. Downvoting can order a post last; it can never remove it. Hiding is
moderation, and moderation is a separate, binding thing.

### The control works; the number behind it does not exist yet

**This is the one thing in this section that will surprise you, so it is stated
plainly rather than left to be discovered.** A vote can now be cast and is stored
permanently — the write path is real. **Nothing reads votes.** There is no score,
no tally, and no ordering that consults them; core returns no vote count on any
post, in the feed or in a thread, and will return none until relevance scoring
lands.

So: **do not design a score readout, a vote count, or a net number, because there
is no value to put in one.** A control that renders "0" beside every post, or
shows a count that never moves, is worse than one that shows nothing — it tells
the reader the mechanism is working and broken rather than not yet arrived.

What the control may legitimately show is **the viewer's own vote**: whether this
reader voted, and which way. That is a local fact the interface knows because the
reader just performed it, and it does not depend on anything core resolves.

This is temporary in the same way obligation 2 is, and it resolves in the same
direction: when scoring lands, a count appears in the read shapes and the control
gains a number without changing shape. Until then the honest affordance is a
button that registers a choice, not a scoreboard.

### Still genuinely unsettled

Whether attachments render as inline images or as links.

---

## Tone

The project's own documents are unusually plain-spoken and refuse to claim more
than they can deliver. The interface should read the same way. Some examples of
the register:

- Not "no posts yet" but "you have not received anything for this Stoa yet".
- Not "trending" but what the ordering actually is.
- When something cannot be done, say what would make it possible.

It is an independent community project built on the Logos stack, not a product
with a growth team. It can afford to be honest about its own limits, and the
limits are interesting rather than embarrassing.

---

## Deliberately out of scope

No DMs, no notifications, no follower graph, no global search, no user profiles
beyond a per-Stoa identity, no reputation score visible to anyone but its owner.
Some may come later; none is designed.
