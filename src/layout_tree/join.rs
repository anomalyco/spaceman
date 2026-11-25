//! The Join Lab's tree edit (`spaceman ctl join DIR`): nest a tile with its neighbor in a split
//! across the direction, the way AeroSpace's `join-with` does.

use crate::{Axis, Direction, SlotId};

use super::{Child, LayoutTree, Node, average_weight};

impl LayoutTree {
    /// Nests `moved` with `target`, the neighbor `direction` points at, in a split across that
    /// direction: in `| A | B | C |`, C joining left gives `| A | B/C |`.
    ///
    /// A target already in a split across the direction that `moved` was not part of (a
    /// stack) takes `moved` at that split's end, so repeated joins build the stack in order:
    /// `| A | B/C | D |`, D joining left, gives `| A | B/C/D |`. Otherwise the target and
    /// `moved` become a new two-tile split in the target's place (`moved` first when it joins
    /// down or right, the side it came from), even when removing `moved` collapsed its own
    /// split, so the pair keeps the target's share: in `| A | B/C |`, C joining up gives
    /// `| A | B C |` with A still half the width. `moved`'s old share goes to its former
    /// siblings. Joined splits count as user-arranged, so Dwindle never re-orients them.
    pub(crate) fn join(&mut self, moved: SlotId, target: SlotId, direction: Direction) -> bool {
        if moved == target || !self.root.contains(target) {
            return false;
        }
        let axis = match direction {
            Direction::Left | Direction::Right => Axis::Vertical,
            Direction::Down | Direction::Up => Axis::Horizontal,
        };
        let append = parent_of(&self.root, target).is_some_and(|parent| {
            matches!(parent, Node::Split { axis: parent_axis, .. } if *parent_axis == axis)
                && !parent.contains(moved)
        });
        let Some((mut remaining, extracted)) = self.take_slot(moved) else {
            return false;
        };
        // Joining down or right, `moved` sits before the target, so it stays first in the pair.
        let first = matches!(direction, Direction::Down | Direction::Right);
        let how = Join {
            axis,
            append,
            first,
        };
        if !join_at(&mut remaining, target, extracted, how) {
            return false;
        }
        self.root = remaining;
        self.unwrap_single_split_root();
        true
    }
}

/// The split that holds `target` as a direct leaf.
fn parent_of(node: &Node, target: SlotId) -> Option<&Node> {
    let Node::Split { children, .. } = node else {
        return None;
    };
    if children
        .iter()
        .any(|child| matches!(child.node, Node::Leaf(existing) if existing == target))
    {
        return Some(node);
    }
    children.iter().find_map(|child| parent_of(&child.node, target))
}

/// How `join_at` places the moved tile.
#[derive(Clone, Copy)]
struct Join {
    /// The axis of the split the pair forms.
    axis: Axis,
    /// The target's split already ran along `axis` without the moved tile: append to it.
    append: bool,
    /// A new pair puts the moved tile before the target.
    first: bool,
}

fn join_at(node: &mut Node, target: SlotId, moved: Node, how: Join) -> bool {
    match node {
        Node::Leaf(existing) if *existing == target => {
            let target = std::mem::replace(node, Node::Leaf(target));
            let (one, two) = if how.first {
                (moved, target)
            } else {
                (target, moved)
            };
            *node = Node::Split {
                axis: how.axis,
                auto_axis: false,
                children: vec![
                    Child {
                        weight: 1,
                        node: one,
                    },
                    Child {
                        weight: 1,
                        node: two,
                    },
                ],
            };
            true
        }
        Node::Leaf(_) => false,
        Node::Split {
            axis: parent_axis,
            auto_axis,
            children,
        } => {
            let Some(index) = children
                .iter()
                .position(|child| child.node.contains(target))
            else {
                return false;
            };
            // Only a stack that already ran along the join axis before `moved` left takes it as
            // a sibling; anything else wraps the target leaf in a new pair below.
            if how.append
                && *parent_axis == how.axis
                && matches!(children[index].node, Node::Leaf(existing) if existing == target)
            {
                // Resize stores pixel-scale weights, so the joined tile takes the average.
                let weight = average_weight(children);
                children.push(Child {
                    weight,
                    node: moved,
                });
                *auto_axis = false;
                true
            } else {
                join_at(&mut children[index].node, target, moved, how)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Axis, Direction, Rect, SlotId};

    use super::LayoutTree;

    const OUTPUT: Rect = Rect {
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
    };

    fn frames(tree: &LayoutTree) -> Vec<(u64, (i32, i32, i32, i32))> {
        tree.placements(OUTPUT)
            .into_iter()
            .map(|(slot, frame)| (slot.0, (frame.x, frame.y, frame.width, frame.height)))
            .collect()
    }

    #[test]
    fn joining_left_stacks_the_tile_under_its_neighbor() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);

        assert!(tree.join(SlotId(3), SlotId(2), Direction::Left));

        // | A | B/C |
        assert_eq!(
            frames(&tree),
            vec![
                (1, (0, 0, 600, 800)),
                (2, (600, 0, 600, 400)),
                (3, (600, 400, 600, 400)),
            ]
        );
    }

    #[test]
    fn joining_onto_a_stack_adds_at_its_end() {
        let mut tree = LayoutTree::flat(
            Axis::Horizontal,
            [SlotId(1), SlotId(2), SlotId(3), SlotId(4)],
        );
        assert!(tree.join(SlotId(3), SlotId(2), Direction::Left));
        // | A | B/C | D |, D joins left onto B: the stack grows at its end, B/C/D.
        assert!(tree.join(SlotId(4), SlotId(2), Direction::Left));

        assert_eq!(tree.slots(), [SlotId(1), SlotId(2), SlotId(3), SlotId(4)]);
        let placed = frames(&tree);
        assert_eq!(placed[0].1, (0, 0, 600, 800));
        for (_, (x, _, width, _)) in &placed[1..] {
            assert_eq!((*x, *width), (600, 600), "{placed:?}");
        }
    }

    #[test]
    fn joining_across_a_collapsed_stack_keeps_the_pair_in_the_targets_share() {
        // | A | B/C |: C joining up onto B, or B joining down onto C, must not turn the
        // layout into three columns; the pair stays in the right half, in order.
        for (moved, target, direction) in [(3, 2, Direction::Up), (2, 3, Direction::Down)] {
            let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2), SlotId(3)]);
            assert!(tree.join(SlotId(3), SlotId(2), Direction::Left));

            assert!(tree.join(SlotId(moved), SlotId(target), direction));

            assert_eq!(
                frames(&tree),
                vec![
                    (1, (0, 0, 600, 800)),
                    (2, (600, 0, 300, 800)),
                    (3, (900, 0, 300, 800)),
                ],
                "{moved} join {direction:?} onto {target}"
            );
        }
    }

    #[test]
    fn joining_up_puts_the_pair_side_by_side() {
        let mut tree = LayoutTree::flat(Axis::Vertical, [SlotId(1), SlotId(2), SlotId(3)]);

        assert!(tree.join(SlotId(3), SlotId(2), Direction::Up));

        let placed = frames(&tree);
        assert_eq!(placed[0].1, (0, 0, 1200, 400));
        assert_eq!(placed[1].1, (0, 400, 600, 400));
        assert_eq!(placed[2].1, (600, 400, 600, 400));
    }

    #[test]
    fn join_refuses_itself_and_unknown_tiles_and_survives_a_round_trip() {
        let mut tree = LayoutTree::flat(Axis::Horizontal, [SlotId(1), SlotId(2)]);
        assert!(!tree.join(SlotId(1), SlotId(1), Direction::Left));
        assert!(!tree.join(SlotId(1), SlotId(9), Direction::Left));
        assert!(!tree.join(SlotId(9), SlotId(1), Direction::Left));

        assert!(tree.join(SlotId(2), SlotId(1), Direction::Left));
        assert_eq!(
            frames(&tree),
            vec![(1, (0, 0, 1200, 400)), (2, (0, 400, 1200, 400))]
        );
        let restored: LayoutTree =
            serde_json::from_slice(&serde_json::to_vec(&tree).unwrap()).unwrap();
        assert_eq!(restored.placements(OUTPUT), tree.placements(OUTPUT));
    }
}
