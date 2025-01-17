use std::f32::MAX;
use super::BaseMap;
use std::mem;
use rayon::prelude::*;

#[allow(dead_code)]
/// Representation of a Dijkstra flow map.
/// map is a vector of floats, having a size equal to size_x * size_y (one per tile).
/// size_x and size_y are stored for overflow avoidance.
/// max_depth is the maximum number of iterations this search shall support.
pub struct DijkstraMap {
    pub map : Vec<f32>,
    size_x : i32,
    size_y : i32,
    max_depth : f32
}

/// Used internally when constructing maps in parallel
struct ParallelDm {
    map : Vec<f32>,
    max_depth : f32,
    starts : Vec<usize>
}

#[allow(dead_code)]
impl DijkstraMap {
    /// Construct a new Dijkstra map, ready to run. You must specify the map size, and link to an implementation
    /// of a BaseMap trait that can generate exits lists. It then builds the map, giving you a result.
    pub fn new(size_x : i32, size_y: i32, starts: &Vec<i32>, map: &BaseMap, max_depth : f32) -> DijkstraMap {
        let result : Vec<f32> = vec![MAX ; (size_x * size_y) as usize];
        let mut d = DijkstraMap{ map : result, size_x : size_x, size_y : size_y, max_depth : max_depth};
        DijkstraMap::build(&mut d, starts, map);
        return d;
    }

    /// Creates an empty Dijkstra map node.
    pub fn new_empty(size_x : i32, size_y: i32, max_depth : f32) -> DijkstraMap {
        let result : Vec<f32> = vec![MAX ; (size_x * size_y) as usize];
        let d = DijkstraMap{ map : result, size_x : size_x, size_y : size_y, max_depth : max_depth};
        return d;
    }

    /// Internal: add a node to the open list if it doesn't exceed max_depth, and isn't on the closed list.
    /// Adds the entry to the closed list.
    fn add_if_open(max_depth: f32, idx : i32, open_list : &mut Vec<(i32, f32)>, closed_list : &mut Vec<bool>, new_depth : f32) {
        if new_depth > max_depth { return; }
        if closed_list[idx as usize] { return; }

        closed_list[idx as usize] = true;
        open_list.push((idx, new_depth));
    }

    /// Clears the Dijkstra map. Uses a parallel for each for performance.
    pub fn clear(dm: &mut DijkstraMap) {
        dm.map.par_iter_mut().for_each(|x| *x = MAX);
    }

    /// Builds the Dijkstra map: iterate from each starting point, to each exit provided by BaseMap's
    /// exits implementation. Each step adds cost to the current depth, and is discarded if the new
    /// depth is further than the current depth.
    /// If you provide more starting points than you have CPUs, automatically branches to a parallel
    /// version.
    pub fn build(dm : &mut DijkstraMap, starts: &Vec<i32>, map: &BaseMap) {
        if starts.len() > rayon::current_num_threads() {
            DijkstraMap::build_parallel(dm, starts, map);
            return;
        }
        let mapsize : usize = (dm.size_x * dm.size_y) as usize;
        let mut open_list : Vec<(i32, f32)> = Vec::with_capacity(mapsize*2);
        let mut closed_list : Vec<bool> = vec![false ; mapsize];

        for start in starts.iter() {
            // Clearing vec in debug mode is stupidly slow, so we do it the hard way!
            unsafe { open_list.set_len(0); }
            // Zeroing the buffer is far too slow, so we're doing it the C way
            unsafe {
                std::ptr::write_bytes(closed_list.as_mut_ptr(), 0, closed_list.len() * mem::size_of::<bool>());
            }
            open_list.push((*start, 0.0));

            while !open_list.is_empty() {
                let last_idx = open_list.len()-1;
                let current_tile = open_list[last_idx];
                let tile_idx = current_tile.0;
                let depth = current_tile.1;
                unsafe { open_list.set_len(last_idx); }

                if dm.map[tile_idx as usize] > depth {
                    dm.map[tile_idx as usize] = depth;

                    let exits = map.get_available_exits(tile_idx);
                    for exit in exits.iter() {
                        DijkstraMap::add_if_open(dm.max_depth, exit.0, &mut open_list, &mut closed_list, depth + exit.1);
                    }
                }
            }
        }
    }    

    /// Implementation of Parallel Dijkstra.
    fn build_parallel(dm : &mut DijkstraMap, starts: &Vec<i32>, map: &BaseMap) {
        let mapsize : usize = (dm.size_x * dm.size_y) as usize;
        let mut layers : Vec<ParallelDm> = Vec::with_capacity(starts.len());
        for start_chunk in starts.chunks(rayon::current_num_threads()) {
            let mut layer = ParallelDm{
                map : vec![MAX;mapsize],
                max_depth : dm.max_depth,
                starts : Vec::new()
            };
            for s in start_chunk.iter() { layer.starts.push(*s as usize); }
            layers.push(layer);
        }

        let mut exits : Vec<Vec<(i32, f32)>> = Vec::with_capacity(mapsize);
        for idx in 0 .. mapsize as i32 {
            exits.push(map.get_available_exits(idx));
        }

        // Run each map in parallel
        layers.par_iter_mut().for_each(|l| {
            let mut open_list : Vec<(i32, f32)> = Vec::with_capacity(mapsize*2);
            let mut closed_list : Vec<bool> = vec![false; mapsize];

            for start in l.starts.iter() {
                open_list.push((*start as i32, 0.0));

                while !open_list.is_empty() {
                    let last_idx = open_list.len()-1;
                    let current_tile = open_list[last_idx];
                    let tile_idx = current_tile.0;
                    let depth = current_tile.1;
                    unsafe { open_list.set_len(last_idx); }

                    if l.map[tile_idx as usize] > depth {
                        l.map[tile_idx as usize] = depth;

                        let exits = &exits[tile_idx as usize];
                        for exit in exits.iter() {
                            DijkstraMap::add_if_open(l.max_depth, exit.0, &mut open_list, &mut closed_list, depth + exit.1);
                        }
                    }
                }
            }
        });

        // Recombine down to a single result
        for l in layers.iter() {
            for i in 0..mapsize {
                dm.map[i] = f32::min(dm.map[i], l.map[i]);
            }
        }
    }

    /// Helper for traversing maps as path-finding. Provides the index of the lowest available
    /// exit from the specified position index, or None if there isn't one.
    /// You would use this for pathing TOWARDS a starting node.
    pub fn find_lowest_exit(dm : &DijkstraMap, position : i32, map: &BaseMap) -> Option<i32> {
        let mut exits = map.get_available_exits(position);

        for exit in exits.iter_mut() {
            exit.1 = dm.map[exit.0 as usize] as f32;
        }

        if exits.is_empty() { return None; }
        exits.par_sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());

        return Some(exits[0].0);
    }

    /// Helper for traversing maps as path-finding. Provides the index of the highest available
    /// exit from the specified position index, or None if there isn't one.
    /// You would use this for pathing AWAY from a starting node, for example if you are running
    /// away.
    pub fn find_highest_exit(dm : &DijkstraMap, position : i32, map: &BaseMap) -> Option<i32> {
        let mut exits = map.get_available_exits(position);

        for exit in exits.iter_mut() {
            exit.1 = dm.map[exit.0 as usize] as f32;
        }

        if exits.is_empty() { return None; }
        exits.par_sort_by(|a,b| a.1.partial_cmp(&b.1).unwrap());

        return Some(exits[0].0);
    }
}

