use std::collections::{HashSet, HashMap, VecDeque};
use rand::{thread_rng};
use std::time::{Duration, Instant};
use std::f64::NEG_INFINITY;
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

const SIMULATION_DEPTH: usize = 1000;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    punter: PunterId,
    punters: PunterId,
    map: InputMap,

    #[serde(default)]
    settings: Settings,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InputMap {
    sites: HashSet<Site>,
    rivers: Vec<River>,
    mines: HashSet<SiteId>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Settings {
    #[serde(default)]
    futures: bool,
    #[serde(default)]
    splurges: bool,
    #[serde(default)]
    options: bool,
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

    #[serde(default)]
    renter: Option<PunterId>,
}

impl River {
    pub fn add_owner(&mut self, punter: PunterId) {
        if self.owner.is_some() {
            assert!(self.renter.is_none());
            self.renter = Some(punter);
        } else {
            assert!(self.owner.is_none());
            self.owner = Some(punter);
        }
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
        println!("Mines {:#?}", input.map.mines);
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
                        self.add_move(punter, source, target);
                    }
                    protocol::Move::option {punter, source, target} => {
                        self.add_move(punter, source, target);
                    }
                    protocol::Move::splurge {punter, route} => {
                        let mut source = route[0];
                        for target in &route[1..] {
                            self.add_move(punter, source, *target);
                            source = *target;
                        }
                    }
                    protocol::Move::pass { punter: _ } => { }
                }
            }
        }
    }

    pub fn make_move(&self, begin_time: Instant, timeout: u8) -> protocol::Move {
        let play = match self.ai {
            PunterType::Random => self.move_random(),
            PunterType::MCTS   => self.move_mcts(begin_time, timeout),
        };

        protocol::Move::claim {
            punter: play.punter,
            source: play.source,
            target: play.target,
        }
    }

    pub fn compute_scores(&self, rivers: &Vec<River>, scores: &mut Vec<u64>) {
        let mut que: VecDeque<SiteId> = VecDeque::with_capacity(self.input.map.sites.len());
        let mut visited = HashSet::<SiteId>::with_capacity(self.input.map.sites.len());
        scores.resize(self.input.punters, 0);
        for punter in 0..self.input.punters {
            scores[punter] = 0;
            for mine in &self.input.map.mines {
                que.clear();
                que.push_back(*mine);
                visited.clear();
                while let Some(site) = que.pop_front() {
                    let dist = *self.shortest_paths.get(&(*mine, site)).unwrap_or(&0) as u64;
                    scores[punter] += dist*dist;
                    if let Some(ref neighbors) = self.edges.get(&site) {
                        for ridx in *neighbors {
                            let river = &rivers[*ridx];
                            if river.owner.map_or(true, |o| o != punter) &&
                               river.renter.map_or(true, |o| o != punter) {
                                continue;
                            }
                            let neighbor = river.other_side(site);
                            if !visited.contains(&neighbor) {
                                visited.insert(neighbor);
                                que.push_back(neighbor);
                            }
                        }
                    }
                }
            }
        }
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

    fn move_mcts(&self, begin_time: Instant, timeout: u8) -> Play {
        // FIXME: c seems wrong here: if we include 2 in the sqrt here,
        // then c here should be 1.0, since 1.4 is actually sqrt(2)
        let mut mcts = MCTS::new(self, 1.4);
        let mut iterations = 0;
        let mut game = InternalGameState::new(&self);
        while begin_time.elapsed() < Duration::from_millis((timeout as u64 * 1000) - 150) {
            game.reset_game();
            mcts.step(&mut game);
            iterations += 1;
            // println!("MCTS: {:#?}", mcts.root);
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

    #[allow(dead_code)]
    fn river(&self, id: RiverId) -> &River {
        &self.input.map.rivers[id]
    }

    fn river_mut(&mut self, id: RiverId) -> &mut River {
        &mut self.input.map.rivers[id]
    }

    fn add_move(&mut self, punter: PunterId, source: SiteId, target: SiteId) {
        let id = self.find_river(source, target).unwrap();
        self.river_mut(id).add_owner(punter);
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

impl GameAction for RiverId {}

#[derive(Debug, PartialEq)]
enum GameStatus {
    NotStarted,
    Playing,
    Finished,
}

struct InternalGameState<'a> {
    // Constant immutable state
    current_punter: PunterId,
    state: &'a Punter,

    // Per-game state
    status: GameStatus,
    rivers: Vec<River>,
    available_rivers: HashSet<RiverId>,
    scores: Vec<u64>,
}

impl<'a> InternalGameState<'a> {
    fn new(state: &'a Punter) -> InternalGameState {
        let available_rivers_len = (0..state.input.map.rivers.len())
                .filter(|x| state.input.map.rivers[*x].owner.is_none())
                .count();
        InternalGameState {
            current_punter: state.id(),
            state: state,
            status: GameStatus::NotStarted,
            rivers: Vec::with_capacity(state.input.map.rivers.len()),
            available_rivers: HashSet::with_capacity(available_rivers_len),
            scores: Vec::with_capacity(state.input.punters),
        }
    }

    fn reset_game(&mut self) {
        let input_rivers = &self.state.input.map.rivers;
        self.status = GameStatus::Playing;
        {
            self.rivers.clear();
            self.rivers.extend_from_slice(input_rivers);
        }
        {
            let available_rivers = (0..input_rivers.len())
                    .filter(|x| input_rivers[*x].owner.is_none());
            self.available_rivers.clear();
            self.available_rivers.extend(available_rivers);
        }
    }
}

impl<'a> Game<RiverId> for InternalGameState<'a> {
    fn available_actions (&self) -> &HashSet<RiverId> {
        assert!(self.status != GameStatus::NotStarted);
        &self.available_rivers
    }

    fn make_move(&mut self, river: RiverId) {
        assert!(self.status == GameStatus::Playing);
        self.rivers[river].add_owner(self.current_punter);
        self.available_rivers.remove(&river);
        self.current_punter = (self.current_punter + 1) % self.state.input.punters;
        if self.available_rivers.is_empty() {
            self.status = GameStatus::Finished;
        }
    }

    fn score(&mut self) -> f64 {
        assert!(self.status == GameStatus::Finished);
        self.state.compute_scores(&self.rivers, &mut self.scores);
        let my_score = self.scores[self.state.id()];
        let mut total: f64 = 0.;
        for (i, score) in self.scores.iter().enumerate() {
            if i != self.state.id() {
                total += my_score as f64 - *score as f64;
            }
        }
        total / (self.scores.len() - 1) as f64
    }
}


#[derive(Debug, Copy, Clone, PartialEq)]
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
    fn available_actions(&self) -> &HashSet<A>;

    fn make_move(&mut self, action: A);

    fn score(&mut self) -> f64;
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

    fn select_uct(&self, c: f64) -> Option<Rc<RefCell<MCTSNode<A>>>> {
        // We must be fully expanded, select a child based on UCT1
        if self.children.len() == 0 {
            return None;
        }
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
        Some(best_child.clone())
    }

    /// Select and return the "best" child
    fn select(&mut self, g: &mut Game<A>, c: f64) -> Option<Rc<RefCell<MCTSNode<A>>>> {
        let mut prev_rc = None;
        let mut node_rc = match self.status {
            NodeStatus::Done => None,
            NodeStatus::Expandable => None,
            NodeStatus::Expanded => self.select_uct(c),
        };
        while let Some(n) = node_rc {
            let node = n.borrow_mut();
            g.make_move(node.play.unwrap());
            prev_rc = Some(n.clone());
            node_rc = match node.status {
                NodeStatus::Done |
                NodeStatus::Expandable => None,
                NodeStatus::Expanded => node.select_uct(c),
            };
        }
        prev_rc
    }

    /// Expand and return a new child of this node.
    fn expand(&mut self, g: &Game<A>) -> Option<Rc<RefCell<MCTSNode<A>>>> {
        // println!("Expanding: {:#?}", self);
        let moves = g.available_actions();
        if moves.len() == 0 {
            self.status = NodeStatus::Done;
            return None;
        }

        let mut available_moves = moves.clone();
        // Remove the children's moves from the available set
        for child in &self.children {
            let child_move = child.borrow().play.unwrap();
            available_moves.remove(&child_move);
        }

        // Set status to fully expanded if expanding the last available move
        if available_moves.len() == 1 {
            self.status = NodeStatus::Expanded;
        }
        assert!(available_moves.len() > 0);

        let mut rng = thread_rng();
        let idx = rng.gen_range(0, available_moves.len());
        let new_child_move = available_moves.iter().nth(idx);
        let new_node = Rc::new(RefCell::new(MCTSNode::new(new_child_move.map(|x| *x))));
        self.children.push(new_node.clone());
        Some(new_node.clone())
    }

    /// Run a simulation from this node with the given game state. Currently a
    /// pure Monte Carlo simulation.
    fn simulate(&self, g: &mut Game<A>) -> f64 {
        let mut rng = thread_rng();
        for _ in 0..SIMULATION_DEPTH {
            let chosen_river = {
                let choices = g.available_actions();
                if choices.len() == 0 {
                    break;
                }
                let rand_idx = rng.gen_range(0, choices.len());
                *choices.iter().nth(rand_idx).unwrap()
            };
            g.make_move(chosen_river);
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
        let mut best_value = NEG_INFINITY;
        for child in &self.children {
            let child_ref = child.borrow();
            // TODO: shouldn't the value here be something like
            // child.score/child.count (average score)
            // SJC: not according to wikipedia...
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
    root: Rc<RefCell<MCTSNode<RiverId>>>,
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

    fn step(&mut self, game: &mut InternalGameState) {
        let leaf = self.root.borrow_mut().select(game, self.c).unwrap_or(self.root.clone());
        let new_child = leaf.borrow_mut().expand(game);
        if let Some(child) = new_child {
            let mut child_ref = child.borrow_mut();
            child_ref.parent = Some(Rc::downgrade(&leaf));
            let score = child_ref.simulate(game);
            child_ref.backpropagate(score);
        } else {
            // If we couldn't expand, that means the leaf is terminal
            // In that case, just backpropagate its score up
            let mut leaf_ref = leaf.borrow_mut();
            assert!(leaf_ref.status == NodeStatus::Done);
            let leaf_score = leaf_ref.score;
            leaf_ref.backpropagate(leaf_score);
        }
    }

    fn best_move(&self) -> Play {
        let river = &self.punter.river(self.root.borrow().best_move());
        Play::new(river, self.punter.id())
    }
}

pub fn handshake(name: String) -> protocol::HandshakeP {
    protocol::HandshakeP {
        me: name,
    }
}
