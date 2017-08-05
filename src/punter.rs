use std::collections::{HashSet, HashMap, VecDeque};

use protocol;

pub type PunterId = usize;
pub type SiteId = usize;
pub type RiverId = usize;

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    input: Input,

    // The edges represented as an incidence matrix:
    // for every site, we keep a list of all its rivers
    edges: HashMap<SiteId, Vec<RiverId>>,
    shortest_paths: HashMap<(SiteId, SiteId), usize>,
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

impl State {
    // Construct the incidence matrix for the graph
    pub fn compute_edges(&mut self) {
        if !self.edges.is_empty() {
            return
        }
        for (idx, ref river) in self.input.map.rivers.iter().enumerate() {
            self.edges.entry(river.source).or_insert_with(|| vec![]).push(idx);
            self.edges.entry(river.target).or_insert_with(|| vec![]).push(idx);
        }
    }

    pub fn compute_shortest_paths(&mut self) {
        if !self.shortest_paths.is_empty() {
            return
        }
        self.compute_edges();
        // Since all edges have the same length of 1,
        // we can compute the shortest path using a simple
        // breadth-first search algorithm; for every mine,
        // we visit all sites exactly once.
        let mut que: VecDeque<SiteId> = VecDeque::with_capacity(self.input.map.sites.len());
        for mine in &self.input.map.sites {
            self.shortest_paths.insert((mine.id, mine.id), 0);
            que.clear();
            que.push_back(mine.id);
            while let Some(site) = que.pop_front() {
                let site_dist = self.shortest_paths[&(mine.id, site)];
                if let Some(ref neighbors) = self.edges.get(&site) {
                    for ridx in *neighbors {
                        let river = &self.input.map.rivers[*ridx];
                        let neighbor = river.other_side(site);
                        let neighbor_key = (mine.id, neighbor);
                        if !self.shortest_paths.contains_key(&neighbor_key) {
                            self.shortest_paths.insert(neighbor_key, site_dist + 1);
                            que.push_back(neighbor);
                        }
                    }
                }
            }
        }
    }
}

pub struct Punter {
    state: State,
}

impl Punter {
    pub fn new(input: Input) -> Punter {
        Punter {
            state: State {
                input: input,
                edges: Default::default(),
                shortest_paths: Default::default(),
            }
        }
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
