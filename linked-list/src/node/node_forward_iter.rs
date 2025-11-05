use std::fmt::Debug;

use super::Node;

pub struct NodeForwardIter<'a, T: Debug + Clone> {
    pub current: Option<&'a Node<T>>,
}

impl<'a, T: Debug + Clone> Iterator for NodeForwardIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(current) = self.current else {
            return None;
        };

        let value = &current.value;

        self.current = current.next.as_deref();

        Some(value)
    }
}
