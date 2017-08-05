use std::collections::{HashSet, HashMap, VecDeque};
use rand::{thread_rng, sample};

use protocol;

pub type PunterId = usize;
pub type SiteId = usize;
pub type RiverId = usize;

type EdgeMatrix = HashMap<SiteId, Vec<RiverId>>;
type ShortestPathsMap = HashMap<(SiteId, SiteId), usize>;

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
            assert!(site == self.target);
            self.source
        }
    }
}

impl Input {
    // Helper function used by the binary search and sort
    fn river_other_side(&self, river: RiverId, site: SiteId) -> SiteId {
        self.map.rivers[river].other_side(site)
    }

    // Construct the incidence matrix for the graph
    fn compute_edges(&self) -> EdgeMatrix {
        let mut edges = EdgeMatrix::new();
        for (idx, ref river) in self.map.rivers.iter().enumerate() {
            edges.entry(river.source).or_insert_with(|| vec![]).push(idx);
            edges.entry(river.target).or_insert_with(|| vec![]).push(idx);
        }
        // Sort the edges of each site by the id of the other side
        for (site, ref mut site_edges) in edges.iter_mut() {
            site_edges.sort_by_key(|river| self.river_other_side(*river, *site));
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
        for mine in &self.map.mines {
            shortest_paths.insert((*mine, *mine), 0);
            que.clear();
            que.push_back(*mine);
            while let Some(site) = que.pop_front() {
                let site_dist = shortest_paths[&(*mine, site)];
                if let Some(ref neighbors) = edges.get(&site) {
                    for ridx in *neighbors {
                        let river = &self.map.rivers[*ridx];
                        let neighbor = river.other_side(site);
                        let neighbor_key = (*mine, neighbor);
                        if !shortest_paths.contains_key(&neighbor_key) {
                            shortest_paths.insert(neighbor_key, site_dist + 1);
                            que.push_back(neighbor);
                        }
                    }
                }
            }
        }
        shortest_paths
    }
}

// This structure contains the entire state of a punter
#[derive(Serialize, Deserialize, Debug)]
pub struct Punter {
    input: Input,

    // The edges represented as an incidence matrix:
    // for every site, we keep a list of all its rivers
    // The list of edges is sorted in increasing order of
    // river.other_side(site)
    edges: EdgeMatrix,
    shortest_paths: ShortestPathsMap,
}

impl Punter {
    pub fn new(input: Input) -> Punter {
        let edges = input.compute_edges();
        let shortest_paths = input.compute_shortest_paths(&edges);
        Punter {
            input: input,
            edges: edges,
            shortest_paths: shortest_paths,
        }
    }

    pub fn id(&self) -> PunterId {
        self.input.punter
    }

    pub fn process_turn(&mut self, turn: protocol::TurnS) {
        if let protocol::TurnS::turn { moves } = turn {
            for m in moves {
                match m {
                    protocol::Move::claim {punter, source, target} => {
                        let id = self.find_river(source, target).unwrap();
                        self.river_mut(id).set_owner(punter);
                    }
                    protocol::Move::pass { punter } => { }
                }
            }
        }
    }

    // Choose a random valid move, for now
    pub fn make_move(&self) -> protocol::Move {
        let river_iter = self.input.map.rivers.iter();
        let mut rng = thread_rng();
        let choice = &sample(&mut rng, river_iter.filter(|x| x.owner.is_none()), 1)[0];

        protocol::Move::claim {
            punter: self.input.punter,
            source: choice.source,
            target: choice.target,
        }
    }

    fn find_river(&self, source: SiteId, target: SiteId) -> Option<RiverId> {
        self.edges.get(&source).and_then(|ref rivers| {
            rivers.binary_search_by_key(&target, |river| self.input.river_other_side(*river, source))
                  .map(|idx| rivers[idx])
                  .ok() // Result -> Option transform
        })
    }

    fn river(&self, id: RiverId) -> &River {
        &self.input.map.rivers[id]
    }

    fn river_mut(&mut self, id: RiverId) -> &mut River {
        &mut self.input.map.rivers[id]
    }
}

pub fn handshake(name: String) -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: name,
    }
}
