# Trustless block confirmation for the verify page

Status: built, for Ethereum and the six L2s. Willow's servers only send data, and the browser
verifies all of it.

## The problem

Every proof the page verifies ends at a block hash: the transaction and receipt proofs prove items
under a header, the SP1 log proofs prove logs under a chain of headers. The page then asks two
public RPC nodes run by third parties (`CHAINS[].nodes`, `confirmOnNodes`, `finalityOn`) whether
that hash is block N of the chain, and whether N is final (on an L2: settled on Ethereum). That answer is the one thing the page
takes on trust. This design replaces it with verification the browser runs itself, so that in the end
the page trusts Ethereum's validators and nothing else.

## Summary

| Chain | What the browser verifies | Levels it can show | Data per block | Left to trust |
|---|---|---|---|---|
| Ethereum | Ethereum's light client (sync committee) from a checkpoint built into the page; then the block's hash in Ethereum's own block-hash history (EIP-2935, last 8,191 blocks), or for older blocks in the beacon state's `block_roots` / `historical_summaries` | **Signed** by Ethereum's validators (seconds), **Final** (about 13 to 19 minutes) | 10 to 16 KB, plus the light client's catch-up (about 60 KB per 27 hours since the checkpoint, once per browser) | Ethereum's current sync committee: a false chain needs 342 of its 512 members' signatures; the checkpoint, which ships with the page's code |
| Base, OP Mainnet, Unichain, Ink | The Ethereum light client, then the chain's dispute games and SystemConfig on Ethereum (storage proofs, Ethereum's contract code run over them), the output root, and the L2's own block-hash history (EIP-2935 on the L2) back to the block | **Signed** by the chain's sequencer (seconds), **Committed** on Ethereum by the chain's proposer (20 minutes to 12 hours), **Final** on Ethereum (about 3.5 to 7 days) | 20 to 60 KB; up to 240 KB more for a sequencer-signed Base block | Ethereum as above; for Final, the chain's fault-proof system and its governance (guardian, game retirement), exactly as a withdrawal to Ethereum does |
| Arbitrum One, Robinhood Chain | The Ethereum light client, then the chain's BoLD assertions on Ethereum, the assertion's block hash, and Arbitrum's block-hash history (EIP-2935 ring of 393,168 blocks) back to the block | **Signed** by the sequencer (feed `signatureV2`, seconds), **Committed** in an assertion on Ethereum (about an hour), **Final** when the assertion is confirmed (about 6.4 days) | 15 to 30 KB | Ethereum as above; for Final, BoLD and the chain's governance; Robinhood's validators are whitelisted |

Robinhood Chain settles on Ethereum directly, not on Arbitrum One: its Rollup
(`0x23A19d23e89166adedbDcB432518AB01e4272D94`), SequencerInbox and Bridge are Ethereum contracts.

When no level can be verified (Willow cannot serve the data, the block is outside what the method
covers, an upgrade the page does not know yet), the page says that the proof is valid and the block
is **not confirmed**, and why, and never shows the green verdict.

## Decisions taken here

1. **Ethereum: beacon-state proofs for history, EIP-2935 for the last day.** Walking execution
   headers back is out: seven days is 50,400 headers, about 30 MB. The beacon state's
   `historical_summaries` reaches any block since Capella (April 2023) with one fixed-size proof
   (about 2.4 KB), which is what it was added for. Its one cost is server side: the proof needs a
   beacon state at least as new as the block, and a state is about 340 MB, so Willow fetches one
   every few hours, not one per block. The day in between is covered by Ethereum's EIP-2935 history contract,
   which holds the hashes of the last 8,191 blocks (27 hours) in the execution state: one storage
   proof of about 5.4 KB against the light client's newest verified state root. Helios (the
   reference) uses the same contract for the same job.
2. **The checkpoint is pinned in the page**, as `ETH_CHECKPOINT` in `app.js`: a finalized beacon
   block root at least two sync-committee periods (2.3 days) older than the build, so the committee
   that signed Willow's daily anchor is always one the browser has. The page's code already comes
   from Willow, so this adds no trust; pinning it in the page rather than fetching it keeps a saved
   or audited copy of the page (`SHA256SUMS.txt`) as trustless as the day it was built. The page
   refuses a checkpoint older than 30 days: past it, forging a chain from the checkpoint needs only
   342 of an old committee's 512 keys, and keys of validators that have since left are cheaper to
   come by. `build.sh` refreshes the checkpoint at each build (`checkpoint.py`: the first block of
   the period two before the newest finalized one, its light-client bootstrap read from two
   independent beacon APIs, which must agree, and its header hashed to the root), so a build
   serves 30 days at most and the page must be rebuilt and deployed within that. Each browser keeps what it
   verified, the bootstrap and each period's update (localStorage), and verifies it again from the
   pinned checkpoint on the next visit: nothing kept is trusted, and only new periods are
   downloaded.
3. **Public nodes stay only as a labelled fallback, in gold, never the green verdict.** Ethereum
   does not use them at all. On the L2s every block has a trustless path now (signed, posted or
   settled), so the two public nodes are asked only when Willow's data for a block cannot be verified
   (Willow down, an upstream without the state it needs). The card then says "not verified against
   Ethereum", in gold, and never shows the block as settled: Willow itself can force this fallback
   by sending nothing, so it must never read as verified. The user's own node, typed under
   Advanced, stays: that is the user's choice.
4. **L1 contract reads are verified, not trusted.** Storage proofs against the light client's
   finalized execution state root. Where the meaning lives in contract logic (`isGameClaimValid`,
   `getAssertion`), the browser runs the contract's own code over the proven state (an EVM in wasm,
   the way Helios does verified `eth_call`), so a contract upgrade cannot silently change what the
   page verifies: unproven state or code makes the call fail.
5. **Willow serves everything the browser needs**, through methods on its log RPC. It adds no
   trust: the browser verifies all of it.

## What each level means, in the page's words

- **Signed** (Ethereum): "Ethereum's validators have signed a chain with this block in it. It is
  not final yet." The sync committee signed a head whose history contains the block. A reorg of
  the newest blocks can still undo it.
- **Final** (Ethereum): "Finalized on Ethereum, can't be reversed." Ethereum's validators finalized a block at or after
  this one, and this block is its ancestor. Undoing it would cost a third of all staked ETH.
- **Signed by the sequencer** (L2): "Signed by Base's sequencer, not on Ethereum yet." The
  chain's operator vouches for the block, its key read from Ethereum; nothing is on Ethereum yet.
- **Posted** (L2): "Posted to Ethereum by Base's proposer, settles in about 5 days" (the chain's
  own number of days; "a bonded Arbitrum One validator" and "a validator on Robinhood Chain's
  list" on those chains). The claim is on Ethereum for a state that includes this block; it can
  still be disputed.
- **Settled** (L2): "Settled on Ethereum." The claim passed the chain's challenge period and
  Ethereum's contracts treat it as valid, the same standard a withdrawal to Ethereum uses.

Below settled (posted, signed, and the public-node fallback) the card adds one line: "Base treats
this block as final about 20 minutes in. This page doesn't verify that" ("will treat" for a block
younger than that). That is the chain's own finality (its batch in a finalized Ethereum block),
which cannot be verified in a browser: it needs the chain's derivation and execution. Willow can
derive it on its server, but that is a claim, not something the page can verify. Nowhere else does the page call an L2 block final; Ethereum's levels keep the word.

---

## Ethereum

### Mechanism

1. **Light client.** The browser starts from the pinned checkpoint's `LightClientBootstrap`: the
   header must hash to the pinned root, and the current sync committee must sit under its state
   root (gindex 54 through Deneb, 86 from Electra). It then applies one `LightClientUpdate` per
   period up to the current one. Each must be signed by at least 342 of the 512 members of the
   committee it already holds (BLS fast-aggregate-verify; signing domain from the fork at
   `signature_slot - 1` and mainnet's `genesis_validators_root`), its attested header and signature
   in the same period, and its `next_sync_committee` under the attested state root (gindex 55 / 87).
   That gives the next committee. Code: `zkvm/logset-guest/anchor-verify`, on the SP1 anchor
   child's core (`zkvm/anchor-guest/core`: header and committee roots, gindices, fork schedule) and
   its BLS verification run on crates.io `bls12_381` (the SP1 fork does not build for wasm32).
2. **The anchor header.** A `LightClientFinalityUpdate` signed by the committee of its signature
   period gives two headers: the attested one (the head, `A`) and the finalized one (`F`) under
   `A`'s state (finality branch, gindex 105 / 169). Each `LightClientHeader` carries its execution
   payload header with a branch to the beacon body root (gindex 25), so the browser knows `A`'s and
   `F`'s execution block number, hash and state root.
3. **The block, last 27 hours (`history`).** For a block N with `A.number - 8191 <= N < A.number`:
   an account proof of the EIP-2935 contract (`0x0000F90827F1C53a10cb7A02335B175320002935`, code
   hash pinned) against `A`'s execution state root, and a storage proof of slot `N mod 8191`. Its
   value is block N's hash on the chain `A` heads. N is **final** when `N <= F.number` (`F` is `A`'s
   finalized ancestor, so every block of `A`'s chain up to `F` is final), else **signed**. N equal
   to `A.number` is `A`'s own hash (`head`).
4. **Older blocks (`beacon`).** Willow keeps one anchor a day: a finalized header `S`, the finality
   update that finalized it, and `S`'s state. The proof is the window-anchor witness
   (`zkvm/window-anchor-guest/core`, statement kind `0x4101`): the target beacon block's execution
   payload leaves (number, hash) up to the beacon block root (gindex 201), then up to `S`'s state
   root through `block_roots[slot mod 8192]` (within 8,192 slots) or through
   `historical_summaries[chunk].block_summary_root[slot mod 8192]` (older, back to Capella), then
   up to `S`'s block root. The browser verifies that root equals the header the finality update
   proved final. **Final**.

The browser compares the proven hash with the one the proof ended at: equal is confirmed; another
hash is a verified "Ethereum has another block N" (the old node-mismatch case, now proven).

### What Willow serves

On `/v1/rpc/1/...` (and `public`):

| Method | Answer | Size |
|---|---|---|
| `willow_lightClientBootstrap [blockRoot]` | the beacon API's bootstrap, as it sends it (`{version, data}`) | about 55 KB |
| `willow_lightClientUpdates [startPeriod, count]` | the beacon API's best update per period, at most 16 | about 58 KB each |
| `willow_getBlockAnchor [blockNumber]` | `{blockNumber, update, proof}`: the finality update the proof hangs from, and `{kind: "head"}`, `{kind: "history", proof: <eth_getProof result>}` or `{kind: "beacon", witness}` | 10 to 16 KB |

Willow's server verifies every answer with this same code before serving it, so bad upstream data
is not passed on to every browser. For `beacon` it keeps finalized beacon states and a table of
block roots by slot, which reach any block back to Capella.

### What the browser verifies

All of it, in a third wasm module (`anchor`: `zkvm/logset-guest/anchor-verify` built by `anchor-wasm`), loaded on first
use, in the worker: the bootstrap against the pinned root, every update's signature, participation
and branches, the finality update, the execution branches, the EIP-2935 account and storage proofs
(alloy-trie, as the transaction proofs already use) or the beacon witness, and the block number and
hash. JSON from servers goes to the wasm as sent; no verdict reads message text.

### Data and time in the browser

- First visit in a browser: bootstrap plus one update per period since the checkpoint. With the
  checkpoint two to nine periods old, 170 to 580 KB of JSON, about half that
  gzipped. Afterwards only the periods since the last visit.
- Per block: the finality update (about 5 KB) plus 5.4 KB of proof (`history`) or 2.4 KB
  (`beacon`), as hex.
- CPU: one BLS aggregate verify per update and one per finality update (measured below).

### Freshness

| Level | Reached | Notes |
|---|---|---|
| Signed | once a head that includes the block is signed: about 12 s | a block newer than the light client's head is asked again, as the node check did; shown only while that head is under 10 minutes old |
| Final | about 13 to 19 minutes | two epochs, plus the light client learning it (median 18 s) |
| Any age back to Capella | final | Willow's beacon anchor; older than its block-root table: not confirmed, said plainly |

### What is left to trust

- No 342 of the 512 validators in the current sync committee sign a false chain: a forged header
  needs 342 of their signatures. That is the light-client assumption: weaker than full finality (a
  third of all stake), and the standard for browsers.
- The checkpoint: it ships with the page's code, which the page already says comes from Willow.
  What the browser keeps between visits is verified again from it, so it adds nothing to trust.
- Not trusted: Willow's RPC, the beacon API, the checkpoint-sync servers, execution RPCs, public
  nodes.

### Glamsterdam (ePBS, EIP-7732)

- Light client: Gloas changes the light-client header (the beacon block no longer carries the
  execution payload; it carries a builder's bid). The page reads the `version` of every
  light-client answer and refuses any fork it does not know, with "Ethereum's latest upgrade is
  newer than this page", so it fails closed until updated, instead of mis-reading.
- `history` keeps working in principle (EIP-2935 is execution state), but the anchor must be a
  payload Ethereum processed, not a bid: the execution state root then comes from the header whose
  hash is the state's `latest_block_hash` (or the next bid's `parent_block_hash`), fetched and
  verified by hash.
- `beacon`: a canonical beacon block's bid no longer proves its payload was included. The proof
  becomes the target's bid plus a later canonical block whose bid's `parent_block_hash` is the
  target's hash (two beacon block proofs instead of one), or the state's
  `execution_payload_availability` bit for the slot while it is within 8,192 slots.
- The slot-to-fork tables (`zkvm/anchor-guest/core/src/schedule.rs`,
  `window-anchor-guest/core/src/fork_schedule.rs`, `beacon-cl-guest/core/src/fork.rs`) have no
  wildcard arms, so adding Gloas is a compile error at every place that needs a decision.

---

## OP stack: Base, OP Mainnet, Unichain, Ink

### Mechanism

1. **Ethereum light client**, as above. L1 reads use the **finalized** header `F`'s execution state
   root, so what the page reads on Ethereum cannot be reorged away.
2. **Settled.** A dispute game `G` whose L2 block `M >= N`, for which the chain's
   AnchorStateRegistry's `isGameClaimValid(G)` is true at `F` (respected type when created,
   resolved for the defender, past the airgap, not blacklisted, retired or paused).
   - Base (game type 621, AggregateVerifier): `rootClaim` is the output root
     `keccak(0^32 ‖ stateRoot ‖ messagePasserStorageRoot ‖ blockHash)`, `l2SequenceNumber` is `M`.
   - OP Mainnet, Unichain, Ink (type 9, super roots): `rootClaim = keccak(0x01 ‖ u64 timestamp ‖
     (u256 chainId ‖ outputRoot)*)`, pairs in ascending chain id, `l2SequenceNumber` is the
     timestamp; today each super root holds only its own chain. The browser hashes the preimage
     Willow sends, takes its chain's output root, and the block whose header timestamp is the
     super root's.
   The browser gets `isGameClaimValid`, `rootClaim` and `l2SequenceNumber` by running the
   contracts' code over proven state (decision 4): account and storage proofs for every account
   and slot the call touches, and code verified against each account's code hash; touching anything
   not proven fails the call.
3. **From `M` to `N`.** The output root gives `M`'s state root and hash. OP-stack chains run
   EIP-2935 since Isthmus (measured today: the contract is there on all four, and its slots hold
   the right hashes), so a storage proof of slot `N mod 8191` against `M`'s state root gives block
   N's hash when `M - N < 8191`. Further back, each hop proves the hash of block `M - 8191`, then
   its header (RLP, verified by hash) gives that block's state root, and so on: about 6 KB a hop.
   Hops by chain (games every 20 minutes on Base, 1 hour on OP and Unichain, 12 hours on Ink): 0
   on Base, OP and Unichain; up to 6 on Ink.
4. **Posted.** A game for `M >= N` that exists in the DisputeGameFactory, is of the respected
   type, is not resolved against, and was created by the chain's proposer. OP Mainnet's, Unichain's
   and Ink's games are permissionless: anyone can post one for a bond. Others' games are there:
   Ink's game 2574, of the respected type, was created by `0xe3B9…7e89` and claims a time 5.4 days
   ahead (read on Ethereum on 2026-10-20). So the proposer is pinned per chain (`OP_CHAINS` in
   `anchor-verify`); the browser declines a game by anyone else (not a verdict on the block: the
   public-node fallback, in gold), and Willow's game search follows only the proposer's games,
   whose positions only grow, skipping the rest. When the proposer changes, games stop reading as
   posted until the page is updated. A game being challenged still reads as posted: the
   claim is the proposer's until it is lost.
5. **Signed by the sequencer.** For a block newer than the newest game: a block `M >= N` the
   sequencer gossiped, `signature ‖ [parent beacon root] ‖ SSZ execution payload` (decompressed),
   signed over `keccak(0^32 ‖ chainId ‖ keccak(data))`. The browser recovers the signer and
   compares it with SystemConfig's `unsafeBlockSigner()`, run over proven state at `F`; reads
   `M`'s number and hash from the payload (the extra-data offset must match the gossip version,
   which the signature does not cover); verifies `M`'s header by hash; and reads `N`'s hash from
   `M`'s EIP-2935 history, hopping as in 3 (Ink's games are 12 hours apart, its history 2.3
   hours). A signed block over 3 hours old is not used: a claim on Ethereum should cover it by
   then, and until one does the sequencer could still replace it. The key is read at `F`, so a
   key Ethereum revokes is still taken until `F` passes the change, an hour at most.

### What Willow serves

On each L2's route, `willow_getBlockAnchor [blockNumber]` answers with the finality update, the
Ethereum state witness `l1` (`eth_getProof` results with code; trie nodes shared by several proofs
are sent once, `{"accounts", "nodes"}`), `headers`, L2 history `proofs`, and either the `game` or
the `signed` gossip message. The browser picks the level from what it verifies, never from a label.
Willow finds the earliest game at or after N, and proves L2 state at `M` from a node that keeps
old state (games for Settled are 3.5 to 7 days old).

### Data and freshness

| Level | Reached | Browser downloads (besides the Ethereum light client) |
|---|---|---|
| Signed by the sequencer | seconds | the signed message and L1 proofs: 245 KB on Base (measured) |
| Posted | Base about 20 to 40 min; OP, Unichain about 1 to 1.5 h; Ink up to 12 h | about 15 to 25 KB of L1 proofs and code, plus 6 KB per L2 hop |
| Settled | Base about 5 days (1 day once both of its proof arms post); OP, Unichain, Ink about 7 days | the same |

### What is left to trust

Ethereum's sync committee as above. For Settled, the chain's fault-proof system and the governance
that can blacklist or retire games: the same as for any withdrawal from the chain. For Posted,
the proposer (and its bond). For Signed, the sequencer. Not trusted: Willow, the L2's RPC nodes.

---

## Arbitrum Nitro: Arbitrum One, Robinhood Chain

### Mechanism

1. Ethereum light client, reads at `F`.
2. **Assertions.** The Rollup (Arbitrum One `0x4DCeB440657f21083db8aDd07665f8ddBe1DCfc0`,
   Robinhood `0x23A19d23e89166adedbDcB432518AB01e4272D94`) stores assertions by hash:
   `keccak(parentAssertionHash ‖ keccak(abi.encode(AssertionState)) ‖ afterInboxBatchAcc)`, where
   the state's first word is the L2 block hash at the assertion's end. Willow sends the state, the
   parent hash and the accumulator (from the `AssertionCreated` log); the browser hashes them and
   runs `getAssertion(hash)` over proven state: status **Confirmed** is Settled. **Pending** is
   Posted only if it and every pending assertion it builds on, back to a confirmed one, is its
   parent's only child (first child, no second): an assertion on a losing branch stays pending
   forever, so a rival anywhere in that chain makes the page say the claim is disputed. Willow
   sends each pending ancestor's preimage (parent, state hash, accumulator); the browser verifies
   every link by hash and reads every node. The end state must be `FINISHED`, and the header's
   `extraData` must be the asserted send root.
3. **From `M` to `N`.** Nitro runs its own EIP-2935 contract at the same address, keyed by L2 block
   number with a ring of 393,168 blocks (measured today: `n mod 393168` holds block n's hash; the
   8,191 ring does not). That is about 27 hours of Arbitrum One and 11 hours of Robinhood Chain;
   assertions are 13,238 and 19,031 blocks apart, so one storage proof against `M`'s state root
   (from `M`'s header, verified by hash) reaches N, no hops. Its code hash is pinned separately from
   Ethereum's.
4. **Signed by the sequencer.** The feed message and its `signatureV2`, over
   `keccak("Arbitrum Nitro Feed:" ‖ chainId ‖ seq ‖ blockHash ‖ blockMetadata ‖ message fields)`
   (Nitro's feed format); `M` is the chain's first block plus `seq`. The signer must
   be a batch poster or sequencer of the SequencerInbox (`isBatchPoster`, `isSequencer`, run over
   proven state at `F`). The message signs the hash; `M`'s header is verified by it, and `N`'s hash
   read from `M`'s history. At most 3 hours old, as on the OP stack.

### What Willow serves

`willow_getBlockAnchor [blockNumber]` on the chain's route: the assertion (state words, parent,
accumulator, from its `AssertionCreated` log, and for a pending one its pending ancestors'
preimages), the L1 state witness (the Rollup's node slots of every assertion read), `M`'s header,
the storage proof at `M`; or, for a block newer than the newest finalized assertion, the newest
signed feed message Willow keeps.

Willow finds the assertion by reading the Rollup's `AssertionCreated` logs forward from the Ethereum
block the sequencer saw when it made `N` (Nitro blocks name it, `l1BlockNumber`), up to a day of
Ethereum blocks: none covering `N` can be older.

### Data and freshness

| Level | Reached | Download |
|---|---|---|
| Signed by the sequencer | under a second | about 95 KB (measured) |
| Posted | about an hour (assertion interval) | grows with the pending chain: 370 KB on Arbitrum One (142 pending assertions), 492 KB on Robinhood Chain (268), measured |
| Settled | about 6.4 days | about 120 KB |

### What is left to trust

Ethereum's sync committee. For Settled, BoLD and the chain's governance; Robinhood Chain's validators
are whitelisted (L2BEAT: not Stage 0), Arbitrum One's are permissionless. For Posted, the
asserter's bond on Arbitrum One (3,600 ETH), Robinhood Chain's validator list there (its bond is
1 ETH). For Signed, the sequencer.

---

## Failing closed

| Case | What the page does |
|---|---|
| Forged update (bad signature, under 342 signers, branch off) | **not verified**: "Willow's data to confirm block N on Ethereum failed verification" |
| Data for another chain (Holesky update, another chain's proof) | **not verified**: fails the signature (another `genesis_validators_root` and committee) or the state root |
| Data the page cannot follow (signed before its checkpoint, an update that hands over no committee) | not confirmed, said plainly; not a verdict on the block |
| A signed head over 10 minutes old, for a block not final yet | not confirmed: a head the validators signed and Ethereum later dropped cannot be replayed |
| A signature from a period that has not begun | **not verified** |
| Stale checkpoint (older than 30 days) | not confirmed, "this page's starting point on Ethereum is too old" |
| A fork the page does not know (Gloas) | not confirmed, "Ethereum's latest upgrade is newer than this page" |
| Proven hash differs from the proof's | **not verified**: "Ethereum has another block N", as the node mismatch did |
| Block newer than the signed head | asked again three times, 4 s apart, then "not confirmed yet" |
| Willow cannot serve the anchor (older than its history, service down) | not confirmed, with Willow's reason; never a fallback on Ethereum |
| L1 reorg | only "signed" can be affected, for at most 10 minutes; "final" reads Ethereum-final state |
| An L2 claim not by the pinned proposer, blacklisted, retired, paused, not respected or lost (`declined`) | the public-node fallback, in gold, never settled |
| An L2 claim with a rival anywhere back to a confirmed one (`disputed`) | the public-node fallback, in gold, never settled |
| A claim or signed block before the block, or claims left out (`not-covered`) | the public-node fallback, in gold, never settled |
| An L2 anchor Willow cannot send (stale Ethereum state, a signed block over 3 hours old, an upstream down) | the public-node fallback, in gold, never settled |
| A forged L2 anchor (another game, header, signature, key, history) | **not verified** |
