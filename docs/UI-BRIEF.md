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
> reconciled against: the vote-and-vouching design (PR #17), the Phase 3 forum
> section (PR #19), and the generated-usernames design.

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
**two adjectives and a noun drawn from science-fiction literature** — something
like *vermilion patient sandworm* — computed from the key itself. Nobody types a
name; there is no registry to hold one and a typed name carried between Stoas
would undo the unlinkability above with a text field.

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
- **A name is not unique and not an identifier** — see obligation 6 below. In a
  Stoa of a thousand there is a ~3% chance two people share a name; at five
  thousand it is better than even.

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
their most recent reply), and **top** (vote-ordered, weighted — a moderator's
vote and a vouched person's vote count for more than a stranger's, and a score
never goes below zero).

**`top` is explicitly temporary and may be withdrawn or restricted before
release.** There is no sybil resistance yet, so vote-ordering is a dial the
cheapest attacker turns; the plan names the conditions that retire it, one of
which is simply "the first sybil attempt is observed". **Design the ordering
control so an option can disappear** — which is the same requirement the labels
below already impose, from a different direction.

**But "most recent" is not currently available.** Both `new` and `active` are
defined in terms of a timestamp that the transport layer does not yet deliver,
so today both fall back to an order derived from content hashes. That order is
*convergent* — every peer computes the same sequence, which matters — but it
carries **no recency information at all.**

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

A reply is just a post that names a parent, so threads nest naturally.

Each post shows: author identity, body, attachments, whether it was edited, the
up/down control, and a report action.

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
a name on every row.

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

*(A visual identicon derived from the same key is an interesting second
recognition channel and is entirely undesigned — but it is not a substitute for
the address. A second forgeable channel is still forgeable.)*

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
