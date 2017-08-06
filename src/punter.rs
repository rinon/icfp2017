use std::collections::{HashSet, HashMap, VecDeque};
use rand::{thread_rng};
use std::time::{Duration, Instant};
use std::f64::NEG_INFINITY;
use std::iter::FromIterator;
use std::fmt::Debug;
use std::hash::Hash;
use rand::Rng;
use std::rc::Rc;
use std::rc::Weak;
use std::cell::RefCell;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

    ai: PunterType,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum PunterType {
    Random,
    MCTS,
}

impl Punter {
    pub fn new(input: Input, ai: PunterType) -> Punter {
        let edges = input.compute_edges();
        let shortest_paths = input.compute_shortest_paths(&edges);
        Punter {
            input: input,
            edges: edges,
            shortest_paths: shortest_paths,
            ai: ai,
        }
    }

    pub fn id(&self) -> PunterId {
        self.input.punter
    }

    /// Add the previous turns moves into the current state
    pub fn process_turn(&mut self, turn: protocol::TurnS) {
        if let protocol::TurnS::turn { moves } = turn {
            for m in moves {
                match m {
                    protocol::Move::claim {punter, source, target} => {
                        let id = self.find_river(source, target).unwrap();
                        self.river_mut(id).set_owner(punter);
                    }
                    protocol::Move::pass { punter: _ } => { }
                }
            }
        }
    }

    pub fn make_move(&self) -> protocol::Move {
        let play = match self.ai {
            PunterType::Random => self.move_random(),
            PunterType::MCTS   => self.move_mcts(),
        };

        protocol::Move::claim {
            punter: play.punter,
            source: play.source,
            target: play.target,
        }
    }

    pub fn score(&self, rivers: &Vec<River>) -> Vec<u64> {
        let mut scores = Vec::new();
        for punter in 0..self.input.punters {
            let mut score: u64 = 0;
            let mut que: VecDeque<SiteId> = VecDeque::with_capacity(self.input.map.sites.len());
            let mut visited = HashSet::<(SiteId, SiteId)>::new();
            for mine in &self.input.map.mines {
                que.clear();
                que.push_back(*mine);
                visited.clear();
                while let Some(site) = que.pop_front() {
                    let dist = *self.shortest_paths.get(&(*mine, site)).unwrap_or(&0) as u64;
                    score += dist*dist;
                    if let Some(ref neighbors) = self.edges.get(&site) {
                        for ridx in *neighbors {
                            let river = &rivers[*ridx];
                            if river.owner.map_or(true, |o| o != punter) {
                                continue;
                            }
                            let neighbor = river.other_side(site);
                            let neighbor_key = (*mine, neighbor);
                            if !visited.contains(&neighbor_key) {
                                visited.insert(neighbor_key);
                                que.push_back(neighbor);
                            }
                        }
                    }
                }
            }

            scores.push(score);
        }
        scores
    }

    ////////////////////////////////////////////////////////////////////////////
    // Implemented AIs
    ////////////////////////////////////////////////////////////////////////////
    fn move_random(&self) -> Play {
        let river_iter = self.input.map.rivers.iter();
        let mut rng = thread_rng();
        let choices = &river_iter.filter(|x| x.owner.is_none()).collect::<Vec<&River>>();
        let choice = rng.choose(choices).unwrap();
        Play {
            punter: self.id(),
            source: choice.source,
            target: choice.target,
        }
    }

    fn move_mcts(&self) -> Play {
        let now = Instant::now();
        // FIXME: c seems wrong here: if we include 2 in the sqrt here,
        // then c here should be 1.0, since 1.4 is actually sqrt(2)
        let mut mcts = MCTS::new(self, 1.4);
        let mut iterations = 0;
        while now.elapsed() < Duration::from_millis(900) {
            mcts.step();
            iterations += 1;
        }
        println!("Ran {} iterations", iterations);
        mcts.best_move()
    }

    ////////////////////////////////////////////////////////////////////////////
    // Utilities
    ////////////////////////////////////////////////////////////////////////////
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
struct Play {
    punter: PunterId,
    source: SiteId,
    target: SiteId,
}

impl Play {
    fn new(river: &River, punter: PunterId) -> Play {
        Play {
            punter: punter,
            source: river.source,
            target: river.target,
        }
    }
}

impl GameAction for Play {}

struct InternalGameState<'a> {
    rivers: Vec<River>,
    current_punter: PunterId,
    state: &'a Punter,
}

impl<'a> InternalGameState<'a> {
    fn new(state: &'a Punter) -> InternalGameState {
        InternalGameState {
            rivers: state.input.map.rivers.clone(),
            current_punter: state.id(),
            state: state,
        }
    }
}

impl<'a> Game<Play> for InternalGameState<'a> {
    fn available_actions(&self) -> Vec<Play> {
        self.rivers.iter()
            .filter(|x| x.owner.is_none())
            .map(|x| Play::new(x, self.current_punter)).collect()
    }

    fn make_move(&mut self, action: &Play) {
        self.rivers[self.state.find_river(action.source, action.target).unwrap()].set_owner(action.punter);
        self.current_punter = (self.current_punter + 1) % self.state.input.punters;
    }

    fn score(&self) -> f64 {
        let scores = self.state.score(&self.rivers);
        let my_score = scores[self.state.id()];
        let mut total: f64 = 0.;
        for (i, score) in scores.iter().enumerate() {
            if i != self.state.id() {
                total += my_score as f64 - *score as f64;
            }
        }
        total / (scores.len() - 1) as f64
    }
}


#[derive(Debug)]
enum NodeStatus {
    Done, Expanded, Expandable,
}

#[derive(Debug)]
struct MCTSNode<A> {
    play: Option<A>,
    children: Vec<Rc<RefCell<MCTSNode<A>>>>,
    parent: Option<Weak<RefCell<MCTSNode<A>>>>,
    status: NodeStatus,
    score: f64,
    count: f64,
}

pub trait GameAction: Debug+Clone+Copy+Eq+Hash {}

trait Game<A: GameAction> {
    fn available_actions(&self) -> Vec<A>;

    fn make_move(&mut self, action: &A);

    fn score(&self) -> f64;
}

impl<'a, A: GameAction> MCTSNode<A> {
    fn new(play: Option<A>) -> MCTSNode<A> {
        MCTSNode::<A> {
            play: play,
            children: Vec::new(),
            parent: None,
            status: NodeStatus::Expandable,
            score: 0.,
            count: 0.,
        }
    }

    fn is_leaf(&self) -> bool {
        self.children.len() == 0
    }

    fn select_best_child(&self, c: f64) -> Rc<RefCell<MCTSNode<A>>> {
        let mut best_value = NEG_INFINITY;
        let mut best_child = &self.children[0];
        for child_ref in &self.children {
            let child = child_ref.borrow();
            let value = child.score / child.count + c*(2.*self.count.ln()/child.count).sqrt();
            if value > best_value {
                best_value = value;
                best_child = child_ref;
            }
        }
        best_child.clone()
    }

    /// Select and return the "best" child
    fn select(&self, g: &mut Game<A>, c: f64) -> Option<Rc<RefCell<MCTSNode<A>>>> {
        if self.is_leaf() {
            return None;
        }

        let root_best_child = self.select_best_child(c);
        g.make_move(&root_best_child.borrow().play.unwrap());
        if root_best_child.borrow().is_leaf() {
            return Some(root_best_child.clone());
        }

        let mut node_rc = root_best_child.clone();
        loop {
            let node_best_child = node_rc.borrow().select_best_child(c);
            g.make_move(&node_best_child.borrow().play.unwrap());
            if node_best_child.borrow().is_leaf() {
                return Some(node_best_child.clone());
            }
            node_rc = node_best_child.clone();
        }
    }

    /// Expand and return a new child of this node.
    fn expand(&mut self, g: &Game<A>) -> Option<Rc<RefCell<MCTSNode<A>>>> {
        let moves = HashSet::from_iter(g.available_actions());
        // FIXME: since we're only expanding one leaf node,
        // isn't expanded_moves always empty???
        let expanded_moves = self.children.iter()
            .map(|ref child| child.borrow().play.unwrap())
            .collect::<HashSet<_>>();
        let available_moves = &moves - &expanded_moves;
        if available_moves.len() == 0 {
            self.status = NodeStatus::Done;
            return None;
        }
        let mut rng = thread_rng();
        let moves_vec = &available_moves.into_iter().collect::<Vec<A>>();
        let new_child_move = rng.choose(moves_vec);
        let new_node = Rc::new(RefCell::new(MCTSNode::new(new_child_move.map(|x| *x))));
        self.children.push(new_node.clone());
        Some(new_node.clone())
    }

    /// Run a simulation from this node with the given game state. Currently a
    /// pure Monte Carlo simulation.
    fn simulate(&self, g: &mut Game<A>) -> f64 {
        let mut rng = thread_rng();
        while g.available_actions().len() > 0 {
            let choices = g.available_actions();
            g.make_move(rng.choose(&choices).unwrap());
        }
        g.score()
    }

    fn backpropagate(&mut self, score: f64) {
        self.count += 1.;
        self.score += score;
        let mut cur_node = self.parent.clone();
        while let Some(weak_ref) = cur_node {
            let node = weak_ref.upgrade().unwrap();
            let mut node_ref = node.borrow_mut();
            node_ref.count += 1.;
            node_ref.score += score;
            cur_node = node_ref.parent.clone();
        }
    }

    fn best_move(&self) -> A {
        let mut best = self.play;
        let mut best_value = 0.;
        for child in &self.children {
            let child_ref = child.borrow();
            // TODO: shouldn't the value here be something like
            // child.score/child.count (average score)
            let child_value = child_ref.count;
            if child_value > best_value {
                best_value = child_value;
                best = child_ref.play;
            }
        }
        best.unwrap()
    }
}

#[derive(Debug)]
struct MCTS<'a> {
    punter: &'a Punter,
    root: Rc<RefCell<MCTSNode<Play>>>,
    c: f64,
}

impl<'a> MCTS<'a> {
    fn new(punter: &Punter, c: f64) -> MCTS {
        MCTS {
            punter: punter,
            root: Rc::new(RefCell::new(MCTSNode::new(None))),
            c: c,
        }
    }

    fn step(&mut self) {
        let mut game = InternalGameState::new(self.punter);
        let mut leaf = self.root.borrow().select(&mut game, self.c).unwrap_or(self.root.clone());
        let new_child = leaf.borrow_mut().expand(&mut game);
        if let Some(child) = new_child {
            let mut child_ref = child.borrow_mut();
            child_ref.parent = Some(Rc::downgrade(&leaf));
            let score = child_ref.simulate(&mut game);
            child_ref.backpropagate(score);
        }
    }

    fn best_move(&self) -> Play {
        self.root.borrow().best_move()
    }
}

pub fn handshake(name: String) -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: name,
    }
}
