# Report the genesis record beside the address that names it

## Why

**The owner cannot open the Stoa they created.** The feed answers *"The store
could not be read… genesis: genesis record ended mid-field"*, and that message
reads like a codec defect. It is not one: the record in the store is well-formed
in every field, `Genesis::decode` accepts it, and the encode/decode round trip is
already pinned in both directions.

The truncation comes from a different input. `read_feed`, `read_thread` and
`join_stoa` each take a `genesis` hex string **supplied by the caller**, and the
view had none to supply, so it sent `""`. An empty string hex-decodes to zero
bytes, and the take of the version byte fails as "ended mid-field". The decode is
behaving correctly on an empty input; the bug is that the input was empty.

**It was empty because no reply carries the record.** A Stoa address is
`stoa_address(&genesis.canonical_bytes())` — a one-way hash. A caller holding an
address holds nothing it can invert back into a record, so a caller that was
never *given* the record cannot obtain one by any means. Creation reported
`stoa`, `foundingTitle` and `policy`; a listing item reported `stoa` and
`foundingTitle`. Neither reported the record, and three calls need it.

The absence is already recorded in the view's own source, together with the fix:
*"Two things need one and neither can work around its absence … sharing a Stoa,
and opening its feed. … Closing this properly is a CORE change — widen the
listing item to carry the retained record."* This is that change.

**Nothing has to be stored, fetched, or derived to close it.** The
`stoa-membership` capability already requires the peer retain the genesis record
for every Stoa it is in, so moderation can be resolved later, and the store holds
`canonical_bytes()` verbatim precisely so it can be handed back without a network
call. The data was on hand the whole time; only the wire shape omitted it.

## What Changes

- **`create_stoa`'s reply carries the genesis record**, hex-encoded, beside the
  address it is the hash of.
- **Each `list_stoas` item carries it too.** This is the half that survives a
  restart: a view can only remember what it created or joined in the current
  session, so without it a relaunched app can open nothing at all.
- The encoding is the **same hex those records are handed back in**, so a reply
  and the request that quotes it cannot disagree about one record.
- The view fills its record map from both replies, which restores **Open** and
  un-hides the **share** affordance — both were gated on the same lookup.

This widens the core API, which is a contract that outlives any particular UI. It
is a widening rather than a change of shape: every existing field keeps its name
and meaning, so a caller reading only the old fields is unaffected.

## Impact

- Affected specs: `stoa-membership` (two requirements modified, one added)
- Affected code: `wire.rs` (`stoa_reply`, `membership_page_json`),
  `DStoaListScreen.qml`
- No storage change, no migration, no new call, no network traffic.
