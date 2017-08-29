Consensus Gossip
===

This is a gossip protocol for agreeing on rumours that require a majority of nodes to vote
on them before they are considered "true" (I guess you could say this makes the rumours
well-founded).

The protocol is based on push-pull _anti-entropy_ (state reconciliation) gossip, quite similar
to the original anti-entropy protocol described in "the original gossip paper" out of Xerox:

https://pdfs.semanticscholar.org/49ed/15db181c74c7067ec01800fb5392411c868c.pdf

## Running It

The binary reads a CSV file with 3 columns, and outputs to another CSV file.

The 3 columns of the input CSV are:

* `n`: The number of nodes in the gossip group/section/network.
* `k`: The number of nodes that vote on a single rumour (k should be > n/2).
* `voting_steps`: The number of steps during which the `k` nodes cast their votes. Roughly
  `k/voting_steps` nodes vote for the rumour in each of the first `k` rounds.

The program will run a simulation for each `(n, k, voting_steps)` triple, and write a row to an
output CSV file.

The CLI program should be invoked as:

```
./gossip <input csv filename> <output csv filename>
```
