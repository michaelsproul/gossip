extern crate csv;
extern crate rand;
#[macro_use]
extern crate serde_derive;

use rand::{weak_rng, Rng};
use std::io;
use std::fs::File;
use std::env;
use std::error::Error;
use std::collections::{BTreeMap, BTreeSet};

/// Parameters to run the simulation with.
#[derive(Debug, Deserialize, Serialize)]
struct Params {
    n: usize,
    k: usize,
    voting_steps: usize,
}

/// Result to write to the output CSV.
#[derive(Debug, Deserialize, Serialize)]
struct SimulationResult {
    n: usize,
    k: usize,
    voting_steps: usize,
    num_iterations: usize,
    num_exchanges: usize,
    average_votes_held: f64,
}

type VoteMap = BTreeMap<usize, VoteInfo>;
type VoteDiff = BTreeMap<usize, BTreeSet<usize>>;

#[derive(Clone, Debug)]
struct Node {
    /// Our node ID.
    id: usize,
    /// Total number of nodes in our universe (constant for now).
    num_nodes: usize,
    /// Map from vote ID to number of voters.
    votes: VoteMap,
}

impl Node {
    fn new(id: usize, num_nodes: usize) -> Self {
        Node {
            id,
            num_nodes,
            votes: VoteMap::new(),
        }
    }
}

#[derive(Clone, Default, Debug)]
struct VoteInfo {
    /// All nodes that voted for this proposal.
    voters: BTreeSet<usize>,
}

impl Node {
    fn vote_for(&mut self, vote_id: usize) {
        let our_id = self.id;
        self.votes
            .entry(vote_id)
            .or_insert_with(VoteInfo::default)
            .voters
            .insert(our_id);
    }

    fn has_voted_for(&self, vote_id: usize) -> bool {
        self.votes
            .get(&vote_id)
            .map(|vote_info| vote_info.voters.contains(&self.id))
            .unwrap_or(false)
    }

    fn has_quorum_for(&self, vote_id: usize) -> bool {
        self.votes
            .get(&vote_id)
            .map(|vote_info| has_quorum(vote_info.voters.len(), self.num_nodes))
            .unwrap_or(false)
    }

    fn apply_diff(&mut self, diff: VoteDiff) {
        for (vote_id, voters) in diff {
            self.votes
                .entry(vote_id)
                .or_insert_with(VoteInfo::default)
                .voters
                .extend(voters);
        }
    }
}

fn has_quorum(k: usize, n: usize) -> bool {
    2 * k > n
}

fn choose_partner<R: Rng>(our_id: usize, n: usize, rng: &mut R) -> usize {
    loop {
        let p = rng.gen_range(0, n);
        if p != our_id {
            return p;
        }
    }
}

// Gossip sent n1 => n2.
fn compute_push_gossip(n1: &Node, n2: &Node) -> Option<VoteDiff> {
    let diff: VoteDiff = n1.votes
        .iter()
        .filter_map(|(&vote_id, vote_info)| {
            // If n2 doesn't have a quorum for one of n1's votes, it gets n1's voters sent to it.
            if !n2.has_quorum_for(vote_id) {
                let new_voters = if let Some(existing_info) = n2.votes.get(&vote_id) {
                    &vote_info.voters - &existing_info.voters
                } else {
                    vote_info.voters.clone()
                };
                if !new_voters.is_empty() {
                    return Some((vote_id, new_voters));
                }
            }
            None
        })
        .collect();

    if !diff.is_empty() {
        Some(diff)
    } else {
        None
    }
}

fn compute_push_pull_gossip(n1: &Node, n2: &Node) -> (Option<VoteDiff>, Option<VoteDiff>) {
    (compute_push_gossip(n2, n1), compute_push_gossip(n1, n2))
}

fn add_updates(updates: &mut BTreeMap<usize, VoteDiff>, node: usize, diff: VoteDiff) {
    let existing_diff = updates.entry(node)
        .or_insert_with(VoteDiff::new);

    for (vote_id, voters) in diff {
        let existing_voters = existing_diff.entry(vote_id).or_insert_with(BTreeSet::new);
        existing_voters.extend(voters);
    }
}

fn construct_voting_schedule(k: usize, voting_steps: usize) -> BTreeMap<usize, usize> {
    let per_step = k / voting_steps;

    (0..voting_steps).map(|i| {
        let num_voters = if i == voting_steps - 1 {
            // Take the remaining votes.
            k - ((voting_steps - 1) * per_step)
        } else {
            per_step
        };
        (i, num_voters)
    }).collect()
}

fn run_simulation<R: Rng>(params: &Params, rng: &mut R) -> SimulationResult {
    let n = params.n;
    let k = params.k;

    let mut nodes: Vec<Node> = (0..n).map(|i| Node::new(i, n)).collect();

    // At each voting step, have roughly an even portion of k vote.
    let voting_schedule = construct_voting_schedule(k, params.voting_steps);

    // Statistics.
    let mut num_iterations = 0;
    let mut num_exchanges = 0;

    // Keep running while any node lacks a quorum.
    while !nodes.iter().all(|node| node.has_quorum_for(0)) {
        // Get nodes to vote according to the schedule.
        if let Some(&num_voters) = voting_schedule.get(&num_iterations) {
            for node in nodes.iter_mut().filter(|node| !node.has_voted_for(0)).take(num_voters) {
                node.vote_for(0);
            }
        }

        // Each node chooses a random gossip partner.
        // Push-pull, so everyone contacts someone and solicits updates.

        // Map from node ID to vote ID to voter set.
        // All updates for this iteration go into this container and get applied atomically
        // at the end of the iteration (removes the need to index mutably into the vec twice).
        let mut updates = BTreeMap::new();

        for (node_id, node) in nodes.iter().enumerate() {
            let partner_id = choose_partner(node_id, n, rng);
            let partner = &nodes[partner_id];

            let (our_updates, their_updates) = compute_push_pull_gossip(node, partner);

            if let Some(our_updates) = our_updates {
                add_updates(&mut updates, node_id, our_updates);
                num_exchanges += 1;
            }

            if let Some(their_updates) = their_updates {
                add_updates(&mut updates, partner_id, their_updates);
                num_exchanges += 1;
            }
        }

        // Apply all those updates.
        for (node_id, diff) in updates {
            nodes[node_id].apply_diff(diff);
        }

        num_iterations += 1;
    }

    // Compute stats.
    let total_votes_collected: usize = nodes.iter().map(|node| node.votes[&0].voters.len()).sum();
    let average_votes_held = total_votes_collected as f64 / n as f64;

    SimulationResult {
        n,
        k,
        voting_steps: params.voting_steps,
        num_iterations,
        num_exchanges,
        average_votes_held,
    }
}

fn read_params(filename: &str) -> io::Result<Vec<Params>> {
    let mut all_params = vec![];
    let f = File::open(filename)?;
    let mut csv_reader = csv::Reader::from_reader(f);

    for row in csv_reader.deserialize() {
        let params: Params = row?;
        all_params.push(params);
    }

    Ok(all_params)
}

fn write_results(output_file: &str, results: Vec<SimulationResult>) -> io::Result<()> {
    let mut writer = csv::Writer::from_path(output_file)?;

    for result in results {
        writer.serialize(result)?;
    }

    Ok(())
}

fn main_with_result() -> Result<(), Box<Error>> {
    let args: Vec<_> = env::args().collect();

    if args.len() != 3 {
        println!("Usage: ./gossip <input csv> <output csv>");
        return Err(From::from(format!("incorrect CLI args: {:?}", args)));
    }

    let input_file = &args[1];
    let output_file = &args[2];

    let all_params = read_params(input_file)?;
    let mut rng = weak_rng();

    let results: Vec<_> = all_params.iter()
        .map(|params| {
            run_simulation(params, &mut rng)
        })
        .collect();

    write_results(output_file, results)?;

    Ok(())
}

fn main() {
    main_with_result().unwrap()
}
