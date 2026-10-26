use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{Axis, Direction, Rect, ResizeDirection, SlotId};

const MIN_TILE_EXTENT: i32 = 50;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct LayoutTree {
    root: Node,
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::flat(Axis::Horizontal, [])
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
enum Node {
    Leaf(SlotId),
    Split { axis: Axis, children: Vec<Child> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Child {
    weight: u32,
    node: Node,
}

impl LayoutTree {
    pub(crate) fn flat(axis: Axis, slots: impl IntoIterator<Item = SlotId>) -> Self {
        Self {
            root: Node::Split {
                axis,
                children: slots
                    .into_iter()
                    .map(|slot| Child {
                        weight: 1,
                        node: Node::Leaf(slot),
                    })
                    .collect(),
            },
        }
    }

    pub(crate) fn weighted_flat<'a>(
        axis: Axis,
        slots: impl IntoIterator<Item = &'a crate::Slot>,
    ) -> Self {
        Self {
            root: Node::Split {
                axis,
                children: slots
                    .into_iter()
                    .map(|slot| Child {
                        weight: slot.weight,
                        node: Node::Leaf(slot.id),
                    })
                    .collect(),
            },
        }
    }

    pub(crate) const fn root_axis(&self) -> Axis {
        match &self.root {
            Node::Split { axis, .. } => *axis,
            Node::Leaf(_) => Axis::Horizontal,
        }
    }

    pub(crate) fn sync(&mut self, fallback_axis: Axis, slots: &[crate::Slot]) {
        let retained = slots.iter().map(|slot| slot.id).collect::<HashSet<_>>();
        let existing = self.slots().into_iter().collect::<HashSet<_>>();
        if existing == retained && existing.len() == slots.len() {
            return;
        }
        let root = std::mem::replace(
            &mut self.root,
            Node::Split {
                axis: fallback_axis,
                children: Vec::new(),
            },
        );
        self.root = match root {
            Node::Split { axis, children } => Node::Split {
                axis,
                children: retain_children(children, &retained),
            },
            root => retain(root, &retained).unwrap_or(Node::Split {
                axis: fallback_axis,
                children: Vec::new(),
            }),
        };
        for slot in slots.iter().filter(|slot| !existing.contains(&slot.id)) {
            self.append(fallback_axis, slot.id, slot.weight);
        }
    }

    pub(crate) fn slots(&self) -> Vec<SlotId> {
        let mut slots = Vec::new();
        collect_slots(&self.root, &mut slots);
        slots
    }

    pub(crate) fn validate(&self, slots: &[crate::Slot]) -> Result<(), String> {
        let expected: HashSet<_> = slots.iter().map(|slot| slot.id).collect();
        if expected.len() != slots.len() {
            return Err("layout slots contain duplicate IDs".into());
        }
        let mut leaves = HashSet::new();
        validate_node(&self.root, true, &mut leaves)?;
        if leaves != expected {
            return Err("layout leaves do not exactly match workspace slots".into());
        }
        Ok(())
    }

    pub(crate) fn weight(&self, slot: SlotId) -> Option<u32> {
        leaf_weight(&self.root, slot)
    }

    pub(crate) fn placements(&self, output: Rect) -> Vec<(SlotId, Rect)> {
        let mut placements = Vec::new();
        place(&self.root, output, &mut placements);
        placements
    }

    pub(crate) fn swap(&mut self, first: SlotId, second: SlotId) -> bool {
        if first == second || !self.root.contains(first) || !self.root.contains(second) {
            return false;
        }
        swap_slots(&mut self.root, first, second);
        true
    }

    pub(crate) fn create_implicit_split(&mut self, slot: SlotId, direction: Direction) -> bool {
        let Some((remaining, moved)) = self.take_slot(slot) else {
            return false;
        };
        self.root = split_pair(direction, moved, remaining);
        true
    }

    pub(crate) fn move_relative(
        &mut self,
        moved: SlotId,
        target: SlotId,
        direction: Direction,
    ) -> bool {
        if moved == target || !self.root.contains(target) {
            return false;
        }
        let Some((mut remaining, extracted)) = self.take_slot(moved) else {
            return false;
        };
        if !insert_relative(&mut remaining, target, extracted, direction) {
            return false;
        }
        self.root = remaining;
        true
    }

    pub(crate) fn toggle_root_axis(&mut self) {
        if let Node::Split { axis, .. } = &mut self.root {
            *axis = match *axis {
                Axis::Horizontal => Axis::Vertical,
                Axis::Vertical => Axis::Horizontal,
            };
        }
    }

    pub(crate) fn resize(&mut self, slot: SlotId, current: Rect, output: Rect) -> bool {
        let Some(expected) = self
            .placements(output)
            .into_iter()
            .find_map(|(candidate, frame)| (candidate == slot).then_some(frame))
        else {
            return false;
        };
        let Some(candidate) =
            resize_candidate(&self.root, slot, current, expected, output, Vec::new())
        else {
            return false;
        };
        if candidate.delta.abs() <= 2 {
            return false;
        }
        let Some((split_frame, axis, children)) = self.split_at_mut(output, &candidate.path) else {
            return false;
        };
        let frames = partition_children(split_frame, *axis, children);
        let before = candidate.boundary;
        let after = before + 1;
        let pair_start = leading(frames[before], *axis);
        let pair_end = trailing(frames[after], *axis);
        let before_minimum = minimum_extent(&children[before].node, *axis);
        let after_minimum = minimum_extent(&children[after].node, *axis);
        if pair_end - pair_start < before_minimum + after_minimum {
            return false;
        }
        let boundary = candidate
            .coordinate
            .clamp(pair_start + before_minimum, pair_end - after_minimum);
        set_pair_extents(
            children,
            &frames,
            *axis,
            (before, boundary - pair_start),
            (after, pair_end - boundary),
        );
        true
    }

    pub(crate) fn resize_by(
        &mut self,
        slot: SlotId,
        direction: ResizeDirection,
        amount: i32,
        output: Rect,
    ) -> bool {
        let Some((path, index)) = immediate_parent(&self.root, slot, Vec::new()) else {
            return false;
        };
        let Some((split_frame, axis, children)) = self.split_at_mut(output, &path) else {
            return false;
        };
        let neighbor = if index + 1 < children.len() {
            index + 1
        } else if index > 0 {
            index - 1
        } else {
            return false;
        };
        let frames = partition_children(split_frame, *axis, children);
        let pair_extent = extent(frames[index], *axis) + extent(frames[neighbor], *axis);
        let focused_minimum = minimum_extent(&children[index].node, *axis);
        let neighbor_minimum = minimum_extent(&children[neighbor].node, *axis);
        if pair_extent < focused_minimum + neighbor_minimum {
            return false;
        }
        let delta = match direction {
            ResizeDirection::Shrink => -amount,
            ResizeDirection::Grow => amount,
        };
        let focused_extent = (extent(frames[index], *axis) + delta)
            .clamp(focused_minimum, pair_extent - neighbor_minimum);
        if focused_extent == extent(frames[index], *axis) {
            return false;
        }
        set_pair_extents(
            children,
            &frames,
            *axis,
            (index, focused_extent),
            (neighbor, pair_extent - focused_extent),
        );
        true
    }

    pub(crate) fn balance(&mut self, slot: SlotId, output: Rect) -> bool {
        let Some((path, _)) = immediate_parent(&self.root, slot, Vec::new()) else {
            return false;
        };
        let Some((split_frame, axis, children)) = self.split_at_mut(output, &path) else {
            return false;
        };
        let total = extent(split_frame, *axis);
        let minimums = children
            .iter()
            .map(|child| minimum_extent(&child.node, *axis))
            .collect::<Vec<_>>();
        if minimums.iter().sum::<i32>() > total {
            return false;
        }
        let mut low = 0;
        let mut high = total;
        while low < high {
            let middle = low + (high - low + 1) / 2;
            if minimums
                .iter()
                .map(|minimum| (*minimum).max(middle))
                .sum::<i32>()
                <= total
            {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let mut extents = minimums
            .into_iter()
            .map(|minimum| minimum.max(low))
            .collect::<Vec<_>>();
        let mut remainder = total - extents.iter().sum::<i32>();
        for extent in extents.iter_mut().filter(|extent| **extent == low) {
            if remainder == 0 {
                break;
            }
            *extent += 1;
            remainder -= 1;
        }
        let changed = partition_children(split_frame, *axis, children)
            .iter()
            .zip(&extents)
            .any(|(frame, target)| extent(*frame, *axis) != *target);
        if !changed {
            return false;
        }
        for (child, extent) in children.iter_mut().zip(extents) {
            child.weight = pixel_weight(extent);
        }
        true
    }

    /// Give every tile at least its window's known minimum `(width, height)`,
    /// taking the space from siblings in proportion to what each can spare.
    /// Splits whose minimums cannot all fit are left unchanged.
    pub(crate) fn fit_minimums(
        &mut self,
        output: Rect,
        minimums: &HashMap<SlotId, (i32, i32)>,
    ) -> bool {
        fit_node(&mut self.root, output, minimums)
    }

    fn take_slot(&self, slot: SlotId) -> Option<(Node, Node)> {
        match extract(self.root.clone(), slot) {
            (Some(remaining), Some(moved)) => Some((remaining, moved)),
            _ => None,
        }
    }

    fn split_at_mut(
        &mut self,
        output: Rect,
        path: &[usize],
    ) -> Option<(Rect, &mut Axis, &mut Vec<Child>)> {
        let frame = output_at(&self.root, output, path);
        let Node::Split { axis, children } = node_at_mut(&mut self.root, path)? else {
            return None;
        };
        Some((frame, axis, children))
    }

    fn append(&mut self, axis: Axis, slot: SlotId, weight: u32) {
        match &mut self.root {
            Node::Split { children, .. } => {
                let weight = if children.is_empty() {
                    weight.max(1)
                } else {
                    average_weight(children)
                };
                children.push(Child {
                    weight,
                    node: Node::Leaf(slot),
                });
            }
            root => {
                let previous = std::mem::replace(root, Node::Leaf(slot));
                *root = Node::Split {
                    axis,
                    children: vec![
                        Child {
                            weight: 1,
                            node: previous,
                        },
                        Child {
                            weight,
                            node: Node::Leaf(slot),
                        },
                    ],
                };
            }
        }
    }
}

fn validate_node(node: &Node, root: bool, leaves: &mut HashSet<SlotId>) -> Result<(), String> {
    match node {
        Node::Leaf(slot) => {
            if !leaves.insert(*slot) {
                return Err(format!("layout contains duplicate leaf {}", slot.0));
            }
        }
        Node::Split { children, .. } => {
            if !root && children.is_empty() {
                return Err("layout contains an empty nested split".into());
            }
            for child in children {
                if child.weight == 0 {
                    return Err("layout contains a zero-weight child".into());
                }
                validate_node(&child.node, false, leaves)?;
            }
        }
    }
    Ok(())
}

const fn direction_axis(direction: Direction) -> Axis {
    match direction {
        Direction::Left | Direction::Right => Axis::Horizontal,
        Direction::Down | Direction::Up => Axis::Vertical,
    }
}

fn split_pair(direction: Direction, moved: Node, anchor: Node) -> Node {
    let moved = Child {
        weight: 1,
        node: moved,
    };
    let anchor = Child {
        weight: 1,
        node: anchor,
    };
    Node::Split {
        axis: direction_axis(direction),
        children: if matches!(direction, Direction::Left | Direction::Up) {
            vec![moved, anchor]
        } else {
            vec![anchor, moved]
        },
    }
}

fn retain(node: Node, retained: &HashSet<SlotId>) -> Option<Node> {
    match node {
        Node::Leaf(slot) => retained.contains(&slot).then_some(Node::Leaf(slot)),
        Node::Split { axis, children } => collapse(axis, retain_children(children, retained)),
    }
}

fn retain_children(children: Vec<Child>, retained: &HashSet<SlotId>) -> Vec<Child> {
    children
        .into_iter()
        .filter_map(|child| {
            retain(child.node, retained).map(|node| Child {
                weight: child.weight,
                node,
            })
        })
        .collect()
}

fn collapse(axis: Axis, mut children: Vec<Child>) -> Option<Node> {
    match children.len() {
        0 => None,
        1 => Some(children.remove(0).node),
        _ => Some(Node::Split { axis, children }),
    }
}

fn extract(node: Node, slot: SlotId) -> (Option<Node>, Option<Node>) {
    match node {
        Node::Leaf(existing) if existing == slot => (None, Some(Node::Leaf(existing))),
        Node::Leaf(existing) => (Some(Node::Leaf(existing)), None),
        Node::Split { axis, mut children } => {
            for index in 0..children.len() {
                let child = children.remove(index);
                let (remaining, moved) = extract(child.node, slot);
                if let Some(moved) = moved {
                    if let Some(remaining) = remaining {
                        children.insert(
                            index,
                            Child {
                                weight: child.weight,
                                node: remaining,
                            },
                        );
                    }
                    return (collapse(axis, children), Some(moved));
                }
                children.insert(
                    index,
                    Child {
                        weight: child.weight,
                        node: remaining.expect("an unmatched subtree remains intact"),
                    },
                );
            }
            (Some(Node::Split { axis, children }), None)
        }
    }
}

fn insert_relative(node: &mut Node, target: SlotId, moved: Node, direction: Direction) -> bool {
    let axis = direction_axis(direction);
    let before = matches!(direction, Direction::Left | Direction::Up);
    match node {
        Node::Leaf(existing) if *existing == target => {
            let target = std::mem::replace(node, Node::Leaf(target));
            *node = split_pair(direction, moved, target);
            true
        }
        Node::Leaf(_) => false,
        Node::Split {
            axis: parent_axis,
            children,
        } => {
            let Some(index) = children
                .iter()
                .position(|child| child.node.contains(target))
            else {
                return false;
            };
            if *parent_axis == axis
                && matches!(children[index].node, Node::Leaf(existing) if existing == target)
            {
                // Resize stores pixel-scale weights. A literal 1 here would
                // collapse the inserted tile beside those larger siblings.
                let weight = average_weight(children);
                children.insert(
                    index + usize::from(!before),
                    Child {
                        weight,
                        node: moved,
                    },
                );
                true
            } else {
                insert_relative(&mut children[index].node, target, moved, direction)
            }
        }
    }
}

fn average_weight(children: &[Child]) -> u32 {
    let total = children
        .iter()
        .map(|child| u64::from(child.weight))
        .sum::<u64>();
    u32::try_from(total / children.len().max(1) as u64)
        .unwrap_or(u32::MAX)
        .max(1)
}

fn collect_slots(node: &Node, slots: &mut Vec<SlotId>) {
    match node {
        Node::Leaf(slot) => slots.push(*slot),
        Node::Split { children, .. } => {
            for child in children {
                collect_slots(&child.node, slots);
            }
        }
    }
}

fn leaf_weight(node: &Node, slot: SlotId) -> Option<u32> {
    let Node::Split { children, .. } = node else {
        return None;
    };
    for child in children {
        if matches!(child.node, Node::Leaf(existing) if existing == slot) {
            return Some(child.weight);
        }
        if let Some(weight) = leaf_weight(&child.node, slot) {
            return Some(weight);
        }
    }
    None
}

fn place(node: &Node, frame: Rect, placements: &mut Vec<(SlotId, Rect)>) {
    match node {
        Node::Leaf(slot) => placements.push((*slot, frame)),
        Node::Split { axis, children } => {
            for (child, child_frame) in children
                .iter()
                .zip(partition_children(frame, *axis, children))
            {
                place(&child.node, child_frame, placements);
            }
        }
    }
}

#[derive(Clone)]
struct ResizeCandidate {
    path: Vec<usize>,
    boundary: usize,
    coordinate: i32,
    delta: i32,
}

fn resize_candidate(
    node: &Node,
    slot: SlotId,
    current: Rect,
    expected: Rect,
    frame: Rect,
    path: Vec<usize>,
) -> Option<ResizeCandidate> {
    let Node::Split { axis, children } = node else {
        return None;
    };
    let frames = partition_children(frame, *axis, children);
    let index = children
        .iter()
        .position(|child| child.node.contains(slot))?;
    let child_frame = frames[index];
    let mut candidates = Vec::new();
    if index > 0 && leading(expected, *axis) == leading(child_frame, *axis) {
        let coordinate = leading(current, *axis);
        candidates.push(ResizeCandidate {
            path: path.clone(),
            boundary: index - 1,
            coordinate,
            delta: coordinate - leading(child_frame, *axis),
        });
    }
    if index + 1 < children.len() && trailing(expected, *axis) == trailing(child_frame, *axis) {
        let coordinate = trailing(current, *axis);
        candidates.push(ResizeCandidate {
            path: path.clone(),
            boundary: index,
            coordinate,
            delta: coordinate - trailing(child_frame, *axis),
        });
    }
    let mut child_path = path;
    child_path.push(index);
    if let Some(nested) = resize_candidate(
        &children[index].node,
        slot,
        current,
        expected,
        child_frame,
        child_path,
    ) {
        candidates.push(nested);
    }
    candidates
        .into_iter()
        .max_by_key(|candidate| candidate.delta.abs())
}

fn partition_children(frame: Rect, axis: Axis, children: &[Child]) -> Vec<Rect> {
    let total = children
        .iter()
        .map(|child| u64::from(child.weight))
        .sum::<u64>();
    if total == 0 {
        return Vec::new();
    }
    let length = extent(frame, axis);
    let mut cursor = 0;
    let mut cumulative = 0_u64;
    children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            cumulative += u64::from(child.weight);
            let end = if index + 1 == children.len() {
                length
            } else {
                (i64::from(length) * cumulative as i64 / total as i64) as i32
            };
            let result = match axis {
                Axis::Horizontal => Rect {
                    x: frame.x + cursor,
                    width: end - cursor,
                    ..frame
                },
                Axis::Vertical => Rect {
                    y: frame.y + cursor,
                    height: end - cursor,
                    ..frame
                },
            };
            cursor = end;
            result
        })
        .collect()
}

fn pixel_weight(extent: i32) -> u32 {
    u32::try_from(extent).unwrap_or(1).max(1)
}

fn set_pair_extents(
    children: &mut [Child],
    frames: &[Rect],
    axis: Axis,
    first: (usize, i32),
    second: (usize, i32),
) {
    for (child, frame) in children.iter_mut().zip(frames) {
        child.weight = pixel_weight(extent(*frame, axis));
    }
    children[first.0].weight = pixel_weight(first.1);
    children[second.0].weight = pixel_weight(second.1);
}

fn output_at(root: &Node, output: Rect, path: &[usize]) -> Rect {
    let mut node = root;
    let mut frame = output;
    for &index in path {
        let Node::Split { axis, children } = node else {
            break;
        };
        frame = partition_children(frame, *axis, children)[index];
        node = &children[index].node;
    }
    frame
}

fn node_at_mut<'a>(node: &'a mut Node, path: &[usize]) -> Option<&'a mut Node> {
    let mut node = node;
    for &index in path {
        let Node::Split { children, .. } = node else {
            return None;
        };
        node = &mut children.get_mut(index)?.node;
    }
    Some(node)
}

fn immediate_parent(node: &Node, slot: SlotId, path: Vec<usize>) -> Option<(Vec<usize>, usize)> {
    let Node::Split { children, .. } = node else {
        return None;
    };
    let index = children
        .iter()
        .position(|child| child.node.contains(slot))?;
    if matches!(children[index].node, Node::Leaf(existing) if existing == slot) {
        return Some((path, index));
    }
    let mut child_path = path;
    child_path.push(index);
    immediate_parent(&children[index].node, slot, child_path)
}

fn minimum_extent(node: &Node, axis: Axis) -> i32 {
    match node {
        Node::Leaf(_) => MIN_TILE_EXTENT,
        Node::Split {
            axis: split_axis,
            children,
        } if *split_axis == axis => children
            .iter()
            .map(|child| minimum_extent(&child.node, axis))
            .sum(),
        Node::Split { children, .. } => children
            .iter()
            .map(|child| minimum_extent(&child.node, axis))
            .max()
            .unwrap_or(0),
    }
}

/// Shortfalls this small are readback noise, not a refused size.
const MINIMUM_TOLERANCE: i32 = 2;

fn fit_node(node: &mut Node, frame: Rect, minimums: &HashMap<SlotId, (i32, i32)>) -> bool {
    let Node::Split { axis, children } = node else {
        return false;
    };
    let extents = partition_children(frame, *axis, children)
        .iter()
        .map(|child| extent(*child, *axis))
        .collect::<Vec<_>>();
    let required = children
        .iter()
        .map(|child| required_extent(&child.node, *axis, minimums))
        .collect::<Vec<_>>();
    let mut changed = false;
    if let Some(fitted) = redistribute(&extents, &required) {
        for (child, extent) in children.iter_mut().zip(fitted) {
            child.weight = pixel_weight(extent);
        }
        changed = true;
    }
    let frames = partition_children(frame, *axis, children);
    for (child, child_frame) in children.iter_mut().zip(frames) {
        changed |= fit_node(&mut child.node, child_frame, minimums);
    }
    changed
}

fn required_extent(node: &Node, axis: Axis, minimums: &HashMap<SlotId, (i32, i32)>) -> i32 {
    match node {
        Node::Leaf(slot) => {
            let known = minimums.get(slot).map_or(0, |&(width, height)| match axis {
                Axis::Horizontal => width,
                Axis::Vertical => height,
            });
            known.max(MIN_TILE_EXTENT)
        }
        Node::Split {
            axis: split_axis,
            children,
        } if *split_axis == axis => children
            .iter()
            .map(|child| required_extent(&child.node, axis, minimums))
            .sum(),
        Node::Split { children, .. } => children
            .iter()
            .map(|child| required_extent(&child.node, axis, minimums))
            .max()
            .unwrap_or(0),
    }
}

/// Raise each extent short of its requirement and take the difference from
/// the others in proportion to their slack. `None` when nothing is short or
/// the requirements do not fit.
fn redistribute(extents: &[i32], required: &[i32]) -> Option<Vec<i32>> {
    if !extents
        .iter()
        .zip(required)
        .any(|(extent, required)| extent + MINIMUM_TOLERANCE < *required)
        || required.iter().sum::<i32>() > extents.iter().sum::<i32>()
    {
        return None;
    }
    let mut fitted = extents
        .iter()
        .zip(required)
        .map(|(extent, required)| (*extent).max(*required))
        .collect::<Vec<_>>();
    let mut deficit = fitted.iter().sum::<i32>() - extents.iter().sum::<i32>();
    let slack = fitted
        .iter()
        .zip(required)
        .map(|(extent, required)| extent - required)
        .collect::<Vec<_>>();
    let total_slack = i64::from(slack.iter().sum::<i32>());
    let mut taken = 0;
    for (extent, slack) in fitted.iter_mut().zip(&slack) {
        let share = (i64::from(deficit) * i64::from(*slack) / total_slack) as i32;
        *extent -= share;
        taken += share;
    }
    deficit -= taken;
    for (extent, required) in fitted.iter_mut().zip(required) {
        if deficit == 0 {
            break;
        }
        if *extent > *required {
            *extent -= 1;
            deficit -= 1;
        }
    }
    Some(fitted)
}

const fn extent(frame: Rect, axis: Axis) -> i32 {
    match axis {
        Axis::Horizontal => frame.width,
        Axis::Vertical => frame.height,
    }
}

const fn leading(frame: Rect, axis: Axis) -> i32 {
    match axis {
        Axis::Horizontal => frame.x,
        Axis::Vertical => frame.y,
    }
}

const fn trailing(frame: Rect, axis: Axis) -> i32 {
    leading(frame, axis) + extent(frame, axis)
}

impl Node {
    fn contains(&self, slot: SlotId) -> bool {
        match self {
            Self::Leaf(existing) => *existing == slot,
            Self::Split { children, .. } => children.iter().any(|child| child.node.contains(slot)),
        }
    }
}

fn swap_slots(node: &mut Node, first: SlotId, second: SlotId) {
    match node {
        Node::Leaf(slot) if *slot == first => *slot = second,
        Node::Leaf(slot) if *slot == second => *slot = first,
        Node::Leaf(_) => {}
        Node::Split { children, .. } => {
            for child in children {
                swap_slots(&mut child.node, first, second);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_insertion_after_resize_uses_the_destination_weight_scale() {
        let output = Rect {
            width: 1000,
            height: 800,
            ..Rect::default()
        };
        for direction in [Direction::Left, Direction::Right] {
            let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);
            assert!(tree.resize_by(SlotId(1), ResizeDirection::Grow, 50, output));
            assert!(tree.move_relative(SlotId(3), SlotId(2), direction));
            let placements = tree.placements(output);
            assert!(
                placements
                    .iter()
                    .all(|(_, frame)| frame.width >= MIN_TILE_EXTENT),
                "{placements:?}"
            );
            let restored: LayoutTree =
                serde_json::from_slice(&serde_json::to_vec(&tree).unwrap()).unwrap();
            assert_eq!(restored.placements(output), placements);
        }
    }

    #[test]
    fn implicit_split_preserves_the_remaining_layout() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);

        assert!(tree.create_implicit_split(SlotId(2), Direction::Up));

        assert_eq!(tree.slots(), vec![SlotId(2), SlotId(1), SlotId(3)]);
        assert_eq!(
            tree.placements(Rect {
                x: 0,
                y: 0,
                width: 1200,
                height: 800,
            }),
            vec![
                (
                    SlotId(2),
                    Rect {
                        x: 0,
                        y: 0,
                        width: 1200,
                        height: 400
                    }
                ),
                (
                    SlotId(1),
                    Rect {
                        x: 0,
                        y: 400,
                        width: 600,
                        height: 400
                    }
                ),
                (
                    SlotId(3),
                    Rect {
                        x: 600,
                        y: 400,
                        width: 600,
                        height: 400
                    }
                ),
            ]
        );
    }

    #[test]
    fn persisted_layout_validation_rejects_duplicate_leaves_and_zero_weights() {
        let slots = [crate::Slot {
            id: SlotId(1),
            weight: 1,
            binding: crate::BindingState::Bound(crate::PhysicalWindowId {
                app: crate::AppInstanceId {
                    pid: 1,
                    launched_at: 1,
                },
                window_id: 1,
            }),
        }];
        let duplicate: LayoutTree = serde_json::from_value(serde_json::json!({
            "root": {"Split": {"axis": "horizontal", "children": [
                {"weight": 1, "node": {"Leaf": 1}},
                {"weight": 1, "node": {"Leaf": 1}}
            ]}}
        }))
        .unwrap();
        let zero_weight: LayoutTree = serde_json::from_value(serde_json::json!({
            "root": {"Split": {"axis": "horizontal", "children": [
                {"weight": 0, "node": {"Leaf": 1}}
            ]}}
        }))
        .unwrap();
        let nested_empty: LayoutTree = serde_json::from_value(serde_json::json!({
            "root": {"Split": {"axis": "horizontal", "children": [
                {"weight": 1, "node": {"Leaf": 1}},
                {"weight": 1, "node": {"Split": {
                    "axis": "vertical",
                    "children": []
                }}}
            ]}}
        }))
        .unwrap();

        assert!(duplicate.validate(&slots).is_err());
        assert!(zero_weight.validate(&slots).is_err());
        assert_eq!(
            nested_empty.validate(&slots).unwrap_err(),
            "layout contains an empty nested split"
        );
        assert!(LayoutTree::default().validate(&[]).is_ok());
    }

    #[test]
    fn edge_insertion_creates_a_split_at_the_target() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);

        assert!(tree.move_relative(SlotId(1), SlotId(2), Direction::Up));

        assert_eq!(
            tree.placements(Rect {
                x: 0,
                y: 0,
                width: 1000,
                height: 800
            }),
            vec![
                (
                    SlotId(1),
                    Rect {
                        x: 0,
                        y: 0,
                        width: 500,
                        height: 400
                    }
                ),
                (
                    SlotId(2),
                    Rect {
                        x: 0,
                        y: 400,
                        width: 500,
                        height: 400
                    }
                ),
                (
                    SlotId(3),
                    Rect {
                        x: 500,
                        y: 0,
                        width: 500,
                        height: 800
                    }
                ),
            ]
        );
    }

    #[test]
    fn resize_updates_the_nearest_nested_boundary() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);
        assert!(tree.move_relative(SlotId(1), SlotId(2), Direction::Up));

        assert!(tree.resize(
            SlotId(1),
            Rect {
                x: 0,
                y: 0,
                width: 500,
                height: 500
            },
            Rect {
                x: 0,
                y: 0,
                width: 1000,
                height: 800
            },
        ));

        let placements = tree.placements(Rect {
            x: 0,
            y: 0,
            width: 1000,
            height: 800,
        });
        assert_eq!(placements[0].1.height, 500);
        assert_eq!(placements[1].1.y, 500);
        assert_eq!(
            placements[2].1,
            Rect {
                x: 500,
                y: 0,
                width: 500,
                height: 800
            }
        );
    }

    #[test]
    fn resize_ignores_splits_too_small_for_both_minimums() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2)]);

        assert!(!tree.resize(
            SlotId(1),
            Rect {
                width: 60,
                height: 100,
                ..Rect::default()
            },
            Rect {
                width: 80,
                height: 100,
                ..Rect::default()
            },
        ));
    }

    fn output() -> Rect {
        Rect {
            width: 1000,
            height: 800,
            ..Rect::default()
        }
    }

    #[test]
    fn fitting_minimums_reaches_into_nested_splits_and_leaves_impossible_splits_alone() {
        let mut layout = LayoutTree {
            root: Node::Split {
                axis: Axis::Horizontal,
                children: vec![
                    Child {
                        weight: 1,
                        node: Node::Leaf(SlotId(1)),
                    },
                    Child {
                        weight: 1,
                        node: Node::Split {
                            axis: Axis::Vertical,
                            children: vec![
                                Child {
                                    weight: 1,
                                    node: Node::Leaf(SlotId(2)),
                                },
                                Child {
                                    weight: 1,
                                    node: Node::Leaf(SlotId(3)),
                                },
                            ],
                        },
                    },
                ],
            },
        };
        let minimums = HashMap::from([(SlotId(3), (700, 500))]);

        assert!(layout.fit_minimums(output(), &minimums));
        let frames = layout.placements(output());
        assert_eq!(frames[0].1.width, 300);
        assert_eq!(frames[2].1.width, 700);
        assert_eq!(frames[2].1.height, 500);
        assert_eq!(frames[1].1.height, 300);
        assert!(!layout.fit_minimums(output(), &minimums));

        let before = layout.clone();
        let impossible = HashMap::from([(SlotId(1), (600, 0)), (SlotId(3), (700, 0))]);
        assert!(!layout.fit_minimums(output(), &impossible));
        assert_eq!(layout, before);
    }

    #[test]
    fn keyboard_resize_grows_and_shrinks_a_flat_tile_by_fifty_pixels() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2)]);

        assert!(tree.resize_by(SlotId(1), ResizeDirection::Grow, 50, output()));
        assert_eq!(tree.placements(output())[0].1.width, 550);
        assert!(tree.resize_by(SlotId(1), ResizeDirection::Shrink, 50, output()));
        assert_eq!(tree.placements(output())[0].1.width, 500);
    }

    #[test]
    fn keyboard_resize_uses_the_immediate_nested_parent() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);
        assert!(tree.move_relative(SlotId(1), SlotId(2), Direction::Up));

        assert!(tree.resize_by(SlotId(1), ResizeDirection::Grow, 50, output()));

        let placements = tree.placements(output());
        assert_eq!(placements[0].1.height, 450);
        assert_eq!(placements[1].1.height, 350);
        assert_eq!(placements[2].1.width, 500);
    }

    #[test]
    fn keyboard_resize_stops_at_recursive_tile_minimums() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);
        assert!(tree.create_implicit_split(SlotId(2), Direction::Right));

        for _ in 0..20 {
            tree.resize_by(SlotId(2), ResizeDirection::Grow, 50, output());
        }

        let placements = tree.placements(output());
        assert_eq!(placements[0].1.width, 50);
        assert_eq!(placements[1].1.width, 50);
        assert_eq!(placements[2].1.width, 900);
        assert!(!tree.resize_by(SlotId(2), ResizeDirection::Grow, 50, output()));
    }

    #[test]
    fn balance_equalizes_the_focused_tiles_immediate_parent() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);
        assert!(tree.move_relative(SlotId(1), SlotId(2), Direction::Up));
        assert!(tree.resize_by(SlotId(1), ResizeDirection::Grow, 50, output()));

        assert!(tree.balance(SlotId(1), output()));

        let placements = tree.placements(output());
        assert_eq!(placements[0].1.height, 400);
        assert_eq!(placements[1].1.height, 400);
        assert_eq!(placements[2].1.width, 500);
    }

    #[test]
    fn newly_discovered_slot_uses_existing_weight_scale() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1)]);
        if let Node::Split { children, .. } = &mut tree.root {
            children[0].weight = 1480;
        }
        let slots = [
            crate::Slot {
                id: SlotId(1),
                weight: 1480,
                binding: crate::BindingState::Unresolved {
                    previous: test_window(1),
                    candidates: Vec::new(),
                    since: 0,
                },
            },
            crate::Slot {
                id: SlotId(2),
                weight: 1,
                binding: crate::BindingState::Unresolved {
                    previous: test_window(2),
                    candidates: Vec::new(),
                    since: 0,
                },
            },
        ];

        tree.sync(Axis::Horizontal, &slots);

        assert_eq!(
            tree.placements(Rect {
                x: 0,
                y: 0,
                width: 1000,
                height: 800,
            }),
            vec![
                (
                    SlotId(1),
                    Rect {
                        x: 0,
                        y: 0,
                        width: 500,
                        height: 800,
                    },
                ),
                (
                    SlotId(2),
                    Rect {
                        x: 500,
                        y: 0,
                        width: 500,
                        height: 800,
                    },
                ),
            ]
        );
    }

    fn test_window(window_id: u32) -> crate::PhysicalWindowId {
        crate::PhysicalWindowId {
            app: crate::AppInstanceId {
                pid: 1,
                launched_at: 1,
            },
            window_id,
        }
    }
}
