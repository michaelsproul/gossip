Consensus Gossip
===

This is a gossip protocol for agreeing on rumours that require a majority of nodes to vote
on them before they are considered "true" (I guess you could say this makes the rumours
well-founded).

The protocol is based on push-pull _anti-entropy_ (state reconciliation) gossip, quite similar
to the anti-entropy protocol described in "the original gossip paper" out of Xerox:

https://pdfs.semanticscholar.org/49ed/15db181c74c7067ec01800fb5392411c868c.pdf

## Summary

In our model, each node keeps track of a set of rumours. For the sake of
simplicity each rumour is identified by a `usize` integer. Nodes cast votes for
rumours, and keep track of the set of nodes who have voted for each rumour. A
node will consider a rumour "confirmed" (or valid) once it has collected votes
for that rumour from a majority of nodes. It's assumed that a real
implementation would use signatures to make votes unforgeable.

The gossip protocol proceeds in "rounds" of approximately fixed length, perhaps 1 second.
In each round, every node *x* does the following:

* It chooses a random peer *y* to gossip with. For now it is assumed that every node
  is connected to every other.
* *x* and *y* engage in a two-way exchange of messages during which:
    + *x* sends a summary of its set of rumours to *y* which allows *y* to
      compute which votes *x* has that it does not (this is the **push** in push-pull gossip).
    + *y* sends a summary of its set of rumours to *x* which allows *x* to
      compute which votes *y* has that it does not (this is the **pull** in push-pull gossip).
    + *x* requests missing votes from *y* for any rumours that do not yet have a quorum of
      votes at *x* (and vice versa).

In the simulation, the details of the two-way exchange (i.e. _set reconciliation_) are elided – the exchange happens perfectly and atomically as one operation (ha!). In reality, an efficient set reconciliation or summary system should be used ([invertible bloom lookup tables][iblts] seem cool).

The simulation deals with the spread of just a single rumour (vote ID=#0), and will terminate once every node has a quorum of votes for this rumour.

In the literature on gossip protocols it is well established that a single rumour will spread in `O(log n)` rounds, with approximately `O(n log n)` messages being sent in the process. A naive analysis of consensus gossip might suggest that each node's _vote_ should count as a rumour, and that therefore `O(n² log n)` messages would be required, but because votes on a rumour are often cast at a similar point in time, it seems like "consensus" is reached after `O(log n)` rounds, and `O(n log n)` individual state exchanges (although we should _measure_ the size of these state exchanges to make sure they aren't huge).

[iblts]: https://www.reddit.com/r/btc/comments/43iup7/mike_hearn_implemented_a_test_version_of_thin/czirz2p/

## Running It

The binary reads a CSV file with 3 columns, and outputs to another CSV file.

The 3 columns of the input CSV are:

* `n`: The number of nodes in the gossip group/section/network.
* `k`: The number of nodes that vote on a single rumour (k should be > n/2).
* `voting_steps`: The number of steps during which the `k` nodes cast their votes. Roughly
  `k / voting_steps` nodes vote for the rumour in each of the first `voting_steps` rounds.

The program will run a simulation for each `(n, k, voting_steps)` triple, and write a row to an
output CSV file.

The CLI program should be invoked as:

```
./gossip <input csv filename> <output csv filename>
```
