use std::{
    ops::ControlFlow,
    sync::{Arc, Mutex},
};

use anyhow::bail;
use matrix_sdk::ruma::{OwnedRoomId, RoomId};

pub type SpaceReference = Arc<Space>;

/// Graph of spaces, referencing the [[Account.rooms]] collection.
///
/// Invariant: The graph of spaces forms a DAG.
#[derive(Debug)]
pub struct Space {
    room_id: OwnedRoomId,
    contained_rooms: Mutex<Vec<OwnedRoomId>>,
    children: Mutex<Vec<SpaceReference>>,
}

impl Space {
    pub fn new(room_id: OwnedRoomId) -> SpaceReference {
        Arc::new(Self {
            room_id,
            contained_rooms: Mutex::new(Vec::new()),
            children: Mutex::new(Vec::new()),
        })
    }

    /// Get the unique matrix room ID of this space.
    pub fn room_id(&self) -> &RoomId {
        &self.room_id
    }

    /// Iterate over all direct children of this space.
    pub fn children(&self) -> Vec<SpaceReference> {
        self.children.lock().unwrap().clone()
    }

    pub fn insert_room(&self, room: &RoomId) {
        self.contained_rooms.lock().unwrap().push(room.to_owned());
    }

    /// Traverse the subgraph reachable from this space.
    ///
    /// Spaces that are reachable via multiple parent spaces are visited multiple times.
    pub fn traverse<Action: FnMut(&Space) -> ControlFlow<()>>(&self, mut action: Action) {
        let mut stack = self.children();

        match action(self) {
            ControlFlow::Continue(_) => {}
            ControlFlow::Break(_) => return,
        }

        while !stack.is_empty() {
            // unwrap is safe, as we exit the loop if the stack is empty
            let current = stack.pop().unwrap();

            for c in current.children() {
                stack.push(c);
            }
            match action(&current) {
                ControlFlow::Continue(_) => {}
                ControlFlow::Break(_) => return,
            }
        }
    }

    pub fn add_child(&self, child: SpaceReference) -> anyhow::Result<()> {
        let mut can_reach_parent = false;
        child.traverse(|current| {
            if std::ptr::eq(current, self) {
                can_reach_parent = true;
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        if can_reach_parent {
            bail!("Adding child would result in a cycle!");
        } else {
            self.children.lock().unwrap().push(child);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_vec_equal(actual: &[SpaceReference], expected: &[SpaceReference]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!(Arc::ptr_eq(a, e));
        }
    }

    #[test]
    fn test_add_child() {
        let parent = Space::new("!parent:example.org".parse().unwrap());
        let child = Space::new("!child:example.org".parse().unwrap());
        assert!(parent.children().is_empty());

        // adding a child works
        assert!(parent.add_child(child.clone()).is_ok());
        assert!(child.children().is_empty());
        assert_vec_equal(&parent.children(), &[child.clone()]);

        // creating a cycle doesn't
        assert!(child.add_child(parent.clone()).is_err());

        let grandchild = Space::new("!grandchild:example.org".parse().unwrap());

        // adding a grandchild works
        assert!(child.add_child(grandchild.clone()).is_ok());

        // creating a cycle doesn't
        assert!(grandchild.add_child(parent).is_err());
    }

    #[test]
    fn test_self_add_child_fails() {
        let node = Space::new("!parent:example.org".parse().unwrap());

        assert!(node.add_child(node.clone()).is_err());
    }
}
