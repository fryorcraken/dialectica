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
>
> Also reconciled against **§9.2** (the first release's scope), which suspends
> one property this brief previously stated as fact — see the box below.

---

## What is in the first release, and why that matters to you

**The project is pushing for a working release, and it is a subset of this
brief.** PLAN.md §9.2 is the authority; this is the designer's-eye version.
**The rest of the brief still describes the intended product** — the point of
naming the subset is so you know which constraints are live now and which are
waiting, not so you design only the subset.

**In the first release:** create an identity; create a Stoa; post; reply; upvote
and downvote; share a Stoa; join a Stoa somebody shared; receive other people's
posts; view a feed and view a thread; and have all of it survive closing the app.

**"Share" and "join" are deliberately not written as "copy the address" and
"paste the address" any more.** An address alone cannot be joined — see *Joining
a Stoa* — so whatever a person shares has to carry the Stoa's founding record
beside its address. What that shareable thing looks like is a design question
this brief does not answer, and it is the first one to answer for this screen.

**Not in the first release:** moderation, attachments, and per-Stoa identity.

Each of those three has a consequence for design rather than just for scope, and
each is marked where it belongs: **moderation** under *Moderation* and in the
obligations, **attachments** under *Thread*, and **per-Stoa identity** in
constraint 2 — which is the one that changes something this brief previously
asserted as true, so read it rather than skimming it.

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

### 2. Identity is permanent and pseudonymous — and in the first release, one per person

**The design is a different, unlinkable identity in every Stoa**, which is a
privacy property by construction: the same human in two Stoas cannot be
correlated. There is no global profile, no avatar service, no display name
registry. No rotation: an identity is permanent.

**The first release ships one identity per person, used in every Stoa** — a
deliberate scope decision (PLAN.md §9.2), taken to get a working product out.
**So the unlinkability above does not hold yet.** In the first release, one key
signs in every Stoa a person joins, and an observer watching two Stoas can tell
it is the same participant. The property is **suspended, not abandoned**: the
mechanism for per-Stoa identity is already built in the core and simply is not
switched on, and switching it on later is not a redesign.

**What that means for you, and it cuts both ways:**

- **Design as though unlinkability holds.** No cross-Stoa profile page, no "also
  active in" links, no unified inbox, no "your identities" comparison view.
  Anything that *displays* a correlation the protocol is meant to prevent becomes
  wrong the moment per-Stoa identity lands, and is a bad idea now regardless.
- **But do not claim it in copy.** No onboarding line promising that the Stoas a
  person joins cannot be connected, and no privacy explainer asserting it. The
  interface must not tell a user they have a property they do not have — that is
  the one failure here that could actually harm someone.
- **What arrives with per-Stoa identity is a flow, and it is worth knowing it is
  coming:** creating or joining a Stoa will ask *which* identity, with a
  create-or-select step at that moment. Nothing needs designing for it now, but a
  join flow (below) that assumes there is exactly one possible identity forever
  will need reopening.

**Identities have generated names, and this is new.** An identity renders as
**two adjectives and two nouns drawn from Greek philosophy and letters** —
something like *measured attic thales praxis* or *sober ionic stoic kairos* —
computed from the key itself. Nobody types a name; there is no registry to hold
one, and a typed name carried between Stoas would undo the unlinkability above
with a text field — which is a reason that outlives the first release's
suspension of it, since a name field would make the property unrestorable rather
than merely switched off.

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

**What the user is choosing is a derivation path over one master key**, not one
of five separate keys. That sounds like an implementation detail and it has a
consequence you have to render, below: **one saved value backs up every identity
the person will ever have** — so a backup flow is one secret, not one per Stoa.

Two consequences for you:

- **An onboarding screen exists that did not before**: five identities, pick
  one, refresh for more. It is the first thing a new user sees in a Stoa.
  **Core now serves this** — a slate call returns five candidates with an address
  and a public key each, a keep call stores the one chosen, and a "who am I" call
  answers afterwards. Refreshing is unlimited and is never refused for having
  been pressed too often.
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

**Split by release, because the two halves separate cleanly.** The first release
ships **no way to publish a hide** — no moderation screen, no hide control (see
*Moderation*). What it can still do is the read half: the core already answers
whether a post is hidden, so omitting hidden posts, and a "show hidden" view, are
available and correct from day one, and they need no key and no moderator status.
The irreversibility warning attaches to the *control*, so it arrives when the
control does.

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

Stoas the user has joined **or created**. **There is no directory to browse**;
a Stoa reaches the user because somebody shared it, or because an affordance in a
post carries it. **An address alone is not enough to join** — see *Joining a
Stoa*, which is the screen where that matters and where it is argued.

A Stoa's *founding* title is fixed forever, while its *current* title can be
changed by moderators — so two Stoas can share a display name and still be
entirely different Stoas. **A title is not an identifier.** The address is.

**Today the core returns only the founding title, and the field is named
`foundingTitle` so that this cannot be missed.** Nothing resolves the
moderator-signed metadata op that carries a current title, so a screen labelling
this value as the Stoa's present name asserts something no peer has checked.
Design for the label you can honestly apply now — and expect a `title` field to
appear *beside* `foundingTitle` later, with an `isGenesisFallback` flag saying
whether it was resolved or fell back. **The layout should absorb that without
being redrawn**, which is the practical reason the distinction is stated here
rather than left to the day it matters.

Two things follow for this screen:

- A Stoa whose title is **empty** is legal and reachable: the founding record has
  no minimum length. A row must render without a title rather than collapsing.
- A founding title carries whatever characters its creator typed, **unnormalised
  and unsanitised**, bidi overrides and zero-width joiners included. The core
  preserves it exactly on purpose, because normalising would change the address
  and split one Stoa into two. Rendering it safely is this screen's job; see the
  Unicode obligation below.

### Joining a Stoa — a security surface, not a form

An address is a copyable string that is **self-authenticating**: pasting it is
enough to verify what you joined, because the address is a hash of the Stoa's
founding record.

**Pasting an address alone is not enough to join, and that is a property of the
address rather than a missing feature.** A hash verifies a record somebody hands
over; it cannot reconstruct one. So the join flow needs the **founding record**
as well as the address — which means whatever a user shares, and whatever an
in-post affordance carries, has to carry both. A screen designed around a
single pasteable field cannot work.

**Requirements:**
- Show what is being joined **before** joining it.
- An address appearing inside a post is attacker-supplied. Render it as an
  affordance the reader chooses to act on. **Never auto-join.**
- Two Stoas may present the same name. Show something that distinguishes them.
- **Joining a Stoa the user is already in is not an error.** The core reports the
  same success either way, deliberately: a pasted address is exactly the input
  someone supplies twice, and it changes nothing about what is already held. Do
  not design an error state for it.

### Creating a Stoa

A title, and nothing else. The creator's key comes from the user's own keystore
and **cannot be supplied** — a Stoa created under someone else's key is one the
creator cannot moderate, and its address cannot be un-minted.

**The key recorded as creator is the same key the user posts under**, which is
the one identity constraint 2 describes. It matters here because a Stoa's creator
is its sole moderator: a creator key the user does not sign with would be a Stoa
nobody can moderate, permanently, since the creator is fixed inside the address.
Nothing on this screen shows any of that today — moderation is out of the first
release — but a later "you moderate this Stoa" badge will be answering the same
question, so do not design as though the creator and the poster could be
different people.

**Requirements:**
- **Show the new Stoa's address after creating it.** It is the only way to share
  the Stoa, and there is no registry to look it up in later.
- **Creating the same title twice is the same Stoa, not a second one.** The
  founding record carries no timestamp and no random value, so the same person
  and the same title produce the same address. The second attempt succeeds and
  reports the same Stoa. If a screen wants "create another", it has to ask for a
  different title — do not present this as a name collision to resolve, and do
  not add a counter to the title on the user's behalf.
- Creation fails when there is no usable key, with the same reason vocabulary the
  posting probe uses — so a failure here reads identically to a failure to post,
  which is deliberate.

  **This screen cannot gate itself on that probe, and an earlier version of this
  brief asked it to.** The probe is `getCapabilities({stoa})` and takes a Stoa
  address; at creation there is no Stoa yet, so there is nothing to ask it about.
  Design the create affordance to be **always offered and able to fail well**:
  show the core's own reason, and route the user to whatever fixes it (creating or
  unlocking an identity), rather than hiding the button. Gating on a build flag is
  still wrong, and so is guessing at key state from anything other than an answer
  the core gave.

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

A reply is just a post that names a parent, so threads nest naturally.

Each post shows: author identity, body, attachments, whether it was edited, the
up/down control, and a report action.

**What core hands you is a flat list, and nesting is yours to compute** (the
`thread-read` spec). Each item names its parent; nothing reports a depth or an
indent level. That is deliberate and it costs you nothing: depth is a count of
parents, and you hold the parents. Two consequences worth designing for.

- **An item's parent may not be on screen.** It can fall on an earlier page, or
  be hidden while the reader has not asked to see hidden posts. So a reply can
  arrive with nothing to attach it to. Do not drop it, and do not render it as a
  root — it is a reply to something not shown, and saying so is honest.
- **Hiding a post does not hide the replies under it.** One hide binds one post.
  A subtree that vanished because its top was hidden would apply a moderation to
  ops no moderator acted on.

**The root is always the first item of the first page and never repeats.** Later
pages are replies only.

**A hidden root behaves unlike a hidden reply, and the asymmetry is deliberate.**
A hidden *reply* is simply absent from the default view. A hidden *root* is still
returned, marked hidden, with its body withheld — because a thread read that
dropped its own subject would be indistinguishable from a thread this machine
has never received, and those mean opposite things. So a thread screen must have
a state for "this thread's opening post was hidden", and it must not look like
the not-found state or the empty state.

**A post whose thread this machine cannot place does not appear.** A reply
whose parent has not arrived yet cannot be positioned, so it is not shown
anywhere rather than being shown at the root. It appears when the parent does.
Nothing is lost and nothing is wrong; it is the ordinary condition of a
peer-to-peer forum, and it is the same fact the reply composer already has to
explain.

**Attachments are not in the first release** — posts are text (PLAN.md §9.2
excludes Logos Storage, which is where attachment bytes live). Design the post so
an attachment area can appear later without the layout changing; do not design a
post that looks unfinished without one.

### Composition

**Gate every posting affordance on whether the user can actually post.** The
core exposes a capability probe that answers `{"canPost": true, "identity": …}`
or `{"canPost": false, "reason": …}`.

Reasons are human-readable and **name a fix** — e.g. "no keystore found; create
one before posting", "keystore permissions are too open (mode 0644); restrict it
to owner-only and replace the key".

**Never gate on a build flag, and never show a compose box that cannot be
submitted** — it loses whatever the user typed. Surface the reason instead.

**Two obligations the core creates and cannot meet itself.** Both come from the
publish contract (`content-authoring`), and neither is visible from a screenshot.

**1. Posting the same thing twice posts once, and the interface has to handle
it.** A post is named by a hash of its own content, and nothing in that content
varies between two submissions — so one person posting the same body into the same
Stoa twice produces **one post**. The second submission succeeds and tells you it
stored nothing new.

That is exactly right for a double-tapped submit button, and it is wrong for
someone deliberately writing "agreed" twice in one thread, which is ordinary
forum behaviour. The core reports which of the two happened; **the interface
decides what the person sees**, and the failing design is the one that reports
success and shows nothing new, because the person concludes their post vanished.
Reasonable answers: say so plainly ("you already posted this"), or scroll to and
highlight the existing post. **Do not** show a spinner that resolves to nothing,
and do not show a generic error — nothing failed.

This is a known gap with a known fix (a timestamp or nonce inside the post), and
it is deliberately not fixed yet. Design for the behaviour that exists.

**2. A reply needs its parent, and a peer does not always have it.** A reply names
the post it answers, and the core works out which thread that is by reading the
parent. If the parent has not reached this peer yet — ordinary in a peer-to-peer
forum where two people legitimately hold different sets of posts — the reply is
**refused**.

So a reply control can fail for a reason that is nobody's fault and is temporary.
The message must say that: the post being replied to has not arrived here yet, try
again shortly. It must not read as an error the person caused, and **the draft must
survive** — this is the one refusal that is expected to succeed on a retry, so
discarding what they typed is the worst possible response to it.

### Moderation

> **Not in the first release** (PLAN.md §9.2). The core's moderation logic is
> built and merged, and the first release ships **no moderation screen and no way
> to publish a moderation action**. It is scope, not a change of design — what
> follows, and every moderation obligation in this brief, is what a later release
> must do. The irreversibility warning below is part of why the sequencing is
> comfortable: a hide control shipped today would have to warn that its action
> cannot be undone.

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
A Stoa's title — the founding one the core returns today, and the moderator-signed
current one that will arrive beside it — is **not unique, not verified against
anything, and freely chosen** by whoever created or renamed the Stoa. Two
unrelated Stoas can present the same name, and one can be named to impersonate
another. **The address is the identity; the title is decoration.** A confirmation
screen that shows "Join *Agora*?" and nothing else has shown the reader precisely
the part an attacker controls.

This applies to the Stoa **list** as much as to the confirmation, and for the same
reason: a row showing only a title cannot be told from a row for a different Stoa
with the same title. The core returns the address on every list item so that a
screen always has the distinguishing half available.

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

*(One thing to know about where the name comes from, and this paragraph has been
wrong twice. The name is **not** derived from the address: it is derived from the
**public key**, on purpose, so that a name tracks the key that signs — while the
**mark** is derived from the address. Two independent digests, two different
inputs. So a view holding only an address **cannot** compute the name.*

*The correction to the previous version: core does **not** hand you a rendered
name, and should not — a name is a pure function of the key, so sending both
would put a derived value on the wire beside the material it comes from, where
the two could disagree. **Core gives you the address and the public key**, and
deriving the name from the key is the interface's job, as deriving the mark from
the address already is.*

*The thread read does this (the `thread-read` spec). **The feed read does not
yet** — it returns an address per row and drops the key, which is why the feed
screen renders an empty name today. That is a known gap with an owner, not a
design decision to build around.)*

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

**7. Do not present a saved master key as a complete backup, because right now
it is not.**
Which of the five candidates a person kept is **metadata stored only on their own
machine**. The master key reproduces every candidate; it does not say which one
they chose. So a person holding an exported master key and nothing else has a
secret that can derive their identity and **no way to know which identity it
was**.

Core tells you this rather than leaving you to infer it: the "who am I" reply
carries a flag saying whether recovery needs more than the master key. **It is
currently always set**, because export and remote backup are not built. When they
are, the flag stops being set and the copy should follow it rather than being
rewritten.

**So a backup screen must not say "save this and you can always get back in."**
It can truthfully say: this is your master key, keep it; and separately, your
identity choices live on this device only, for now. The failure to avoid is the
reassuring version — a person who believes they are covered, loses the machine,
and discovers otherwise.

**8. An unencrypted master key is a state you have to be able to show.**
Whether the key on disk is encrypted depends on whether a passphrase was
available, and **there is currently no passphrase UI**, so on an ordinary install
it is stored **in the clear**. That is a real configuration, not a bug — a
machine with an encrypted disk is a reasonable place for it — and core reports
which of the two happened rather than leaving it to be guessed: the keep reply
carries an `encrypted` flag.

Two things follow, and the second is the one that is easy to get wrong:

- **Somewhere reachable, a person should be able to see the answer.** "The secret
  on your disk is in the clear" is not a fact someone should have to read the
  source to learn.
- **Do not render a padlock, a shield, or the word "secure" on the strength of
  this flag being unset.** Protection that *reads* as strong while being absent
  is worse than visible plaintext, because plaintext is something a person can
  act on. If the flag says unencrypted, the interface should say so plainly or
  say nothing — never imply the opposite.

Whether a passphrase gets asked for at all is undecided upstream, so this
obligation is about reporting the state honestly rather than about a flow.

One detail that matters if you show this per Stoa: **the flag describes the master
key, which is one file for the whole install, not one per Stoa.** The first keep
creates that file and reports the protection it wrote; a keep in a second Stoa writes
no key and reports the protection the existing file *has*. Both answers are true about
the same single secret, so do not render them as two independent facts — "this
identity is encrypted, that one is not" is not a state that can occur.

---

## The vote control — settled, and smaller than it was

**This was the most uncertain part of the brief and it is now decided.** An
earlier version of this document described a live three-way choice between one
axis, two axes, and a one-control/three-gesture shape. **Ignore that if you saw
it.** The design is:

> **In the first release a vote is recorded and ranks nothing, and this is the
> hardest honesty problem in the brief.** Pressing up or down publishes a real,
> signed, permanent record. Nothing reads it yet: there is no score, and neither
> feed ordering consults votes. The plan was originally to ship no vote control
> for exactly this reason — *"a control with no visible effect teaches users the
> app is broken"* — and the owner decided votes ship anyway.
>
> So the obligation lands here. **The control must not imply a ranking it does not
> produce.** One thing is safe, one is not available yet, and one is never safe:
>
> - **Safe:** showing the reader their own vote back, as state on the button. That
>   is real, immediate and true — they voted, and the interface remembers.
> - **Not available yet, though it would be safe:** a plain count of votes on a
>   post. No call returns one — the thread read carries no score or tally, and the
>   publish reply carries only an op id — so a count cannot be rendered today.
>   Design for its absence. If a later change exposes one, it is safe only
>   presented as a count, never as a position, a rank, or a reason this post
>   appears where it does.
> - **Not safe:** anything suggesting the vote moved the post, changed what anyone
>   else sees, or fed an ordering. It did not. No "trending", no arrow, no implied
>   effect on the feed.
>
> **Do not let this leak into the ordering controls either.** The feed offers two
> orderings and neither is vote-based; a "top" or "best" option must not appear.

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

Whether attachments render as inline images or as links — and note that
attachments are out of the first release entirely (see Thread, above), so this is
a question for a later one.

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
