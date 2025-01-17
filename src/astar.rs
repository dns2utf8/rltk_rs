use super::BaseMap;
use std::collections::HashMap;

#[allow(dead_code)]
/// Bail out if the A* search exceeds this many steps.
const MAX_DIRECT_PATH_CHECK : f32 = 2048.0;

#[allow(dead_code)]
/// Bail out if the A* search exceeds this many steps.
const MAX_ASTAR_STEPS :i32 = 2048;

#[allow(dead_code)]
/// Request an A-Star search. The start and end are specified as index numbers (compatible with your
/// BaseMap implementation), and it requires access to your map so as to call distance and exit
/// determinations.
pub fn a_star_search(start:i32, end:i32, map: &mut BaseMap) -> NavigationPath {
    let mut searcher = AStar::new(start, end);
    return searcher.search(map);
}

#[allow(dead_code)]
#[derive(Clone)]
/// Holds the result of an A-Star navigation query.
/// `destination` is the index of the target tile.
/// `success` is true if it reached the target, false otherwise.
/// `steps` is a vector of each step towards the target, *including* the starting position.
pub struct NavigationPath {
    pub destination: i32,
    pub success: bool,
    pub steps: Vec<i32>
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
/// Node is an internal step inside the A-Star path (not exposed/public). Idx is the current cell,
/// f is the total cost, g the neighbor cost, and h the heuristic cost.
/// See: https://en.wikipedia.org/wiki/A*_search_algorithm
struct Node {
    idx : i32,
    f : f32,
    g : f32,
    h : f32
}

#[allow(dead_code)]
impl NavigationPath {
    /// Makes a new (empty) NavigationPath
    pub fn new() -> NavigationPath {
        return NavigationPath{destination:0, success:false, steps: Vec::new() };
    }
}

#[allow(dead_code)]
/// Private structure for calculating an A-Star navigation path.
struct AStar {
    start: i32,
    end : i32,
    open_list: Vec<Node>,
    closed_list: HashMap<i32, f32>,
    parents: HashMap<i32, i32>,
    step_counter: i32
}

impl AStar {
    /// Creates a new path, with specified starting and ending indices.
    fn new(start : i32, end: i32) -> AStar {
        let mut open_list : Vec<Node> = Vec::new();
        open_list.push(Node{ idx : start, f: 0.0, g: 0.0, h: 0.0 });

        return AStar{ start: start, 
            end : end, 
            open_list : open_list, 
            parents: HashMap::new(), 
            closed_list: HashMap::new(),
            step_counter: 0
        };
    }

    /// Wrapper to the BaseMap's distance function.
    fn distance_to_end(&self, idx :i32, map: &BaseMap) -> f32 {
        return map.get_pathing_distance(idx, self.end);
    }

    /// Adds a successor; if we're at the end, marks success.
    fn add_successor(&mut self, q:Node, idx:i32, cost:f32, map: &BaseMap) -> bool {
        // Did we reach our goal?
        if idx == self.end {
            self.parents.insert(idx, q.idx);
            return true;
        } else {
            let distance = self.distance_to_end(idx, map);
            let s = Node{ idx:idx, f:distance + cost, g:cost, h:distance };

            // If a node with the same position as successor is in the open list with a lower f, skip add
            let mut should_add = true;
            for e in self.open_list.iter() {
                if e.f < s.f && e.idx == idx { 
                    should_add = false; 
                }
            }

            // If a node with the same position as successor is in the closed list, with a lower f, skip add
            if should_add && self.closed_list.contains_key(&idx) && self.closed_list[&idx] < s.f { 
                should_add = false; 
            }

            if should_add {
                self.open_list.push(s);
                self.parents.insert(idx, q.idx);
            }

            return false;
        }
    }

    /// Helper function to unwrap a path once we've found the end-point.
    fn found_it(&self) -> NavigationPath {
        let mut result = NavigationPath::new();
        result.success = true;
        result.destination = self.end;

        result.steps.push(self.end);
        let mut current = self.end;
        while current != self.start {
            let parent = self.parents[&current];
            result.steps.insert(0, parent); 
            current = parent;
        }

        return result;
    }

    /// Performs an A-Star search
    fn search(&mut self, map: &BaseMap) -> NavigationPath {
        let result = NavigationPath::new();
        while self.open_list.len() != 0 && self.step_counter < MAX_ASTAR_STEPS {
            self.step_counter += 1;

            // Pop Q off of the list
            let q = self.open_list[0];
            self.open_list.remove(0);

            // Generate successors
            let successors = map.get_available_exits(q.idx);

            for s in successors.iter() {
                if self.add_successor(q, s.0, s.1 + q.f, map) {
                    let success = self.found_it();
                    return success;
                }
            }

            if self.closed_list.contains_key(&q.idx) { self.closed_list.remove(&q.idx); }
            self.closed_list.insert(q.idx, q.f);
            self.open_list.sort_by(|a,b| a.f.partial_cmp(&b.f).unwrap());            
        }
        return result;
    }

}