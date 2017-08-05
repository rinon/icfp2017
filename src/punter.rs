use std::collections::{HashSet, HashMap, VecDeque};

use protocol;

pub type PunterId = usize;
pub type SiteId = usize;
pub type RiverId = usize;

type EdgeMatrix = HashMap<SiteId, Vec<RiverId>>;
type ShortestPathsMap = HashMap<(SiteId, SiteId), usize>;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    input: Input,

    // The edges represented as an incidence matrix:
    // for every site, we keep a list of all its rivers
    edges: EdgeMatrix,
    shortest_paths: ShortestPathsMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    punter: PunterId,
    punters: PunterId,
    map: InputMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InputMap {
    sites: HashSet<Site>,
    rivers: Vec<River>,
    mines: HashSet<SiteId>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq)]
pub struct Site {
    id: SiteId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct River {
    source: SiteId,
    target: SiteId,

    #[serde(default)]
    owner: Option<PunterId>,
}

impl River {
    pub fn set_owner(&mut self, punter: PunterId) {
        self.owner = Some(punter)
    }

    pub fn other_side(&self, site: SiteId) -> SiteId {
        if site == self.source {
            self.target
        } else {
            self.source
        }
    }
}

impl Input {
    // Construct the incidence matrix for the graph
    fn compute_edges(&self) -> EdgeMatrix {
        let mut edges = EdgeMatrix::new();
        for (idx, ref river) in self.map.rivers.iter().enumerate() {
            edges.entry(river.source).or_insert_with(|| vec![]).push(idx);
            edges.entry(river.target).or_insert_with(|| vec![]).push(idx);
        }
        edges
    }

    fn compute_shortest_paths(&self, edges: &EdgeMatrix) -> ShortestPathsMap {
        // Since all edges have the same length of 1,
        // we can compute the shortest path using a simple
        // breadth-first search algorithm; for every mine,
        // we visit all sites exactly once.
        let mut shortest_paths = ShortestPathsMap::new();
        let mut que: VecDeque<SiteId> = VecDeque::with_capacity(self.map.sites.len());
        for mine in &self.map.sites {
            shortest_paths.insert((mine.id, mine.id), 0);
            que.clear();
            que.push_back(mine.id);
            while let Some(site) = que.pop_front() {
                let site_dist = shortest_paths[&(mine.id, site)];
                let last_idx = que.len();
                if let Some(ref neighbors) = edges.get(&site) {
                    // Collect all yet-unseen neighbors
                    // FIXME: would be nice if we wouldn't need to collect them
                    // in a new vector, but Rust won't let us
                    let new_neighbors = neighbors.iter()
                        .map(|ridx| (mine.id, self.map.rivers[*ridx].other_side(site)))
                        .filter(|nkey| !shortest_paths.contains_key(&nkey))
                        .collect::<Vec<_>>();

                    // Add all the new neigbors to the queue and the map
                    let new_dist = site_dist + 1;
                    shortest_paths.extend(new_neighbors.iter().map(|nkey| (*nkey, new_dist)));
                    que.extend(new_neighbors.iter().map(|nkey| nkey.1));
                }
            }
        }
        shortest_paths
    }
}

pub struct Punter {
    state: State,
}

impl Punter {
    pub fn new(input: Input) -> Punter {
        let edges = input.compute_edges();
        let shortest_paths = input.compute_shortest_paths(&edges);
        Punter {
            state: State {
                input: input,
                edges: edges,
                shortest_paths: shortest_paths,
            }
        }
    }

    // Immutable accessor for state
    pub fn state(&self) -> &State {
        &self.state
    }

    // Mutable accessor for state
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn ready(&self) -> PunterId {
        self.state.input.punter
    }

    // pub fn state(&self) -> State {
    //     self.state
    // }
}

pub fn handshake() -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: String::from("test"),
    }
}
