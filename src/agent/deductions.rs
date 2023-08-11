use crate::TilePos;

use super::*;
use itertools::Itertools;
use std::collections::HashMap;

fn subsets(elts: &[TilePos], max_size: usize) -> Vec<Vec<&TilePos>> {
    (2..=max_size)
        .flat_map(|k| elts.iter().combinations(k))
        .collect()
}

fn set_difference(this: &Vec<TilePos>, other: &Vec<TilePos>) -> Vec<TilePos> {
    this.iter()
        .filter(|x| !other.contains(x))
        .cloned()
        .collect()
}

fn max_in_subset(
    tiles: &Vec<TilePos>,
    max_bombs: &mut HashMap<Vec<TilePos>, u8>,
) -> u8 {
    let mut smallest_max = if let Some(&max) = max_bombs.get(tiles) {
        max
    } else {
        tiles.len() as u8
    };
    // base case: we can't break down a group of 2 or 1 tiles into useful
    // subsets
    if tiles.len() <= 2 {
        return smallest_max;
    }
    // recursive case: use information about any subsets to further narrow the
    // bounds
    let max_size = tiles.len().saturating_sub(1);
    for subset in subsets(&tiles, max_size) {
        let subset = subset.iter().copied().copied().collect_vec();
        if let Some(&sub_max) = max_bombs.get(&subset) {
            let rest = set_difference(tiles, &subset);
            let tiles_max = sub_max + max_in_subset(&rest, max_bombs);
            if tiles_max < smallest_max {
                smallest_max = tiles_max;
            }
        }
    }
    if let Some(&max) = max_bombs.get(tiles) {
        if smallest_max < max {
            max_bombs.insert(tiles.clone(), smallest_max);
        }
    } else {
        max_bombs.insert(tiles.clone(), smallest_max);
    };
    smallest_max
}

fn min_in_subset(
    tiles: &Vec<TilePos>,
    min_bombs: &mut HashMap<Vec<TilePos>, u8>,
) -> u8 {
    let mut biggest_min = if let Some(&min) = min_bombs.get(tiles) {
        min
    } else {
        0
    };
    // base case: we can't break down a group of 2 or 1 tiles into useful
    // subsets
    if tiles.len() <= 2 {
        return biggest_min;
    }
    // recursive case: use information about any subsets to further narrow the
    // bounds
    let max_size = tiles.len().saturating_sub(1);
    for subset in subsets(&tiles, max_size) {
        let subset = subset.iter().cloned().cloned().collect_vec();
        if let Some(&sub_min) = min_bombs.get(&subset) {
            let rest = set_difference(tiles, &subset);
            let tiles_min = sub_min + min_in_subset(&rest, min_bombs);
            if tiles_min > biggest_min {
                biggest_min = tiles_min;
            }
        }
    }
    if let Some(&min) = min_bombs.get(tiles) {
        if biggest_min > min {
            min_bombs.insert(tiles.clone(), biggest_min);
        }
    } else {
        min_bombs.insert(tiles.clone(), biggest_min);
    };
    biggest_min
}

pub fn get_subset_bounds(
    board: &Board,
) -> (HashMap<Vec<TilePos>, u8>, HashMap<Vec<TilePos>, u8>) {
    let mut min_bombs: HashMap<Vec<TilePos>, u8> = HashMap::new();
    let mut max_bombs: HashMap<Vec<TilePos>, u8> = HashMap::new();
    for _ in 0..3 {
        update_subset_bounds(board, &mut min_bombs, &mut max_bombs);
    }
    (min_bombs, max_bombs)
}

fn update_subset_bounds(
    board: &Board,
    min_bombs: &mut HashMap<Vec<TilePos>, u8>,
    max_bombs: &mut HashMap<Vec<TilePos>, u8>,
) {
    for col in 0..board.width() {
        for row in 0..board.height() {
            let pos = TilePos { col, row };
            if let TileState::UncoveredSafe(n) = board.tile_state(pos) {
                let n = n - num_bombs_around(board, pos);
                let covered = covered_neighbours(board, pos);
                let num_covered = covered.len();
                for subset in subsets(&covered, num_covered) {
                    let subset = subset.iter().cloned().cloned().collect_vec();
                    // rule 1: at most n bombs in all subsets around the tile
                    if subset.len() > n as usize {
                        if let Some(max) = max_bombs.get(&subset) {
                            if n < *max {
                                max_bombs.insert(subset.clone(), n);
                            }
                        } else {
                            max_bombs.insert(subset.clone(), n);
                        }
                    }
                    // rule 2: if we exclude tiles with a max of k bombs there
                    // are at least n - k bombs in the remaining subset
                    let rest = set_difference(&covered.clone(), &subset);
                    let max_omitted = max_in_subset(&rest, max_bombs);
                    if n > max_omitted {
                        if let Some(min) = min_bombs.get(&subset) {
                            if n - max_omitted > *min {
                                min_bombs
                                    .insert(subset.clone(), n - max_omitted);
                            }
                        } else {
                            min_bombs.insert(subset.clone(), n - max_omitted);
                        }
                    }
                    // rule 3: if we exclude tiles with a min of k bombs there
                    // are at most n - k bombs in the remaining subset
                    let min_omitted = min_in_subset(&rest, min_bombs);
                    if n > min_omitted {
                        if let Some(max) = max_bombs.get(&subset) {
                            if n - min_omitted < *max {
                                max_bombs.insert(subset, n - min_omitted);
                            }
                        } else {
                            max_bombs.insert(subset, n - min_omitted);
                        }
                    }
                }
            }
        }
    }
}

pub fn get_non_trivial_actions(board: &Board) -> Vec<Action> {
    let mut output = vec![];
    let (mut min_bombs, mut max_bombs) = get_subset_bounds(board);
    // check each uncovered tile to see if we have helpful adjacent subsets
    (0..board.width())
        .cartesian_product(0..board.height())
        .filter_map(|(col, row)| {
            let pos = TilePos { col, row };
            if let TileState::UncoveredSafe(n) = board.tile_state(pos) {
                Some((pos, n))
            } else {
                None
            }
        })
        .for_each(|(pos, n)| {
            let n = n - num_bombs_around(board, pos);
            let covered = covered_neighbours(board, pos);
            let num_covered = covered.len();
            if num_covered == 0 {
                return;
            }
            for subset in subsets(&covered, num_covered - 1) {
                // need so few bombs in subset that the rest must be bombs
                let max = max_in_subset(
                    &subset.iter().cloned().cloned().collect(),
                    &mut max_bombs,
                );
                let rest_size = (num_covered - subset.len()) as u8;
                if max + rest_size == n {
                    covered
                        .iter()
                        .filter(|x| !subset.contains(&x))
                        .map(|&pos| Action::flag(pos))
                        .for_each(|x| output.push(x));
                }

                // need at least n bombs in the subset, then rest are safe
                let min = min_in_subset(
                    &subset.iter().cloned().cloned().collect(),
                    &mut min_bombs,
                );
                if min == n {
                    covered
                        .iter()
                        .filter(|x| !subset.contains(&x))
                        .map(|&pos| Action::uncover(pos))
                        .for_each(|x| output.push(x));
                }
            }
        });
    deduplicate(output)
}
