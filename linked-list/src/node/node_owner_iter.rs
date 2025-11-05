use std::fmt::Debug;

use super::Node;

pub struct NodeOwnerIter<T: Debug + Clone> {
    pub current: Option<Node<T>>,
}

impl<T: Debug + Clone> Iterator for NodeOwnerIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        // move Node<T> which is hold insider current field inside Option
        // this value will be destroyed as we return from this method
        let Some(mut current) = self.current.take() else {
            return None;
        };

        // we can't move value from Node<T> as it implements the Drop trait
        let value = current.value.clone();
        // I check if there is one more node after the current one
        // if so - I move it and prevent it from being dropped along with "current"
        if let Some(next_node) = current.next.take() {
            // move Node<T> from the Box
            self.current = Some(*next_node);
        }

        Some(value)
    }
}
